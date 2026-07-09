use digimon_engine::enums::{CardColor, Keyword};

use super::support::{
    black_sw, hand_index, plain_digimon, play_declining_optional_materials, select_first_non_pass,
    top_card_id, DebugRunner,
};

const CARD_ID: &str = "EX12-056";

#[test]
fn ex12_056_has_guard_keyword() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-056 YAML loads")
        .start();

    let cho_hakkaimon = runner.place_on_field(0, CARD_ID, Some(0));

    assert!(
        runner.game.has_keyword(cho_hakkaimon, Keyword::Guard),
        "Cho-Hakkaimon has printed <Guard>"
    );
}

#[test]
fn ex12_056_on_play_de_digivolves_then_grants_alliance_and_attack_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-056 YAML loads")
        .add_card(black_sw("SW-ALLY", 4))
        .add_card(plain_digimon("SRC", CardColor::Black, 3, 3000))
        .add_card(plain_digimon("OPP-LV4", CardColor::Black, 4, 5000))
        .add_card(plain_digimon("SEC", CardColor::Blue, 3, 3000))
        .hand(0, &[CARD_ID])
        .security(1, &["SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    let ally = runner.place_on_field(0, "SW-ALLY", Some(0));
    let opp = runner.place_stack(1, &["SRC", "OPP-LV4"]);

    let play_slot = hand_index(&runner, 0, CARD_ID);
    play_declining_optional_materials(&mut runner, 0, play_slot);
    select_first_non_pass(&mut runner);
    assert_eq!(
        top_card_id(&runner, 1, opp.index as usize),
        "SRC",
        "De-Digivolve 1 removes the opponent's top card"
    );

    select_first_non_pass(&mut runner);
    let view = runner
        .pending_selection_view()
        .expect("Alliance rider should expose optional attack targets");
    assert!(view.is_optional, "the immediate attack rider is optional");
    assert!(
        runner.game.has_keyword(ally, Keyword::Alliance),
        "the selected other [SW] Digimon gains Alliance"
    );
}
