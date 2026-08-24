//! EX9-027 Kokeshimon.
//! Printed text covered here: [When Digivolving] [On Deletion] by trashing
//! 1 card in your hand, 1 opponent Digimon gets -4000 DP for the turn.
//!
//! Inherited: [Opponent's Turn] [Once Per Turn] when an opponent's Digimon
//! attacks, by deleting this Digimon, end the attack.
//!
//! # §15-7 optional processing condition
//!
//! "By trashing 1 card in your hand, ..." is an OPTIONAL PROCESSING CONDITION
//! (§15-7-1 — its worked example is this exact template), so the clause carries
//! `optional: true` + `outer_prompt: true` and surfaces an accept/decline
//! prompt BEFORE the hand-trash selection. Accepting runs the cost then the
//! debuff; declining does neither (§15-7-2). DCGO reaches the same behavior via
//! `SelectHandEffect(..., canNoSelect: true, ...)` + `if (cardDiscarded)`.

use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

/// Accept the outer accept/decline confirm the OPTIONAL PROCESSING CONDITION
/// ("By trashing 1 card in your hand, ...") installs before its body runs.
/// §15-7-1/§15-7-4 — see the clause comment in `cards/ex9/EX9-027.yaml`.
fn accept_optional_cost(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("the optional processing condition must surface a prompt (rule 17)");
    assert!(
        view.is_optional,
        "the outer accept/decline prompt must be declinable (§15-7-4)"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the 'by trashing 1 card in your hand' cost");
}

#[test]
fn ex9_027_when_digivolving_trashes_hand_card_for_minus_4000_dp() {
    let mut runner = base_runner();
    let koko = runner.place_stack(0, &["BASE", "EX9-027"]);
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(koko),
    );
    runner.game.drain_effect_queue();

    accept_optional_cost(&mut runner);

    let discard_view = runner.pending_selection_view().expect("discard prompt");
    assert_eq!(discard_view.kind, SelectionKind::Hand);
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("trash hand card");

    let target_view = runner.pending_selection_view().expect("opponent target");
    assert_eq!(target_view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, target_view.valid_action_ids[0])
        .expect("choose opponent");
    runner.auto_resolve().expect("finish effect");

    assert_eq!(runner.game.effective_dp(opp), Some(2000));
}

#[test]
fn ex9_027_on_deletion_trashes_hand_card_for_minus_4000_dp() {
    let mut runner = base_runner();
    let koko = runner.place_on_field(0, "EX9-027", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner
        .game
        .delete_permanent_with_cause(koko, ReplacementCause::OpponentEffect);

    accept_optional_cost(&mut runner);

    let discard_view = runner.pending_selection_view().expect("discard prompt");
    assert_eq!(discard_view.kind, SelectionKind::Hand);
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("trash hand card");

    let target_view = runner.pending_selection_view().expect("opponent target");
    assert_eq!(target_view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, target_view.valid_action_ids[0])
        .expect("choose opponent");
    runner.auto_resolve().expect("finish effect");

    assert_eq!(runner.game.effective_dp(opp), Some(2000));
}

// ─── §15-7 optional processing condition — decline paths ─────────────────────
//
// "[When Digivolving] [On Deletion] By trashing 1 card in your hand, 1 of your
// opponent's Digimon gets -4000 DP for the turn."
//
// §15-7-1 names this exact shape ("by X, Y") an OPTIONAL PROCESSING CONDITION,
// and its worked example is the same template. §15-7-4: the player chooses
// whether to execute it. §15-7-2: when it is not executed, "the processing
// after the conditions can't be executed" — so declining must trash NO card
// AND apply NO -4000 DP. EX9-027's own official Q&A repeats this: "If the 'by
// doing X' condition isn't met, the rest of the effect isn't processed."
//
// DCGO reaches the same behavior through the coroutine rather than the flag:
// `EX9_027.cs` passes `isOptional = false` (lines 34 / 138) but calls
// `SelectHandEffect.SetUp(..., canNoSelect: true, ...)` and guards the DP
// debuff behind `if (cardDiscarded)`.

#[test]
fn ex9_027_when_digivolving_optional_cost_may_be_declined() {
    let mut runner = base_runner();
    let koko = runner.place_stack(0, &["BASE", "EX9-027"]);
    let opp = runner.place_on_field(1, "OPP", Some(0));

    let hand_before = runner.hand_size(0);
    let trash_before = runner.trash_size(0);
    let dp_before = runner.game.effective_dp(opp);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(koko),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("the optional processing condition must surface a prompt (rule 17)");
    assert!(
        view.is_optional,
        "the outer accept/decline prompt must be declinable (§15-7-4)"
    );
    runner
        .decline_optional_trigger()
        .expect("declining must be reachable from the action space");
    let _ = runner.auto_resolve();

    // §15-7-4 — the cost is NOT paid.
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declining must not trash a card from hand"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "declining must leave the trash untouched"
    );
    // §15-7-2 — the processing after the condition can't be executed.
    assert_eq!(
        runner.game.effective_dp(opp),
        dp_before,
        "with the optional condition declined, the -4000 DP must not apply"
    );
}

#[test]
fn ex9_027_on_deletion_optional_cost_may_be_declined() {
    let mut runner = base_runner();
    let koko = runner.place_on_field(0, "EX9-027", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    let hand_before = runner.hand_size(0);
    let dp_before = runner.game.effective_dp(opp);

    runner
        .game
        .delete_permanent_with_cause(koko, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("the optional processing condition must surface a prompt (rule 17)");
    assert!(
        view.is_optional,
        "the outer accept/decline prompt must be declinable (§15-7-4)"
    );
    runner
        .decline_optional_trigger()
        .expect("declining must be reachable from the action space");
    let _ = runner.auto_resolve();

    // §15-7-4 — the cost is NOT paid. (Trash size is not asserted here: the
    // deleted Kokeshimon itself lands in trash, so only the hand proves the
    // discard never happened.)
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declining must not trash a card from hand"
    );
    // §15-7-2 — the processing after the condition can't be executed.
    assert_eq!(
        runner.game.effective_dp(opp),
        dp_before,
        "with the optional condition declined, the -4000 DP must not apply"
    );
}

#[test]
fn ex9_027_inherited_may_delete_carrier_to_end_opponent_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-027")
        .expect("EX9-027 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("SECURITY", "Security"))
        .security(0, &["SECURITY"])
        .start();
    runner.place_stack(0, &["EX9-027", "CARRIER"]);
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();

    runner.attack_player(attacker, 0, false);

    let view = runner
        .pending_selection_view()
        .expect("attack-cancel choice");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose to delete carrier and end attack");
    runner.auto_resolve().expect("finish attack cancel");

    assert_eq!(runner.security_count(0), 1, "security was not checked");
    assert!(
        runner.game.players[0].battle_area.is_empty(),
        "carrier was deleted as the printed cost"
    );
    assert!(
        runner.game.pending_attack.is_none(),
        "attack state is fully cleared"
    );
}

fn base_runner() -> DebugRunner {
    let mut opp = make_test_card("OPP", "Opponent");
    opp.dp = Some(6000);
    DebugRunner::builder()
        .dsl_card("EX9-027")
        .expect("EX9-027 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_test_card("DISCARD", "Discard"))
        .add_card(opp)
        .hand(0, &["DISCARD"])
        .memory(10)
        .start()
}
