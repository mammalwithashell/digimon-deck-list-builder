//! BT1-090 Gravity Crush — Option, Cost 0, Black.
//!
//! # Card text (cards.json)
//!
//! [Main] Gain 2 memory. At end of turn, lose 2 memory.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT1/Red/BT1_090.cs
//!
//! Note: DCGO stores this in Red/ directory despite the card being Black
//! in the physical card. The behavioral implementation is identical:
//!   1. `card.Owner.AddMemory(2, activateClass)` — immediate gain
//!   2. `CardEffectCommons.AddEffectToPlayer(effectDuration: EffectDuration.UntilEachTurnEnd,
//!      timing: EffectTiming.OnEndTurn)` — schedules `AddMemory(-2)` at end of this turn
//!
//! # Patterns this test covers
//! - B1-adjacent: memory-swing option card (immediate gain + scheduled loss)
//! - schedule_delayed: end_of_your_turn timing (Phase 2f4 DSL verb)
//! - No conditions, no OPT, no branching — simplest possible option card
//!
//! # Clause analysis
//!
//! Clause 1 (main_from_hand):
//!   - Step 1: gain 2 memory immediately
//!   - Step 2: schedule a `lose_memory: 2` to fire at `end_of_your_turn`
//!   Net effect within the turn: memory is swapped (+2 now, −2 at EOT → net 0 after EOT).
//!
//! # Key behavioral insight (from DCGO)
//! The lose-2 is registered via `UntilEachTurnEnd` / `OnEndTurn`, meaning it fires
//! at the END of the CURRENT turn (the turn this card was played). This matches
//! `schedule_delayed: { when: end_of_your_turn }` — not "next turn", not "opponent's turn".
//!
//! # Testing approach
//! Option card MainFromHand effects are fired via `runner.game.activate_hand_main(player, idx)`,
//! which matches the `EffectTiming::MainFromHand` dispatch path. This mirrors the
//! P-035 pattern established for DSL option card tests.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

fn gravity_crush() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT1-090")
        .expect("BT1-090 YAML parses and compiles")
        .hand(0, &["BT1-090"])
        .memory(0)
        .start()
}

// =============================================================================
// Section 1 — Structural assertions
// =============================================================================

/// BT1-090 has exactly one triggered clause (the [Main] when: main_from_hand).
#[test]
fn bt1_090_has_one_main_from_hand_clause() {
    let runner = gravity_crush();
    let compiled = runner
        .compiled_card("BT1-090")
        .expect("BT1-090 compiled card must be registered");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(triggered.len(), 1, "exactly one triggered clause");
    let clause = &triggered[0];
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "scope is FaceUp (own-card, non-inherited)"
    );
    assert!(
        clause.when.contains(&CompiledTiming::MainFromHand),
        "when must include MainFromHand"
    );
}

/// The [Main] clause is not optional and not once-per-turn.
#[test]
fn bt1_090_clause_is_mandatory_and_not_opt() {
    let runner = gravity_crush();
    let compiled = runner
        .compiled_card("BT1-090")
        .expect("BT1-090 compiled card");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("main clause must exist");

    assert!(
        !clause.optional,
        "card text has no 'you may' — clause must be mandatory"
    );
    assert!(
        !clause.once_per_turn,
        "no [Once Per Turn] in card text — must be false"
    );
}

// =============================================================================
// Section 3 — Behavioral outcomes (integrated through activate_hand_main / end_turn)
// =============================================================================

/// Playing Gravity Crush gains 2 memory immediately.
#[test]
fn bt1_090_activating_gains_2_memory_immediately() {
    let mut runner = gravity_crush();
    runner.game.memory = 5; // set a known starting value
    let memory_before = runner.memory();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for BT1-090");

    assert_eq!(
        runner.memory(),
        memory_before + 2,
        "gain_memory: 2 must fire immediately on activation"
    );
}

/// At end of the turn the card was activated, lose 2 memory (scheduled effect fires).
/// Note: `end_turn()` fires the scheduled body THEN flips the memory seesaw
/// (`self.memory = -self.memory`). We verify by checking the queue drains
/// (see `bt1_090_scheduled_effect_queued_then_drained`) and that memory
/// after EOT equals `-(memory_after_activation - 2)` due to the flip.
#[test]
fn bt1_090_end_of_turn_loses_2_memory() {
    let mut runner = gravity_crush();
    runner.game.memory = 5;

    runner.game.activate_hand_main(0, 0);
    // After activation: memory = 7 (5 + 2).
    assert_eq!(runner.memory(), 7, "precondition: gain_memory: 2 fired");

    // Advance to end of player 0's turn:
    //   1. fire_end_of_your_turn fires scheduled lose_memory: 2 → memory = 5
    //   2. rotate_turn_player flips the seesaw → memory = -5
    runner.end_turn();

    // The seesaw flip turns the post-loss value negative.
    // Pre-flip value was 5 (7 - 2 = 5). Post-flip: -5.
    assert_eq!(
        runner.memory(),
        -5,
        "after EOT: lose_memory fired (7→5), then seesaw flip (5→-5)"
    );
}

/// Net memory change over a full turn-cycle: +2 (activation) then −2 (EOT),
/// followed by seesaw flip. The net before the flip is zero (+2 - 2 = 0).
/// This test directly asserts the intermediate pre-flip value by checking
/// the queue state (queue empty = both steps fired).
#[test]
fn bt1_090_net_memory_is_zero_before_seesaw_flip() {
    let mut runner = gravity_crush();
    runner.game.memory = 0;

    runner.game.activate_hand_main(0, 0);
    // After activation: memory = 2.
    assert_eq!(runner.memory(), 2);

    // The queue holds the scheduled body before end_turn.
    assert_eq!(runner.game.scheduled_effects.len(), 1);

    // End P0's turn:
    //   1. lose_memory: 2 fires → memory = 0
    //   2. queue drains
    //   3. seesaw flip → memory = 0 (negating 0 = 0)
    runner.end_turn();

    assert!(
        runner.game.scheduled_effects.is_empty(),
        "scheduled body must have fired (queue drained)"
    );
    // Starting from 0: +2 (activation) - 2 (EOT) = 0. Flip of 0 = 0.
    assert_eq!(
        runner.memory(),
        0,
        "net memory is 0 after activation+EOT (starting from 0, flip of 0 = 0)"
    );
}

/// The scheduled lose_memory fires only on the turn the card was activated (not on future turns).
/// Note: memory_range is (-10, 10); start at 8 so 8+2=10 is within range.
#[test]
fn bt1_090_scheduled_loss_fires_only_once() {
    let mut runner = gravity_crush();
    runner.game.memory = 8; // 8 + 2 = 10, within the (-10, 10) range

    runner.game.activate_hand_main(0, 0);

    // After activation: memory = 10 (8 + 2).
    assert_eq!(runner.memory(), 10, "gain_memory: 2 fires immediately");

    // End P0's turn: scheduled body fires (10→8), then seesaw flip (8→-8).
    runner.end_turn();

    // Queue drained after P0's EOT.
    assert!(
        runner.game.scheduled_effects.is_empty(),
        "scheduled_effects queue must be empty after EOT drain — no second body"
    );

    // End P1's turn (P0's turn again): no second scheduled body fires.
    // Memory before P1's end_turn = -8 (P0's perspective after seesaw flip).
    // After P1 ends their turn: seesaw flips again (+8), then no new scheduled effects.
    runner.end_turn();
    // Still empty — no second scheduled body queued.
    assert!(
        runner.game.scheduled_effects.is_empty(),
        "no second scheduled body — lose_memory fires only once"
    );
}

// =============================================================================
// Section 4 — Scheduling queue assertions
// =============================================================================

/// Verify the scheduling is queued (not fired) immediately after activation,
/// and drains after end_turn.
#[test]
fn bt1_090_scheduled_effect_queued_then_drained() {
    let mut runner = gravity_crush();
    runner.game.memory = 0;

    // Before activation: queue is empty.
    assert!(
        runner.game.scheduled_effects.is_empty(),
        "precondition: no scheduled effects"
    );

    runner.game.activate_hand_main(0, 0);

    // After activation: exactly one scheduled effect queued, memory already +2.
    assert_eq!(
        runner.game.scheduled_effects.len(),
        1,
        "exactly one scheduled body queued for end_of_your_turn"
    );
    assert_eq!(runner.memory(), 2, "gain_memory: 2 fired immediately");

    // End P0's turn: drain fires, queue clears, memory drops by 2.
    runner.end_turn();

    assert!(
        runner.game.scheduled_effects.is_empty(),
        "scheduled_effects drained after end_of_your_turn"
    );
    assert_eq!(runner.memory(), 0, "lose_memory: 2 fired at EOT");
}
