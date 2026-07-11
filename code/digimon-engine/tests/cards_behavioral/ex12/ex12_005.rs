use digimon_engine::enums::CardColor;

use super::support::{hand_contains, select_hand_card, trash_contains, vb_digimon, DebugRunner};

const CARD_ID: &str = "EX12-005";

#[test]
fn ex12_005_on_play_trashes_greymon_or_vb_card_to_draw_two() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-005 YAML loads")
        .add_card(vb_digimon("VB-COST", CardColor::Red, 3, 2000))
        .add_card(vb_digimon("DRAW-1", CardColor::Red, 3, 2000))
        .add_card(vb_digimon("DRAW-2", CardColor::Red, 3, 2000))
        .hand(0, &[CARD_ID, "VB-COST"])
        .deck(0, &["DRAW-1", "DRAW-2"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play EX12-005");
    select_hand_card(&mut runner, 0, "VB-COST");
    runner.auto_resolve().expect("resolve EX12-005 On Play");

    assert!(trash_contains(&runner, 0, "VB-COST"));
    assert!(hand_contains(&runner, 0, "DRAW-1"));
    assert!(hand_contains(&runner, 0, "DRAW-2"));
}

#[test]
fn ex12_005_inherited_dp_aura_applies_only_on_your_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-005 YAML loads")
        .add_card(vb_digimon("CARRIER", CardColor::Red, 3, 3000))
        .memory(1)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert_eq!(runner.effective_dp(carrier), Some(5000));

    runner.end_turn();
    assert_eq!(runner.effective_dp(carrier), Some(3000));
}
