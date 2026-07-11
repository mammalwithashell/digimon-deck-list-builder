use crate::dsl_card_data::compiled;

use digimon_engine::permanent::OptionState;

use super::support::{field_contains, hand_index, select_hand_card, tb_digimon, DebugRunner};
use digimon_engine::enums::CardColor;

const CARD_ID: &str = "EX12-070";

#[test]
fn ex12_070_has_tb_use_requirement() {
    let card = compiled(CARD_ID);
    let requirement = card
        .use_requirement
        .as_ref()
        .and_then(|pred| pred.any_field_permanent.as_ref())
        .expect("EX12-070 prints Use Req. ([TB] trait)");

    assert_eq!(
        requirement.predicate.trait_has.as_deref(),
        Some("TB"),
        "Use Req. should look for an own [TB] trait permanent"
    );
}

#[test]
fn ex12_070_main_trashes_tb_card_draws_two_and_places_delay_option() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-070 YAML loads")
        .add_card(tb_digimon("TB-DISCARD", CardColor::Blue, 4, 5000))
        .add_card(tb_digimon("DRAW-A", CardColor::Blue, 3, 3000))
        .add_card(tb_digimon("DRAW-B", CardColor::Blue, 3, 3000))
        .hand(0, &[CARD_ID, "TB-DISCARD"])
        .deck(0, &["DRAW-A", "DRAW-B"])
        .memory(3)
        .start();

    let option_slot = hand_index(&runner, 0, CARD_ID);
    assert!(runner.game.activate_hand_main(0, option_slot));
    select_hand_card(&mut runner, 0, "TB-DISCARD");
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
