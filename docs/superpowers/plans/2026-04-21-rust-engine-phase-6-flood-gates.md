# Rust Engine Phase 6 — Flood-Gate + Restriction Modifiers

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add player-scoped flood-gate modifiers that clamp entire action categories (play Digimon by effect, gain memory from non-Tamer sources, activate Main effects, reduce play costs, etc.) at both the action-mask layer (actions disappear from `valid_action_ids`) and the resolver layer (backstop no-op with warn). Unblocks Dark Masters lockout, Medusamon Petrification, TS Olympos Tamer-anchoring, Rocks Plug-In lockouts.

**Architecture:**
- Extend `ModifierRegistry` with a second tier `player_modifiers: HashMap<PlayerId, Vec<PlayerModifierEntry>>` parallel to the existing permanent-keyed HashMap.
- Add ~12 new `ModifierType` variants for the specific flood gates identified in audits.
- Thread a new `PlaySource` enum through play/digivolve helpers so `CannotPlayDigimonByEffect` can distinguish hand-cost from effect-initiated plays.
- Wire gates at both mask (for RL-visible suppression) and resolver (for defense-in-depth).
- Reuse existing `Expiry` enum — `UntilLeaveField` handles the common "while this is in play" pattern. No new expiry variants needed for v1.

**Tech Stack:** Rust 2021 (`digimon-engine/`), DebugRunner test harness, existing `ModifierRegistry` + `ModifierEntry` patterns.

---

## Background

Phase 6 closes Cluster H from [.claude/plans/recursive-coalescing-candle.md](.claude/plans/recursive-coalescing-candle.md):

- **~55 meta-pool cards** install flood gates across all 5 audited archetypes.
- Dark Masters lockout shell: 12+ cards ("opponent can't play Digimon by effect", "opponent can't reduce play costs", "opponent can't attack")
- Medusamon Petrification: 8 cards ("opponent's Digimon can't activate their effects" — timing-granular: Main / WhenDigivolving / SecuritySkill)
- TS Olympos Tamer-anchoring: 10+ cards ("opponent can't gain memory except from Tamer effects")
- Rocks Plug-In lockouts: 6+ cards ("opponent's Digimon can't be suspended")

**What exists today** (from 2026-04-21 survey):
- `ModifierRegistry` is **purely permanent-scoped** — `HashMap<PermanentHandle, Vec<ModifierEntry>>`. No player-level storage.
- `any_with_type(ModifierType) -> bool` is the only cross-permanent query (used for `CannotPlayFromHand` global gate in `action/mask.rs:58-64`).
- 8 restriction-flavored variants already declared; 3 enforced at mask (`CannotPlayFromHand`, `CannotDigivolve`, `CannotAttackTarget`); 5 declared-but-unenforced (`CannotAttack`, `CannotSuspend`, `CannotBlock`, `CannotCounter`, `CannotUnsuspend`).
- `Expiry` enum has 6 variants, no closure-based conditions. `UntilLeaveField` already exists and handles "while this permanent is in play" semantics.
- **No `PlaySource` context**. Neither mask nor resolver can distinguish hand-cost plays from effect-initiated plays. Prerequisite for `CannotPlayDigimonByEffect`.
- No would-replacement framework — passive "cannot be X'd" modifiers (`CannotBeDestroyed*`, `CannotReturnToHand`, `CannotTrash`) are Phase 7 scope and **NOT** part of Phase 6.

**Python cross-reference:**
- Python stores modifiers as `HashMap<ModifierType, Vec<Entry>>` (flat, per-type). Each entry has a **closure-valued `condition`** so queries like `_is_play_blocked_by_modifier(card)` can inspect card-specific context.
- Python scripts check `ctx.get('played_by_effect', False)` to discriminate; BT24-023 and BT17-068 are examples.
- **Anti-pattern to avoid:** Python's `_temp_play_cost_reduction` workaround (Issue 24, addressed in Phase 5). Rust's closure hooks (`.condition`) and player-scoped registry are strictly more faithful.
- **Pattern to borrow:** the `condition` closure on `ModifierEntry` — Phase 6 doesn't strictly need it for v1 (simple flags + card-script conditions can cover most cases), but we'll design the storage so Phase 7 can add closures without a breaking change.

**Design principles (carry-forward):**
1. **Action-mask clamping is mandatory for every flood gate with a corresponding action bit.** A resolver that silently no-ops while the mask still offers the action corrupts the RL reward signal.
2. **Resolver-layer backstop.** Every flood gate also gates at the resolver (memory-gain, draw, cost-reduction, etc.) so effect-initiated actions that don't have mask bits still get blocked.
3. **`UntilLeaveField` is the dominant expiry** — Shamanmon, Queen Device, MetalSeadramon Ace all have the "while this is in play" pattern. No new Expiry variants needed for v1.
4. **`source_player` discrimination** for player-scoped modifiers: a modifier installed by player 0 on player 1 restricts player 1's actions. Stored as `target_player: PlayerId` on `PlayerModifierEntry`.
5. **No closure-valued `condition` in v1.** Card scripts use `.condition` on the Effect to gate WHEN a modifier is installed; the modifier itself is a simple flag. Closure-valued modifier conditions are a Phase 7 addition.
6. TDD per working rule 18 — failing test first.

**Cards motivating Phase 6** (from archetype gap logs):
- BT18-009 Shamanmon (TS Olympos) — `CannotGainMemoryByEffect` + Tamer-source discrimination
- BT19-093 Queen Device Option (Medusamon) — `CannotActivateMainEffects` + `CannotActivateWhenDigivolvingEffects` (and `CannotActivateSecurityEffects`)
- EX8-026 MetalSeadramon Ace (Dark Masters) — player-scoped `CannotAttack`
- ST20-15 Island of Adventure Option (Rocks) — player-scoped `CannotSuspend`
- BT15-102 Apocalymon (Dark Masters) — `CannotReducePlayCost`
- Various Dark Masters tamers — `CannotPlayDigimonByEffect`

---

## File Structure

**Modified:**
- `digimon-engine/src/enums.rs` — add new `ModifierType` variants; add `PlaySource` enum
- `digimon-engine/src/modifiers.rs` — add `player_modifiers` HashMap + `PlayerModifierEntry` struct + install/query/expiry methods
- `digimon-engine/src/game_actions.rs` — thread `PlaySource` through `play_from_hand_with_cost` / `play_from_trash_with_cost` / `effect_initiated_digivolve` / digivolve paths; resolver-layer gates
- `digimon-engine/src/effect_context/mod.rs` — resolver-layer gates in `gain_memory`, `draw`, `trash_opponent_security` (or equivalent helpers); expose `ctx.player_has_restriction(player, modifier)` query
- `digimon-engine/src/action/mask.rs` — add mask-layer gates in attack/suspend/field-effect loops; upgrade `CannotPlayFromHand` check to player-scoped
- `digimon-engine/src/effect_context/selections.rs` — if any selection helpers need restriction-checking (likely minimal)
- `docs/RUST_ENGINE_API.md` — new §Phase 6 section
- `docs/RUST_PYTHON_PARITY.md` — §6 entry

**New tests:**
- `digimon-engine/tests/flood_gates/main.rs` — module harness
- `digimon-engine/tests/flood_gates/player_scoped_registry.rs`
- `digimon-engine/tests/flood_gates/play_source_context.rs`
- `digimon-engine/tests/flood_gates/mask_gates.rs`
- `digimon-engine/tests/flood_gates/resolver_gates.rs`
- `digimon-engine/tests/flood_gates/memory_gain_tamer_discrimination.rs`
- `digimon-engine/tests/flood_gates/behavioral_end_to_end.rs`

---

## Tasks

### Task 1: Player-scoped ModifierRegistry storage + new ModifierType variants

**Files:**
- Modify: `digimon-engine/src/enums.rs` — add new variants to `ModifierType`
- Modify: `digimon-engine/src/modifiers.rs` — add `player_modifiers` HashMap + `PlayerModifierEntry` struct + methods
- Create: `digimon-engine/tests/flood_gates/main.rs`
- Create: `digimon-engine/tests/flood_gates/player_scoped_registry.rs`

**New `ModifierType` variants:**
```rust
// Existing (do not re-add): CannotPlayFromHand, CannotAttack, CannotSuspend,
// CannotDigivolve, CannotBlock, CannotCounter, CannotAttackPlayer, etc.

// Phase 6 additions — player-scoped flood gates:
CannotPlayDigimonByEffect,
CannotGainMemoryByEffect,
CannotGainMemoryExceptFromTamers,
CannotReducePlayCost,
CannotActivateMainEffects,
CannotActivateWhenDigivolvingEffects,
CannotActivateSecurityEffects,
CannotDigivolveDigimonByEffect,
CannotDrawByEffect,
CannotAddSecurityByEffect,
CannotTrashOpponentSecurity,
CannotReduceOpponentSecurity,
IgnoreColorRequirement,  // positive modifier — "you may digivolve ignoring color"
```

(Keep `CannotPlayFromHand` as-is; Task 3 upgrades its enforcement to player-scoped.)

**`PlayerModifierEntry` struct:**
```rust
pub struct PlayerModifierEntry {
    pub modifier: ModifierType,
    pub value: i32,                         // for future parametric variants; ignored for boolean flags
    pub expiry: Expiry,                     // reuses existing enum (UntilLeaveField, EndOfTurn, etc.)
    pub source_permanent: Option<PermanentHandle>,  // for UntilLeaveField expiry
    pub source_player: PlayerId,            // who installed it (for EndOfOpponentsTurn expiry)
    // NOTE: no closure-valued condition in v1. Card scripts gate WHEN they install
    // the modifier via the Effect's `.condition` closure. Phase 7 may add
    // `condition: Option<Box<dyn Fn(&EffectReadContext) -> bool + Send + Sync>>`.
}
```

**New `ModifierRegistry` methods:**
```rust
impl ModifierRegistry {
    pub fn add_player_modifier(&mut self, target_player: PlayerId, entry: PlayerModifierEntry);
    pub fn player_has(&self, target_player: PlayerId, modifier: ModifierType) -> bool;
    pub fn player_modifier_value(&self, target_player: PlayerId, modifier: ModifierType) -> i32; // sum of values; 0 if none
    pub fn player_modifiers_iter(&self, target_player: PlayerId) -> impl Iterator<Item = &PlayerModifierEntry>;
    // Expiry integration:
    pub fn expire_player_end_of_turn(&mut self, ending_player: PlayerId);
    pub fn expire_player_on_permanent_leave(&mut self, handle: PermanentHandle);
}
```

The `UntilLeaveField` expiry path triggers when the source permanent leaves battle area — hook into existing `delete_permanent` / `return_to_hand` / `return_to_deck` flows.

- [ ] **Step 1: Write failing tests**

Create `digimon-engine/tests/flood_gates/main.rs`:
```rust
mod player_scoped_registry;
```

Create `digimon-engine/tests/flood_gates/player_scoped_registry.rs`:
```rust
#[test]
fn player_has_returns_false_when_no_modifier_installed() { ... }

#[test]
fn player_has_returns_true_after_install() {
    // Install CannotGainMemoryByEffect on player 1
    // Assert r.game.modifiers.player_has(1, CannotGainMemoryByEffect) == true
    // Assert r.game.modifiers.player_has(0, CannotGainMemoryByEffect) == false
}

#[test]
fn end_of_turn_expiry_removes_player_modifier() {
    // Install with Expiry::EndOfTurn
    // Call expire_player_end_of_turn at phase boundary
    // Assert modifier removed
}

#[test]
fn until_leave_field_expiry_removes_modifier_when_source_permanent_leaves() {
    // Install with Expiry::UntilLeaveField, source_permanent = A
    // Delete A
    // Assert modifier removed
}

#[test]
fn source_player_recorded_for_end_of_opponents_turn_expiry() {
    // Install with Expiry::EndOfOpponentsTurn from player 0 onto player 1
    // End player 1's turn (should NOT expire — Opponent's turn = player 0)
    // End player 0's turn (should expire)
}

#[test]
fn multiple_player_modifiers_coexist() {
    // Install CannotPlayDigimonByEffect + CannotGainMemoryByEffect on same player
    // Assert both present; remove one; other unaffected
}
```

Register `mod flood_gates;` entry in `digimon-engine/Cargo.toml`'s `[[test]]` list.

- [ ] **Step 2: Run — compile failures expected**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test flood_gates`

- [ ] **Step 3: Implement**

In `enums.rs`, add the new `ModifierType` variants + any exhaustive match arms that need updating (check `modifiers.rs` match statements).

In `modifiers.rs`:
1. Add `PlayerModifierEntry` struct
2. Add `player_modifiers: HashMap<PlayerId, Vec<PlayerModifierEntry>>` field to `ModifierRegistry`
3. Implement `add_player_modifier`, `player_has`, `player_modifier_value`, `player_modifiers_iter`
4. Implement `expire_player_end_of_turn` mirroring existing `expire_end_of_turn` logic
5. Implement `expire_player_on_permanent_leave` that removes entries with matching `source_permanent`
6. Hook into permanent-deletion paths: search for call sites of `modifiers.remove_all_for(handle)` or similar; add a parallel `expire_player_on_permanent_leave(handle)` call nearby

- [ ] **Step 4: Run — PASS**

- [ ] **Step 5: Full suite green**

Expected: 525 (Phase 5 baseline) + 6 new = 531 passing. Zero warnings.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/enums.rs digimon-engine/src/modifiers.rs digimon-engine/tests/flood_gates/main.rs digimon-engine/tests/flood_gates/player_scoped_registry.rs digimon-engine/Cargo.toml
git commit -m "rust-engine(phase-6): add player-scoped ModifierRegistry + 13 new restriction variants"
```

---

### Task 2: PlaySource context threading

**Files:**
- Modify: `digimon-engine/src/enums.rs` — add `PlaySource` enum
- Modify: `digimon-engine/src/game_actions.rs` — thread `PlaySource` through play/digivolve functions
- Modify: `digimon-engine/src/effect_context/mod.rs` — effect-initiated helpers pass `PlaySource::ByEffect`
- Create: `digimon-engine/tests/flood_gates/play_source_context.rs`

**`PlaySource` enum:**
```rust
pub enum PlaySource {
    ByHand,      // player spent memory for printed cost
    ByEffect,    // another effect triggered this play (free or effect-paid)
    ByDigivolve, // digivolving onto a pre-digi (not strictly "play", but relevant for some gates)
}
```

**Threading strategy:**

Option A (preferred): **Add a new parameter** to `play_from_hand_with_cost` / `play_from_trash_with_cost` / `effect_initiated_digivolve`:
```rust
pub fn play_from_hand_with_cost(
    &mut self,
    player_id: PlayerId,
    hand_index: usize,
    cost_delta: CostDelta,
    source: PlaySource,  // NEW
) -> Option<usize>
```

Callers:
- Main-phase player action (`action/decode.rs::decode_play_action`): pass `PlaySource::ByHand`
- Effect-initiated plays (e.g., `EffectContext::play_from_hand_free`): pass `PlaySource::ByEffect`

Option B: **Overload with helper** — existing signature delegates to a new `_with_source(.., source)` variant, default = `ByHand`. Cleaner migration, but doubles the API surface.

Recommended: **Option A.** One-shot migration. Every existing call site is under `digimon-engine/src/` so there's no external compat concern.

**Storing the context:** For v1, the PlaySource is consumed by enforcement logic during the play action and NOT persisted on the resulting permanent. If a future gate needs to check "was this Digimon played by effect in the past," we'll add it then.

- [ ] **Step 1: Failing tests**

Create `digimon-engine/tests/flood_gates/play_source_context.rs`:
```rust
#[test]
fn play_from_hand_by_hand_is_default_for_player_action() {
    // DebugRunner decodes a Main-phase play action
    // Assert PlaySource::ByHand was used (verify by testing that a CannotPlayFromHand
    // modifier correctly gates it — see Task 3)
}

#[test]
fn play_from_hand_by_effect_from_effect_context() {
    // EffectContext::play_from_hand_free passes PlaySource::ByEffect
    // (Verify by testing that a CannotPlayDigimonByEffect modifier gates it)
}
```

These tests are forward-referenced against Task 3 enforcement. For Task 2 as a standalone, test the signature + parameter-passing by inspection and a compile check.

A more direct Task 2 test:
```rust
#[test]
fn play_source_enum_exists_and_has_expected_variants() {
    use digimon_engine::enums::PlaySource;
    let _ = PlaySource::ByHand;
    let _ = PlaySource::ByEffect;
    let _ = PlaySource::ByDigivolve;
}

#[test]
fn play_from_hand_with_cost_accepts_play_source() {
    // Compile-check: call play_from_hand_with_cost(p, i, delta, PlaySource::ByHand)
    // and play_from_hand_with_cost(p, i, delta, PlaySource::ByEffect)
    // Both should compile and behave identically (no enforcement yet in Task 2)
}
```

- [ ] **Step 2: Run — compile failures**

- [ ] **Step 3: Implement**

1. Add `PlaySource` enum to `enums.rs`.
2. Update `play_from_hand_with_cost`, `play_from_trash_with_cost`, and all `effect_initiated_*` functions to take `source: PlaySource`.
3. Update all call sites:
   - `action/decode.rs`: pass `PlaySource::ByHand`
   - `effect_context/mod.rs::play_from_hand_free` (and any similar effect-initiated helpers): pass `PlaySource::ByEffect`
   - DebugRunner helpers (`r.play(...)`, `r.place_on_field(...)`): pass appropriate source (most likely `ByHand` for Main-phase analog, `ByEffect` for effect-initiated test setups)
4. The parameter is currently unused (enforcement lives in Task 3); add `#[allow(unused_variables)]` annotations or just ignore the parameter in function bodies for now.

- [ ] **Step 4: Run — PASS**

- [ ] **Step 5: Full suite green**

Expected: 531 + 2 = 533 passing. Zero warnings (if unused-variable warnings appear, silence them with `_source: PlaySource` or `let _ = source;`).

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/enums.rs digimon-engine/src/game_actions.rs digimon-engine/src/effect_context/mod.rs digimon-engine/src/action/decode.rs digimon-engine/src/debug_runner.rs digimon-engine/tests/flood_gates/play_source_context.rs digimon-engine/tests/flood_gates/main.rs
git commit -m "rust-engine(phase-6): thread PlaySource context through play/digivolve pipelines"
```

---

### Task 3: Mask-layer gates for action-category flood gates

**Files:**
- Modify: `digimon-engine/src/action/mask.rs` — add gates in play/attack/suspend/field-effect loops
- Create: `digimon-engine/tests/flood_gates/mask_gates.rs`

**Gates to wire up:**

1. **`CannotPlayFromHand` — upgrade existing global check to player-scoped.** Existing `any_with_type(CannotPlayFromHand)` becomes:
   ```rust
   let play_blocked = game.modifiers.player_has(player_id, ModifierType::CannotPlayFromHand)
       || game.modifiers.any_with_type(ModifierType::CannotPlayFromHand);  // backward compat
   ```
   (The `any_with_type` fallback preserves any existing permanent-scoped usage; cards can gradually migrate to player-scoped installation.)

2. **`CannotAttack` (existing, unenforced) — add mask gate.** In the attack-loop in `action/mask.rs`, before each attack bit, check `game.modifiers.player_has(attacker_player_id, CannotAttack)`. If true, skip all attack bits for that player.

3. **`CannotSuspend` — add mask gate.** If the engine has a suspend-action bit, gate it. If suspend is only triggered by effects (not a player action), this is a resolver-only gate (Task 4).

4. **`CannotActivateMainEffects` — add mask gate on FIELD_EFFECT bits.** In the field-effect loop, if `game.modifiers.player_has(acting_player, CannotActivateMainEffects)`, skip all field-effect bits for that player.

**Mask gates that are OPTIONAL for Task 3 (could defer to Task 4 as resolver-only):**
- `CannotPlayDigimonByEffect` — no mask bit for effect-plays (they're triggered internally, not from the action space). Task 4 resolver gate.
- `CannotGainMemoryByEffect`, `CannotDrawByEffect` — no mask bits. Resolver-only.

- [ ] **Step 1: Failing tests**

Create `digimon-engine/tests/flood_gates/mask_gates.rs`:
```rust
#[test]
fn cannot_play_from_hand_player_scoped_zeroes_hand_bits_for_target_player() {
    // Install CannotPlayFromHand on player 0 (player-scoped, not permanent-scoped)
    // Build mask for player 0 → hand bits all zero
    // Build mask for player 1 → hand bits unaffected
}

#[test]
fn cannot_attack_player_scoped_zeroes_attack_bits() {
    // Install CannotAttack on turn player
    // Build mask → all attack bits zero
    // Remove modifier → attack bits restored
}

#[test]
fn cannot_activate_main_effects_zeroes_field_effect_bits() {
    // Install CannotActivateMainEffects on turn player
    // Place a permanent with an activatable Main effect
    // Build mask → FIELD_EFFECT bits for that permanent are zero
}

#[test]
fn player_scoped_mask_gates_are_symmetric_per_player() {
    // Install CannotPlayFromHand on player 1
    // Player 0's mask unaffected
    // Player 1's mask has hand bits zeroed
    // (Validates player-scope is actually per-player, not global)
}
```

- [ ] **Step 2: Run — failures**

- [ ] **Step 3: Implement**

In `action/mask.rs`:
1. Upgrade the `CannotPlayFromHand` global check around line 58 to also check `player_has(player_id, CannotPlayFromHand)`.
2. Find the attack-action loop; add `game.modifiers.player_has(player_id, CannotAttack)` check that zeros all attack bits.
3. Find the field-effect loop; add `game.modifiers.player_has(player_id, CannotActivateMainEffects)` check that zeros field-effect bits.
4. If the engine has a player-issued suspend-action bit, gate it with `CannotSuspend`.

- [ ] **Step 4: Run — PASS**

- [ ] **Step 5: Full suite + commit**

Expected: 533 + 4 = 537 passing.

```bash
git add digimon-engine/src/action/mask.rs digimon-engine/tests/flood_gates/mask_gates.rs digimon-engine/tests/flood_gates/main.rs
git commit -m "rust-engine(phase-6): mask-layer gates for CannotPlayFromHand (player-scoped) + CannotAttack + CannotActivateMainEffects"
```

---

### Task 4: Resolver-layer gates for effect-initiated actions

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs` — gates in `gain_memory`, `draw`, security helpers
- Modify: `digimon-engine/src/game_actions.rs` — gates in `play_from_hand_with_cost` / `play_from_trash_with_cost` / digivolve (check `PlaySource::ByEffect` + `CannotPlayDigimonByEffect`)
- Modify: `digimon-engine/src/game_actions.rs::scan_before_pay_cost_reduction` — check `CannotReducePlayCost` on the player receiving the reduction
- Create: `digimon-engine/tests/flood_gates/resolver_gates.rs`
- Create: `digimon-engine/tests/flood_gates/memory_gain_tamer_discrimination.rs`

**Gates to wire up:**

1. **`CannotPlayDigimonByEffect`:** In `play_from_hand_with_cost` and `play_from_trash_with_cost`, if `source == PlaySource::ByEffect` AND the card being played is a Digimon AND `game.modifiers.player_has(player_id, CannotPlayDigimonByEffect)`, early-return `None` (play fails silently).

2. **`CannotGainMemoryByEffect`:** In `EffectContext::gain_memory` (or wherever memory gains from effects route), check `game.modifiers.player_has(current_player, CannotGainMemoryByEffect)`. If true, no-op. (The player action of "pass turn to opponent = gain memory" is NOT an effect-initiated gain and should not be gated.)

3. **`CannotGainMemoryExceptFromTamers`:** Same gate, but additionally check whether the effect source is a Tamer. Introduce a **new read-only helper** `EffectContext::source_is_tamer(&self) -> bool` (and a parallel `EffectReadContext::source_is_tamer(&self) -> bool`) that looks up `self.source_card` in `card_data` and returns `card_kind == CardKind::Tamer`. This helper matches DCGO's `ICardEffect.IsTamerEffect` property (see `DCGO/Assets/Scripts/CardEffect/BT3/Green/BT3_046.cs:41-55` — BT3-046's `CardEffectCondition` closure checks `!cardEffect.IsTamerEffect` to gate opponent memory gains).

   Why a helper (not ad-hoc inline lookup): three different flood gates plus future cards will want the same check. A single helper keeps the Tamer-source predicate in one place; if we later want to treat "Tamer-derived" effects (e.g., Plug-In cards attached to a Tamer) as Tamer-sourced, there's one call site to update.

   In `gain_memory`:
   ```rust
   pub fn gain_memory(&mut self, amount: i32) {
       let target = self.player; // or whichever player this ctx belongs to
       if self.game.modifiers.player_has(target, ModifierType::CannotGainMemoryByEffect) {
           return;
       }
       if self.game.modifiers.player_has(target, ModifierType::CannotGainMemoryExceptFromTamers)
           && !self.source_is_tamer()
       {
           return;
       }
       self.game.gain_memory_raw(target, amount);
   }
   ```

   The helper definition (in `effect_context/mod.rs`):
   ```rust
   impl<'g> EffectContext<'g> {
       /// Returns true if this effect's source card is a Tamer.
       ///
       /// Used by flood-gate discriminators like `CannotGainMemoryExceptFromTamers`
       /// that allow Tamer-sourced effects but block Digimon/Option-sourced ones.
       /// Matches DCGO's `ICardEffect.IsTamerEffect` property.
       pub fn source_is_tamer(&self) -> bool {
           self.game
               .card_data_for(self.source_card)
               .map(|data| data.card_kind == CardKind::Tamer)
               .unwrap_or(false)
       }
   }
   ```
   (Adjust `card_data_for` to whatever the existing lookup method is — `CardSource::card_data`, `Game::card_data_for_handle`, etc.)

   Mirror the same helper on `EffectReadContext` for use in `condition` closures.

4. **`CannotDrawByEffect`:** In `EffectContext::draw` (effect-initiated draw helper, distinct from the start-of-turn draw), check `game.modifiers.player_has(current_player, CannotDrawByEffect)`. If true, no-op.

5. **`CannotReducePlayCost`:** In `scan_before_pay_cost_reduction` (Phase 5 helper), if the player whose cost is being reduced has `CannotReducePlayCost`, return 0 (or skip the scan entirely for that player). **Note:** Phase 5's scan signature was `&mut self` — easy to add the check at the top:
   ```rust
   // Phase 6: if the acting player has CannotReducePlayCost, suppress all reductions
   if self.modifiers.player_has(acting_player, ModifierType::CannotReducePlayCost) {
       return 0;
   }
   ```
   (The scan needs to know the acting player — currently it iterates both; verify whether a "for-this-play" context exists, and if not, wire it in.)

6. **`CannotAddSecurityByEffect`:** In the security-add helpers (whatever `EffectContext` methods add cards to security), check the target player's modifier. If true, no-op.

7. **`CannotTrashOpponentSecurity` / `CannotReduceOpponentSecurity`:** In the security-trash helpers, check the acting player's modifier. If true, no-op.

- [ ] **Step 1: Write failing tests (resolver_gates.rs)**

```rust
#[test]
fn cannot_play_digimon_by_effect_blocks_effect_initiated_play_but_not_hand_play() {
    // Install CannotPlayDigimonByEffect on player 0
    // Effect tries to play a Digimon from hand via play_from_hand_with_cost(..., PlaySource::ByEffect)
    //   → returns None (no play)
    // Player-action play from hand via PlaySource::ByHand
    //   → succeeds (unaffected by this modifier)
}

#[test]
fn cannot_gain_memory_by_effect_blocks_effect_gains_but_not_turn_pass() {
    // Install CannotGainMemoryByEffect on player 0
    // ctx.gain_memory(2) inside an effect context → no-op
    // Turn-pass memory delta → unchanged (turn-pass goes through a different path)
}

#[test]
fn cannot_draw_by_effect_blocks_ctx_draw() {
    // Install CannotDrawByEffect on player 0
    // ctx.draw(1) → no-op (hand size unchanged)
}

#[test]
fn cannot_reduce_play_cost_suppresses_before_pay_cost_scan() {
    // Install CannotReducePlayCost on player 0
    // Player 0 has a battle-area permanent with BeforePayCost + cost_reduction_fn(|_| 3)
    // Player 0 plays a Digimon with printed_cost = 5
    // Expected: effective cost = 5 (reduction suppressed), memory -5
}

#[test]
fn cannot_add_security_by_effect_blocks_add_security_helper() {
    // Install CannotAddSecurityByEffect on player 0
    // ctx.add_security(...) → no-op (security count unchanged)
}
```

Create `memory_gain_tamer_discrimination.rs`:
```rust
#[test]
fn source_is_tamer_helper_returns_true_for_tamer_card() {
    // Build a DebugRunner with a Tamer card.
    // Construct an EffectContext whose source_card points to the Tamer.
    // Assert ctx.source_is_tamer() == true
}

#[test]
fn source_is_tamer_helper_returns_false_for_digimon_and_option() {
    // Repeat the above for a Digimon source (false) and an Option source (false)
}

#[test]
fn source_is_tamer_helper_mirrored_on_effect_read_context() {
    // Construct a read-only EffectReadContext (condition closure shape)
    // Assert the same helper exists and returns matching results
}

#[test]
fn cannot_gain_memory_except_from_tamers_allows_tamer_source() {
    // Install CannotGainMemoryExceptFromTamers on player 0
    // An effect with source_card = Tamer calls ctx.gain_memory(2) → SUCCEEDS (memory +2)
    // An effect with source_card = Digimon calls ctx.gain_memory(2) → NO-OP (helper returned false)
}

#[test]
fn cannot_gain_memory_except_from_tamers_blocks_option_source() {
    // Install on player 0
    // An effect with source_card = Option calls ctx.gain_memory(2) → NO-OP
    // (Only Tamer sources are allowed; Option is not Tamer)
}
```

- [ ] **Step 2: Run — failures**

- [ ] **Step 3: Implement gates**

Implement in order of simplicity:
1. **`EffectContext::source_is_tamer(&self) -> bool`** + mirror on `EffectReadContext` — the shared Tamer-source predicate helper (DCGO parity: `ICardEffect.IsTamerEffect`). This is a prerequisite for #6 and should land first so test 6 can use it.
2. `CannotDrawByEffect` in `ctx.draw` — simplest
3. `CannotGainMemoryByEffect` in `ctx.gain_memory` — similar
4. `CannotAddSecurityByEffect` in `ctx.add_security`-family
5. `CannotTrashOpponentSecurity` in `ctx.trash_security`-family
6. `CannotReducePlayCost` in `scan_before_pay_cost_reduction` — needs acting-player context, so may require a small refactor to pass it in
7. `CannotGainMemoryExceptFromTamers` in `ctx.gain_memory` (combines with #3 via `source_is_tamer`) — uses the helper from #1
8. `CannotPlayDigimonByEffect` in `play_from_hand_with_cost` / `play_from_trash_with_cost` — check `source == ByEffect && card_kind == Digimon`

- [ ] **Step 4: Run — PASS**

- [ ] **Step 5: Full suite + commit**

Expected: 537 + 10 = 547 passing (7 gate tests + 3 `source_is_tamer` helper tests).

```bash
git add digimon-engine/src/effect_context/mod.rs digimon-engine/src/game_actions.rs digimon-engine/tests/flood_gates/resolver_gates.rs digimon-engine/tests/flood_gates/memory_gain_tamer_discrimination.rs digimon-engine/tests/flood_gates/main.rs
git commit -m "rust-engine(phase-6): resolver-layer gates for effect-initiated memory/draw/play/security + Tamer-source discrimination"
```

---

### Task 5: End-to-end behavioral test

**File:** Create `digimon-engine/tests/flood_gates/behavioral_end_to_end.rs`, register in `main.rs`.

**Scenario (Shamanmon-style TS Olympos Tamer lockout):**

> Player 0 has a Tamer permanent on field with a `Declarative` effect that installs `CannotGainMemoryExceptFromTamers` on player 1 with `Expiry::UntilLeaveField`.
>
> Test flow:
> 1. Player 1 tries to gain memory from a Digimon effect → BLOCKED (no-op)
> 2. Player 1 gains memory from a Tamer effect → ALLOWED
> 3. Player 0 deletes the Shamanmon-like Tamer → modifier expires
> 4. Player 1 tries again to gain memory from a Digimon effect → ALLOWED (modifier is gone)

Exercises:
- Player-scoped registry install at effect resolution
- `UntilLeaveField` expiry triggered by permanent deletion
- Tamer-source discrimination in gain_memory
- Round-trip: install → enforce → expire → re-allow

- [ ] **Step 1: Write test**
- [ ] **Step 2: Run — PASS** (all underlying infrastructure from Tasks 1-4 should compose)
- [ ] **Step 3: Commit**

Expected: 547 + 1 = 548 passing.

```bash
git add digimon-engine/tests/flood_gates/behavioral_end_to_end.rs digimon-engine/tests/flood_gates/main.rs
git commit -m "rust-engine(phase-6): end-to-end behavioral test — Tamer-anchored memory lockout with UntilLeaveField expiry"
```

---

### Task 6: Docs + roadmap

**Files:**
- Modify: `docs/RUST_ENGINE_API.md` — new §Phase 6 Flood Gates & Restrictions section
- Modify: `docs/RUST_PYTHON_PARITY.md` — §6 entry
- Modify: `C:\Users\james\.claude\plans\recursive-coalescing-candle.md` — flip Phase 6 row to ✅ LANDED; update Immediate Next Steps
- Commit: untracked `docs/superpowers/plans/2026-04-21-rust-engine-phase-6-flood-gates.md` plan file

**API doc structure:**

1. **Player-scoped ModifierRegistry** subsection
   - `add_player_modifier(target_player, entry)` signature + example
   - `player_has(target_player, modifier) -> bool`
   - `player_modifiers_iter(target_player)`
   - Expiry semantics (`UntilLeaveField` dominant pattern)

2. **PlaySource context** subsection
   - `PlaySource::{ByHand, ByEffect, ByDigivolve}`
   - Threading through play helpers
   - Why it matters for `CannotPlayDigimonByEffect`

3. **Flood-gate catalog** — list all 13 new ModifierType variants with:
   - Variant name
   - Semantics (one sentence)
   - Enforcement layer (mask / resolver / both)
   - Example card text that installs it

4. **Tamer-source discrimination helper** — document `ctx.source_is_tamer() -> bool` (and `EffectReadContext::source_is_tamer`) with signature + semantics + the `CannotGainMemoryExceptFromTamers` usage example. Note the DCGO parity: equivalent to `ICardEffect.IsTamerEffect`.

5. **Worked example:** Shamanmon-style Tamer installing `CannotGainMemoryExceptFromTamers` with `UntilLeaveField`. Show the full `.condition` + `.process` pattern where `.process` installs the modifier via `ctx.game.modifiers.add_player_modifier(...)`, and show how the downstream `gain_memory` gate combines `player_has` + `source_is_tamer`.

**Parity doc entry (§6):**

```markdown
### §6.1 Player-scoped flood gates — Rust (Phase 6)

Rust adds a parallel `player_modifiers` tier to `ModifierRegistry` plus 13 new ModifierType variants for action-category flood gates (CannotPlayDigimonByEffect, CannotGainMemoryByEffect/ExceptFromTamers, CannotReducePlayCost, CannotActivateMainEffects/WhenDigivolvingEffects/SecurityEffects, etc.). Gates are enforced at BOTH the action-mask layer (RL-visible) and the resolver layer (defense-in-depth).

Python stores modifiers as a flat HashMap<ModifierType, Vec<Entry>> with closure-valued per-entry conditions (see `_is_play_blocked_by_modifier`). Rust v1 uses simple flag-based entries without closure conditions; card scripts gate install-time via the Effect's `.condition` closure instead. Phase 7 may add condition closures to `ModifierEntry` for the would-replacement framework.

Python's `ctx.get('played_by_effect', False)` context is matched by Rust's `PlaySource` enum, which is threaded through play/digivolve helpers. This is strictly cleaner than Python's dict-based context (typed, exhaustive, no silent defaulting).

Cards unblocked (per audits): ~55 across all 5 audited archetypes.
```

**Roadmap update:**

Flip Phase 6 row in the cumulative readiness table:
```
| Phase 6 | flood-gate + restriction modifiers | ~312 | ~95% | ✅ Landed 2026-04-21 (re-audit pending) |
```

Add entry 7 to Immediate Next Steps with commit range. Update "Suggested next phase" to **Phase 7 (would-replacement framework)** — note that Phase 7 REQUIRES a design spec under `docs/superpowers/specs/` before plan/implementation.

- [ ] **Step 1: Draft each doc section**
- [ ] **Step 2: Apply edits**
- [ ] **Step 3: Commit (repo files only; roadmap lives at user-home path)**

```bash
git add docs/RUST_ENGINE_API.md docs/RUST_PYTHON_PARITY.md docs/superpowers/plans/2026-04-21-rust-engine-phase-6-flood-gates.md
git commit -m "docs(phase-6): RUST_ENGINE_API + PARITY + plan — Phase 6 flood gates landed"
```

---

## Verification

After all tasks land:

1. `cargo test --manifest-path digimon-engine/Cargo.toml` — full suite green, +23 new tests beyond Phase 5's 525 (target ~548)
2. Grep for `player_modifiers` — HashMap present with full API surface
3. Grep for `PlaySource` — threaded through all 5+ play/digivolve paths
4. Grep for each new ModifierType variant — used in at least one test
5. `docs/RUST_ENGINE_API.md` has §Phase 6 with 4 subsections
6. `docs/RUST_PYTHON_PARITY.md` has §6 entry
7. Roadmap Phase 6 row = ✅ LANDED at user-home path

## Non-Goals (deferred to Phase 7)

- **Passive "cannot be X'd" modifiers** (`CannotBeReturnedToDeck`, `CannotBeDeDigivolved`, `CannotBeTrashedByEffect`) — these are replacement effects under the "would" framework. Phase 7.
- **Closure-valued `condition` on `PlayerModifierEntry`** — card scripts use the Effect's `.condition` to gate install-time instead. Phase 7 adds closures to modifiers for fine-grained per-action discrimination.
- **`EndOfOpponentsNextTurn` expiry variant** — no audited card uses this; current `EndOfOpponentsTurn` with `source_player` discrimination covers most cases. Add when a real card demands it.
- **`WhileConditionHolds(closure)` expiry** — overkill for v1. `UntilLeaveField` + Effect's `.condition` covers all identified use cases.
- **Per-card context discrimination beyond PlaySource** — e.g., "cannot play Lv.6 Digimon by effect" with level filter. Use `.condition` on the Effect that installs the modifier; don't parameterize the ModifierType variant itself.
- **Inheritance filter on player modifiers** — player modifiers have no inheritance concept (they're not attached to a permanent's digivolution stack). N/A.
