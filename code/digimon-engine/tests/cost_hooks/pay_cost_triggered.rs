//! Tests for `pay_cost_fn` wiring in `run_queued_effect`.
//!
//! Verifies that a `pay_cost_fn` closure on a triggered effect (OnPlay, etc.)
//! is invoked after the condition check and before `process`, and that
//! returning `false` silently aborts the effect while returning `true`
//! continues to `process`.

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{DebugRunner, make_test_card};
use digimon_engine::effect::{CardEffect, Effect};
use std::sync::{Arc, Mutex};

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

/// OnPlay effect: pay_cost_fn and process each push their name into a shared
/// log. Used to verify that pay_cost_fn runs BEFORE process (order-sensitive).
struct PayCostOrderLog {
    log: Arc<Mutex<Vec<&'static str>>>,
}
impl CardEffect for PayCostOrderLog {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let log_pay = Arc::clone(&self.log);
        let log_proc = Arc::clone(&self.log);
        vec![Effect::on_play(card)
            .name("pay_cost order log")
            .pay_cost_fn(move |_ctx| {
                log_pay.lock().unwrap().push("pay_cost");
                true
            })
            .process(move |_ctx| {
                log_proc.lock().unwrap().push("process");
            })
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
fn pay_cost_runs_before_process() {
    // Ordering test: pay_cost_fn and process each append their name to a
    // shared log. After the card is played, the log must read
    // ["pay_cost", "process"] — confirming pay_cost_fn ran first.
    //
    // This is genuinely order-sensitive: if the implementation accidentally
    // reversed the call sites (process before pay_cost_fn), the log would
    // read ["process", "pay_cost"] and the assertion would fail. The previous
    // net-arithmetic test (lose 2, gain 1 → net -1) could not catch this
    // because arithmetic is commutative — the numbers come out the same
    // regardless of execution order.
    let log: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));
    let log_clone = Arc::clone(&log);

    let card = make_test_card("PCORDER", "PayCostOrder");

    let mut r = DebugRunner::builder()
        .add_card(card)
        .hand(0, &["PCORDER"])
        .memory(10)
        .start();

    r.register_effect("PCORDER", Arc::new(PayCostOrderLog { log: log_clone }));

    r.play(0, 0);

    let observed = log.lock().unwrap().clone();
    assert_eq!(
        observed,
        vec!["pay_cost", "process"],
        "pay_cost_fn must run before process; observed order: {:?}",
        observed,
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
