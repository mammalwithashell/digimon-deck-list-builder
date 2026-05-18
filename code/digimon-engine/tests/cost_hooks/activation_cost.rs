//! Substrate tests for `Effect::activation_cost(...)` — the generic
//! triggered-ability cost hook. Distinct from `.pay_cost_fn` which is
//! used at play/digivolve cost calculation; `.activation_cost` fires on
//! triggered-ability resolution, between the condition gate and the
//! body `process`.
//!
//! Plan reference: `.claude/plans/phase-2-track-b-activation-cost-builder.md`.

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use std::sync::{Arc, Mutex};

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// Observer that suspends itself as activation cost, then gains 1 memory.
/// Fires on `OnEnterFieldAnyone` so it triggers when any Digimon enters
/// either player's battle area.
struct SuspendSelfThenGainMemory;
impl CardEffect for SuspendSelfThenGainMemory {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_enter_field_anyone(card)
            .name("suspend-self cost → gain 1 memory")
            .activation_cost(|ctx| ctx.suspend_self_as_cost())
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

/// Observer with a once_per_turn cap. Used to verify OPT-slot consumption
/// on cost failure (next trigger same turn must NOT re-fire).
struct OncePerTurnSuspendSelfGainMemory;
impl CardEffect for OncePerTurnSuspendSelfGainMemory {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_enter_field_anyone(card)
            .name("OPT suspend-self cost → gain 1 memory")
            .once_per_turn()
            .activation_cost(|ctx| ctx.suspend_self_as_cost())
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

/// Observer that always fails the activation cost. Used to verify body
/// does NOT run on cost failure.
struct ActivationCostFalseSkipsProcess;
impl CardEffect for ActivationCostFalseSkipsProcess {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_enter_field_anyone(card)
            .name("activation_cost false → skip process")
            .activation_cost(|_ctx| false)
            .process(|ctx| ctx.gain_memory(99))
            .build()]
    }
}

/// Ordering log: activation_cost and process each append their name to
/// a shared log. Used to verify activation_cost fires BEFORE process.
struct ActivationCostOrderLog {
    log: Arc<Mutex<Vec<&'static str>>>,
}
impl CardEffect for ActivationCostOrderLog {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let log_cost = Arc::clone(&self.log);
        let log_proc = Arc::clone(&self.log);
        vec![Effect::on_enter_field_anyone(card)
            .name("activation_cost order log")
            .activation_cost(move |_ctx| {
                log_cost.lock().unwrap().push("activation_cost");
                true
            })
            .process(move |_ctx| {
                log_proc.lock().unwrap().push("process");
            })
            .build()]
    }
}

/// `condition` false ⇒ `activation_cost` must never fire.
struct ConditionFalseActivationCostPanics;
impl CardEffect for ConditionFalseActivationCostPanics {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_enter_field_anyone(card)
            .name("condition false → activation_cost never fires")
            .condition(|_| false)
            .activation_cost(|_ctx| panic!("activation_cost must not fire when condition is false"))
            .process(|_| {})
            .build()]
    }
}

/// Observer that returns the source Tamer to the bottom of the deck as
/// activation cost, then draws 1.
struct ReturnSelfToBottomThenDraw;
impl CardEffect for ReturnSelfToBottomThenDraw {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_enter_field_anyone(card)
            .name("return-self-to-bottom cost → draw 1")
            .activation_cost(|ctx| ctx.return_self_to_deck_bottom_as_cost())
            .process(|ctx| {
                ctx.draw(ctx.player, 1);
            })
            .build()]
    }
}

// ─── Test scaffolding helpers ────────────────────────────────────────────────

/// Build a runner with `OBSERVER` on player 0's field and a vanilla
/// Digimon `DUMMY` in player 0's hand that, when played, will fire the
/// observer's `OnEnterFieldAnyone`. Memory is set so the play does not
/// negative-cap the test.
fn runner_with_observer<T: CardEffect + 'static>(observer_id: &str, effect: T) -> DebugRunner {
    let observer = make_test_card(observer_id, observer_id);
    let mut dummy = make_test_card("DUMMY-FIELD-TRIGGER", "Dummy");
    dummy.play_cost = 0;

    let mut r = DebugRunner::builder()
        .add_card(observer)
        .add_card(dummy)
        .hand(0, &["DUMMY-FIELD-TRIGGER"])
        .memory(0)
        .start();
    r.register_effect(observer_id, Arc::new(effect));
    r.place_on_field(0, observer_id, Some(0));
    r
}

fn memory(r: &DebugRunner) -> i16 {
    r.memory()
}

// ─── Builder surface ─────────────────────────────────────────────────────────

#[test]
fn effect_builder_exposes_activation_cost_fn() {
    let effect = Effect::on_enter_field_anyone(CardHandle(0))
        .name("smoke")
        .activation_cost(|_ctx| true)
        .process(|_| {})
        .build();
    assert!(
        effect.activation_cost_fn.is_some(),
        "activation_cost(...) builder method must populate Effect::activation_cost_fn"
    );
}

// ─── Cost-paid happy path ────────────────────────────────────────────────────

#[test]
fn activation_cost_returning_true_runs_process() {
    let mut r = runner_with_observer("AC-TRUE", SuspendSelfThenGainMemory);
    let memory_before = memory(&r);
    r.play(0, 0);
    assert_eq!(
        memory(&r),
        memory_before + 1,
        "activation_cost true should allow process to run and gain 1 memory"
    );
    assert!(
        r.game.players[0].battle_area[0].is_suspended,
        "suspend-self cost should leave the observer suspended"
    );
}

// ─── Cost failure silently aborts ────────────────────────────────────────────

#[test]
fn activation_cost_returning_false_skips_process() {
    let mut r = runner_with_observer("AC-FALSE", ActivationCostFalseSkipsProcess);
    let memory_before = memory(&r);
    r.play(0, 0);
    assert_eq!(
        memory(&r),
        memory_before,
        "activation_cost false should silently abort; process must not run"
    );
}

#[test]
fn pre_suspended_source_fails_suspend_self_cost() {
    let mut r = runner_with_observer("AC-PRESUSP", SuspendSelfThenGainMemory);
    // Pre-suspend the observer so the suspend-self cost fails on trigger.
    r.game.players[0].battle_area[0].is_suspended = true;
    let memory_before = memory(&r);
    r.play(0, 0);
    assert_eq!(
        memory(&r),
        memory_before,
        "already-suspended source must fail suspend-self cost and skip the body"
    );
    assert!(
        r.game.pending_selection.is_none(),
        "cost failure must not install a pending selection"
    );
}

// ─── Ordering / condition gating ────────────────────────────────────────────

#[test]
fn activation_cost_runs_before_process() {
    let log: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));
    let mut r = runner_with_observer(
        "AC-ORDER",
        ActivationCostOrderLog {
            log: Arc::clone(&log),
        },
    );
    r.play(0, 0);
    let observed = log.lock().unwrap().clone();
    assert_eq!(
        observed,
        vec!["activation_cost", "process"],
        "activation_cost must run before process; observed order: {:?}",
        observed
    );
}

#[test]
fn condition_gates_activation_cost() {
    let mut r = runner_with_observer("AC-CONDGATE", ConditionFalseActivationCostPanics);
    // Would panic in the cost closure if the condition didn't gate it.
    r.play(0, 0);
    // No panic = condition gated activation_cost.
}

// ─── OPT-slot consumption on cost failure ───────────────────────────────────

#[test]
fn cost_failure_consumes_opt_slot_for_same_activation_key() {
    // Pre-suspend the observer, fire two triggers: the first fails the
    // suspend-self cost. Even after manually unsuspending the source
    // before the second trigger, the OPT slot must already be consumed
    // — the observer must NOT fire again this turn.
    let mut r = runner_with_observer("AC-OPT", OncePerTurnSuspendSelfGainMemory);
    r.game.players[0].battle_area[0].is_suspended = true;

    let memory_before = memory(&r);
    r.play(0, 0); // first trigger — cost fails (already suspended)
    assert_eq!(
        memory(&r),
        memory_before,
        "first trigger should fail suspend-self cost"
    );

    // Manually unsuspend the source so a *fresh* cost attempt would succeed.
    r.game.players[0].battle_area[0].is_suspended = false;

    // Play another dummy to fire a second trigger. The OPT slot must be
    // consumed by the prior cost failure — body must NOT run.
    let mut another = make_test_card("AC-OPT-DUMMY-2", "Dummy2");
    another.play_cost = 0;
    let card_idx = r.game.card_data.len();
    r.game.card_data.push(another);
    let next_idx = r.game.next_card_index();
    let mut src = digimon_engine::card_source::CardSource::new(card_idx, 0, next_idx);
    src.card_index = next_idx;
    r.game.players[0].hand.push(src);
    r.play(0, 0); // second trigger this turn

    assert_eq!(
        memory(&r),
        memory_before,
        "OPT slot must be consumed by the prior cost failure; the second \
         trigger must NOT re-fire even though the source is now unsuspended"
    );
    assert!(
        !r.game.players[0].battle_area[0].is_suspended,
        "second trigger must not have re-suspended the source"
    );
}

// ─── return_self_to_deck_bottom_as_cost helper ──────────────────────────────

#[test]
fn return_self_to_deck_bottom_as_cost_moves_source_and_draws() {
    let mut r = runner_with_observer("AC-RETSELF", ReturnSelfToBottomThenDraw);

    // Load player 0's deck with a sentinel so we can witness draw().
    let mut sentinel = make_test_card("AC-RETSELF-SENTINEL", "Sentinel");
    sentinel.play_cost = 0;
    let card_idx = r.game.card_data.len();
    r.game.card_data.push(sentinel);
    let next_idx = r.game.next_card_index();
    let mut src = digimon_engine::card_source::CardSource::new(card_idx, 0, next_idx);
    src.card_index = next_idx;
    r.game.players[0].deck.push(src);

    let deck_before = r.game.players[0].deck.len();
    let field_before = r.game.players[0].battle_area.len();

    r.play(0, 0); // fires trigger; cost = return self to deck bottom

    // Source permanent left the field as part of cost payment.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        field_before - 1 + 1,
        "field size: -1 for returned-self cost, +1 for the played dummy"
    );

    // Deck size: -1 (drew the sentinel) + 1 (returned-self) = unchanged.
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_before,
        "deck size unchanged: -1 drawn + 1 returned-self"
    );

    // The returned card is at the bottom of the deck. Confirm we don't
    // see a battle-area permanent with the observer id any more.
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .all(|perm| perm.top_card().card_id(&r.game.card_data) != "AC-RETSELF"),
        "returned-self cost should remove the observer from the battle area"
    );
}
