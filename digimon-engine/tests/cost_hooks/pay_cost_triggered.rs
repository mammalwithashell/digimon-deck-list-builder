//! Tests for `pay_cost_fn` wiring in `run_queued_effect`.
//!
//! Verifies that a `pay_cost_fn` closure on a triggered effect (OnPlay, etc.)
//! is invoked after the condition check and before `process`, and that
//! returning `false` silently aborts the effect while returning `true`
//! continues to `process`.

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{DebugRunner, make_test_card};
use digimon_engine::effect::{CardEffect, Effect};
use std::sync::Arc;

// ─── Helper CardEffect structs ────────────────────────────────────────────────

/// OnPlay effect: pay_cost_fn returns true → process gains 1 memory.
struct PayCostTrueGainsMemory;
impl CardEffect for PayCostTrueGainsMemory {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("pay_cost true → gain 1")
            .pay_cost_fn(|_ctx| true)
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

/// OnPlay effect: pay_cost_fn returns false → process should NOT run.
struct PayCostFalseSkipsProcess;
impl CardEffect for PayCostFalseSkipsProcess {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("pay_cost false → skip process")
            .pay_cost_fn(|_ctx| false)
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

/// OnPlay effect: pay_cost_fn loses 2 memory (side-effect), returns true →
/// process gains 1 memory.
struct PayCostMutatesState;
impl CardEffect for PayCostMutatesState {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("pay_cost mutates state")
            .pay_cost_fn(|ctx| {
                ctx.lose_memory(2);
                true
            })
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

/// OnPlay effect: condition(|_| false) — pay_cost_fn panics if called.
/// Validates that a failing condition gates pay_cost_fn.
struct ConditionFalsePayCostPanics;
impl CardEffect for ConditionFalsePayCostPanics {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("condition false → pay_cost never fires")
            .condition(|_| false)
            .pay_cost_fn(|_ctx| panic!("pay_cost_fn must not be called when condition is false"))
            .process(|_ctx| {})
            .build()]
    }
}

/// OnPlay effect: pay_cost_fn returns true but there is NO process closure.
/// Verifies that the absence of a process doesn't panic.
struct PayCostTrueNoProcess;
impl CardEffect for PayCostTrueNoProcess {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("pay_cost true, no process")
            .pay_cost_fn(|ctx| {
                // Side effect so we can detect that this fired.
                ctx.gain_memory(3);
                true
            })
            // No .process() call — process field remains None.
            .build()]
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn pay_cost_returning_true_runs_process() {
    // OnPlay effect: .pay_cost_fn(|_| true).process(|ctx| ctx.gain_memory(1))
    // Play the card. Assert memory changed by (-printed_cost + 1).
    // make_test_card has play_cost=3, so net = -3 + 1 = -2.
    let card = make_test_card("PCTRUE", "PayCostTrue");

    let mut r = DebugRunner::builder()
        .add_card(card)
        .hand(0, &["PCTRUE"])
        .memory(10)
        .start();

    r.register_effect("PCTRUE", Arc::new(PayCostTrueGainsMemory));

    let printed_cost = r.game.card_data
        .iter()
        .find(|c| c.card_id == "PCTRUE")
        .map(|c| c.play_cost as i16)
        .unwrap_or(0);

    let memory_before = r.memory(); // 10
    r.play(0, 0);

    // Playing costs printed_cost; pay_cost returned true → process ran → gain_memory(1)
    let expected = memory_before - printed_cost + 1;
    assert_eq!(
        r.memory(),
        expected,
        "pay_cost_fn returning true should allow process to run and gain 1 memory; \
         expected {} - {} + 1 = {}",
        memory_before, printed_cost, expected,
    );
}

#[test]
fn pay_cost_returning_false_skips_process() {
    // OnPlay effect: .pay_cost_fn(|_| false).process(|ctx| ctx.gain_memory(1))
    // Play the card. Assert that `process` did NOT run (memory only reflects the
    // play cost, not the gain_memory(1) that process would have added).
    let card = make_test_card("PCFALSE", "PayCostFalse");

    let mut r = DebugRunner::builder()
        .add_card(card)
        .hand(0, &["PCFALSE"])
        .memory(10)
        .start();

    r.register_effect("PCFALSE", Arc::new(PayCostFalseSkipsProcess));

    let printed_cost = r.game.card_data
        .iter()
        .find(|c| c.card_id == "PCFALSE")
        .map(|c| c.play_cost as i16)
        .unwrap_or(0);

    let memory_before = r.memory(); // 10
    r.play(0, 0);

    // pay_cost returned false → process skipped → only the play cost was deducted
    let expected = memory_before - printed_cost;
    assert_eq!(
        r.memory(),
        expected,
        "pay_cost_fn returning false should silently abort; process must not run; \
         expected {} - {} = {}",
        memory_before, printed_cost, expected,
    );
}

#[test]
fn pay_cost_can_mutate_game_state() {
    // OnPlay effect: .pay_cost_fn(|ctx| { ctx.lose_memory(2); true })
    //                .process(|ctx| ctx.gain_memory(1))
    // Use printed_cost = 0 so the play cost is 0 and doesn't interfere.
    // Start memory = 0. Play the card.
    // Expected: memory = 0 - 2 + 1 = -1.
    let card = make_test_card("PCMUTATE", "PayCostMutate");
    // make_test_card defaults play_cost to 0 (it uses level-3 with play_cost 3,
    // but we rely on the actual default — if the cost is non-zero, set memory
    // high enough so the play doesn't fail, then account for it in the assert).
    // To keep the math clean and explicit, start at memory=10 and account for
    // the printed play cost separately.
    let mut r = DebugRunner::builder()
        .add_card(card)
        .hand(0, &["PCMUTATE"])
        .memory(10)
        .start();

    r.register_effect("PCMUTATE", Arc::new(PayCostMutatesState));

    // Determine the card's printed play cost so we can account for it.
    let printed_cost = r.game.card_data
        .iter()
        .find(|c| c.card_id == "PCMUTATE")
        .map(|c| c.play_cost as i16)
        .unwrap_or(0);

    let memory_before = r.memory(); // 10
    r.play(0, 0);

    // Playing the card costs `printed_cost` memory.
    // pay_cost_fn fires: lose_memory(2) → memory drops by 2.
    // process fires: gain_memory(1) → memory gains 1.
    // Net: memory_before - printed_cost - 2 + 1
    let expected = memory_before - printed_cost - 2 + 1;
    assert_eq!(
        r.memory(),
        expected,
        "pay_cost_fn side-effect (lose 2) and process (gain 1) should both apply; \
         expected {} - {} - 2 + 1 = {}",
        memory_before,
        printed_cost,
        expected,
    );
}

#[test]
fn condition_gates_pay_cost() {
    // OnPlay effect: .condition(|_| false).pay_cost_fn(|_| panic!("should not fire")).process(|_| {})
    // Play the card. Assert no panic (condition gated the pay_cost_fn).
    // Memory only moves by the play cost — neither pay_cost_fn nor process runs.
    let card = make_test_card("CONDGATE", "CondGate");

    let mut r = DebugRunner::builder()
        .add_card(card)
        .hand(0, &["CONDGATE"])
        .memory(10)
        .start();

    r.register_effect("CONDGATE", Arc::new(ConditionFalsePayCostPanics));

    let printed_cost = r.game.card_data
        .iter()
        .find(|c| c.card_id == "CONDGATE")
        .map(|c| c.play_cost as i16)
        .unwrap_or(0);

    let memory_before = r.memory(); // 10
    // This would panic inside the pay_cost_fn if the condition didn't gate it.
    r.play(0, 0);

    // No panic = condition correctly prevented pay_cost_fn from firing.
    // Memory only reflects the play cost.
    let expected = memory_before - printed_cost;
    assert_eq!(
        r.memory(),
        expected,
        "condition(false) should gate pay_cost_fn; only play cost deducted; \
         expected {} - {} = {}",
        memory_before, printed_cost, expected,
    );
}

#[test]
fn pay_cost_without_process_field_is_noop() {
    // Edge case: effect has .pay_cost_fn(|ctx| { ctx.gain_memory(3); true }) but NO .process.
    // Play the card. Assert pay_cost_fn WAS called (memory gained 3 from its side-effect)
    // and process didn't fire (no further state change).
    let card = make_test_card("PCNOPROCESS", "PayCostNoProcess");

    let mut r = DebugRunner::builder()
        .add_card(card)
        .hand(0, &["PCNOPROCESS"])
        .memory(10)
        .start();

    r.register_effect("PCNOPROCESS", Arc::new(PayCostTrueNoProcess));

    let printed_cost = r.game.card_data
        .iter()
        .find(|c| c.card_id == "PCNOPROCESS")
        .map(|c| c.play_cost as i16)
        .unwrap_or(0);

    let memory_before = r.memory(); // 10
    r.play(0, 0);

    // pay_cost_fn fired and gained 3 memory; no process to run after.
    // Net: memory_before - printed_cost + 3
    let expected = memory_before - printed_cost + 3;
    assert_eq!(
        r.memory(),
        expected,
        "pay_cost_fn (gain 3) should fire even with no process; \
         expected {} - {} + 3 = {}",
        memory_before,
        printed_cost,
        expected,
    );
}
