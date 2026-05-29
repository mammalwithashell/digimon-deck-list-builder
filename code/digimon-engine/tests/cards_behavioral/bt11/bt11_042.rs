//! BT11-042 Angewomon.

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::SEL_MY_SECURITY_START;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/bt11/BT11-042.yaml");

#[test]
fn bt11_042_when_digivolving_adds_security_angel_and_recovers() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-042 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .add_card(trait_digimon("ANGEL", "Angel"))
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_test_card("RECOVER", "Recover"))
        .security(0, &["FILLER", "ANGEL"])
        .deck(0, &["RECOVER"])
        .start();
    let angewomon = runner.place_stack(0, &["BASE", "BT11-042"]);

    fire(&mut runner, EffectTiming::WhenDigivolving, angewomon);
    let action = security_action_for(&runner, "ANGEL");
    assert_mask_allows(&runner, 0, action);
    runner.execute_action(0, action).expect("select ANGEL");
    runner.auto_resolve().expect("recover and shuffle");

    let hand = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand.contains(&"ANGEL".to_string()));
    assert_eq!(runner.security_count(0), 2, "add one, recover one");
    assert!(security_contains(&runner, 0, "RECOVER"));
}

#[test]
fn bt11_042_on_ladydevimon_or_mirei_play_gains_memory_once() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-042 YAML loads")
        .add_card(zero_cost_digimon("LADY", "LadyDevimon"))
        .hand(0, &["LADY"])
        .memory(0)
        .start();
    runner.place_on_field(0, "BT11-042", Some(0));

    runner.play(0, 0).expect("play LadyDevimon");
    runner.auto_resolve().expect("gain memory");

    assert_eq!(runner.memory(), 1);
}

#[test]
fn bt11_042_inherited_grants_blocker_with_purple_digimon() {
    let mut purple = make_test_card("PURPLE", "Purple");
    purple.card_kind = CardKind::Digimon;
    purple.colors = vec![digimon_engine::enums::CardColor::Purple];

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-042 YAML loads")
        .add_card(trait_digimon("CARRIER", "Archangel"))
        .add_card(purple)
        .start();
    let carrier = runner.place_stack(0, &["BT11-042", "CARRIER"]);
    runner.place_on_field(0, "PURPLE", Some(0));
    runner.end_turn();
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(carrier, Keyword::Blocker));
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

fn trait_digimon(id: &str, trait_name: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.traits = vec![trait_name.to_string()];
    card
}

fn zero_cost_digimon(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.play_cost = 0;
    card
}

fn security_action_for(runner: &DebugRunner, id: &str) -> u16 {
    let idx = runner.game.players[0]
        .security
        .iter()
        .position(|card| card.card_id(&runner.game.card_data) == id)
        .expect("security card exists");
    SEL_MY_SECURITY_START + idx as u16
}

fn security_contains(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player]
        .security
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == id)
}

fn zone_ids(cards: &[digimon_engine::card_source::CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

fn assert_mask_allows(runner: &DebugRunner, player: u8, action: u16) {
    assert_eq!(
        build_action_mask(&runner.game, player)[action as usize],
        1.0,
        "pending action {action} must be legal in the action mask"
    );
}
