use digimon_engine::enums::CardColor;

use super::support::{
    fire_own_security_removed, hand_contains, hand_index, plain_digimon, select_first_non_pass,
    select_hand_card, yellow_sw, DebugRunner,
};

const CARD_ID: &str = "EX12-045";

#[test]
fn ex12_045_on_play_adds_top_security_then_recovers_when_at_two_or_less() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-045 YAML loads")
        .add_card(plain_digimon("SEC-A", CardColor::Yellow, 3, 3000))
        .add_card(plain_digimon("SEC-B", CardColor::Yellow, 3, 3000))
        .add_card(plain_digimon("SEC-C", CardColor::Yellow, 3, 3000))
        .add_card(plain_digimon("RECOVER", CardColor::Yellow, 3, 3000))
        .hand(0, &[CARD_ID])
        .security(0, &["SEC-A", "SEC-B", "SEC-C"])
        .deck(0, &["RECOVER"])
        .memory(10)
        .start();

    let hand_before = runner.game.players[0].hand.len();
    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-045");
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish EX12-045 on-play");

    assert_eq!(
        runner.security_count(0),
        3,
        "starting at 3 security: add 1 to hand leaves 2, then Recovery +1 returns to 3"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "played EX12-045 from hand and added one security card to hand"
    );
    assert_eq!(runner.deck_size(0), 0, "Recovery +1 uses the deck card");
}

#[test]
fn ex12_045_security_removed_plays_sw_from_hand_with_cost_reduced_by_2() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-045 YAML loads")
        .add_card(yellow_sw("SW-PLAY", 3))
        .add_card(plain_digimon("SEC", CardColor::Yellow, 3, 3000))
        .hand(0, &["SW-PLAY"])
        .security(0, &["SEC"])
        .memory(10)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let field_before = runner.battle_area_size(0);
    let memory_before = runner.memory();

    fire_own_security_removed(&mut runner, 0);
    select_hand_card(&mut runner, 0, "SW-PLAY");
    runner
        .auto_resolve()
        .expect("finish EX12-045 security-removed play");

    assert_eq!(runner.battle_area_size(0), field_before + 1);
    assert!(!hand_contains(&runner, 0, "SW-PLAY"));
    assert_eq!(memory_before - runner.memory(), 1);
}

#[test]
fn ex12_045_inherited_when_attacking_reduces_one_opponent_digimon_by_4000() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-045 YAML loads")
        .add_card(plain_digimon("CARRIER", CardColor::Yellow, 4, 7000))
        .add_card(plain_digimon("OPP", CardColor::Blue, 4, 6000))
        .add_card(plain_digimon("SEC", CardColor::Blue, 3, 3000))
        .security(1, &["SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.attack_player(carrier, 1, false);
    select_first_non_pass(&mut runner);
    runner
        .auto_resolve()
        .expect("finish inherited DP reduction");

    assert_eq!(runner.effective_dp(opp), Some(2000));
}
