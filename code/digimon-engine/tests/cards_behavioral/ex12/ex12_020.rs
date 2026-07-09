use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};

const CARD_ID: &str = "EX12-020";

fn blue_digimon(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut card = make_test_card_with_level(id, id, level);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue];
    card.play_cost = if level <= 3 { 3 } else { 4 };
    card.dp = Some(if level <= 3 { 3000 } else { 5000 });
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn blue_lv4(id: &str, traits: &[&str]) -> CardData {
    let mut card = blue_digimon(id, 4, traits);
    card.evo_costs = vec![EvoCost {
        card_color: 1,
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

fn hand_size(runner: &DebugRunner, player: u8) -> usize {
    runner.game.players[player as usize].hand.len()
}

#[test]
fn ex12_020_reduces_own_digivolve_into_tb_by_1() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-020 YAML loads")
        .add_card(blue_lv4("TB-LV4", &["TB"]))
        .hand(0, &["TB-LV4"])
        .memory(5)
        .start();

    let source = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_slot = hand_index(&runner, 0, "TB-LV4");
    let memory_before = runner.memory();
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_slot, source.index as usize, PlaySource::ByHand);

    assert!(digivolved, "EX12-020 should digivolve into the TB level 4");
    assert_eq!(
        runner.memory(),
        memory_before,
        "cost 1 into [TB] should be reduced to 0"
    );
}

#[test]
fn ex12_020_does_not_reduce_non_tb_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-020 YAML loads")
        .add_card(blue_lv4("PLAIN-LV4", &[]))
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

    assert!(digivolved, "EX12-020 should still normally digivolve");
    assert_eq!(
        memory_before - runner.memory(),
        1,
        "non-TB target should pay the printed cost"
    );
}

#[test]
fn ex12_020_inherited_when_attacking_draws_if_hand_has_7_or_fewer_cards() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-020 YAML loads")
        .add_card(blue_digimon("CARRIER", 3, &[]))
        .add_card(blue_digimon("DRAW", 3, &[]))
        .add_card(blue_digimon("SEC", 3, &[]))
        .deck(0, &["DRAW"])
        .security(1, &["SEC"])
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let hand_before = hand_size(&runner, 0);
    let deck_before = runner.deck_size(0);

    runner.attack_player(carrier, 1, false);
    runner
        .auto_resolve()
        .expect("resolve EX12-020 attack trigger");

    assert_eq!(hand_size(&runner, 0), hand_before + 1, "draw 1");
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "deck loses drawn card"
    );
}

#[test]
fn ex12_020_inherited_when_attacking_does_not_draw_if_hand_has_8_or_more_cards() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-020 YAML loads")
        .add_card(blue_digimon("CARRIER", 3, &[]))
        .add_card(blue_digimon("DRAW", 3, &[]))
        .add_card(blue_digimon("SEC", 3, &[]))
        .add_card(blue_digimon("FILL", 3, &[]))
        .hand(
            0,
            &[
                "FILL", "FILL", "FILL", "FILL", "FILL", "FILL", "FILL", "FILL",
            ],
        )
        .deck(0, &["DRAW"])
        .security(1, &["SEC"])
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let hand_before = hand_size(&runner, 0);
    let deck_before = runner.deck_size(0);

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(hand_size(&runner, 0), hand_before, "no draw at 8 cards");
    assert_eq!(runner.deck_size(0), deck_before, "deck unchanged");
}
