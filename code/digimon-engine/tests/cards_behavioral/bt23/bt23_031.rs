//! BT23-031 Angewomon.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Keyword};

const YAML: &str = include_str!("../../../cards/bt23/BT23-031.yaml");

#[test]
fn bt23_031_play_cost_reduces_with_ladydevimon_or_mirei() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT23-031 YAML loads")
        .add_card(digimon("LADY", "LadyDevimon"))
        .hand(0, &["BT23-031"])
        .memory(10)
        .start();
    runner.place_on_field(0, "LADY", Some(0));

    let before = runner.memory();
    runner.play(0, 0).expect("play BT23-031");
    assert_eq!(before - runner.memory(), 4, "7 printed cost - 3");
}

#[test]
fn bt23_031_on_play_adds_security_then_recovers_at_three_or_less() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT23-031 YAML loads")
        .add_card(make_test_card("S1", "S1"))
        .add_card(make_test_card("S2", "S2"))
        .add_card(make_test_card("S3", "S3"))
        .add_card(make_test_card("S4", "S4"))
        .add_card(make_test_card("RECOVER", "Recover"))
        .security(0, &["S1", "S2", "S3", "S4"])
        .deck(0, &["RECOVER"])
        .hand(0, &["BT23-031"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play BT23-031");
    runner.auto_resolve().expect("resolve recovery");

    let hand = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand.contains(&"S4".to_string()));
    assert_eq!(runner.security_count(0), 4);
    assert_eq!(
        runner.game.players[0]
            .security
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "RECOVER"
    );
}

#[test]
fn bt23_031_inherited_grants_alliance() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT23-031 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .start();
    let carrier = runner.place_stack(0, &["BT23-031", "CARRIER"]);

    assert!(runner.game.has_keyword(carrier, Keyword::Alliance));
}

fn digimon(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
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
