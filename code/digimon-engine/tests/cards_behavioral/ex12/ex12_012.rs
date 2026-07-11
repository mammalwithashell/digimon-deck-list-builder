use digimon_engine::enums::{CardColor, Keyword};

use super::support::{
    hand_contains, hand_index, plain_digimon, red_sw, select_hand_card, trash_contains, DebugRunner,
};

const CARD_ID: &str = "EX12-012";

#[test]
fn ex12_012_has_raid_keyword() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-012 YAML loads")
        .start();

    let apemon = runner.place_on_field(0, CARD_ID, Some(0));

    assert!(
        runner.game.has_keyword(apemon, Keyword::Raid),
        "Apemon has printed <Raid>"
    );
}

#[test]
fn ex12_012_on_play_trashes_sw_card_then_draws_two() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-012 YAML loads")
        .add_card(red_sw("SW-COST", 3))
        .add_card(plain_digimon("DRAW-1", CardColor::Red, 3, 3000))
        .add_card(plain_digimon("DRAW-2", CardColor::Red, 3, 3000))
        .hand(0, &[CARD_ID, "SW-COST"])
        .deck(0, &["DRAW-1", "DRAW-2"])
        .memory(10)
        .start();

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-012");
    select_hand_card(&mut runner, 0, "SW-COST");
    runner.auto_resolve().expect("finish EX12-012 on-play");

    assert!(trash_contains(&runner, 0, "SW-COST"));
    assert!(hand_contains(&runner, 0, "DRAW-1"));
    assert!(hand_contains(&runner, 0, "DRAW-2"));
    assert_eq!(runner.deck_size(0), 0, "draw 2 empties the test deck");
}

#[test]
fn ex12_012_inherited_dp_aura_applies_only_on_your_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-012 YAML loads")
        .add_card(plain_digimon("CARRIER", CardColor::Red, 3, 3000))
        .memory(1)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    assert_eq!(runner.effective_dp(carrier), Some(5000));
    runner.end_turn();
    assert_eq!(runner.effective_dp(carrier), Some(3000));
}
