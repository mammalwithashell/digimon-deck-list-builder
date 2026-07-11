use digimon_engine::enums::{CardColor, Keyword};

use super::support::{
    hand_index, plain_digimon, play_declining_optional_materials, red_sw, select_first_non_pass,
    DebugRunner,
};

const CARD_ID: &str = "EX12-015";

#[test]
fn ex12_015_on_play_reduces_dp_then_grants_alliance_and_attack_prompt_to_other_sw() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-015 YAML loads")
        .add_card(red_sw("SW-ALLY", 4))
        .add_card(plain_digimon("OPP", CardColor::Blue, 4, 6000))
        .add_card(plain_digimon("SEC", CardColor::Blue, 3, 3000))
        .hand(0, &[CARD_ID])
        .security(1, &["SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    let ally = runner.place_on_field(0, "SW-ALLY", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    let play_slot = hand_index(&runner, 0, CARD_ID);
    play_declining_optional_materials(&mut runner, 0, play_slot);
    select_first_non_pass(&mut runner);
    assert_eq!(runner.effective_dp(opp), Some(2000), "target gets -4000 DP");

    select_first_non_pass(&mut runner);
    let view = runner
        .pending_selection_view()
        .expect("Alliance rider should expose optional attack targets");
    assert!(view.is_optional, "the immediate attack rider is optional");
    assert!(
        runner.game.has_keyword(ally, Keyword::Alliance),
        "the selected other [SW] Digimon gains Alliance for the turn"
    );
}

#[test]
fn ex12_015_inherited_attack_deletes_dp_6000_or_less() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-015 YAML loads")
        .add_card(plain_digimon("CARRIER", CardColor::Red, 4, 7000))
        .add_card(plain_digimon("OPP", CardColor::Blue, 4, 6000))
        .add_card(plain_digimon("SEC", CardColor::Blue, 3, 3000))
        .security(1, &["SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.place_on_field(1, "OPP", Some(0));

    runner.attack_player(carrier, 1, false);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish inherited deletion");

    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "OPP"),
        "Gokuumon inherited effect deletes the selected 6000 DP Digimon"
    );
}
