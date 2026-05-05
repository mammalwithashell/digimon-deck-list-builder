//! BT22-032 ShoeShoemon.
//! Printed text: [On Deletion] You may play 1 level 3 Digimon card with the
//! [Puppet] trait from your hand without paying the cost. Inherited: [When
//! Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -2000 DP for
//! the turn.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::selection::SelectionKind;

#[test]
fn bt22_032_on_deletion_may_play_level3_puppet_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-032")
        .expect("BT22-032 YAML loads")
        .add_card(make_puppet_level3("PUPPET-L3"))
        .hand(0, &["PUPPET-L3"])
        .memory(0)
        .start();
    let shoe = runner.place_on_field(0, "BT22-032", Some(0));

    runner.game.delete_permanent_with_cause(
        shoe,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );

    assert!(
        runner.pending_is_optional(),
        "play-from-hand prompt is optional"
    );
    let view = runner
        .pending_selection_view()
        .expect("level 3 Puppet prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose level 3 Puppet");
    runner.auto_resolve().expect("finish play");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "PUPPET-L3"),
        "selected level 3 Puppet enters battle area"
    );
}

#[test]
fn bt22_032_on_deletion_decline_keeps_hand_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-032")
        .expect("BT22-032 YAML loads")
        .add_card(make_puppet_level3("PUPPET-L3"))
        .hand(0, &["PUPPET-L3"])
        .memory(0)
        .start();
    let shoe = runner.place_on_field(0, "BT22-032", Some(0));

    runner.game.delete_permanent_with_cause(
        shoe,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    runner
        .execute_action(0, PASS)
        .expect("decline optional play");

    assert_eq!(runner.hand_size(0), 1, "declining leaves Puppet in hand");
}

fn make_puppet_level3(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(3);
    card.traits = vec!["Puppet".to_string()];
    card
}
