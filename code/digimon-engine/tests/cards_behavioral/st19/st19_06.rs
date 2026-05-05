//! ST19-06 Doggymon.
//! Printed text: [On Play] [On Deletion] 1 opponent Digimon gains
//! <Security A. -1> until the end of their turn.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::ModifierType;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

#[test]
fn st19_06_on_play_grants_security_attack_minus_to_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-06")
        .expect("ST19-06 YAML loads")
        .add_card(make_test_card("OPP", "Opponent"))
        .hand(0, &["ST19-06"])
        .memory(10)
        .start();
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.play(0, 0).expect("play Doggymon");

    let view = runner.pending_selection_view().expect("opponent target");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose opponent Digimon");
    runner.auto_resolve().expect("finish grant");

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp, ModifierType::SecurityAttackChange),
        -1,
        "target opponent Digimon has Security Attack -1"
    );
}

#[test]
fn st19_06_on_deletion_grants_security_attack_minus_to_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-06")
        .expect("ST19-06 YAML loads")
        .add_card(make_test_card("OPP", "Opponent"))
        .start();
    let doggy = runner.place_on_field(0, "ST19-06", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner
        .game
        .delete_permanent_with_cause(doggy, ReplacementCause::OpponentEffect);

    let view = runner.pending_selection_view().expect("opponent target");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose opponent Digimon");
    runner.auto_resolve().expect("finish grant");

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp, ModifierType::SecurityAttackChange),
        -1,
        "target opponent Digimon has Security Attack -1"
    );
}
