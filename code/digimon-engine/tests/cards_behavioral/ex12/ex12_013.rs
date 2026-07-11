use digimon_engine::enums::{CardColor, Keyword};

use super::support::{select_hand_card, vb_digimon, DebugRunner};

const CARD_ID: &str = "EX12-013";

#[test]
fn ex12_013_main_plays_vb_card_from_hand_with_cost_reduced_by_two() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-013 YAML loads")
        .add_card(vb_digimon("VB-PLAY", CardColor::Red, 3, 2000))
        .hand(0, &["VB-PLAY"])
        .memory(10)
        .start();
    let betel = runner.place_on_field(0, CARD_ID, Some(0));
    let field_before = runner.battle_area_size(0);
    let memory_before = runner.memory();

    assert!(
        runner.game.activate_field_main(0, betel.index as usize),
        "BetelGammamon [Main] should activate from field"
    );
    select_hand_card(&mut runner, 0, "VB-PLAY");
    runner.auto_resolve().expect("resolve EX12-013 Main");

    assert_eq!(runner.battle_area_size(0), field_before + 1);
    assert_eq!(
        memory_before - runner.memory(),
        1,
        "play cost 3 reduced by 2 spends 1 memory"
    );
}

#[test]
fn ex12_013_inherited_grants_barrier() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-013 YAML loads")
        .add_card(vb_digimon("CARRIER", CardColor::Red, 4, 6000))
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    assert!(
        runner.game.has_keyword(carrier, Keyword::Barrier),
        "BetelGammamon inherited effect grants Barrier"
    );
}
