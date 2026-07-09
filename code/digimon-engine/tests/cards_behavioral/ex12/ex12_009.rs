use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "EX12-009";

fn red_digimon(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card_with_level(id, id, 3);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.play_cost = 3;
    card.dp = Some(3000);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn option_with_trait(id: &str, trait_name: &str) -> CardData {
    let mut card = red_digimon(id, &[trait_name]);
    card.card_kind = CardKind::Option;
    card.level = None;
    card.dp = None;
    card
}

fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .hand
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

#[test]
fn ex12_009_on_play_reveals_three_adds_shambala_and_tb_cards_rest_to_bottom() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-009 YAML loads")
        .add_card(option_with_trait("SHAMBALA-OPTION", "Shambala"))
        .add_card(red_digimon("TB-DIGIMON", &["TB"]))
        .add_card(red_digimon("FILLER", &[]))
        .add_card(red_digimon("PAD", &[]))
        .hand(0, &[CARD_ID])
        .deck(0, &["PAD", "SHAMBALA-OPTION", "TB-DIGIMON", "FILLER"])
        .memory(10)
        .start();
    let deck_before = runner.deck_size(0);

    assert!(runner.play(0, 0).is_some(), "EX12-009 should be playable");
    runner.auto_resolve().expect("resolve EX12-009 reveal");

    let hand = hand_ids(&runner, 0);
    assert!(
        hand.iter().any(|id| id == "SHAMBALA-OPTION"),
        "hand={hand:?}"
    );
    assert!(hand.iter().any(|id| id == "TB-DIGIMON"), "hand={hand:?}");
    assert!(
        !hand.iter().any(|id| id == "FILLER"),
        "non-matching revealed card must not be added: {hand:?}"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 2,
        "two selected cards leave deck; the unselected card returns to bottom"
    );
    assert_eq!(
        runner.game.players[0].deck[0].card_id(&runner.game.card_data),
        "FILLER",
        "unpicked reveal remainder returns to deck bottom"
    );
}

#[test]
fn ex12_009_inherited_dp_aura_applies_only_on_your_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-009 YAML loads")
        .add_card(red_digimon("CARRIER", &[]))
        .memory(1)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert_eq!(
        runner.effective_dp(carrier),
        Some(5000),
        "EX12-009 inherited +2000 DP applies on your turn"
    );

    runner.end_turn();
    assert_eq!(
        runner.effective_dp(carrier),
        Some(3000),
        "EX12-009 inherited +2000 DP turns off outside your turn"
    );
}
