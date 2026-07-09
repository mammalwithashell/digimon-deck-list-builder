use digimon_engine::enums::CardColor;

use super::support::{
    bottom_source_id, field_contains, hand_contains, hand_index, plain_digimon,
    select_first_non_pass, select_hand_card, source_count, tb_digimon, DebugRunner,
};

const CARD_ID: &str = "EX12-031";

#[test]
fn ex12_031_on_play_places_matching_hand_card_as_bottom_source_then_bounces_low_source_digimon() {
    let aqua = super::support::digimon("AQUATIC-HAND", CardColor::Blue, 4, 4, 5000, &["Aquatic"]);
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-031 YAML loads")
        .add_card(aqua)
        .add_card(plain_digimon("OPP-SRC", CardColor::Blue, 3, 3000))
        .add_card(plain_digimon("OPP-TOP", CardColor::Blue, 4, 5000))
        .hand(0, &[CARD_ID, "AQUATIC-HAND"])
        .memory(10)
        .start();
    runner.place_stack(1, &["OPP-SRC", "OPP-TOP"]);

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-031");
    select_hand_card(&mut runner, 0, "AQUATIC-HAND");
    assert_eq!(bottom_source_id(&runner, 0, 0), "AQUATIC-HAND");
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish bounce");

    assert_eq!(source_count(&runner, 0, 0), 1);
    assert!(hand_contains(&runner, 1, "OPP-TOP"));
    assert_eq!(runner.battle_area_size(1), 0);
}

#[test]
fn ex12_031_decode_plays_matching_tb_source_on_non_battle_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-031 YAML loads")
        .add_card(tb_digimon("TB-SRC", CardColor::Blue, 4, 5000))
        .memory(10)
        .start();
    let marine_bullmon = runner.place_stack(0, &["TB-SRC", CARD_ID]);

    runner.game.return_to_hand_from_effect(marine_bullmon, 1);
    select_first_non_pass(&mut runner);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish Decode");

    assert!(
        field_contains(&runner, 0, "TB-SRC"),
        "Decode plays the matching [TB] source"
    );
    assert_eq!(
        runner.hand_size(0),
        1,
        "the original non-battle leave still returns EX12-031 to hand"
    );
}
