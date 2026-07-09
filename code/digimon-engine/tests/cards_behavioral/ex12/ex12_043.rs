use digimon_engine::enums::{CardColor, Keyword};

use super::support::{hand_index, select_hand_card, top_card_id, yellow_sw, DebugRunner};

const CARD_ID: &str = "EX12-043";

#[test]
fn ex12_043_main_plays_sw_from_hand_with_cost_reduced_by_2() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-043 YAML loads")
        .add_card(yellow_sw("SW-PLAY", 3))
        .hand(0, &["SW-PLAY"])
        .memory(10)
        .start();
    let hakubamon = runner.place_on_field(0, CARD_ID, Some(0));
    let field_before = runner.battle_area_size(0);
    let memory_before = runner.memory();

    assert!(
        runner.game.activate_field_main(0, hakubamon.index as usize),
        "Hakubamon's [Main] effect should activate from field"
    );
    select_hand_card(&mut runner, 0, "SW-PLAY");
    runner.auto_resolve().expect("finish Hakubamon [Main]");

    assert_eq!(runner.battle_area_size(0), field_before + 1);
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "SW-PLAY"),
        "the selected [SW] Digimon is played"
    );
    assert_eq!(
        memory_before - runner.memory(),
        1,
        "play cost 3 reduced by 2 spends 1 memory"
    );
}

#[test]
fn ex12_043_inherited_grants_barrier() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-043 YAML loads")
        .add_card(super::support::plain_digimon(
            "CARRIER",
            CardColor::Yellow,
            3,
            3000,
        ))
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    assert!(
        runner.game.has_keyword(carrier, Keyword::Barrier),
        "Hakubamon inherited effect grants Barrier"
    );
    assert_eq!(top_card_id(&runner, 0, carrier.index as usize), "CARRIER");
}
