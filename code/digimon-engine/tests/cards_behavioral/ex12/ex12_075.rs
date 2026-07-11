use crate::dsl_card_data::compiled;

use super::support::{
    field_contains, hand_contains, hand_index, plain_digimon, select_first_non_pass, tb_digimon,
    DebugRunner,
};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::OptionState;

const CARD_ID: &str = "EX12-075";

#[test]
fn ex12_075_has_shambala_use_requirement() {
    let card = compiled(CARD_ID);
    let requirement = card
        .use_requirement
        .as_ref()
        .and_then(|pred| pred.any_field_permanent.as_ref())
        .expect("EX12-075 prints Use Req. ([Shambala] trait)");

    assert_eq!(
        requirement.predicate.trait_has.as_deref(),
        Some("Shambala"),
        "Use Req. should look for an own [Shambala] trait permanent"
    );
}

#[test]
fn ex12_075_main_adds_shambala_from_reveal_and_places_delay_option() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-075 YAML loads")
        .add_card(tb_digimon("HIT", CardColor::Purple, 4, 5000))
        .add_card(plain_digimon("MISS-A", CardColor::Purple, 3, 3000))
        .add_card(plain_digimon("MISS-B", CardColor::Purple, 3, 3000))
        .hand(0, &[CARD_ID])
        .deck(0, &["HIT", "MISS-A", "MISS-B"])
        .memory(3)
        .start();

    let option_slot = hand_index(&runner, 0, CARD_ID);
    assert!(runner.game.activate_hand_main(0, option_slot));
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve search");

    assert!(hand_contains(&runner, 0, "HIT"));
    assert!(field_contains(&runner, 0, CARD_ID));
    let option = runner.game.players[0]
        .battle_area
        .iter()
        .find(|perm| perm.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("placed option");
    assert!(matches!(option.option_state, OptionState::Delayed { .. }));
}
