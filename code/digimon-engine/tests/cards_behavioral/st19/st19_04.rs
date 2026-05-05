//! ST19-04 PawnChessmon.
//! Printed text covered here: [On Play] by trashing 1 Puppet card in your hand,
//! Draw 2. Inherited: <Reboot>.

use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

#[test]
fn st19_04_on_play_trashes_puppet_from_hand_and_draws_two() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-04")
        .expect("ST19-04 YAML loads")
        .add_card(make_trait_card("PUPPET-HAND", &["Puppet"]))
        .add_card(make_trait_card("BLANK-HAND", &[]))
        .add_card(make_test_card("DRAW-1", "Draw 1"))
        .add_card(make_test_card("DRAW-2", "Draw 2"))
        .deck(0, &["DRAW-2", "DRAW-1"])
        .hand(0, &["ST19-04", "PUPPET-HAND", "BLANK-HAND"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play PawnChessmon");

    let view = runner.pending_selection_view().expect("Puppet discard");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(runner.pending_is_optional(), "cost payment can be declined");
    assert!(
        view.valid_action_ids.contains(&(PLAY_HAND_START)),
        "Puppet card is legal after ST19-04 leaves hand"
    );
    assert!(
        !view.valid_action_ids.contains(&(PLAY_HAND_START + 1)),
        "non-Puppet card must not be legal for the printed cost"
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

    let trash_ids = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert!(trash_ids.contains(&"PUPPET-HAND".to_string()));
}

#[test]
fn st19_04_on_play_decline_does_not_trash_or_draw() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-04")
        .expect("ST19-04 YAML loads")
        .add_card(make_trait_card("PUPPET-HAND", &["Puppet"]))
        .add_card(make_test_card("DRAW-1", "Draw 1"))
        .deck(0, &["DRAW-1"])
        .hand(0, &["ST19-04", "PUPPET-HAND"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play PawnChessmon");
    runner.execute_action(0, PASS).expect("decline discard");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert_eq!(hand_ids, vec!["PUPPET-HAND".to_string()]);
    assert!(runner.game.players[0].trash.is_empty());
}

#[test]
fn st19_04_inherited_reboot_is_available_from_stack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-04")
        .expect("ST19-04 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .start();
    let carrier = runner.place_stack(0, &["ST19-04", "CARRIER"]);

    assert!(
        runner.game.has_keyword(carrier, Keyword::Reboot),
        "carrier inherits Reboot from ST19-04"
    );
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
