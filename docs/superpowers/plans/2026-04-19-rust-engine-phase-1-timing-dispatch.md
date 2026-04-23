# Rust Engine Phase 1 — Timing Dispatch Infrastructure

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Wire every declared-but-unfired `EffectTiming` variant and add 2 new variants (`OnOpponentSecurityRemoved`, `OnDigivolutionCardTrashed`) so card scripts can hook into turn phases, combat events, and global observers.

**Architecture:** Add fire sites at the natural transition points in [game_phases.rs](digimon-engine/src/game_phases.rs), [combat.rs](digimon-engine/src/combat.rs), [game_actions.rs](digimon-engine/src/game_actions.rs), and [player.rs](digimon-engine/src/player.rs). Each new dispatch pairs an `enqueue_triggered(timing, trigger_source)` call with a `drain_effect_queue()` call. `TriggerSource::PlayerBattleArea` fans out observer-style timings over a player's field; `TriggerSource::Permanent` scopes to a single permanent. Where a new observer needs global reach (all players' battle areas), add a `TriggerSource::GlobalBattleAreas` variant.

**Tech Stack:** Rust 2021, `digimon-engine` crate.

**Roadmap context:** [.claude/plans/recursive-coalescing-candle.md](../../../.claude/plans/recursive-coalescing-candle.md) Phase 1 — ~120 cards unblocked, highest ROI per engineering effort. Zero new design; mostly plumbing.

---

## Current State (from inventory)

**Wired dispatches** (don't touch):
- `OnPlay` → `fire_on_play` at `game_actions.rs:109`
- `WhenDigivolving` → `effect_initiated_digivolve` + counter-timing fire sites
- `OnAttack` → `combat.rs:1116-1120`
- `OnSecurityCheck` → `combat.rs:955` (Phase 0)
- `OnLoseSecurity` → `combat.rs:969`
- `EndOfYourTurn` → `game_phases.rs:345-349`

**Declared-but-unfired** (Phase 1 wires these):
- `StartOfYourTurn`, `StartOfOpponentsTurn`, `EndOfOpponentsTurn`
- `WhenAttacking`, `EndOfAttack`, (new: `EndOfBattle` — not yet a variant, may add)
- `OnAnyDigimonPlayed` (remap: use existing `OnEnterFieldAnyone`), `OnAnyDeletion` (new variant)
- `OnDigivolve`, `OnSuspend`, `OnUnsuspend`
- `OnAttackTargetChange`, `OnHatch` (may not exist — check)
- `StartOfYourMainPhase` (variant may not exist — check)

**New variants to add:**
- `OnOpponentSecurityRemoved` (Medusamon core)
- `OnDigivolutionCardTrashed` (Rocks core)
- `EndOfBattle` (if not present)
- `StartOfYourMainPhase` (if not present)
- `OnAnyDeletion` (if not present)

---

## File Structure

**Files modified (all phases):**
- `digimon-engine/src/enums.rs` — add 2–5 new `EffectTiming` variants; possibly a `TriggerSource::GlobalBattleAreas` variant (decide in Task 1)
- `digimon-engine/src/effect.rs` — add 10–15 new `Effect::*` builder constructors
- `digimon-engine/src/selection.rs` — new `TriggerSource` variants if needed
- `digimon-engine/src/effect_queue.rs` — dispatch branches for any new `TriggerSource` variants
- `digimon-engine/src/game_phases.rs` — turn-phase fire sites
- `digimon-engine/src/combat.rs` — combat-phase fire sites
- `digimon-engine/src/game_actions.rs` — play/digivolve/delete-permanent observer hooks
- `digimon-engine/src/player.rs` — per-permanent suspend/unsuspend/hatch hook sites

**Files created:**
- `digimon-engine/tests/timing_dispatch.rs` — one integration test per new dispatch

**Docs:**
- `docs/RUST_ENGINE_API.md` — document new Effect builder constructors
- `docs/RUST_ENGINE_GAPS.md` — annotate closed entries

---

## Task 1: Add new EffectTiming variants + builder constructors

**Files:**
- `digimon-engine/src/enums.rs` — add new `EffectTiming` variants as needed
- `digimon-engine/src/effect.rs` — add `Effect::*` builder constructors
- `digimon-engine/tests/timing_dispatch.rs` — create

**Steps:**

- [ ] **1.1 Audit current variants**

Read `digimon-engine/src/enums.rs` EffectTiming enum. Confirm which of the following exist:
- `StartOfYourMainPhase`
- `EndOfBattle`
- `OnHatch`
- `OnAnyDeletion`
- `OnAnyDigimonPlayed` (may be `OnEnterFieldAnyone`)

For each that does NOT exist, add a new variant below similar-category variants. For those that already exist, skip.

- [ ] **1.2 Add new variants**

Add to `EffectTiming` enum in `enums.rs` (only the ones not yet present):

```rust
/// Fires when an opponent's security card is removed from the security
/// stack by any means (security check, effect, etc.). Medusamon core
/// archetype observer. Context: opponent PlayerId, removed CardHandle.
OnOpponentSecurityRemoved,

/// Fires when a source card is trashed from a permanent's digivolution
/// stack during digivolve cost payment or other source-manipulation.
/// Rocks core archetype observer. Context: permanent handle, trashed
/// CardHandle.
OnDigivolutionCardTrashed,
```

Plus any of `StartOfYourMainPhase`, `EndOfBattle`, `OnHatch`, `OnAnyDeletion` that were missing.

- [ ] **1.3 Verify tests still compile**

```bash
cargo build --manifest-path digimon-engine/Cargo.toml 2>&1 | tail -5
```

Expect compile to succeed — new enum variants without match arms should trigger warnings but not errors as long as every `match EffectTiming` has a `_ =>` wildcard. If the build fails due to non-exhaustive matches, add `_ => {}` arms to the affected matches.

- [ ] **1.4 Add Effect builder constructors in `effect.rs`**

For each NEW or unexposed timing, add a builder constructor mirroring `Effect::on_play`. Include one for each of:

- `Effect::start_of_your_turn(card)`
- `Effect::start_of_your_main_phase(card)` (if variant exists)
- `Effect::end_of_opponents_turn(card)`
- `Effect::when_attacking(card)`
- `Effect::end_of_attack(card)`
- `Effect::end_of_battle(card)` (if variant added)
- `Effect::on_any_digimon_played(card)` — maps to OnEnterFieldAnyone OR OnAnyDigimonPlayed, whichever is the canonical variant
- `Effect::on_any_deletion(card)` (if variant added)
- `Effect::on_digivolve(card)`
- `Effect::on_suspend(card)`
- `Effect::on_unsuspend(card)`
- `Effect::on_attack_target_change(card)`
- `Effect::on_hatch(card)` (if variant added)
- `Effect::on_opponent_security_removed(card)`
- `Effect::on_digivolution_card_trashed(card)`

Template:

```rust
/// Fires at the start of the controller's turn.
pub fn start_of_your_turn(card: CardHandle) -> EffectBuilder {
    EffectBuilder::new(card, EffectTiming::StartOfYourTurn)
}
```

- [ ] **1.5 Create `digimon-engine/tests/timing_dispatch.rs`**

With a helper + one sanity-check test that the enum variants are constructible:

```rust
//! Phase 1 timing-dispatch integration tests.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};

pub fn plain_digimon(card_id: &str, name: &str, play_cost: u16) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
    }
}

#[test]
fn new_effect_timings_are_constructible() {
    // Sanity-check that builder constructors compile. Actual dispatch tested
    // in subsequent tasks.
    let card = digimon_engine::card_source::CardHandle(0);
    let _ = digimon_engine::effect::Effect::start_of_your_turn(card).build();
    let _ = digimon_engine::effect::Effect::end_of_opponents_turn(card).build();
    let _ = digimon_engine::effect::Effect::when_attacking(card).build();
    let _ = digimon_engine::effect::Effect::end_of_attack(card).build();
    let _ = digimon_engine::effect::Effect::on_suspend(card).build();
    let _ = digimon_engine::effect::Effect::on_unsuspend(card).build();
    let _ = digimon_engine::effect::Effect::on_digivolve(card).build();
    let _ = digimon_engine::effect::Effect::on_attack_target_change(card).build();
    let _ = digimon_engine::effect::Effect::on_opponent_security_removed(card).build();
    let _ = digimon_engine::effect::Effect::on_digivolution_card_trashed(card).build();
}
```

- [ ] **1.6 Run + commit**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test timing_dispatch 2>&1 | tail -10
cargo test --manifest-path digimon-engine/Cargo.toml 2>&1 | tail -5
git add digimon-engine/src/enums.rs digimon-engine/src/effect.rs digimon-engine/tests/timing_dispatch.rs
git commit -m "feat(engine): Phase 1 — new EffectTiming variants + builder constructors"
```

---

## Task 2: Wire `StartOfYourTurn`

**Files:**
- `digimon-engine/src/game_phases.rs` — add fire site in `begin_turn`
- `digimon-engine/tests/timing_dispatch.rs` — integration test

**Steps:**

- [ ] **2.1 Write failing test**

Append to `tests/timing_dispatch.rs`. A `StartOfYourTurn` effect on a permanent should fire when the controller's turn begins. The test registers a card whose inherited `StartOfYourTurn` effect gains memory, plays it, passes a turn, confirms memory gained on the return turn.

Pattern: use a custom `CardEffect` with `Effect::start_of_your_turn(card).process(|ctx| ctx.gain_memory(1)).build()`. Register via `r.register_effect(...)`. Play the card, pass turn twice so it's the controller's turn again, assert memory reflects the +1 gain.

- [ ] **2.2 Verify compile failure or test failure**

Expected: test fails because no fire site exists.

- [ ] **2.3 Add fire site in `begin_turn`**

In `game_phases.rs`, find `fn begin_turn` (~line 17). The function handles Unsuspend + Draw + Main phase transitions. Add the StartOfYourTurn fire at the top of the method body (before Unsuspend):

```rust
pub(crate) fn begin_turn(&mut self) {
    let tp = self.turn_player();

    // StartOfYourTurn fires BEFORE Unsuspend — matches Python OnStartTurn.
    self.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        crate::selection::TriggerSource::PlayerBattleArea(tp),
    );
    self.drain_effect_queue();

    // ... rest of existing begin_turn body ...
}
```

Also consider: should `StartOfOpponentsTurn` fire here for the non-turn-player's field? Check the Python reference — if yes, add a second enqueue after the turn-player enqueue. Otherwise skip; `StartOfOpponentsTurn` is declared but deferred.

- [ ] **2.4 Run; verify test passes**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test timing_dispatch start_of_your_turn 2>&1 | tail -10
```

- [ ] **2.5 Run full suite; no regressions.**

- [ ] **2.6 Commit**

```bash
git add -u && git commit -m "feat(engine): Phase 1 — wire StartOfYourTurn dispatch in begin_turn"
```

---

## Task 3: Wire `StartOfYourMainPhase`

Similar shape to Task 2 but fires at Main-phase entry (after Draw, before player has agency).

- [ ] **3.1 Write failing test** (mirror Task 2; memory gain on entering Main phase)
- [ ] **3.2 Verify fail**
- [ ] **3.3 Add fire site** in `begin_turn` in `game_phases.rs` right before setting `self.current_phase = GamePhase::Main` (find this line; likely near the end of `begin_turn` after the Draw logic).
- [ ] **3.4 Run test**
- [ ] **3.5 Full suite**
- [ ] **3.6 Commit**

---

## Task 4: Wire `EndOfOpponentsTurn`

Fires for the non-turn-player's field at turn rotation, just after `EndOfYourTurn` drains.

- [ ] **4.1 Write failing test** (a `EndOfOpponentsTurn` effect on Player 1's permanent triggers when Player 0's turn ends)
- [ ] **4.2 Verify fail**
- [ ] **4.3 Add fire site** in `rotate_turn_player` in `game_phases.rs`, just after `self.fire_end_of_your_turn(ending_player)` but BEFORE the memory flip. Iterate opponents:

```rust
for opp in self.opponents(ending_player) {
    self.enqueue_triggered(
        EffectTiming::EndOfOpponentsTurn,
        crate::selection::TriggerSource::PlayerBattleArea(opp),
    );
}
self.drain_effect_queue();
```

- [ ] **4.4 Run test**
- [ ] **4.5 Full suite**
- [ ] **4.6 Commit**

---

## Task 5: Wire `WhenAttacking`

Fires after attack declaration, in the attacker's battle area.

- [ ] **5.1 Write failing test** (effect on a permanent gains memory when the permanent attacks)
- [ ] **5.2 Verify fail**
- [ ] **5.3 Add fire site** in `combat.rs` — inside `advance_pending_attack` at the `AttackState::Declared` → transition. Fire for the attacker's controller:

```rust
// WhenAttacking fires after OnAttack, before Alliance window.
self.enqueue_triggered(
    EffectTiming::WhenAttacking,
    crate::selection::TriggerSource::PlayerBattleArea(attacker.player),
);
self.drain_effect_queue();
```

Reuse the existing `fire_on_attack` pattern (`combat.rs:1116-1120`) as reference — add WhenAttacking dispatch right after OnAttack.

- [ ] **5.4 Run test**
- [ ] **5.5 Full suite**
- [ ] **5.6 Commit**

---

## Task 6: Wire `EndOfAttack`

Fires when the attack state machine transitions to `Cleanup`, after battle resolves.

- [ ] **6.1 Failing test** (memory gain at end of attack)
- [ ] **6.2 Verify fail**
- [ ] **6.3 Add fire site** in `combat.rs` — find the Cleanup transition (grep `AttackState::Cleanup`). Fire across both players' battle areas using `PlayerBattleArea` for each:

```rust
// EndOfAttack — fire for every player (attacker + defender both see).
for p in 0..self.rules.player_count {
    self.enqueue_triggered(
        EffectTiming::EndOfAttack,
        crate::selection::TriggerSource::PlayerBattleArea(p as PlayerId),
    );
}
self.drain_effect_queue();
```

- [ ] **6.4 Run + full suite + commit**

---

## Task 7: Wire `EndOfBattle`

Distinct from `EndOfAttack`: fires after battle-phase DP comparison but before post-battle cleanup. Used by "If this Digimon wins a battle, ..." effects.

- [ ] **7.1 Failing test** (memory gain when a Digimon wins a battle)
- [ ] **7.2 Verify fail**
- [ ] **7.3 Add fire site** in `combat.rs` — in `resolve_pending_battle`, after the DP comparison resolves but before `AttackState::Cleanup` is set. Fire `PlayerBattleArea` for attacker and defender.
- [ ] **7.4 Run + commit**

---

## Task 8: Wire `OnAnyDigimonPlayed` / `OnEnterFieldAnyone`

Global observer — fires when any Digimon enters the battle area (including the one being played).

- [ ] **8.1 Failing test** — Player 1's permanent has an `OnEnterFieldAnyone` effect that gains memory when Player 0 plays a Digimon.
- [ ] **8.2 Verify fail**
- [ ] **8.3 Add fire site** in `game_actions.rs` in `play_from_hand_with_cost` — AFTER the Play event is emitted but BEFORE (or after?) `fire_on_play`. Decide based on Python: Python fires `OnEnterFieldAnyone` after the card's own `OnPlay`. Do the same here.

```rust
self.fire_on_play(player_id, field_index);

// Global observer — every player's battle area sees the entry.
for p in 0..self.rules.player_count {
    self.enqueue_triggered(
        EffectTiming::OnEnterFieldAnyone,
        crate::selection::TriggerSource::PlayerBattleArea(p as PlayerId),
    );
}
self.drain_effect_queue();
```

Also fire from `play_from_trash_with_cost` so trash-plays trigger observers.

- [ ] **8.4 Run + commit**

---

## Task 9: Wire `OnAnyDeletion`

Global observer when any permanent is deleted.

- [ ] **9.1 Failing test** — Player's permanent has an `OnAnyDeletion` effect that gains memory when any Digimon dies.
- [ ] **9.2 Verify fail**
- [ ] **9.3 Add fire site** — find every deletion path (`delete_permanent_with_effects`, `Game::delete_permanent`, `Player::delete_permanent`). Central fire site: in the method that's called by all paths (likely `Game::delete_permanent_with_effects` in combat.rs or game.rs). After `OnDeletion` self-deletion drains, enqueue `OnAnyDeletion` across all players' battle areas.
- [ ] **9.4 Run + commit**

---

## Task 10: Wire `OnSuspend` / `OnUnsuspend`

- [ ] **10.1 Failing tests** (2 tests: one per timing)
- [ ] **10.2 Verify fail**
- [ ] **10.3 Add fire sites** in `EffectContext::suspend` / `EffectContext::unsuspend` AND in `Player::unsuspend_all` (which fires during turn-begin Unsuspend phase). Both should enqueue an observer `TriggerSource::PlayerBattleArea` for the controller.
- [ ] **10.4 Run + commit**

---

## Task 11: Wire `OnHatch`

- [ ] **11.1 Failing test** — a permanent's `OnHatch` effect gains memory when the controller hatches an egg.
- [ ] **11.2 Verify fail**
- [ ] **11.3 Add fire site** in `Player::hatch` or `Game::hatch` — after the egg moves to breeding. Enqueue for the hatching player's battle area.
- [ ] **11.4 Run + commit**

---

## Task 12: Wire `OnDigivolve`

Trait-filter observer — fires when any Digimon digivolves. Different from `WhenDigivolving` which is the self-timing.

- [ ] **12.1 Failing test** — a permanent's `OnDigivolve` effect fires when any Digimon on the field digivolves (including its own controller's other Digimon).
- [ ] **12.2 Verify fail**
- [ ] **12.3 Add fire site** in `effect_initiated_digivolve` AND in the RL-action digivolve path (`digivolve_from_hand`). After `WhenDigivolving` drains, add `OnDigivolve` observer enqueue across all players' battle areas.
- [ ] **12.4 Run + commit**

---

## Task 13: Wire `OnAttackTargetChange`

Fires when an attack's target is rewritten mid-flight (Block interrupt).

- [ ] **13.1 Failing test** — Block redirect triggers `OnAttackTargetChange`.
- [ ] **13.2 Verify fail**
- [ ] **13.3 Add fire site** in `combat.rs` — in the Block resolution path where `effective_target` is rewritten. Fire for the attacker's battle area (attacker may have "If your attack is blocked, ..." effects).
- [ ] **13.4 Run + commit**

---

## Task 14: Wire `OnOpponentSecurityRemoved` (NEW — Medusamon core)

Fires when opponent's security card is removed from the stack.

- [ ] **14.1 Failing test** — Player 0 has a permanent with `OnOpponentSecurityRemoved` effect. Player 0 attacks Player 1's player directly; security reveal removes a card; the effect fires.
- [ ] **14.2 Verify fail**
- [ ] **14.3 Add fire site** in `combat.rs` — inside `SecurityPhase::Dispose` (around combat.rs:982) right after the revealed card moves to trash or stays played. Fire only for the *attacker's* battle area (the one whose controller caused the removal). Also fire for any non-combat security-removal paths (effect-driven security trashes).
- [ ] **14.4 Run + commit**

---

## Task 15: Wire `OnDigivolutionCardTrashed` (NEW — Rocks core)

Fires when a source card is trashed from a permanent's digivolution stack.

- [ ] **15.1 Failing test** — a source is trashed via some mechanism (e.g. a card effect that trashes a source); a listening permanent's `OnDigivolutionCardTrashed` effect fires.
- [ ] **15.2 Verify fail**
- [ ] **15.3 Add fire site** — find all paths that trash sources from a permanent's `card_sources` vec. Candidate paths: any `return_to_hand` / `return_to_deck` (which trashes sources), any digivolve cost-payment that discards sources, any `place_as_bottom_source` flow that displaces sources. Add the observer enqueue at the centralized "source trashed" point.

If there's no single chokepoint, add a helper on Game: `fn trash_source(&mut self, handle: PermanentHandle, source: CardSource)` that moves the source to trash AND fires `OnDigivolutionCardTrashed`. Refactor callers to use the helper.

Fire for all players' battle areas (the observer can be on either side).

- [ ] **15.4 Run + commit**

---

## Task 16: Docs + gap-log cleanup

- [ ] **16.1 Add builder constructors to docs/RUST_ENGINE_API.md** — append a §Phase 1 Timing Dispatch section listing the new builder constructors with one-line descriptions.
- [ ] **16.2 Annotate closed entries in docs/RUST_ENGINE_GAPS.md** — every gap-log entry naming "phase-granular timings", "OnOpponentSecurityRemoved", "OnDigivolutionCardTrashed", "OnEnterFieldAnyone global", etc. gets a "Closed by Phase 1 (2026-04-19)" annotation.
- [ ] **16.3 Commit**

```bash
git add docs/RUST_ENGINE_API.md docs/RUST_ENGINE_GAPS.md
git commit -m "docs(engine): Phase 1 timing-dispatch API + close gap entries"
```

---

## Self-Review Checklist

- [x] **Spec coverage:** Every item from the roadmap's §Cluster B list has a task.
- [x] **No placeholders:** Each task has concrete file paths + target methods. Open questions (e.g. "does StartOfOpponentsTurn fire?") are tagged for the implementer to resolve by reading Python.
- [x] **Type consistency:** Uses the `enqueue_triggered(timing, TriggerSource) + drain_effect_queue()` idiom everywhere. Same `PlayerBattleArea(PlayerId)` semantic.
- [x] **Task independence:** Tasks 2–15 are independent of each other after Task 1 lands the variants + builders. They can be executed in any order.

## Verification

After all tasks complete:

1. `cargo test --manifest-path digimon-engine/Cargo.toml` — all tests pass, including the new timing_dispatch suite.
2. `cargo test --manifest-path digimon-engine/Cargo.toml --test timing_dispatch` — 14 new tests pass (one per wired timing).
3. `docs/RUST_ENGINE_GAPS.md` — all timing-dispatch entries annotated closed.
