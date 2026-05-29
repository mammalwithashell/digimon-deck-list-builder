//! BT22-054 Hagurumon.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef};

#[test]
fn bt22_054_source_placement_gives_opponent_digimon_minus_3000_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-054")
        .expect("BT22-054 YAML loads")
        .add_card(digimon("BASE", "Base", &[], 3))
        .add_card(digimon("PLACED-CS", "Placed CS", &["CS"], 3))
        .add_card(make_test_card("EFFECT", "Effect Source"))
        .add_card(digimon("OPPONENT", "Opponent", &[], 4))
        .deck(0, &["PLACED-CS"])
        .memory(0)
        .start();

    let host = runner.place_stack(0, &["BASE", "BT22-054"]);
    let opponent = runner.place_on_field(1, "OPPONENT", Some(1));
    assert_eq!(runner.effective_dp(opponent), Some(1000));

    place_deck_top_source_by_effect(&mut runner, host);
    let action = runner
        .pending_selection_view()
        .expect("opponent Digimon choice should be pending")
        .valid_action_ids[0];
    runner
        .execute_action(0, action)
        .expect("select opponent Digimon");

    assert_eq!(runner.effective_dp(opponent), Some(-2000));
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
    card.colors = vec![CardColor::Black, CardColor::Yellow];
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}
