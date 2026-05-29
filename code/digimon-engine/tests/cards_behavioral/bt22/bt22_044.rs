//! BT22-044 Palmon.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef};

#[test]
fn bt22_044_gains_memory_when_effect_places_cs_source_under_it() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-044")
        .expect("BT22-044 YAML loads")
        .add_card(digimon("BASE", "Base", &[], 3))
        .add_card(digimon("PLACED-CS", "Placed CS", &["CS"], 3))
        .add_card(make_test_card("EFFECT", "Effect Source"))
        .deck(0, &["PLACED-CS"])
        .memory(0)
        .start();

    let host = runner.place_stack(0, &["BASE", "BT22-044"]);
    place_deck_top_source_by_effect(&mut runner, host);

    assert_eq!(runner.memory(), 1);
}

#[test]
fn bt22_044_inherited_main_rotates_top_source_and_draws() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-044")
        .expect("BT22-044 YAML loads")
        .add_card(digimon("MID", "Middle", &["CS"], 3))
        .add_card(digimon("CARRIER", "Carrier", &[], 4))
        .add_card(digimon("DRAW", "Draw", &[], 3))
        .deck(0, &["DRAW"])
        .start();

    let carrier = runner.place_stack(0, &["BT22-044", "MID", "CARRIER"]);
    assert_eq!(runner.deck_size(0), 1);

    assert!(
        runner.game.activate_field_main(0, carrier.index as usize),
        "inherited [Main] should activate from BT22-044 as a source"
    );

    assert_eq!(runner.deck_size(0), 0, "Draw 1 should consume the deck top");
    let ids: Vec<String> = runner.game.players[0].battle_area[carrier.index as usize]
        .card_sources
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        ids,
        vec!["MID", "BT22-044", "CARRIER"],
        "the top stacked card should rotate to the bottom"
    );
}

fn place_deck_top_source_by_effect(
    runner: &mut DebugRunner,
    host: digimon_engine::permanent::PermanentHandle,
) {
    let effect_perm = runner.place_on_field(0, "EFFECT", None);
    let effect_card = runner.top_card(effect_perm);
    let mut ctx = EffectContext::new(&mut runner.game, effect_card, Some(effect_perm), 0);
    assert!(ctx.place_as_bottom_source(CardSourceRef::DeckTop(0), host, false));
}

fn digimon(id: &str, name: &str, traits: &[&str], level: u8) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(1000);
    card.colors = vec![CardColor::Green, CardColor::Yellow];
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}
