## Context

The match log is produced by a three-stage pipeline:

```
engine GameEvent  →  adapter (event_to_dto desktop / event_to_pydict browser)  →  gameLogFormat.ts
```

Today the engine `GameEvent` variants carry only minimal identity:

- `Attack { attacker_field_index, target_field_index, target_player }` — **no card id at all**.
- `Play`/`Digivolve`/`Trash`/`Mill`/`SecurityReveal` — carry `card_id` but **no `card_name`**.
- `MemoryChange { player, delta, total }` — **no source**.

`gameLogFormat.ts` therefore reconstructs names from **live board state** (`battleArea[slot].topCardName`). That only works while the card is still in that slot, so by the time a game-length log is read the board has moved on: attacks fall back to `slot N`, and trash/security/scrolled-off cards fall back to a bare id. Investigation confirmed the names are present in `cards.json` (e.g. `BT25-061` = Offmon) — they are discarded by the pipeline, not missing from data.

Constraints:
- `GameEvent` is `#[non_exhaustive]`; existing consumers already use a default-skip match arm, so new variants/fields are additive and non-breaking.
- The no-approximations rule (CLAUDE.md rule 17) routes effect target choices through `pending_selection`, whose `PendingSelection` already carries `source_card` / `source_permanent` — so target attribution is available at selection-commit time.
- The two adapter wires drift (memory `project_desktop_dto_lags_browser`); they must be edited in lockstep.
- The `engine-event-emission` spec already governs the existing variants; this change modifies it rather than inventing a parallel surface.

## Goals / Non-Goals

**Goals:**
- Every card-bearing log line renders `[CARD-ID: Name]`, sourced from the event, immune to later board mutation.
- Kill `slot N` for attacks (attacker + target named).
- Attribute effect-driven memory changes to their source card (tamers and any other effect source).
- Add log lines for effect targeting and the three non-security reveal sites.
- Keep desktop and browser adapters identical.

**Non-Goals:**
- Not changing the engine's action space, tensor, or RL reward surface (events are additive; reward components ignore unknown fields).
- Not describing *what* an effect did to a target (delete/bounce/-DP) beyond the existing consequence events — `EffectTarget` only records "this effect chose these targets". Concrete consequences keep surfacing as their own lines (`Trash`, `MemoryChange`, etc.).
- Not special-casing tamers for memory attribution — all effect-sourced changes are attributed uniformly.
- Not (yet) committing the DCGO recording schema / replay to consume the new fields — see Open Questions.

## Decisions

### D1 — Carry identity on the event, not reconstruct it downstream
Add `card_name` (and ids where missing) to event payloads at emission, where `card_data` and the card's current location are both in hand. Rationale: the only robust fix; a render-time board lookup is structurally unable to name a card that has left its slot. Alternative (frontend-only board lookup) rejected — it cannot recover the card id the user explicitly wants and breaks on board mutation.

### D2 — `Attack` gains four fields
`attacker_card_id`, `attacker_card_name`, `target_card_id: Option<String>`, `target_card_name: Option<String>`. Target fields are `Option` (None ⇒ security-stack attack, rendered as the literal `security`). Populated at the single emission site in `combat/mod.rs` from the attacker handle and, when `AttackTarget::Digimon`, the target handle. The existing `attacker_field_index` / `target_field_index` / `target_player` fields stay for replay/RL consumers.

### D3 — Name-bearing fields are non-`Option` where the id is non-`Option`
`Play`/`Digivolve`/`Trash`/`Mill`/`SecurityReveal` already carry a non-optional `card_id`; their new `card_name` is likewise non-optional, so the compiler flags any emission site that forgets it (same discipline the existing spec uses for `cost_paid`/`memory_paid`). `Attack` attacker fields non-optional; target fields optional (genuinely absent for security).

### D4 — `MemoryChange` source is optional and threaded from `EffectContext`
Add `source_card_id: Option<String>`, `source_card_name: Option<String>`. The low-level `Game::gain_memory_for_player` / `set_memory` / `pay_memory` do not know the cause, so thread the source via an explicit parameter from `EffectContext::gain_memory`/`lose_memory` (which hold `self.source_card`). Chosen over an ambient "current effect source" field on `Game` because explicit threading is local, testable, and avoids a stateful trap where a stale source leaks into an unrelated change. Non-effect paths (cost payment, pass/structural) pass `None`.

### D5 — One generic `EffectTarget` event, emitted at selection commit
`EffectTarget { seq, player, source_card_id, source_card_name, targets: Vec<{card_id, card_name}> }`. Emitted where a `PendingSelection` resolves to its chosen handle(s). Chosen over typed per-consequence events (delete-target/bounce-target/…) to bound scope: one event answers "who targeted whom"; the consequence is already its own line. Fires for all targets including forced/single — the engine resolves single-legal-target picks through the same selection path, and QA wants every targeting visible.

### D6 — Reveal events mirror the DCGO recorder chokepoints
Add reveal events for reveal-deck-top, trash-from-deck-top, and reveal-hand — the exact sites the DCGO recorder already hooks (rule 27), giving an authoritative, non-arbitrary list. Each carries `player` and revealed `card_id` + `card_name`. Whether this is one parameterized `Reveal { source_zone, .. }` variant or three variants is a spec-level detail (see specs); a single parameterized variant is preferred to keep the enum small and the formatter switch flat.

### D7 — Format `[CARD-ID: Name]`, board lookup as fallback only
`gameLogFormat.ts` reads name from the event; `cardRef` renders `[id: name]`. When the event lacks identity (older recordings, structural events), fall back to existing board lookup, then to a bare id, then to `slot N`. This keeps replays of pre-change recordings working.

### D8 — Both adapters edited together; PyO3 under `engine-event-emission`
`event_to_dto` (desktop) and `event_to_pydict` (browser/server) get matching arms for every new field/variant in the same task. PyO3 surfacing is owned by the existing `engine-event-emission` spec's "PyO3 binding surfaces events" requirement, extended here.

## Risks / Trade-offs

- **[Forgotten emission site → blank name]** Non-`Option` `card_name` fields make this a compile error rather than a silent blank. → Mitigation built into D3.
- **[MemoryChange threading touches many call sites]** Adding a source parameter ripples through `gain_memory_for_player` callers. → Default the non-effect callers to `None` via a thin wrapper or explicit `None`; covered by a focused emission test.
- **[`EffectTarget` log spam]** Firing on every forced/single target could make the log noisy for multi-target effects. → Accepted per agreed scope (QA wants completeness); the formatter can later collapse multi-target lines if needed — out of scope here.
- **[Adapter drift recurring]** Editing only one wire reintroduces the desktop/browser gap. → Single task edits both; a cross-adapter test asserts identical field population.
- **[Recording/replay schema]** New fields in the event stream may surprise replay/recording consumers. → Consumers use default-skip; the recording-schema decision is deferred (Open Questions) — until resolved, treat new fields as log-only and do not assert on them in replay.

## Open Questions (resolved during implementation)

- **DCGO recording schema / replay** — RESOLVED: log-only for now. The new event fields/variants ride the existing `serde` derivation and the PyO3/desktop adapters, but `docs/DCGO_RECORDING_SCHEMA.md` and replay consumers were left unchanged; they default-skip the new `EffectTarget`/`Reveal` variants and ignore the new optional fields. Revisit if QA wants enriched recordings.
- **`Reveal` shape** — RESOLVED: single parameterized variant `Reveal { source_zone: RevealZone }` with `RevealZone { DeckTop, TrashFromDeckTop, Hand }`.
- **Line wording** — settled: `played [ID: Name]`, `attacked [ID: Name] with [ID: Name]` / `attacked security with [ID: Name]`, `gained N memory from [ID: Name]`, `[ID: Name]'s effect targeted [ID: Name]`, `revealed/trashed [ID: Name] from the top of their deck`.

## Implementation notes / known limitations

- **Reveal-from-hand removed (gap closed).** Investigation confirmed the engine has no reveal-from-hand primitive and no card in the pool reveals from hand (all DSL "reveal" is deck-top). `RevealZone` therefore has only `DeckTop` / `TrashFromDeckTop`; a `Hand` variant would be dead code. Re-add a variant + emission site if a real reveal-from-hand card is ever implemented.
- **`EffectTarget` coverage** spans the card-bearing selection installers: field (`Target`/`OwnField`/`OppField`), hand, trash, reveal, security, **union-zone, material, breeding-permanent** (single-pick), and the **DP-budget, play-cost-budget, and count-capped multi-selects** (one `EffectTarget` listing all chosen targets). Still excluded — by design — are the reveal-pool kinds (`OrderedPermutation`, `RevealBucket`: already surfaced via `Reveal` events, would double-log) and `SourceMulti`/partition source picks (low-value source selections); these emit no `EffectTarget` rather than a possibly-wrong one.
- **Mill emits both `Reveal{TrashFromDeckTop}` and a canonical `Trash` (gap closed).** `trash_from_top` routes the deck→trash movement through `Game::trash_card`, so event-stream consumers (reward/replay) see the trash, satisfying the existing "every card-to-trash emits Trash" rule. The match log renders only the reveal line; `formatEvents` multiset-suppresses the paired `Trash` line by `(player, card_id)` within the batch. The standalone `Mill` variant carries `card_name` but remains unused.
- **Memory source threading** uses a `gain_memory_for_player_sourced` variant so only the effect path (`EffectContext::gain_memory`/`lose_memory`) carries a source; `pay_memory`/`pay_memory_unchecked`/`set_memory` stay `None` with no signature churn.
