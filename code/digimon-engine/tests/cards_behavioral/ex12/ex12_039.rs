use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, PlaySource};

const CARD_ID: &str = "EX12-039";

fn yellow_digimon(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut card = make_test_card_with_level(id, id, level);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.play_cost = if level <= 3 { 3 } else { 4 };
    card.dp = Some(if level <= 3 { 3000 } else { 5000 });
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn yellow_lv4(id: &str, traits: &[&str]) -> CardData {
    let mut card = yellow_digimon(id, 4, traits);
    card.evo_costs = vec![EvoCost {
        card_color: 2,
        level: 3,
        memory_cost: 1,
    }];
    card
}

fn hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|card| card.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

#[test]
fn ex12_039_reduces_own_digivolve_into_sw_by_1() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-039 YAML loads")
        .add_card(yellow_lv4("SW-LV4", &["SW"]))
        .hand(0, &["SW-LV4"])
        .memory(5)
        .start();

    let source = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_slot = hand_index(&runner, 0, "SW-LV4");
    let memory_before = runner.memory();
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_slot, source.index as usize, PlaySource::ByHand);

    assert!(digivolved, "EX12-039 should digivolve into the SW level 4");
    assert_eq!(runner.memory(), memory_before, "cost 1 should reduce to 0");
}

#[test]
fn ex12_039_does_not_reduce_non_sw_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-039 YAML loads")
        .add_card(yellow_lv4("PLAIN-LV4", &[]))
        .hand(0, &["PLAIN-LV4"])
        .memory(5)
        .start();

    let source = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_slot = hand_index(&runner, 0, "PLAIN-LV4");
    let memory_before = runner.memory();
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_slot, source.index as usize, PlaySource::ByHand);

    assert!(digivolved, "EX12-039 should still normally digivolve");
    assert_eq!(memory_before - runner.memory(), 1);
}

#[test]
fn ex12_039_inherited_barrier_is_available_from_stack() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-039 YAML loads")
        .add_card(yellow_digimon("CARRIER", 3, &[]))
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert!(
        runner.game.has_keyword(carrier, Keyword::Barrier),
        "carrier inherits Barrier from EX12-039"
    );
}
