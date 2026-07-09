use digimon_engine::enums::{CardColor, Keyword};

use super::support::{
    blue_sw, hand_contains, hand_index, plain_digimon, select_first_non_pass, DebugRunner,
};

const CARD_ID: &str = "EX12-025";

#[test]
fn ex12_025_has_blocker_keyword() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-025 YAML loads")
        .start();

    let gawappamon = runner.place_on_field(0, CARD_ID, Some(0));

    assert!(
        runner.game.has_keyword(gawappamon, Keyword::Blocker),
        "Gawappamon has printed <Blocker>"
    );
}

#[test]
fn ex12_025_on_play_may_return_level4_or_lower_opponent_digimon_to_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-025 YAML loads")
        .add_card(plain_digimon("OPP-LV4", CardColor::Blue, 4, 5000))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-LV4", Some(0));

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-025");
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish EX12-025 bounce");

    assert!(hand_contains(&runner, 1, "OPP-LV4"));
    assert_eq!(runner.battle_area_size(1), 0);
}

#[test]
fn ex12_025_inherited_when_attacking_draws_if_hand_has_7_or_fewer_cards() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-025 YAML loads")
        .add_card(plain_digimon("CARRIER", CardColor::Blue, 3, 3000))
        .add_card(blue_sw("DRAW", 3))
        .add_card(plain_digimon("SEC", CardColor::Blue, 3, 3000))
        .deck(0, &["DRAW"])
        .security(1, &["SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let hand_before = runner.game.players[0].hand.len();

    runner.attack_player(carrier, 1, false);
    runner.auto_resolve().expect("finish inherited draw");

    assert_eq!(runner.game.players[0].hand.len(), hand_before + 1);
    assert!(hand_contains(&runner, 0, "DRAW"));
}
