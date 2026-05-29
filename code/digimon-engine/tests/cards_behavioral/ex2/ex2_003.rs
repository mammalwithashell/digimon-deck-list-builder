//! EX2-003 Viximon
//!
//! Inherited Effect [Your Turn] [Once Per Turn] When you use an Option card
//! with a cost 2 or more, <Draw 1>.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::OptionPlayResult;

const VIXIMON_YAML: &str = include_str!("../../../cards/ex2/EX2-003.yaml");

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

fn viximon_runner(option_id: &str, option_cost: u16) -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(VIXIMON_YAML)
        .expect("EX2-003 YAML parses")
        .add_card(yellow_digimon("CARRIER"))
        .add_card(yellow_option(option_id, option_cost))
        .add_card(make_test_card("DRAWN", "Drawn"))
        .hand(0, &[option_id])
        .deck(0, &["DRAWN"])
        .memory(10)
        .start();
    runner.place_stack(0, &["EX2-003", "CARRIER"]);
    runner.game.enter_main_phase();
    runner
}

#[test]
fn ex2_003_compiles_as_inherited_on_use_option_once_per_turn() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(VIXIMON_YAML)
        .expect("EX2-003 YAML parses")
        .start();
    let compiled = runner
        .compiled_card("EX2-003")
        .expect("EX2-003 compiled card present");
    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect();

    assert_eq!(triggered.len(), 1);
    assert_eq!(triggered[0].scope, CompiledScope::Inherited);
    assert_eq!(triggered[0].when, vec![CompiledTiming::OnUseOption]);
    assert!(triggered[0].once_per_turn);
}

#[test]
fn ex2_003_draws_when_owner_uses_option_with_cost_two_or_more() {
    let mut runner = viximon_runner("COST-2-OPTION", 2);

    let result = runner.game.play_option_from_hand(0, 0);

    assert_eq!(result, OptionPlayResult::Trashed);
    assert!(hand_ids(&runner).contains(&"DRAWN".to_string()));
}

#[test]
fn ex2_003_does_not_draw_for_option_below_cost_floor() {
    let mut runner = viximon_runner("COST-1-OPTION", 1);

    let result = runner.game.play_option_from_hand(0, 0);

    assert_eq!(result, OptionPlayResult::Trashed);
    assert!(!hand_ids(&runner).contains(&"DRAWN".to_string()));
}
