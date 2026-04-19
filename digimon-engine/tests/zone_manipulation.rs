//! Phase 2 zone-manipulation integration tests.
//!
//! See docs/superpowers/plans/2026-04-19-rust-engine-phase-2-zone-manipulation.md.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, CostDelta};

/// Helper: a Lv.3 Red Digimon with configurable play_cost and no effects.
fn plain_digimon(card_id: &str, name: &str, play_cost: u16) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
    }
}

#[test]
fn play_from_hand_free_ignores_printed_cost() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("COSTLY", "Costly", 10))
        .hand(0, &["COSTLY"])
        .memory(0)
        .start();

    assert_eq!(r.memory(), 0);
    assert_eq!(r.hand_size(0), 1);

    let result = r.game_mut().play_from_hand_with_cost(0, 0, CostDelta::Free);

    assert_eq!(result, Some(0), "play should succeed at free cost");
    assert_eq!(r.hand_size(0), 0, "card leaves hand");
    assert_eq!(r.battle_area_size(0), 1, "card enters battle area");
    assert_eq!(r.memory(), 0, "memory unchanged — CostDelta::Free pays 0");
}

#[test]
fn play_from_hand_reduce_subtracts_from_cost() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("C6", "SixCost", 6))
        .hand(0, &["C6"])
        .memory(5)
        .start();

    let before = r.memory();
    let res = r.game_mut().play_from_hand_with_cost(0, 0, CostDelta::Reduce(4));
    assert_eq!(res, Some(0));
    assert_eq!(r.memory(), before - 2, "6 - 4 = 2 memory paid");
}

#[test]
fn play_from_hand_reduce_clamps_at_zero() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("C3", "ThreeCost", 3))
        .hand(0, &["C3"])
        .memory(5)
        .start();

    let before = r.memory();
    let res = r.game_mut().play_from_hand_with_cost(0, 0, CostDelta::Reduce(10));
    assert_eq!(res, Some(0));
    assert_eq!(r.memory(), before, "reducing below 0 pays 0, not negative");
}

#[test]
fn play_from_hand_fixed_pays_exactly() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("C10", "TenCost", 10))
        .hand(0, &["C10"])
        .memory(0)
        .start();

    let before = r.memory();
    let res = r.game_mut().play_from_hand_with_cost(0, 0, CostDelta::Fixed(5));
    assert_eq!(res, Some(0), "fixed cost 5 at memory 0 is affordable (goes to -5)");
    assert_eq!(r.memory(), before - 5, "exactly 5 memory paid");
}
