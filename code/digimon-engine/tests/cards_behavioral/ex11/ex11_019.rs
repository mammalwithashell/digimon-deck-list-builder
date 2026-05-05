//! EX11-019 Shoemon.
//! Printed text covered here: [On Deletion] you may play 1 Familiar Token.
//! Inherited: <Barrier>.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

#[test]
fn ex11_019_on_deletion_may_play_familiar_token() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-019")
        .expect("EX11-019 YAML loads")
        .start();
    let shoe = runner.place_on_field(0, "EX11-019", Some(0));

    runner
        .game
        .delete_permanent_with_cause(shoe, ReplacementCause::OpponentEffect);
    let view = runner.pending_selection_view().expect("token choice");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose to play token");
    runner.auto_resolve().expect("finish token play");

    let familiar_count = runner.game.players[0]
        .battle_area
        .iter()
        .filter(|perm| perm.top_card().card_name(&runner.game.card_data) == "Familiar Token")
        .count();
    assert_eq!(familiar_count, 1, "one Familiar Token is played");
}

#[test]
fn ex11_019_inherited_barrier_is_available_from_stack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-019")
        .expect("EX11-019 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .start();
    let carrier = runner.place_stack(0, &["EX11-019", "CARRIER"]);

    assert!(
        runner.game.has_keyword(carrier, Keyword::Barrier),
        "carrier inherits Barrier from EX11-019"
    );
}
