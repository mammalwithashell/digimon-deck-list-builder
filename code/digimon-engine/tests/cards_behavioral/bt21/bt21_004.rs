//! BT21-004 Koromon.
//!
//! Inherited: [Your Turn] [Once Per Turn] When any of your yellow or red
//! Tamers suspend, draw 1.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

#[test]
fn bt21_004_inherited_draws_when_own_yellow_or_red_tamer_suspends() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-004")
        .expect("BT21-004 YAML loads")
        .add_card(digimon("CARRIER"))
        .add_card(tamer("YELLOW-TAMER", CardColor::Yellow))
        .add_card(make_test_card("DRAW", "Draw"))
        .deck(0, &["DRAW"])
        .start();

    runner.place_stack(0, &["BT21-004", "CARRIER"]);
    let tamer = runner.place_on_field(0, "YELLOW-TAMER", Some(0));
    let hand_before = runner.game.players[0].hand.len();

    runner.game.suspend(tamer);
    runner.auto_resolve().expect("draw has no choices");

    assert_eq!(runner.game.players[0].hand.len(), hand_before + 1);
}

#[test]
fn bt21_004_ignores_non_red_yellow_tamers_and_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-004")
        .expect("BT21-004 YAML loads")
        .add_card(digimon("CARRIER"))
        .add_card(tamer("GREEN-TAMER", CardColor::Green))
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(make_test_card("DRAW", "Draw"))
        .deck(0, &["DRAW", "DRAW"])
        .start();

    runner.place_stack(0, &["BT21-004", "CARRIER"]);
    let green = runner.place_on_field(0, "GREEN-TAMER", Some(0));
    let red = runner.place_on_field(0, "RED-TAMER", Some(0));

    runner.game.suspend(green);
    runner.auto_resolve().expect("no draw");
    assert_eq!(runner.game.players[0].hand.len(), 0);

    runner.game.turn_player_idx = 1;
    runner.game.suspend(red);
    runner.auto_resolve().expect("no draw on opponent turn");
    assert_eq!(runner.game.players[0].hand.len(), 0);
}

fn digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.level = Some(3);
    card.dp = Some(3000);
    card
}

fn tamer(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![color];
    card.level = None;
    card.dp = None;
    card
}
