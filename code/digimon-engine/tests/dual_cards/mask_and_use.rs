use std::sync::Arc;

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::card_data::{CardData, DualCardData, DualDigimonFace, DualOptionFace, EvoCost};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::OptionPlayResult;

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

fn base_lv5(card_id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.level = Some(5);
    card.dp = Some(7000);
    card.colors = vec![color];
    card
}

fn color_anchor(card_id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.level = Some(3);
    card.colors = vec![color];
    card
}

fn dual_card() -> CardData {
    let mut card = make_test_card("DUAL-MASK", "Dual Mask");
    card.card_kind = CardKind::Dual;
    card.level = Some(6);
    card.dp = Some(12000);
    card.play_cost = 5;
    card.colors = vec![CardColor::Red];
    card.traits = vec!["DigimonTrait".to_string()];
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 5,
        memory_cost: 3,
    }];
    card.dual = Some(DualCardData {
        digimon: DualDigimonFace {
            level: 6,
            dp: 12000,
            colors: vec![CardColor::Red],
            traits: vec!["DigimonTrait".to_string()],
            evo_costs: card.evo_costs.clone(),
            effect_text: "[When Digivolving] Draw 1.".to_string(),
            inherited_text: String::new(),
            keywords: Vec::new(),
        },
        option: DualOptionFace {
            use_cost: 5,
            colors: vec![CardColor::Purple],
            effect_text: "[Main] Gain 2 memory.".to_string(),
            security_text: String::new(),
            keywords: Vec::new(),
        },
    });
    card
}

struct GainTwo;
impl CardEffect for GainTwo {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Dual option main")
            .option_main()
            .process(|ctx| ctx.gain_memory(2))
            .build()]
    }
}

#[test]
fn dual_play_bit_uses_option_face_color_and_cost() {
    let mut r = DebugRunner::builder()
        .add_card(dual_card())
        .add_card(color_anchor("PURPLE-ANCHOR", CardColor::Purple))
        .hand(0, &["DUAL-MASK"])
        .memory(5)
        .start();
    r.register_effect("DUAL-MASK", Arc::new(GainTwo));
    r.place_on_field(0, "PURPLE-ANCHOR", Some(0));
    advance_to_main(&mut r);

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[0], 1.0, "DUAL play bit means use as Option");

    r.game.decode_action(0, 0);
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(r.trash_size(0), 1);
    assert_eq!(r.memory(), 2, "paid 5 from 5, then gained 2");
}

#[test]
fn dual_option_use_does_not_accept_digimon_face_color() {
    let mut r = DebugRunner::builder()
        .add_card(dual_card())
        .add_card(color_anchor("RED-ANCHOR", CardColor::Red))
        .hand(0, &["DUAL-MASK"])
        .memory(5)
        .start();
    r.register_effect("DUAL-MASK", Arc::new(GainTwo));
    r.place_on_field(0, "RED-ANCHOR", Some(0));
    advance_to_main(&mut r);

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[0], 0.0,
        "Digimon-face red must not satisfy purple Option face"
    );
    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid
    );
}

#[test]
fn dual_emits_digivolve_bit_using_digimon_face() {
    use digimon_engine::action::space::encode_digivolve;

    let mut r = DebugRunner::builder()
        .add_card(dual_card())
        .add_card(base_lv5("BASE-RED", CardColor::Red))
        .hand(0, &["DUAL-MASK"])
        .memory(5)
        .start();
    r.place_on_field(0, "BASE-RED", Some(0));
    advance_to_main(&mut r);

    let mask = build_action_mask(&r.game, 0);
    let bit = encode_digivolve(0, 0) as usize;
    assert_eq!(mask[bit], 1.0, "DUAL can digivolve as a Digimon");
}
