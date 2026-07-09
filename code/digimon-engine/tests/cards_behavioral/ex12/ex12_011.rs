use digimon_engine::enums::{CardColor, Keyword};

use super::support::{
    field_contains, hand_index, plain_digimon, select_first_non_pass, DebugRunner,
};

const CARD_ID: &str = "EX12-011";

#[test]
fn ex12_011_has_raid_keyword() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-011 YAML loads")
        .start();

    let seasarmon = runner.place_on_field(0, CARD_ID, Some(0));

    assert!(
        runner.game.has_keyword(seasarmon, Keyword::Raid),
        "Seasarmon has printed <Raid>"
    );
}

#[test]
fn ex12_011_on_play_deletes_opponent_digimon_with_5000_dp_or_less() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-011 YAML loads")
        .add_card(plain_digimon("OPP-LOW", CardColor::Red, 4, 5000))
        .add_card(plain_digimon("OPP-HIGH", CardColor::Red, 4, 6000))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-LOW", Some(0));
    runner.place_on_field(1, "OPP-HIGH", Some(0));

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-011");
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish delete");

    assert!(!field_contains(&runner, 1, "OPP-LOW"));
    assert!(field_contains(&runner, 1, "OPP-HIGH"));
}

#[test]
fn ex12_011_inherited_dp_aura_applies_only_on_your_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-011 YAML loads")
        .add_card(plain_digimon("CARRIER", CardColor::Red, 3, 3000))
        .memory(1)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    assert_eq!(runner.effective_dp(carrier), Some(5000));
    runner.end_turn();
    assert_eq!(runner.effective_dp(carrier), Some(3000));
}
