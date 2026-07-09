use digimon_engine::enums::CardColor;
use digimon_engine::permanent::OptionState;

use super::support::{field_contains, hand_index, red_sw, select_hand_card, DebugRunner};

const CARD_ID: &str = "EX12-071";

#[test]
fn ex12_071_main_trashes_sw_card_draws_two_and_places_delay_option() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-071 YAML loads")
        .add_card(red_sw("SW-DISCARD", 4))
        .add_card(super::support::digimon(
            "DRAW-A",
            CardColor::Black,
            3,
            3,
            3000,
            &["SW"],
        ))
        .add_card(super::support::digimon(
            "DRAW-B",
            CardColor::Blue,
            3,
            3,
            3000,
            &["SW"],
        ))
        .hand(0, &[CARD_ID, "SW-DISCARD"])
        .deck(0, &["DRAW-A", "DRAW-B"])
        .memory(3)
        .start();

    let option_slot = hand_index(&runner, 0, CARD_ID);
    assert!(runner.game.activate_hand_main(0, option_slot));
    select_hand_card(&mut runner, 0, "SW-DISCARD");
    runner.auto_resolve().expect("resolve main");

    assert!(field_contains(&runner, 0, CARD_ID));
    assert_eq!(runner.hand_size(0), 2);
    let option = runner.game.players[0]
        .battle_area
        .iter()
        .find(|perm| perm.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("placed option");
    assert!(matches!(option.option_state, OptionState::Delayed { .. }));
}
