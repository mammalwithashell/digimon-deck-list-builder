//! Phase 2 zone-manipulation integration tests.
//!
//! See docs/superpowers/plans/2026-04-19-rust-engine-phase-2-zone-manipulation.md.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, CostDelta};
use std::sync::Arc;

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

// ─── Script-driven test: EffectContext::play_from_hand_with_cost ─────────────

/// TEST-P2-001: on play, if hand slot 0 has a card, play it free via
/// EffectContext::play_from_hand_with_cost.
struct TestP2_001;
impl CardEffect for TestP2_001 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Play top of hand free")
            .process(|ctx| {
                let me = ctx.player;
                if ctx.hand(me).is_empty() {
                    return;
                }
                ctx.play_from_hand_with_cost(me, 0, CostDelta::Free);
            })
            .build()]
    }
}

#[test]
fn ctx_play_from_hand_free_plays_target() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("TEST-P2-001", "P2-001", 3))
        .add_card(plain_digimon("TARGET", "Target", 10))
        .hand(0, &["TEST-P2-001", "TARGET"])
        .memory(3)
        .start();

    r.register_effect("TEST-P2-001", Arc::new(TestP2_001));

    // Play TEST-P2-001 (hand slot 0). After OnPlay fires, it should have
    // played TARGET (now hand slot 0 since TEST-P2-001 was removed first)
    // for free.
    let res = r.play(0, 0);
    assert_eq!(res, Some(0));
    assert_eq!(r.battle_area_size(0), 2, "both cards entered battle area");
    assert_eq!(r.hand_size(0), 0, "hand emptied");
    // Memory: started 3, paid 3 for TEST-P2-001, then 0 for TARGET (free).
    assert_eq!(r.memory(), 0);
}

// ─── play_from_trash_with_cost ────────────────────────────────────────────────

#[test]
fn play_from_trash_free_moves_card_to_field() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BURIED", "Buried", 6))
        .memory(0)
        .start();

    // Seed trash directly using the same CardSource idiom as place_on_field.
    {
        let g = r.game_mut();
        let data_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "BURIED")
            .expect("BURIED not in card_data");
        let next_idx = g.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 0, next_idx);
        g.players[0].trash.push(card);
    }

    assert_eq!(r.trash_size(0), 1);
    let res = r
        .game_mut()
        .play_from_trash_with_cost(0, 0, CostDelta::Free);
    assert_eq!(res, Some(0));
    assert_eq!(r.trash_size(0), 0, "card left trash");
    assert_eq!(r.battle_area_size(0), 1, "card entered battle area");
}

// ─── add_to_hand_from_deck / add_to_hand_from_trash / shuffle_deck ────────────

#[test]
fn add_to_hand_from_deck_moves_specific_card() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("WANTED", "Wanted", 4))
        .add_card(plain_digimon("FILLER", "Filler", 4))
        .deck(0, &["FILLER", "WANTED", "FILLER"])
        .start();

    // Grab the CardHandle of the WANTED card (deck slot 1).
    let target_handle = r.game_mut().player(0).deck[1].handle();

    let ok = r.game_mut().add_to_hand_from_deck(0, target_handle);
    assert!(ok);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.deck_size(0), 2, "one card left deck");

    // Confirm the correct card moved.
    let moved_id = {
        let g = r.game_mut();
        g.player(0).hand[0].card_id(&g.card_data).to_string()
    };
    assert_eq!(moved_id, "WANTED");
}

#[test]
fn add_to_hand_from_trash_moves_card() {
    // Seed trash the same way Task 4's test does — mirror that idiom.
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("DEAD", "Dead", 5))
        .start();

    let handle = {
        let g = r.game_mut();
        let data_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "DEAD")
            .expect("DEAD not in card_data");
        let card_idx = g.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 0, card_idx);
        let h = card.handle();
        g.players[0].trash.push(card);
        h
    };

    let ok = r.game_mut().add_to_hand_from_trash(0, handle);
    assert!(ok);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.trash_size(0), 0);
}

#[test]
fn add_to_hand_missing_handle_returns_false() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("DEAD", "Dead", 5))
        .deck(0, &["DEAD"])
        .start();

    // CardHandle(u16::MAX) is guaranteed to never be assigned by the builder
    // (the builder starts card indices at 0 and counts up; u16::MAX is sentinel).
    let bogus_handle = CardHandle(u16::MAX);

    let ok = r.game_mut().add_to_hand_from_deck(0, bogus_handle);
    assert!(!ok);
    assert_eq!(r.hand_size(0), 0);
}

// ─── reveal_top_deck ──────────────────────────────────────────────────────────

#[test]
fn reveal_top_deck_populates_reveal_pool() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("A", "A", 1))
        .add_card(plain_digimon("B", "B", 1))
        .add_card(plain_digimon("C", "C", 1))
        .deck(0, &["A", "B", "C"])
        .start();

    let revealed = r.game_mut().reveal_top_deck(0, 2);
    assert_eq!(revealed.len(), 2);
    assert_eq!(r.deck_size(0), 1);
    assert_eq!(r.game_mut().revealed_cards.len(), 2);

    // Pop() returns the top of Vec, so reveal order is the reverse of
    // insertion order. deck was built [A, B, C] with C on top. First
    // reveal should be C, second B.
    let card_ids: Vec<String> = {
        let g = r.game_mut();
        g.revealed_cards
            .iter()
            .map(|c| c.card_id(&g.card_data).to_string())
            .collect()
    };
    assert_eq!(card_ids, vec!["C".to_string(), "B".to_string()], "revealed top-first");
}

#[test]
fn reveal_top_deck_handles_empty_deck() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("A", "A", 1))
        .deck(0, &["A"])
        .start();
    let revealed = r.game_mut().reveal_top_deck(0, 5);
    assert_eq!(revealed.len(), 1);
    assert_eq!(r.deck_size(0), 0);
}
