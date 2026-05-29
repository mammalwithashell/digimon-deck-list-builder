//! BT11-083 LadyDevimon.

use digimon_engine::action::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/bt11/BT11-083.yaml");

#[test]
fn bt11_083_when_digivolving_discards_then_returns_mirei_or_angel_trait() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-083 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_test_card("DISCARD", "Discard"))
        .add_card(tamer("MIREI", "Mirei Mikagura"))
        .hand(0, &["DISCARD"])
        .start();
    push_to_trash(&mut runner, 0, "MIREI");
    let lady = runner.place_stack(0, &["BASE", "BT11-083"]);

    fire(&mut runner, EffectTiming::WhenDigivolving, lady);
    let discard_pick = runner
        .pending_selection_view()
        .expect("discard prompt")
        .valid_action_ids[0];
    assert_mask_allows(&runner, 0, discard_pick);
    runner.execute_action(0, discard_pick).expect("discard");
    let trash_pick = runner
        .pending_selection_view()
        .expect("trash return prompt")
        .valid_action_ids[0];
    assert_mask_allows(&runner, 0, trash_pick);
    runner.execute_action(0, trash_pick).expect("return Mirei");

    let hand = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    let trash = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert!(hand.contains(&"MIREI".to_string()));
    assert!(trash.contains(&"DISCARD".to_string()));
}

#[test]
fn bt11_083_on_angewomon_or_mirei_play_gains_memory_once() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-083 YAML loads")
        .add_card(zero_cost_digimon("ANGE", "Angewomon"))
        .hand(0, &["ANGE"])
        .memory(0)
        .start();
    runner.place_on_field(0, "BT11-083", Some(0));

    runner.play(0, 0).expect("play Angewomon");
    runner.auto_resolve().expect("gain memory");

    assert_eq!(runner.memory(), 1);
}

#[test]
fn bt11_083_inherited_grants_retaliation_with_yellow_digimon() {
    let mut yellow = make_test_card("YELLOW", "Yellow");
    yellow.card_kind = CardKind::Digimon;
    yellow.colors = vec![CardColor::Yellow];

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-083 YAML loads")
        .add_card(trait_digimon("CARRIER", "Fallen Angel"))
        .add_card(yellow)
        .start();
    let carrier = runner.place_stack(0, &["BT11-083", "CARRIER"]);
    runner.place_on_field(0, "YELLOW", Some(0));
    runner.end_turn();
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(carrier, Keyword::Retaliation));
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

fn zero_cost_digimon(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.play_cost = 0;
    card
}

fn trait_digimon(id: &str, trait_name: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.traits = vec![trait_name.to_string()];
    card
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card
}

fn push_to_trash(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} not found in card_data"));
    let next = runner.game.next_card_index();
    runner.game.players[player]
        .trash
        .push(CardSource::new(data_idx, player as u8, next));
}

fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
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
