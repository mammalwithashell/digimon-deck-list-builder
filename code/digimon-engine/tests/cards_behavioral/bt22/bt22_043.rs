//! BT22-043 Terriermon.

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef};

#[test]
fn bt22_043_source_placement_can_play_cs_tamer_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-043")
        .expect("BT22-043 YAML loads")
        .add_card(digimon("BASE", "Base", &[], 3))
        .add_card(digimon("PLACED-CS", "Placed CS", &["CS"], 3))
        .add_card(make_test_card("EFFECT", "Effect Source"))
        .add_card(tamer("CS-TAMER", "CS Tamer"))
        .deck(0, &["PLACED-CS"])
        .hand(0, &["CS-TAMER"])
        .memory(0)
        .start();

    let host = runner.place_stack(0, &["BASE", "BT22-043"]);
    place_deck_top_source_by_effect(&mut runner, host);

    let action = runner
        .pending_selection_view()
        .expect("CS Tamer choice should be pending")
        .valid_action_ids
        .iter()
        .copied()
        .find(|action| *action != PASS)
        .expect("non-pass hand choice");
    runner.execute_action(0, action).expect("play CS Tamer");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "CS-TAMER"),
        "Terriermon should free-play the selected CS Tamer"
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

fn tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::White];
    card.traits = vec!["CS".to_string()];
    card
}
