//! ST19-05 PawnChessmon.
//! Printed text covered here: <Blocker>. [On Deletion] by trashing 1 Puppet card
//! in your hand, Draw 2.

use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

#[test]
fn st19_05_has_blocker_while_face_up() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-05")
        .expect("ST19-05 YAML loads")
        .start();
    let pawn = runner.place_on_field(0, "ST19-05", Some(0));

    assert!(
        runner.game.has_keyword(pawn, Keyword::Blocker),
        "ST19-05 has printed Blocker"
    );
}

#[test]
fn st19_05_on_deletion_trashes_puppet_from_hand_and_draws_two() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-05")
        .expect("ST19-05 YAML loads")
        .add_card(make_trait_card("PUPPET-HAND", &["Puppet"]))
        .add_card(make_trait_card("BLANK-HAND", &[]))
        .add_card(make_test_card("DRAW-1", "Draw 1"))
        .add_card(make_test_card("DRAW-2", "Draw 2"))
        .deck(0, &["DRAW-2", "DRAW-1"])
        .hand(0, &["PUPPET-HAND", "BLANK-HAND"])
        .start();
    let pawn = runner.place_on_field(0, "ST19-05", Some(0));

    runner
        .game
        .delete_permanent_with_cause(pawn, ReplacementCause::OpponentEffect);

    let view = runner.pending_selection_view().expect("Puppet discard");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(runner.pending_is_optional(), "cost payment can be declined");
    assert!(
        view.valid_action_ids.contains(&PLAY_HAND_START),
        "Puppet card is legal"
    );
    assert!(
        !view.valid_action_ids.contains(&(PLAY_HAND_START + 1)),
        "non-Puppet card must not be legal"
    );

    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("trash Puppet");
    runner.auto_resolve().expect("draw two");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"BLANK-HAND".to_string()));
    assert!(hand_ids.contains(&"DRAW-1".to_string()));
    assert!(hand_ids.contains(&"DRAW-2".to_string()));
    assert!(!hand_ids.contains(&"PUPPET-HAND".to_string()));
}

fn make_trait_card(id: &str, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn zone_ids(
    cards: &[digimon_engine::card_source::CardSource],
    data: &[digimon_engine::card_data::CardData],
) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}
