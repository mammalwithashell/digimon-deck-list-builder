//! BT8-035 Candlemon
//!
//! Inherited Effect [Your Turn] [Once Per Turn] When you play another purple
//! Digimon, gain 1 memory.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

#[test]
fn bt8_035_inherited_gains_memory_when_another_purple_digimon_is_played() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-035")
        .expect("BT8-035 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(purple_digimon("PURPLE"))
        .hand(0, &["PURPLE"])
        .memory(10)
        .start();
    runner.place_stack(0, &["BT8-035", "CARRIER"]);

    runner.play(0, 0).expect("play purple Digimon");

    assert_eq!(runner.memory(), 8, "play cost 3 then inherited gain 1");
}

#[test]
fn bt8_035_requires_purple_digimon_and_your_turn_once_per_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-035")
        .expect("BT8-035 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(yellow_digimon("YELLOW"))
        .add_card(purple_digimon("PURPLE1"))
        .add_card(purple_digimon("PURPLE2"))
        .hand(0, &["YELLOW", "PURPLE1", "PURPLE2"])
        .memory(10)
        .start();
    runner.place_stack(0, &["BT8-035", "CARRIER"]);

    runner.play(0, 0).expect("play yellow Digimon");
    assert_eq!(runner.memory(), 7, "non-purple Digimon should not trigger");

    runner.play(0, 0).expect("play first purple Digimon");
    assert_eq!(runner.memory(), 5, "first purple play gets +1 net");

    runner.play(0, 0).expect("play second purple Digimon");
    assert_eq!(runner.memory(), 2, "OPT blocks second memory gain");
}

fn purple_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.play_cost = 3;
    card
}

fn yellow_digimon(id: &str) -> CardData {
    let mut card = purple_digimon(id);
    card.colors = vec![CardColor::Yellow];
    card
}
