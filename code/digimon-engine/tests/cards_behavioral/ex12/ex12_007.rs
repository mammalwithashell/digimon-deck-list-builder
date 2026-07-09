use digimon_engine::enums::CardColor;

use super::support::{hand_ids, plain_digimon, vb_digimon, vb_text_digimon, DebugRunner};

const CARD_ID: &str = "EX12-007";

#[test]
fn ex12_007_on_play_reveals_three_adds_gammamon_text_and_vb_cards_rest_to_bottom() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-007 YAML loads")
        .add_card(vb_text_digimon("GAMMA-TEXT", CardColor::Red, 3, 2000))
        .add_card(vb_digimon("VB-PICK", CardColor::Yellow, 3, 2000))
        .add_card(plain_digimon("FILLER", CardColor::Blue, 3, 2000))
        .add_card(vb_digimon("PAD", CardColor::Red, 3, 2000))
        .hand(0, &[CARD_ID])
        .deck(0, &["PAD", "GAMMA-TEXT", "VB-PICK", "FILLER"])
        .memory(10)
        .start();

    let deck_before = runner.deck_size(0);
    runner.play(0, 0).expect("play EX12-007");
    runner.auto_resolve().expect("resolve EX12-007 reveal");

    let hand = hand_ids(&runner, 0);
    assert!(hand.iter().any(|id| id == "GAMMA-TEXT"), "hand={hand:?}");
    assert!(hand.iter().any(|id| id == "VB-PICK"), "hand={hand:?}");
    assert_eq!(
        runner.deck_size(0),
        deck_before - 2,
        "two selected cards leave the deck; one reveal remainder returns to bottom"
    );
}

#[test]
fn ex12_007_inherited_dp_aura_applies_only_on_your_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-007 YAML loads")
        .add_card(vb_digimon("CARRIER", CardColor::Yellow, 3, 3000))
        .memory(1)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert_eq!(runner.effective_dp(carrier), Some(5000));

    runner.end_turn();
    assert_eq!(runner.effective_dp(carrier), Some(3000));
}
