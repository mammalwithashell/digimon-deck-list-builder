//! BT11-080 Devimon
//!
//! [Your Turn] While you have a yellow Digimon or yellow Tamer in play, this
//! Digimon gains <Rush> and <Retaliation>.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};

const YAML: &str = include_str!("../../../cards/bt11/BT11-080.yaml");

#[test]
fn bt11_080_gains_rush_and_retaliation_with_own_yellow_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-080 YAML parses")
        .add_card(yellow_digimon("YELLOW-DIGIMON"))
        .start();
    let devimon = runner.place_on_field(0, "BT11-080", Some(0));
    runner.place_on_field(0, "YELLOW-DIGIMON", Some(0));

    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(devimon, Keyword::Rush));
    assert!(runner.game.has_keyword(devimon, Keyword::Retaliation));
}

#[test]
fn bt11_080_gains_keywords_with_own_yellow_tamer() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-080 YAML parses")
        .add_card(yellow_tamer("YELLOW-TAMER"))
        .start();
    let devimon = runner.place_on_field(0, "BT11-080", Some(0));
    runner.place_on_field(0, "YELLOW-TAMER", Some(0));

    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(devimon, Keyword::Rush));
    assert!(runner.game.has_keyword(devimon, Keyword::Retaliation));
}

#[test]
fn bt11_080_does_not_gain_keywords_without_yellow_permanent_or_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-080 YAML parses")
        .add_card(make_test_card("PURPLE", "Purple"))
        .add_card(yellow_digimon("YELLOW-DIGIMON"))
        .start();
    let devimon = runner.place_on_field(0, "BT11-080", Some(0));
    runner.place_on_field(0, "PURPLE", Some(0));

    runner.game.tick_declarative_effects();
    assert!(!runner.game.has_keyword(devimon, Keyword::Rush));
    assert!(!runner.game.has_keyword(devimon, Keyword::Retaliation));

    runner.game.players[0].battle_area.clear();
    let devimon = runner.place_on_field(0, "BT11-080", Some(0));
    runner.place_on_field(0, "YELLOW-DIGIMON", Some(0));
    runner.end_turn();
    runner.game.tick_declarative_effects();
    assert!(!runner.game.has_keyword(devimon, Keyword::Rush));
    assert!(!runner.game.has_keyword(devimon, Keyword::Retaliation));
}

fn yellow_digimon(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card
}

fn yellow_tamer(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Yellow];
    card
}
