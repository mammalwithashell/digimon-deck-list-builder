//! ST10-04 Gatomon.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::SEL_REVEAL_START;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const YAML: &str = include_str!("../../../cards/st10/ST10-04.yaml");

#[test]
fn st10_04_reveal_adds_yellow_and_purple_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("ST10-04 YAML loads")
        .add_card(colored_digimon("YELLOW", CardColor::Yellow))
        .add_card(colored_digimon("PURPLE", CardColor::Purple))
        .add_card(colored_tamer("YELLOW-TAMER", CardColor::Yellow))
        .deck(0, &["YELLOW", "PURPLE", "YELLOW-TAMER"])
        .hand(0, &["ST10-04"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play ST10-04");
    pick_revealed(&mut runner, "YELLOW");
    pick_revealed(&mut runner, "PURPLE");
    runner.auto_resolve().expect("bottom reveal remainder");

    let hand = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand.contains(&"YELLOW".to_string()));
    assert!(hand.contains(&"PURPLE".to_string()));
    assert!(
        !hand.contains(&"YELLOW-TAMER".to_string()),
        "yellow Tamer must not satisfy the yellow Digimon bucket"
    );
}

#[test]
fn st10_04_cost_reduction_and_inherited_dna_registration_compile() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("ST10-04 YAML loads")
        .start();
    let card = runner.compiled_card("ST10-04").expect("compiled card");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
            amount: Some(2),
            active_when: Some(_),
            ..
        })
    )));
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::AltPathRegistration {
            scope: CompiledScope::Inherited,
            trigger: CompiledTiming::EndOfYourTurn,
            registers,
            ..
        }) if registers.kind == CompiledAltPathKind::DnaDigivolve
    )));
}

fn colored_digimon(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.colors = vec![color];
    card.card_kind = CardKind::Digimon;
    card
}

fn colored_tamer(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.colors = vec![color];
    card.card_kind = CardKind::Tamer;
    card
}

fn pick_revealed(runner: &mut DebugRunner, id: &str) {
    let action = runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
        .expect("revealed card exists");
    let view = runner.pending_selection_view().expect("reveal selection");
    assert!(
        view.valid_action_ids.contains(&action),
        "{id} should be selectable from reveal"
    );
    assert_mask_allows(runner, view.selecting_player, action);
    runner
        .execute_action(0, action)
        .expect("pick revealed card");
}

fn assert_mask_allows(runner: &DebugRunner, player: u8, action: u16) {
    assert_eq!(
        build_action_mask(&runner.game, player)[action as usize],
        1.0,
        "pending action {action} must be legal in the action mask"
    );
}

fn zone_ids(cards: &[digimon_engine::card_source::CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}
