use digimon_engine::enums::CardColor;

use super::support::{
    field_contains, hand_contains, hand_index, plain_digimon, select_first_non_pass,
    select_hand_card, DebugRunner,
};

const CARD_ID: &str = "EX12-062";

#[test]
fn ex12_062_on_play_deletes_own_digimon_as_cost_to_delete_opponent_level4_or_lower() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-062 YAML loads")
        .add_card(plain_digimon("OWN-COST", CardColor::Purple, 3, 3000))
        .add_card(plain_digimon("OPP-LV4", CardColor::Purple, 4, 5000))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    runner.place_on_field(0, "OWN-COST", Some(0));
    runner.place_on_field(1, "OPP-LV4", Some(0));

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-062");
    select_first_non_pass(&mut runner);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish delete");

    assert!(!field_contains(&runner, 0, "OWN-COST"));
    assert!(field_contains(&runner, 0, CARD_ID));
    assert!(!field_contains(&runner, 1, "OPP-LV4"));
}

#[test]
fn ex12_062_inherited_when_attacking_draws_then_trashes_one_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-062 YAML loads")
        .add_card(plain_digimon("CARRIER", CardColor::Purple, 3, 3000))
        .add_card(plain_digimon("DRAW", CardColor::Purple, 3, 3000))
        .add_card(plain_digimon("SEC", CardColor::Purple, 3, 3000))
        .deck(0, &["DRAW"])
        .security(1, &["SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    runner.attack_player(carrier, 1, false);
    select_hand_card(&mut runner, 0, "DRAW");
    runner.auto_resolve().expect("finish draw then trash");

    assert!(!hand_contains(&runner, 0, "DRAW"));
}
