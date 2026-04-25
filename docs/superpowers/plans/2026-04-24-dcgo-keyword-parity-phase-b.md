# DCGO Keyword Parity — Phase B Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the source-attribution substrate from spec §4.1 — gate every opponent-sourced mutation entry point on `Game::progress_excludes`, and expose the deletion cause to `OnDeletion` observers so Phase E keywords (Retaliation, Scapegoat, Mephistomon-style riders) can be authored.

**Architecture:** All gates apply at the `EffectContext` script-API layer where the source player is known statically (`self.player`). Game-level fire-sites (`delete_permanent_with_effects`, `return_to_hand`, `return_to_deck`, `de_digivolve`, `Game::suspend`) stay agnostic — they're called from rule-driven paths (cleanup, security check, battle resolution) that must NOT be gated by Progress. The `OnDeletion` cause is exposed via a new `Game::current_deletion_cause: Option<ReplacementCause>` slot, set by `delete_permanent_with_cause` immediately before `enqueue_triggered(OnDeletion, ...)` and cleared after the drain.

**Tech Stack:** Rust 2021, `cargo test --manifest-path digimon-engine/Cargo.toml`, `DebugRunner` test harness.

---

## Background — read before starting

- **Spec:** `docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md` §4.1 (source-attribution thread) and §5 Phase B.
- **Phase A landed:** `Game::current_attacker`, `Game::progress_excludes(target, source)`, gating in `select_opponent_permanent`, `Game::security_attack_keyword_bonus`, `Keyword::Blast → BlastDigivolve` rename, `Keyword::MaterialSave(u8)` split, dead-variant cleanup. See commits `616f22ab..2e1bb30f` for reference patterns.
- **Pre-existing infrastructure (do not re-derive):**
  - `Game::infer_effect_cause(target_player) -> ReplacementCause` reads `effect_source_player` and returns `OwnEffect`/`OpponentEffect` relative to `target_player`. Used by every `try_replace` fire-site.
  - `Game::infer_deletion_cause(target_handle)` is the deletion-specific variant (adds Battle / SecurityCheck branches).
  - `Modifier.source_player: PlayerId` is already populated by `EffectContext::add_modifier` etc. via `self.player`.
  - `Game::progress_excludes(target, source)` returns `true` only when `target` is the current attacker, has Progress (printed or granted, or `ImmunityToOpponentEffects` modifier), AND `source != target.player`. Returning `false` for a `None` source is intentional — rule-driven mutations (battle, cost, security check) are NOT gated.
- **Pre-existing pattern (test seam):** `EffectContext::new(&mut game, CardHandle(0), None, selecting_player)` constructs a context impersonating any player. See `tests/combat/progress_partial.rs::select_opponent_permanent_excludes_progress_attacker` for the canonical example. This is how Phase A tested "opponent-sourced effect" without driving a real card script.
- **Bypass note:** `EffectContext::delete_permanent` (in `effect_context/mod.rs:442`) currently calls `player.delete_permanent(idx)` directly, bypassing `Game::delete_permanent_with_effects` and therefore skipping OnDeletion + replacement windows. Task 2 fixes this by routing through the Game-level fire-site. This is a structural prerequisite for Progress gating to bite at the script-API layer (and also a pre-existing OnDeletion-skipping bug; the fix lands here as part of Phase B).

---

## File Structure

**Modified files:**
- `digimon-engine/src/game.rs` — add `current_deletion_cause` slot + `Game::opponent_sourced_mutation` helper.
- `digimon-engine/src/combat.rs` — set/clear `current_deletion_cause` around the OnDeletion enqueue+drain in `commit_permanent_deletion`.
- `digimon-engine/src/effect_context/mod.rs` — re-route `EffectContext::delete_permanent` through `delete_permanent_with_cause`; gate `delete_permanent`, `return_to_hand`, `return_to_deck`, `de_digivolve`, `suspend`, `add_dp_modifier`/`add_modifier` on `progress_excludes`; add `ctx.deletion_cause()` / `ctx.was_deleted_by_effect()` / `ctx.was_deleted_by_opponent()` accessors.
- `docs/DCGO_KEYWORD_PARITY.md` — flip Progress row from 🟡 partial to ✅; add summary entry for Phase B landing.
- `docs/RUST_ENGINE_GAPS.md` — mark "OnDeletion cause discriminator" resolved.
- `docs/RUST_PYTHON_PARITY.md` — confirm/extend the §2.5c Progress divergence note already added in Phase A.

**Created files:**
- `digimon-engine/tests/combat/progress_mutation_gates.rs` — behavioral tests for §B1–B4 mutation-site gating (delete, return-to-hand, return-to-deck, de-digivolve, suspend, negative DP).
- `digimon-engine/tests/combat/deletion_cause_observer.rs` — behavioral test for §B5 `ctx.was_deleted_by_effect()` / `was_deleted_by_opponent()`.

**Wired into `digimon-engine/tests/combat/main.rs`** (the integration-test entry point) — both new test files registered as modules.

---

## Task 1: Test seam — expose `effect_source_player` for tests

**Files:**
- Modify: `digimon-engine/src/game.rs`

Phase B tests need to simulate "an opponent's effect is currently resolving" without driving the queue. Add a `#[doc(hidden)] pub` setter so integration tests can flip the slot.

- [ ] **Step 1: Add the test seam to `Game`**

In `digimon-engine/src/game.rs`, immediately after the existing `effect_source_player` field's `pub(crate)` declaration (around line 211), add a setter:

```rust
    /// Test-only setter for `effect_source_player`. Production code MUST go
    /// through `run_queued_effect` (which sets/restores around the dispatch).
    /// Exposed `#[doc(hidden)] pub` so behavioral tests under
    /// `digimon-engine/tests/` can simulate "opponent effect currently
    /// resolving" without driving the queue.
    #[doc(hidden)]
    pub fn set_effect_source_player_for_test(
        &mut self,
        source: Option<crate::enums::PlayerId>,
    ) {
        self.effect_source_player = source;
    }
```

Find the insertion point with `Grep -n "effect_source_player: Option<PlayerId>" digimon-engine/src/game.rs`. Place the setter inside the `impl Game { ... }` block — search for `pub fn next_clockwise` (≈ line 582) and insert just above it.

- [ ] **Step 2: Build to check the symbol resolves**

Run: `cargo build --manifest-path digimon-engine/Cargo.toml`
Expected: clean build.

- [ ] **Step 3: Commit**

```bash
git add digimon-engine/src/game.rs
git commit -m "test: expose Game::set_effect_source_player_for_test seam for Phase B"
```

---

## Task 2: Re-route `EffectContext::delete_permanent` through the Game-level fire-site

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs:442-450`

Today `EffectContext::delete_permanent` calls `player.delete_permanent(idx)` directly, skipping OnDeletion + replacements. Phase B needs both. Re-route through `Game::delete_permanent_with_effects` (which infers the cause from live state). This is a no-op for cards that don't use OnDeletion, but it's a prerequisite for Progress gating to fire and for B5's deletion-cause observer to have a path to be set.

- [ ] **Step 1: Write the failing test**

Create the test module file `digimon-engine/tests/combat/progress_mutation_gates.rs` with:

```rust
//! Phase B §B1–B4 — Progress gates every opponent-sourced mutation entry point.
//!
//! Each test sets up a Progress carrier on player 0's field as the active
//! attacker, then has player 1 (opponent) drive a script-API mutation against
//! the carrier via `EffectContext`. The mutation must be skipped because of
//! the Progress gate.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
use digimon_engine::selection::{AttackState, AttackTarget, PendingAttack};

fn fighter(id: &str, dp: i32, keywords: Vec<Keyword>) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(dp),
        play_cost: 5,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// Build a runner with one Progress carrier on P0 (attacker) and one
/// opponent permanent on P1. Marks the Progress carrier as the active
/// attacker via a fake `PendingAttack` so `progress_excludes` engages.
/// Returns `(runner, progress_handle, opp_handle)`.
fn setup_progress_attacker() -> (
    DebugRunner,
    digimon_engine::permanent::PermanentHandle,
    digimon_engine::permanent::PermanentHandle,
) {
    let mut r = DebugRunner::builder()
        .add_card(fighter("PROG", 6000, vec![Keyword::Progress]))
        .add_card(fighter("OPP", 4000, vec![]))
        .start();
    let progress = r.place_on_field(0, "PROG", None);
    let opp = r.place_on_field(1, "OPP", None);
    r.game.pending_attack = Some(PendingAttack {
        attacker: progress,
        original_target: AttackTarget::Player(1),
        effective_target: AttackTarget::Player(1),
        is_blocked: false,
        blocker: None,
        is_vortex: false,
        is_overclock: false,
        cancelled: false,
        battle_occurred: false,
        return_phase: GamePhase::Main,
        state: AttackState::Declared,
        counter_depth: 0,
    });
    (r, progress, opp)
}

#[test]
fn opponent_effect_delete_does_not_remove_progress_attacker() {
    let (mut r, progress, _opp) = setup_progress_attacker();

    // Simulate "P1's effect is resolving" so infer_deletion_cause returns
    // OpponentEffect for a target on P0.
    r.game.set_effect_source_player_for_test(Some(1));

    {
        // Opponent (P1) script API.
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.delete_permanent(progress);
    }

    r.game.set_effect_source_player_for_test(None);

    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Progress attacker must survive opponent-effect delete"
    );
}
```

Wire the new module into `digimon-engine/tests/combat/main.rs` — open the file and add `mod progress_mutation_gates;` alongside the existing `mod` declarations.

- [ ] **Step 2: Run the test to confirm it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test combat opponent_effect_delete_does_not_remove_progress_attacker -- --nocapture`
Expected: FAIL — assertion fires because today's `EffectContext::delete_permanent` bypasses both the cause-inference fire-site AND any progress gate.

- [ ] **Step 3: Re-route `EffectContext::delete_permanent` through `Game::delete_permanent_with_effects` and add the Progress gate**

In `digimon-engine/src/effect_context/mod.rs`, replace the body of `EffectContext::delete_permanent` (currently at line 442):

```rust
    pub fn delete_permanent(&mut self, target: PermanentHandle) {
        // Phase B §B4: gate opponent-sourced effect deletes on Progress.
        // Source is statically known here: `self.player` is the controller of
        // the running effect.
        if self.game.progress_excludes(target, Some(self.player)) {
            return;
        }
        // Route through the Game-level fire-site so OnDeletion observers and
        // WhenWouldBeDeleted replacements run. `delete_permanent_with_effects`
        // infers cause from `effect_source_player` / `pending_attack` /
        // `security_resolution`.
        self.game.delete_permanent_with_effects(target);
    }
```

- [ ] **Step 4: Run the failing test plus the broader combat test set**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test combat`
Expected: the new test passes. Watch for regressions in existing combat tests — `delete_permanent` was previously a fire-and-forget path; routing through the Game-level fire-site makes it OnDeletion-firing for the first time. If a test fails due to OnDeletion now firing where it didn't before, that's a real correctness improvement — investigate the test, not the code.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/effect_context/mod.rs digimon-engine/tests/combat/progress_mutation_gates.rs digimon-engine/tests/combat/main.rs
git commit -m "engine: route ctx.delete_permanent through Game fire-site + Progress gate"
```

---

## Task 3: Gate `EffectContext::return_to_hand` and `return_to_deck` on Progress

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs:872-886`
- Modify: `digimon-engine/tests/combat/progress_mutation_gates.rs`

`Game::return_to_hand` / `Game::return_to_deck` already fire `WhenWouldLeaveBattleArea` + the route-specific timing; they just need the Progress gate at the script-API entry point.

- [ ] **Step 1: Append the failing tests**

Add to `digimon-engine/tests/combat/progress_mutation_gates.rs`:

```rust
#[test]
fn opponent_effect_return_to_hand_does_not_bounce_progress_attacker() {
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        let _ = ctx.return_to_hand(progress);
    }
    r.game.set_effect_source_player_for_test(None);
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Progress attacker must survive opponent-effect return-to-hand"
    );
    assert!(
        r.game.players[0].hand.is_empty(),
        "no card returned to hand"
    );
}

#[test]
fn opponent_effect_return_to_deck_does_not_bounce_progress_attacker() {
    use digimon_engine::enums::StackPosition;
    let (mut r, progress, _opp) = setup_progress_attacker();
    let deck_size_before = r.game.players[0].deck.len();
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        let _ = ctx.return_to_deck(progress, StackPosition::Bottom);
    }
    r.game.set_effect_source_player_for_test(None);
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Progress attacker must survive opponent-effect return-to-deck"
    );
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_size_before,
        "deck size unchanged"
    );
}
```

- [ ] **Step 2: Run the new tests; confirm both fail**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test combat opponent_effect_return_to_ -- --nocapture`
Expected: both fail.

- [ ] **Step 3: Add the Progress gate to both wrappers**

In `digimon-engine/src/effect_context/mod.rs`, replace the existing wrappers (around lines 872 and 880):

```rust
    /// Bounce a permanent to its owner's hand. See `Game::return_to_hand`.
    /// Phase B §B4: gated on Progress when the target is opponent-controlled.
    pub fn return_to_hand(
        &mut self,
        target: PermanentHandle,
    ) -> Option<crate::card_source::CardHandle> {
        if self.game.progress_excludes(target, Some(self.player)) {
            return None;
        }
        self.game.return_to_hand(target)
    }

    /// Return a permanent's top card to its owner's deck. See `Game::return_to_deck`.
    /// Phase B §B4: gated on Progress when the target is opponent-controlled.
    pub fn return_to_deck(
        &mut self,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
    ) -> bool {
        if self.game.progress_excludes(target, Some(self.player)) {
            return false;
        }
        self.game.return_to_deck(target, position)
    }
```

- [ ] **Step 4: Re-run the targeted tests**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test combat opponent_effect_return_to_ -- --nocapture`
Expected: both pass.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/effect_context/mod.rs digimon-engine/tests/combat/progress_mutation_gates.rs
git commit -m "engine: Progress-gate ctx.return_to_hand / return_to_deck"
```

---

## Task 4: Gate `EffectContext::de_digivolve` on Progress

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs:467-555`
- Modify: `digimon-engine/tests/combat/progress_mutation_gates.rs`

`de_digivolve` is one of the few `EffectContext` mutators that already fires `try_replace`. Add the Progress gate immediately before the try_replace call so we never even open the replacement window for an excluded target.

- [ ] **Step 1: Append the failing test**

Add to `digimon-engine/tests/combat/progress_mutation_gates.rs`:

```rust
#[test]
fn opponent_effect_de_digivolve_does_not_pop_progress_attacker_stack() {
    // Build a Progress carrier with two stack sources so de_digivolve has
    // something to pop. We layer a second source manually because Phase B
    // doesn't depend on the digivolve action path.
    use digimon_engine::card_source::CardSource;
    let mut r = DebugRunner::builder()
        .add_card(fighter("PROG", 6000, vec![Keyword::Progress]))
        .add_card(fighter("BOTTOM", 2000, vec![]))
        .add_card(fighter("OPP", 4000, vec![]))
        .start();
    let progress = r.place_on_field(0, "PROG", None);
    let _opp = r.place_on_field(1, "OPP", None);
    // Inject a second card under the top so the stack has 2 sources.
    {
        let bottom_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "BOTTOM")
            .unwrap();
        let next = r.game.next_card_index();
        let bottom_card = CardSource::new(bottom_idx, 0, next);
        let perm = &mut r.game.players[0].battle_area[progress.index as usize];
        perm.card_sources.insert(0, bottom_card);
    }
    let stack_size_before = r.game.players[0].battle_area[progress.index as usize]
        .card_sources
        .len();

    r.game.pending_attack = Some(PendingAttack {
        attacker: progress,
        original_target: AttackTarget::Player(1),
        effective_target: AttackTarget::Player(1),
        is_blocked: false,
        blocker: None,
        is_vortex: false,
        is_overclock: false,
        cancelled: false,
        battle_occurred: false,
        return_phase: GamePhase::Main,
        state: AttackState::Declared,
        counter_depth: 0,
    });
    r.game.set_effect_source_player_for_test(Some(1));
    let popped = {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.de_digivolve(progress, None, Some(1))
    };
    r.game.set_effect_source_player_for_test(None);

    assert_eq!(popped, 0, "de_digivolve must report 0 pops on Progress carrier");
    assert_eq!(
        r.game.players[0].battle_area[progress.index as usize]
            .card_sources
            .len(),
        stack_size_before,
        "Progress attacker stack must be unchanged"
    );
}
```

- [ ] **Step 2: Confirm the test fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test combat opponent_effect_de_digivolve_does_not_pop_progress_attacker_stack -- --nocapture`
Expected: FAIL — `popped >= 1`.

- [ ] **Step 3: Add the Progress gate to `de_digivolve`**

In `digimon-engine/src/effect_context/mod.rs`, modify the `de_digivolve` function. Find this block (around line 472):

```rust
    pub fn de_digivolve(
        &mut self,
        target: PermanentHandle,
        stop_at_level: Option<u8>,
        amount: Option<u8>,
    ) -> u8 {
        use crate::enums::EffectTiming;
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // Phase 7 Task 4: fire WhenWouldBeDeDigivolved once at entry (not
```

Insert the Progress gate as the first statement in the body, BEFORE the existing `use` statements:

```rust
    pub fn de_digivolve(
        &mut self,
        target: PermanentHandle,
        stop_at_level: Option<u8>,
        amount: Option<u8>,
    ) -> u8 {
        // Phase B §B4: opponent-sourced de-digivolve on a Progress attacker
        // is suppressed before any replacement window opens.
        if self.game.progress_excludes(target, Some(self.player)) {
            return 0;
        }

        use crate::enums::EffectTiming;
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // Phase 7 Task 4: fire WhenWouldBeDeDigivolved once at entry (not
```

- [ ] **Step 4: Re-run the test**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test combat opponent_effect_de_digivolve_does_not_pop_progress_attacker_stack -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/effect_context/mod.rs digimon-engine/tests/combat/progress_mutation_gates.rs
git commit -m "engine: Progress-gate ctx.de_digivolve"
```

---

## Task 5: Gate `EffectContext::suspend` on Progress

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs:609-611`
- Modify: `digimon-engine/tests/combat/progress_mutation_gates.rs`

`Game::suspend` is the canonical single-target chokepoint and is also called from rule paths (battle, OnAttack triggers). Don't gate the Game-level method — gate the script-API wrapper.

- [ ] **Step 1: Append the failing test**

Add to `digimon-engine/tests/combat/progress_mutation_gates.rs`:

```rust
#[test]
fn opponent_effect_suspend_does_not_suspend_progress_attacker() {
    let (mut r, progress, _opp) = setup_progress_attacker();
    // Confirm starting state: attacker is unsuspended (the fake PendingAttack
    // does not flip is_suspended; placement defaults to unsuspended).
    assert!(
        !r.game.players[0].battle_area[progress.index as usize].is_suspended,
        "precondition: attacker starts unsuspended"
    );

    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.suspend(progress);
    }
    r.game.set_effect_source_player_for_test(None);

    assert!(
        !r.game.players[0].battle_area[progress.index as usize].is_suspended,
        "Progress attacker must not be suspended by opponent effect"
    );
}
```

- [ ] **Step 2: Run the test; confirm it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test combat opponent_effect_suspend_does_not_suspend_progress_attacker -- --nocapture`
Expected: FAIL.

- [ ] **Step 3: Add the gate to `EffectContext::suspend`**

In `digimon-engine/src/effect_context/mod.rs`, replace the existing `suspend` wrapper (around line 609):

```rust
    /// Suspend a permanent and fire `OnSuspend` observers.
    /// Delegates to `Game::suspend` — the canonical single-target chokepoint.
    /// Phase B §B4: gated on Progress when the target is opponent-controlled.
    pub fn suspend(&mut self, target: PermanentHandle) {
        if self.game.progress_excludes(target, Some(self.player)) {
            return;
        }
        self.game.suspend(target);
    }
```

- [ ] **Step 4: Re-run the test**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test combat opponent_effect_suspend_does_not_suspend_progress_attacker -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/effect_context/mod.rs digimon-engine/tests/combat/progress_mutation_gates.rs
git commit -m "engine: Progress-gate ctx.suspend"
```

---

## Task 6: Gate negative-DP `EffectContext::add_dp_modifier` on Progress

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs:913-923`
- Modify: `digimon-engine/tests/combat/progress_mutation_gates.rs`

Negative DP from opponent effects is the most common Progress vector in real cards (pre-attack -DP combat tricks). Gate `add_dp_modifier` when `value < 0` AND target is opponent-controlled. Also gate the more general `add_modifier` for the `ChangeDp` modifier type with `value < 0` (so card scripts that go through the generic API are also covered).

Positive DP and non-DP modifiers (Blocker grants etc.) are NOT gated — Progress is "cannot be affected by opponent's effects" but DCGO's actual implementation only blocks effects that target the carrier negatively. Cross-check: DCGO's `CanNotAffectedClass` filters at target-selection time, so any effect that passes selection (positive grants, all-board buffs that incidentally hit the carrier, etc.) still applies. We mirror that here by gating negatives only.

- [ ] **Step 1: Append the failing test**

Add to `digimon-engine/tests/combat/progress_mutation_gates.rs`:

```rust
#[test]
fn opponent_effect_negative_dp_does_not_apply_to_progress_attacker() {
    use digimon_engine::enums::{Expiry, ModifierType};
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.add_dp_modifier(progress, -3000, Expiry::EndOfTurn);
    }
    r.game.set_effect_source_player_for_test(None);
    let dp_sum = r.game.modifiers.sum(progress, ModifierType::ChangeDp);
    assert_eq!(
        dp_sum, 0,
        "Progress attacker must not receive opponent-effect -DP modifier; \
         got accumulated ChangeDp = {}",
        dp_sum
    );
}

#[test]
fn opponent_effect_positive_dp_still_applies_to_progress_attacker() {
    // Sanity check: Progress only excludes opponent effects that target the
    // carrier negatively (matching DCGO's CanNotAffectedClass semantics for
    // hostile effects). A positive DP buff from an opponent effect — rare but
    // possible via card text like "Your opponent's Digimon gets +1000 DP" —
    // still applies.
    use digimon_engine::enums::{Expiry, ModifierType};
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.add_dp_modifier(progress, 1000, Expiry::EndOfTurn);
    }
    r.game.set_effect_source_player_for_test(None);
    let dp_sum = r.game.modifiers.sum(progress, ModifierType::ChangeDp);
    assert_eq!(dp_sum, 1000, "positive DP buffs are not gated by Progress");
}
```

- [ ] **Step 2: Run both tests; first must fail, second must already pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test combat opponent_effect_negative_dp opponent_effect_positive_dp -- --nocapture`
Expected: `negative_dp` test FAILs, `positive_dp` test PASSes.

- [ ] **Step 3: Add the Progress gate to `add_dp_modifier` and `add_modifier`**

In `digimon-engine/src/effect_context/mod.rs`, replace the body of `add_dp_modifier` (around line 913):

```rust
    pub fn add_dp_modifier(&mut self, target: PermanentHandle, value: i32, expiry: Expiry) {
        // Phase B §B4: negative DP from opponent effects is gated by Progress.
        // Positive grants are not gated — see `progress_excludes` doc and the
        // `opponent_effect_positive_dp_still_applies_to_progress_attacker` test.
        if value < 0 && self.game.progress_excludes(target, Some(self.player)) {
            return;
        }
        self.game.modifiers.add(
            target,
            ModifierEntry::simple(
                ModifierType::ChangeDp,
                value,
                expiry,
                self.player,
            ),
        );
    }
```

And update `add_modifier` (the generic API at ~line 925) to gate the `ChangeDp` case symmetrically:

```rust
    pub fn add_modifier(
        &mut self,
        target: PermanentHandle,
        modifier: ModifierType,
        value: i32,
        expiry: Expiry,
    ) {
        // Phase B §B4: route ChangeDp through the same negative-DP Progress
        // gate as `add_dp_modifier`. Other modifier types are not gated here
        // (cross-check against DCGO before extending the gate to additional
        // ModifierType variants).
        if modifier == ModifierType::ChangeDp
            && value < 0
            && self.game.progress_excludes(target, Some(self.player))
        {
            return;
        }
        self.game.modifiers.add(
            target,
            ModifierEntry::simple(
                modifier,
                value,
                expiry,
                self.player,
            ),
        );
    }
```

- [ ] **Step 4: Re-run both tests**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test combat opponent_effect_negative_dp opponent_effect_positive_dp -- --nocapture`
Expected: both pass.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/effect_context/mod.rs digimon-engine/tests/combat/progress_mutation_gates.rs
git commit -m "engine: Progress-gate negative DP on ctx.add_dp_modifier / add_modifier"
```

---

## Task 7: Add own-sourced regression tests

**Files:**
- Modify: `digimon-engine/tests/combat/progress_mutation_gates.rs`

The whole point of `progress_excludes(target, source)` returning `false` when `source == target.player` is that own-sourced effects must still apply. Lock this down with one test per gated mutation entry point so a future change that broadens the gate is caught immediately.

- [ ] **Step 1: Append the regression tests**

Add to `digimon-engine/tests/combat/progress_mutation_gates.rs`:

```rust
#[test]
fn own_effect_delete_still_removes_progress_attacker() {
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(0));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.delete_permanent(progress);
    }
    r.game.set_effect_source_player_for_test(None);
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "own-sourced delete must still apply to a Progress attacker"
    );
}

#[test]
fn own_effect_negative_dp_still_applies_to_progress_attacker() {
    use digimon_engine::enums::{Expiry, ModifierType};
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(0));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.add_dp_modifier(progress, -1000, Expiry::EndOfTurn);
    }
    r.game.set_effect_source_player_for_test(None);
    assert_eq!(
        r.game.modifiers.sum(progress, ModifierType::ChangeDp),
        -1000,
        "own-sourced -DP must still apply to Progress carrier"
    );
}

#[test]
fn rule_driven_delete_still_removes_progress_attacker() {
    // No EffectContext, no script-API mutation — direct Game-level call
    // simulates a rule-driven cleanup (e.g. cost-payment cascade). Source is
    // None; progress_excludes returns false; deletion proceeds.
    let (mut r, progress, _opp) = setup_progress_attacker();
    // effect_source_player stays None.
    r.game.delete_permanent_with_effects(progress);
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "rule-driven (None-source) delete must still remove Progress attacker"
    );
}
```

- [ ] **Step 2: Run all three; expect all pass on first run**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test combat own_effect rule_driven_delete -- --nocapture`
Expected: all pass.

If any fail, the gate is too aggressive — go back and re-check `progress_excludes` is honoring the `source == target.player` and `source == None` exemptions.

- [ ] **Step 3: Commit**

```bash
git add digimon-engine/tests/combat/progress_mutation_gates.rs
git commit -m "test: regression coverage for own-sourced + rule-driven Progress non-exclusion"
```

---

## Task 8: Add `Game::current_deletion_cause` slot + plumb through `commit_permanent_deletion`

**Files:**
- Modify: `digimon-engine/src/game.rs` (struct field + setter)
- Modify: `digimon-engine/src/combat.rs:2232-2392` (set/clear around OnDeletion enqueue)

OnDeletion observers need to see the cause that triggered them. Add a slot, set it before the OnDeletion enqueue, clear it after the drain. Same panic-safe pattern as `in_replacement_commit` (use a guard so a panic during OnDeletion doesn't leak the slot).

- [ ] **Step 1: Add the field to `Game`**

In `digimon-engine/src/game.rs`, find the `Game` struct (search for `pub struct Game`). Add a new field next to `effect_source_player` (around line 211):

```rust
    /// The cause of the deletion currently being observed by `OnDeletion`
    /// effects. Set by `commit_permanent_deletion` immediately before
    /// `enqueue_triggered(OnDeletion, ...)`; cleared after the drain via the
    /// `commit_deletion_cause_guard` RAII helper. Read by
    /// `EffectContext::deletion_cause()` / `was_deleted_by_effect()` /
    /// `was_deleted_by_opponent()`.
    ///
    /// `None` outside an OnDeletion observer body. Phase B §B5.
    #[doc(hidden)]
    pub(crate) current_deletion_cause: Option<crate::replacement::ReplacementCause>,
```

In the `Game::new` constructor's struct-literal initialization (search for `effect_source_player: None,` around line 351), add the new field's initializer:

```rust
            effect_source_player: None,
            current_deletion_cause: None,
```

- [ ] **Step 2: Set/clear around the OnDeletion enqueue**

In `digimon-engine/src/combat.rs`, find `commit_permanent_deletion` (around line 2312). Today it does:

```rust
    fn commit_permanent_deletion(&mut self, handle: PermanentHandle) {
        self.enqueue_triggered(
            crate::enums::EffectTiming::OnDeletion,
            crate::selection::TriggerSource::Permanent(handle),
        );
        self.drain_effect_queue();
        // ... linked-card cascade, removal, modifier cleanup, OnAnyDeletion ...
    }
```

The function only knows `handle`, not the cause. Move the cause-aware work into `delete_permanent_with_cause`'s `ReplacementOutcome::None` arm (around line 2270). Before invoking `commit_permanent_deletion`, set the slot; after, clear it. Use a panic-safe scope.

Modify `delete_permanent_with_cause` so the `None` outcome path looks like this (search for `ReplacementOutcome::None => {` near line 2270):

```rust
        match outcome {
            ReplacementOutcome::None => {
                // Phase B §B5: expose the cause to OnDeletion observers via
                // `current_deletion_cause`. Set before the enqueue; cleared
                // after the drain via panic-safe guard.
                let prior = self.current_deletion_cause;
                self.current_deletion_cause = Some(cause);
                let result = std::panic::catch_unwind(
                    std::panic::AssertUnwindSafe(|| self.commit_permanent_deletion(handle)),
                );
                self.current_deletion_cause = prior;
                if let Err(payload) = result {
                    std::panic::resume_unwind(payload);
                }
            }
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                // ... existing arm unchanged ...
            }
            // ... remaining arms unchanged ...
        }
```

The `Substituted(Permanent(other))` arm calls `delete_permanent_with_cause(other, cause)` recursively; the recursion will set the slot to the same cause for the substituted target's OnDeletion. No additional plumbing needed there.

The `Redirected(Zone::Deck)` / `Redirected(Zone::Hand)` arms route to `return_to_deck` / `return_to_hand` instead of deleting; OnDeletion does NOT fire for those, so the slot stays unset for them. No change needed.

- [ ] **Step 3: Build to confirm compile**

Run: `cargo build --manifest-path digimon-engine/Cargo.toml`
Expected: clean build.

- [ ] **Step 4: Run combat tests; expect no regressions**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test combat`
Expected: all pass (no observer reads the slot yet, so this is a no-op behaviorally; we're just plumbing).

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/game.rs digimon-engine/src/combat.rs
git commit -m "engine: plumb current_deletion_cause through commit_permanent_deletion"
```

---

## Task 9: Expose deletion-cause accessors on `EffectContext` / `EffectReadContext`

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs` (both `EffectReadContext` and `EffectContext`)
- Create: `digimon-engine/tests/combat/deletion_cause_observer.rs`
- Modify: `digimon-engine/tests/combat/main.rs`

This is the consumer-facing API. Phase E (Retaliation, Scapegoat) and any future `OnDeletion`-bound script will read these.

- [ ] **Step 1: Write the failing test**

Create `digimon-engine/tests/combat/deletion_cause_observer.rs`:

```rust
//! Phase B §B5 — `ctx.was_deleted_by_effect()` / `was_deleted_by_opponent()`
//! report the correct cause to `OnDeletion` observers.
//!
//! These accessors unblock Phase E (Retaliation = "only fires on battle
//! deletion"; Scapegoat = "only fires when not OwnEffect").

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::replacement::ReplacementCause;

fn fighter(id: &str, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(dp),
        play_cost: 5,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

#[test]
fn current_deletion_cause_set_during_opponent_effect_delete() {
    // Dispatcher-level proof: drive an opponent-effect delete and snapshot
    // `Game::current_deletion_cause` during the OnDeletion enqueue path. We
    // observe it via the public accessor we're adding (`ctx.deletion_cause()`)
    // by manually peeking — full observer dispatch is exercised in the next
    // test once the accessor exists.
    let mut r = DebugRunner::builder()
        .add_card(fighter("VICTIM", 4000))
        .start();
    let victim = r.place_on_field(0, "VICTIM", None);

    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.delete_permanent(victim);
    }
    r.game.set_effect_source_player_for_test(None);

    // After the deletion completes, `current_deletion_cause` is cleared back
    // to None (the slot is scoped to the OnDeletion drain).
    assert_eq!(r.game.players[0].battle_area.len(), 0, "victim deleted");
    // We can still inspect the inferred cause directly.
    // (The slot-during-drain assertion lives in the in-OnDeletion test below
    // once the accessors exist.)
    let _ = ReplacementCause::OpponentEffect; // silence unused
}
```

Wire the new module into `digimon-engine/tests/combat/main.rs` — add `mod deletion_cause_observer;`.

- [ ] **Step 2: Run; expect pass (no behavior assertion yet)**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test combat current_deletion_cause_set_during_opponent_effect_delete -- --nocapture`
Expected: PASS — this just exercises the plumbing from Task 8.

- [ ] **Step 3: Add the accessors to `EffectReadContext` and `EffectContext`**

In `digimon-engine/src/effect_context/mod.rs`, add to the `impl<'a> EffectReadContext<'a>` block (after `turn_player_at_check`, around line 145):

```rust
    /// The `ReplacementCause` of the deletion currently being observed by
    /// this `OnDeletion` (or `OnAnyDeletion`) effect. `None` outside such an
    /// observer body. Phase B §B5.
    pub fn deletion_cause(&self) -> Option<crate::replacement::ReplacementCause> {
        self.game.current_deletion_cause
    }

    /// `true` when the current OnDeletion observer is firing because of an
    /// effect (own or opponent), as opposed to battle / security-check / cost.
    /// Convenience for keywords like Scapegoat (cause ≠ OwnEffect) and
    /// Retaliation (cause == Battle, hence false here).
    pub fn was_deleted_by_effect(&self) -> bool {
        use crate::replacement::ReplacementCause;
        matches!(
            self.game.current_deletion_cause,
            Some(ReplacementCause::OwnEffect | ReplacementCause::OpponentEffect)
        )
    }

    /// `true` when the current OnDeletion observer is firing because of an
    /// opponent's effect specifically. Drives Mephistomon-style "when this is
    /// deleted by your opponent's effect" riders.
    pub fn was_deleted_by_opponent(&self) -> bool {
        matches!(
            self.game.current_deletion_cause,
            Some(crate::replacement::ReplacementCause::OpponentEffect)
        )
    }
```

Then add the same three accessors to `impl<'a> EffectContext<'a>` (in the same file). The mut-context version delegates to the read-context view via `self.as_read()`:

```rust
    /// See `EffectReadContext::deletion_cause`.
    pub fn deletion_cause(&self) -> Option<crate::replacement::ReplacementCause> {
        self.game.current_deletion_cause
    }

    /// See `EffectReadContext::was_deleted_by_effect`.
    pub fn was_deleted_by_effect(&self) -> bool {
        use crate::replacement::ReplacementCause;
        matches!(
            self.game.current_deletion_cause,
            Some(ReplacementCause::OwnEffect | ReplacementCause::OpponentEffect)
        )
    }

    /// See `EffectReadContext::was_deleted_by_opponent`.
    pub fn was_deleted_by_opponent(&self) -> bool {
        matches!(
            self.game.current_deletion_cause,
            Some(crate::replacement::ReplacementCause::OpponentEffect)
        )
    }
```

Place the EffectContext copies near the existing security-check sugar (around line 285, before `as_read`).

- [ ] **Step 4: Append the in-observer behavioral test**

This test registers an ad-hoc card effect with a process that captures the cause into a shared cell. The cleanest pattern uses `std::sync::atomic::AtomicI32` (or a `Mutex<Option<ReplacementCause>>`) shared across the test thread and the observer closure.

Actually, registering ad-hoc CardEffect impls per test is awkward. Lean on a simpler verification: drive the deletion, then directly read `r.game.current_deletion_cause` snapshot via a hook. Since the slot is cleared after the drain, "during the drain" requires intercepting from inside an OnDeletion observer.

Use the existing `cards/test_cards.rs` infrastructure if it has a card with an OnDeletion hook. If none does, write a minimal `#[cfg(test)]` shim card. Add to `digimon-engine/tests/combat/deletion_cause_observer.rs`:

```rust
use std::sync::{Arc, Mutex};

#[test]
fn was_deleted_by_opponent_true_inside_ondeletion_for_opp_effect_delete() {
    use digimon_engine::card_registry::register_card_effect;
    use digimon_engine::cards::CardEffect;
    use digimon_engine::effect::Effect;
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::effect_context::EffectContext as Ctx;

    // Shared cell the OnDeletion observer writes into.
    let captured: Arc<Mutex<Option<digimon_engine::replacement::ReplacementCause>>> =
        Arc::new(Mutex::new(None));

    // Register an ad-hoc card effect for VICTIM with one OnDeletion observer.
    let captured_for_effect = Arc::clone(&captured);
    struct VictimEffect {
        captured: Arc<Mutex<Option<digimon_engine::replacement::ReplacementCause>>>,
    }
    impl CardEffect for VictimEffect {
        fn build(&self) -> Vec<Effect> {
            let captured = Arc::clone(&self.captured);
            vec![Effect::on_deletion(move |ctx: &mut Ctx<'_>| {
                let cause = ctx.deletion_cause();
                let _ = captured.lock().unwrap().insert(cause.unwrap_or(
                    digimon_engine::replacement::ReplacementCause::OwnEffect,
                ));
            })]
        }
    }
    register_card_effect(
        "VICTIM_OBS",
        Box::new(VictimEffect {
            captured: captured_for_effect,
        }),
    );

    let mut r = DebugRunner::builder()
        .add_card(CardData {
            effect_class_name: "VICTIM_OBS".to_string(),
            ..fighter("VICTIM_OBS", 4000)
        })
        .start();
    let victim = r.place_on_field(0, "VICTIM_OBS", None);

    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.delete_permanent(victim);
    }
    r.game.set_effect_source_player_for_test(None);

    let observed = captured.lock().unwrap().clone();
    assert_eq!(
        observed,
        Some(digimon_engine::replacement::ReplacementCause::OpponentEffect),
        "OnDeletion observer must see OpponentEffect cause for opp-sourced delete"
    );
}
```

**Cross-check before writing this test:** the `register_card_effect`, `CardEffect`, and `Effect::on_deletion` API names above are best-guess based on the project layout. If your build fails because these don't exist exactly as written, search the codebase for the canonical names:

```bash
# What's the actual trait + registration helper called?
rg -n "trait CardEffect|register_card_effect|fn on_deletion" digimon-engine/src
# What does an existing OnDeletion test use?
rg -n "EffectTiming::OnDeletion" digimon-engine/tests
```

Then mirror the patterns from `digimon-engine/src/cards/test_cards.rs` (TEST-001..022 are hand-written examples of CardEffect impls) and use whatever helper name lines up. The exact symbol names are stable enough across Phase 7 that the search-and-mirror should be a 5-minute job.

- [ ] **Step 5: Run the in-observer test**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test combat was_deleted_by_opponent_true_inside_ondeletion -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/effect_context/mod.rs digimon-engine/tests/combat/deletion_cause_observer.rs digimon-engine/tests/combat/main.rs
git commit -m "engine: expose ctx.deletion_cause / was_deleted_by_effect / was_deleted_by_opponent"
```

---

## Task 10: Add `Game::opponent_sourced_mutation` helper (audit aid)

**Files:**
- Modify: `digimon-engine/src/game.rs`

The spec calls out `opponent_sourced_mutation(target)` as a helper future Phase D + E auto-installs will use to check "is the currently-resolving effect an opponent's effect targeting `target`?" without re-deriving the comparison. Build it now so consumers in later phases pick it up for free.

- [ ] **Step 1: Add the helper next to `progress_excludes`**

In `digimon-engine/src/game.rs`, immediately after `pub fn progress_excludes(...)` (around line 947), add:

```rust
    /// Returns `true` when an effect is currently resolving AND its
    /// controller is not `target`'s controller. The "opponent effect is
    /// targeting me" predicate that drives Mephistomon-style OnDeletion
    /// riders, Scapegoat eligibility (cause ≠ OwnEffect), and the
    /// `was_deleted_by_opponent` accessor.
    ///
    /// Returns `false` when:
    ///   - no effect is currently resolving (`effect_source_player == None`),
    ///   - the resolving effect's controller equals `target.player`.
    ///
    /// Phase B §B5.
    pub fn opponent_sourced_mutation(
        &self,
        target: crate::permanent::PermanentHandle,
    ) -> bool {
        match self.effect_source_player {
            Some(src) => src != target.player,
            None => false,
        }
    }
```

- [ ] **Step 2: Add a unit test**

Append to the existing `mod current_attacker_tests` block at the bottom of `digimon-engine/src/game.rs`:

```rust
    #[test]
    fn opponent_sourced_mutation_only_when_effect_source_differs() {
        let mut r = DebugRunner::builder()
            .add_card(card("A"))
            .add_card(card("B"))
            .start();
        let a = r.place_on_field(0, "A", None);
        let _b = r.place_on_field(1, "B", None);

        // No effect resolving → false.
        assert!(!r.game.opponent_sourced_mutation(a));

        // Own effect resolving → false.
        r.game.set_effect_source_player_for_test(Some(0));
        assert!(!r.game.opponent_sourced_mutation(a));

        // Opponent effect resolving → true.
        r.game.set_effect_source_player_for_test(Some(1));
        assert!(r.game.opponent_sourced_mutation(a));

        r.game.set_effect_source_player_for_test(None);
    }
```

- [ ] **Step 3: Run the unit test**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --lib opponent_sourced_mutation -- --nocapture`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add digimon-engine/src/game.rs
git commit -m "engine: add Game::opponent_sourced_mutation helper for Phase D/E consumers"
```

---

## Task 11: Documentation updates

**Files:**
- Modify: `docs/DCGO_KEYWORD_PARITY.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `docs/RUST_PYTHON_PARITY.md`
- Modify: `docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md` (status section)
- Modify: `docs/RUST_ENGINE_API.md` (new `EffectContext` accessors)

- [ ] **Step 1: Flip Progress to ✅ in DCGO_KEYWORD_PARITY.md**

Open `docs/DCGO_KEYWORD_PARITY.md`. Find the Progress row in the summary table (currently 🟡 Divergent / Partial). Change to ✅ with a parenthetical note: "(Phase B: gated at ctx.delete_permanent / return_to_hand / return_to_deck / de_digivolve / suspend / negative DP)". Update the bucket counts at the top of §3 — Progress moves out of 🟡 into ✅.

Also update the Progress detailed section if one exists — describe the source-attribution model: gates apply at the EffectContext layer where source is known, Game-level fire-sites stay agnostic for rule-driven mutations.

- [ ] **Step 2: Mark `OnDeletion cause discriminator` resolved in RUST_ENGINE_GAPS.md**

Open `docs/RUST_ENGINE_GAPS.md`. Search for "OnDeletion cause discriminator" or similar phrasing. Mark the row Resolved with date `2026-04-24` and a one-liner pointing at `EffectContext::was_deleted_by_effect` / `was_deleted_by_opponent`.

- [ ] **Step 3: Update RUST_PYTHON_PARITY.md §2.5c (Progress divergence row)**

Phase A added a divergence row noting Rust gates Progress at selection time, Python skipped SecuritySkill. Extend it to note Phase B closed the mutation-site coverage so Rust now hard-gates: delete, return-to-hand, return-to-deck, de-digivolve, suspend, negative DP. Format consistent with neighboring rows. Confirm the "Status" column reads "Rust correct (Phase A + B); Python sunsetted".

- [ ] **Step 4: Update the Phase B status in the spec**

In `docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md`, locate the Phase B entry in §5. Mark deliverables B1–B5 as landed with commit ref `(claude/gracious-ptolemy-744e69)` or the actual branch name. Confirm the "Exit criteria" bullet matches what was actually tested (every opponent-mutation entry point gated; `ctx.was_deleted_by_effect` verified in observer test).

- [ ] **Step 5: Document new accessors in RUST_ENGINE_API.md**

Open `docs/RUST_ENGINE_API.md`. Find the `EffectContext` API section. Add documentation for the three new accessors — `deletion_cause()`, `was_deleted_by_effect()`, `was_deleted_by_opponent()` — with one example each showing typical usage (e.g. Retaliation skipping fire on cause=OwnEffect, Scapegoat eligibility check). Keep entries terse and pattern-mirroring — match the surrounding doc tone.

- [ ] **Step 6: Commit**

```bash
git add docs/DCGO_KEYWORD_PARITY.md docs/RUST_ENGINE_GAPS.md docs/RUST_PYTHON_PARITY.md docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md docs/RUST_ENGINE_API.md
git commit -m "docs: Phase B parity tracker + RUST_ENGINE_API + RUST_PYTHON_PARITY updates"
```

---

## Task 12: Final verification

Same shape as Phase A's final verification. No code changes; just the run-through.

- [ ] **Step 1: Full Rust engine test sweep**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: all tests pass except the known pre-existing `security_effects::two_security_effects_same_source_auto_fire_in_order` failure already flagged as a separate spawn task.

If a previously-passing test now fails, root-cause it. The most likely culprit is Task 2's re-route of `ctx.delete_permanent` through the Game-level fire-site — that flips OnDeletion from "did not fire" to "fires", which can shift event-count assertions. Triage each: if the test was wrong (assumed OnDeletion didn't fire when it should have), update the test. If a card's behavior actually broke, surface it as DONE_WITH_CONCERNS.

- [ ] **Step 2: PyO3 binding rebuild**

Run: `cd digimon-engine-py && maturin build --release`
Expected: clean build. Phase B's surface changes are all on `EffectContext` (script-API, not exposed to Python) and a new internal slot on `Game`; no PyO3-visible signatures changed.

- [ ] **Step 3: Python parity smoke**

Run: `DIGIMON_BACKEND=rust python -m pytest tests/engine/test_rust_backend_parity.py -v`
Expected: all 13 still pass. If anything new fails, the most likely cause is a card script that depends on the OnDeletion-skipping behavior of the old `ctx.delete_permanent`; treat as a real bug worth fixing in the affected script.

- [ ] **Step 4: Tauri tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: 34/34 pass.

- [ ] **Step 5: Report final status**

Final commit (if any cleanup) or "no changes — verification passed". Use `git log --oneline -20` to summarize what landed.

---

## Self-review (run before declaring Phase B done)

- **Completeness vs spec §4.1:** every entry point in the spec list is covered — delete, return-to-hand, return-to-deck, de-digivolve, suspend, negative-DP modifier add. The "move-to-stack" entry point named in the spec deferred — no such API exists yet (it lands in Phase D as part of Save / MaterialSave / Decoy). Note in the docs sweep.
- **No new card-side regressions:** Task 12 step 1 catches OnDeletion re-firing for `ctx.delete_permanent` callers. Triage list, don't blanket-skip.
- **Test seam is doc-hidden, not removed:** `set_effect_source_player_for_test` stays in the codebase as a `#[doc(hidden)] pub` API for future Phase D/E tests. Don't remove it after Phase B.
- **Cause field stayed simple:** the spec called out adding a `source: Option<PlayerId>` field to `ReplacementCause`. Phase A + B cover the consumer needs via the existing OwnEffect/OpponentEffect variants; the absolute-source field is **not** added here. Anyone writing Phase D/E who needs the absolute source can add it without breaking the cause variants.

