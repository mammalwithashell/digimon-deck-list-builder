use digimon_engine::card_data::{CardData, DualCardData, DualDigimonFace, DualOptionFace, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn base_lv5(card_id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.level = Some(5);
    card.dp = Some(7000);
    card.colors = vec![color];
    card
}

fn dual_card() -> CardData {
    let mut card = make_test_card("DUAL-HELPER", "Dual Helper");
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
            effect_text: "Use Requirement: Test trait\n[Main] Delete 1 Digimon.".to_string(),
            security_text: String::new(),
            keywords: Vec::new(),
        },
    });
    card
}

#[test]
fn dual_card_helpers_expose_separate_faces() {
    let r = DebugRunner::builder()
        .add_card(base_lv5("BASE", CardColor::Red))
        .add_card(dual_card())
        .hand(0, &["DUAL-HELPER"])
        .start();

    let card = &r.game.player(0).hand[0];
    assert_eq!(card.card_kind(&r.game.card_data), CardKind::Dual);
    assert_eq!(card.digimon_level(&r.game.card_data), Some(6));
    assert_eq!(card.digimon_dp(&r.game.card_data), Some(12000));
    assert_eq!(card.option_use_cost(&r.game.card_data), Some(5));
    assert_eq!(card.digimon_colors(&r.game.card_data), &[CardColor::Red]);
    assert_eq!(card.option_colors(&r.game.card_data), &[CardColor::Purple]);
    assert!(card.is_digimon_card_for_search(&r.game.card_data));
    assert!(card.is_option_card_for_search(&r.game.card_data));
    assert!(card
        .text_for_search_all_faces(&r.game.card_data)
        .contains("Delete 1 Digimon"));
}

#[test]
fn dual_text_search_sees_both_faces() {
    let r = DebugRunner::builder()
        .add_card(dual_card())
        .hand(0, &["DUAL-HELPER"])
        .start();
    let card = &r.game.player(0).hand[0];
    let text = card.text_for_search_all_faces(&r.game.card_data);
    assert!(text.contains("When Digivolving"));
    assert!(text.contains("Delete 1 Digimon"));
}

#[test]
fn dual_on_field_is_digimon_not_option() {
    let mut r = DebugRunner::builder()
        .add_card(base_lv5("BASE", CardColor::Red))
        .add_card(dual_card())
        .start();
    let h = r.place_on_field(0, "DUAL-HELPER", Some(0));
    let perm = &r.game.player(0).battle_area[h.index as usize];
    assert!(perm.is_digimon(&r.game.card_data));
    assert!(!perm.is_option(&r.game.card_data));
}
