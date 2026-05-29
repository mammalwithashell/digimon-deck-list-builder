use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::OptionPlayResult;

fn yellow_digimon(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.colors = vec![CardColor::Yellow];
    card
}

fn yellow_option(card_id: &str, cost: u16) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Option;
    card.level = None;
    card.dp = None;
    card.play_cost = cost;
    card.colors = vec![CardColor::Yellow];
    card
}

fn hand_ids(runner: &DebugRunner) -> Vec<String> {
    runner.game.players[0]
        .hand
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

const OPTION_USE_OBSERVER_YAML: &str = r#"
card: TEST-OPTION-USE-OBSERVER
name: Option Use Observer
kind: digi_egg
color: [yellow]
level: 2
traits: [Lesser]
effects:
  - scope: inherited
    when: on_use_option
    once_per_turn: true
    active_when:
      all_of:
        - your_turn: true
        - event_target_owner: you
        - event_card_use_cost_gte: 2
    process:
      - draw: { of: you, count: 1 }
"#;

#[test]
fn inherited_on_use_option_observer_sees_used_option_owner_and_cost() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(OPTION_USE_OBSERVER_YAML)
        .expect("option-use observer YAML parses")
        .add_card(yellow_digimon("CARRIER"))
        .add_card(yellow_option("COST-2-OPTION", 2))
        .add_card(make_test_card("DRAWN", "Drawn"))
        .hand(0, &["COST-2-OPTION"])
        .deck(0, &["DRAWN"])
        .memory(10)
        .start();
    runner.place_stack(0, &["TEST-OPTION-USE-OBSERVER", "CARRIER"]);
    runner.game.enter_main_phase();

    let result = runner.game.play_option_from_hand(0, 0);

    assert_eq!(result, OptionPlayResult::Trashed);
    assert!(
        hand_ids(&runner).contains(&"DRAWN".to_string()),
        "inherited observer should draw from the option-use event"
    );
}

#[test]
fn inherited_on_use_option_observer_rejects_options_below_cost_floor() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(OPTION_USE_OBSERVER_YAML)
        .expect("option-use observer YAML parses")
        .add_card(yellow_digimon("CARRIER"))
        .add_card(yellow_option("COST-1-OPTION", 1))
        .add_card(make_test_card("DRAWN", "Drawn"))
        .hand(0, &["COST-1-OPTION"])
        .deck(0, &["DRAWN"])
        .memory(10)
        .start();
    runner.place_stack(0, &["TEST-OPTION-USE-OBSERVER", "CARRIER"]);
    runner.game.enter_main_phase();

    let result = runner.game.play_option_from_hand(0, 0);

    assert_eq!(result, OptionPlayResult::Trashed);
    assert!(
        !hand_ids(&runner).contains(&"DRAWN".to_string()),
        "cost floor should suppress the inherited observer"
    );
}
