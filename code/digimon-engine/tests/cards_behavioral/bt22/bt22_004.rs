//! BT22-004 Wanyamon.

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef};

#[test]
fn bt22_004_inherited_source_placement_can_digivolve_into_cs_from_hand_cost_minus_1() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-004")
        .expect("BT22-004 YAML loads")
        .add_card(digimon("CARRIER", "Carrier", &[], 3))
        .add_card(digimon("PLACED-CS", "Placed CS", &["CS"], 3))
        .add_card(cs_lv4_evo("CS-EVO"))
        .add_card(make_test_card("EFFECT", "Effect Source"))
        .deck(0, &["PLACED-CS"])
        .hand(0, &["CS-EVO"])
        .memory(5)
        .start();

    let host = runner.place_stack(0, &["BT22-004", "CARRIER"]);
    place_deck_top_source_by_effect(&mut runner, host);

    let action = runner
        .pending_selection_view()
        .expect("CS digivolve choice should be pending")
        .valid_action_ids
        .iter()
        .copied()
        .find(|action| *action != PASS)
        .expect("non-pass hand choice");
    runner
        .execute_action(0, action)
        .expect("digivolve into CS card");

    assert_eq!(
        runner.game.players[0].battle_area[host.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "CS-EVO"
    );
    assert_eq!(
        runner.memory(),
        4,
        "cost 2 reduced by 1 should pay 1 memory"
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
    card.colors = vec![CardColor::Green];
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn cs_lv4_evo(id: &str) -> CardData {
    let mut card = digimon(id, id, &["CS"], 4);
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Green as u8,
        level: 3,
        memory_cost: 2,
    }];
    card
}
