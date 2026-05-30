# Task 1.1 — Game clone/snapshot audit

Goal: determine what must be Arc-wrapped vs intentionally non-`Clone`, to size the
snapshot/restore work (design D3, Group 1).

## Current state

`Game` is `#[derive(Debug)]` only — **not `Clone`**, and there is no existing
snapshot mechanism. Backward `ReplayRunner::seek` rebuilds from scratch:
`snapshot_card_data()` clones all ~4085 `CardData`, then `build_game()` calls
`Game::new` (rebuilding every registry + token registry) and re-restores zones,
then re-walks forward. The expensive parts are the full `CardData` clone and the
registry rebuild — **not** the per-action re-walk.

## Field classification (`game.rs:206`–~`680`)

**Immutable shared state → Arc-wrap candidates** (built once at construction, never mutated):
- `card_data: Vec<CardData>` (251) — the big one (~4085 cards + tokens)
- `effect_registry: CardEffectRegistry` (258)
- `formula_extensions: FormulaExtensionRegistry` (260)
- `token_registry: TokenRegistry` (267)
- `alt_path_registry: HashMap<…, CompiledAltPath>` (254, dsl-yaml feature)

**Intentionally non-`Clone` trait objects → exclude from any snapshot, re-attach on restore:**
- `logger: Box<dyn GameLogger>` (371) — session owns the logger choice
- `reveal_source: Option<Box<dyn RevealSource>>` (671) — re-attach a fresh `RevealQueue` at a saved cursor (design D4)

**Mutable, plain-data (would clone fine):** `players` (mostly), `turn_*`, `memory*`,
counters, `rng: StdRng` (StdRng is Clone), `revealed_cards`, `events`, `event_seq`,
`mulligan_*`, `replacement_fired: HashSet`, the boolean/enum continuation markers.

## ★ Blocker: the mutable graph is pervasively closure-bearing

The design (D3) assumed the only obstacle was Arc-wrapping `card_data` (the
`DEBUG_MCP.md` v1.5 note). That diagnosis is **incomplete**. The live mutable
state holds boxed closures that cannot be cloned **or** serialized:

- `modifiers: ModifierRegistry` — `ModifierEntry` is **explicitly `Not Clone`**
  (`modifiers.rs:87`): `replacement_condition: Box<dyn Fn + Send + Sync>`.
  Conditional modifiers (DP buffs with conditions, granted-keyword conditions)
  are normal live state in essentially every real game.
- `pending_selection: Option<PendingSelection>` — carries a `SelectionCallback`
  = `Box<dyn FnOnce(&mut Game, u16) …>`. Whenever a recorded step leaves the
  engine mid-selection (very common), a live closure is parked here.
- Numerous parked continuations park `Box<dyn FnOnce>` closures:
  `pending_pay_cost_effect`/`_stack`, `parked_replacement`, `dsl_outer_tail`,
  the would-play/link/digivolve resumes, etc. (see `effect_context/selections.rs`,
  `game_actions.rs:2137`, `replacement.rs`).
- `granted_effect_bodies` uses `Arc<dyn Fn>` bodies — those *are* shareable via
  Arc, so that slot alone is fine.

**Conclusion:** a full-state checkpoint *snapshot* of `Game` (clone or serde) is
**not implementable** without an engine-wide refactor that makes every parked
continuation closure-free (id-keyed bodies, the way `granted_effect_bodies`
already works) and serde-serializable. That is a multi-week effort far beyond
this change's scope. Arc-wrapping the immutable data does **not** unblock it.

## What IS achievable for "snappy back-step"

Arc-wrapping the immutable shared state makes **reset-and-replay** cheap:
backward seek = reset to the Arc-shared reconstructed initial state (no CardData
clone, no registry rebuild) + replay forward to the target via the existing cheap
`decode_action` path. This removes the *actual* expensive part of today's
backward seek. Cost is O(target) cheap re-walk (tens of ms for realistic
~tens-of-steps games) instead of O(1) restore — but with no closure problem
because live state is never cloned.

All user-facing capabilities survive on this mechanism:
- `step_back` / `seek(n)` → reset + replay to n
- `restore_checkpoint(n)` → reset + replay to n
- counterfactual A/B (gap F) → reset+replay to n, then submit a different action;
  reset+replay again for the recorded line (deterministic, so reproducible)
- opaque games → reset + fresh `RevealQueue` at cursor + replay (reveals consumed in order)

Only the *mechanism* changes (deterministic reset+replay instead of stored
state snapshots); the behavior contract is unchanged.

## Recommendation

Adopt **Arc + reset-and-replay** for this change; drop the "full-state checkpoint
ring" from design D3 and the recording-replay "Snapshot and Restore via Checkpoint
Ring" requirement, replacing them with a reset-and-replay contract. File true
mid-game state snapshotting (the serializability refactor) as a separate future
change if reset+replay proves too slow for very long games.
