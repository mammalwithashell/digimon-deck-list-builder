//! EX9-027 Kokeshimon.
//! Printed text covered here: [When Digivolving] [On Deletion] by trashing
//! 1 card in your hand, 1 opponent Digimon gets -4000 DP for the turn.
//!
//! Inherited: [Opponent's Turn] [Once Per Turn] when an opponent's Digimon
//! attacks, by deleting this Digimon, end the attack.

use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

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
