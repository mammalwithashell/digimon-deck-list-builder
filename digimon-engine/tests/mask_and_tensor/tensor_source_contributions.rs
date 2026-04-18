//! Integration tests for §3.1 / §3.2: per-source DP contributions and OPT
//! state inside the observation tensor produced by `build_tensor`.
//!
//! These pin the float values at specific offsets so a regression in
//! either `write_slot` or the underlying Game helpers shows up
//! immediately. Unlike `tensor_helpers.rs` (which unit-tests the
//! standalone helpers), this file goes through `build_tensor` end-to-end.

use std::sync::Arc;

use digimon_engine::card_registry::CardRegistry;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::cards::CardEffectRegistry;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::tensor::{
    build_tensor, OFF_MY_BATTLE, OFF_OPP_BATTLE, SLOT_SIZE, SOURCE_ENTRY_SIZE,
};

// ─── Test cards for this suite ───────────────────────────────────────

/// TEST-007 — non-inherited +3000 DP, always on.
pub struct Test007;
impl CardEffect for Test007 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::declarative(card)
            .name("+3000 DP (non-inherited)")
            .dp_modifier(3000)
            .build()]
    }
}

/// TEST-007-INH — inherited −3000 DP, always on.
pub struct Test007Inh;
impl CardEffect for Test007Inh {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::inherited(card)
            .name("-3000 DP (inherited)")
            .dp_modifier(-3000)
            .build()]
    }
}

/// TEST-007-COND — +3000 DP only when memory >= 0.
pub struct Test007Cond;
impl CardEffect for Test007Cond {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::declarative(card)
            .name("+3000 DP when memory >= 0")
            .dp_modifier(3000)
            .condition(|ctx| ctx.memory() >= 0)
            .build()]
    }
}

/// TEST-008 — once-per-turn effect.
pub struct Test008;
impl CardEffect for Test008 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Once-per-turn marker")
            .once_per_turn()
            .process(|_| {})
            .build()]
    }
}

fn test_registry() -> CardEffectRegistry {
    let mut r = CardEffectRegistry::default();
    r.insert("TEST-007", Arc::new(Test007));
    r.insert("TEST-007-INH", Arc::new(Test007Inh));
    r.insert("TEST-007-COND", Arc::new(Test007Cond));
    r.insert("TEST-008", Arc::new(Test008));
    r
}

fn fresh_runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(make_test_card("TEST-007", "Test007"))
        .add_card(make_test_card("TEST-007-INH", "Test007Inh"))
        .add_card(make_test_card("TEST-007-COND", "Test007Cond"))
        .add_card(make_test_card("TEST-008", "Test008"))
        .with_registry(test_registry())
        .start()
}

fn registry(r: &DebugRunner) -> CardRegistry {
    // Build a CardRegistry from the game's card_data store. The same factory
    // runs inside build_tensor; we just need a shared instance here.
    let mut map = std::collections::HashMap::new();
    for c in &r.game.card_data {
        map.insert(c.card_id.clone(), c.clone());
    }
    CardRegistry::from_cards(&map)
}

/// Compute tensor index for slot field at (player_perspective, field_index,
/// offset_within_slot).
fn slot_offset(perspective_is_me: bool, field_index: usize, off: usize) -> usize {
    let battle_start = if perspective_is_me {
        OFF_MY_BATTLE
    } else {
        OFF_OPP_BATTLE
    };
    battle_start + field_index * SLOT_SIZE + off
}

/// Compute tensor index for the j-th source sub-slot of a permanent.
fn source_offset(
    perspective_is_me: bool,
    field_index: usize,
    source_index: usize,
    within: usize,
) -> usize {
    slot_offset(
        perspective_is_me,
        field_index,
        7 + source_index * SOURCE_ENTRY_SIZE + within,
    )
}

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn non_inherited_dp_buff_on_top_card_normalizes_to_tensor() {
    let mut r = fresh_runner();
    r.place_on_field(0, "TEST-007", Some(0));
    let reg = registry(&r);
    let t = build_tensor(&r.game, 0, &reg);

    // Source 0 is the only source. Its dp_contribution should be +3000
    // (non-inherited buff on the top card) → 0.1 normalized.
    let off = source_offset(true, 0, 0, 2);
    assert!(
        (t[off] - 0.1).abs() < 1e-6,
        "expected 0.1 at {}, got {}",
        off,
        t[off]
    );
}

#[test]
fn inherited_dp_buff_on_top_is_zero() {
    // Inherited effect on the top card is NOT active (only counts when
    // under). Expect dp_contribution = 0.0.
    let mut r = fresh_runner();
    r.place_on_field(0, "TEST-007-INH", Some(0));
    let reg = registry(&r);
    let t = build_tensor(&r.game, 0, &reg);

    let off = source_offset(true, 0, 0, 2);
    assert!((t[off] - 0.0).abs() < 1e-6, "expected 0.0, got {}", t[off]);
}

#[test]
fn inherited_dp_buff_contributes_once_digivolved_over() {
    // Place TEST-007-INH, then digivolve TEST-007 onto it. Source 0 (under,
    // TEST-007-INH) contributes -3000 → -0.1; source 1 (top, TEST-007)
    // contributes +3000 → +0.1.
    let mut r = fresh_runner();
    r.place_on_field(0, "TEST-007-INH", Some(0));
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TEST-007")
        .unwrap();
    let next = r.game.next_card_index();
    let top = CardSource::new(data_idx, 0, next);
    r.game.digivolve_onto(0, 0, top);

    let reg = registry(&r);
    let t = build_tensor(&r.game, 0, &reg);

    let under = source_offset(true, 0, 0, 2);
    let top_off = source_offset(true, 0, 1, 2);
    assert!(
        (t[under] - -0.1).abs() < 1e-6,
        "under slot expected -0.1, got {}",
        t[under]
    );
    assert!(
        (t[top_off] - 0.1).abs() < 1e-6,
        "top slot expected 0.1, got {}",
        t[top_off]
    );
}

#[test]
fn conditional_dp_buff_suppressed_when_memory_negative() {
    let mut r = fresh_runner();
    r.place_on_field(0, "TEST-007-COND", Some(0));
    let reg = registry(&r);

    // Memory = 0 → condition true → +3000 contributes.
    r.game.set_memory(0);
    let t = build_tensor(&r.game, 0, &reg);
    let off = source_offset(true, 0, 0, 2);
    assert!(
        (t[off] - 0.1).abs() < 1e-6,
        "memory=0 expected 0.1, got {}",
        t[off]
    );

    // Memory = -1 → condition false → 0.
    r.game.set_memory(-1);
    let t = build_tensor(&r.game, 0, &reg);
    assert!(
        (t[off] - 0.0).abs() < 1e-6,
        "memory=-1 expected 0.0, got {}",
        t[off]
    );
}

#[test]
fn opt_total_and_opt_used_written_to_tensor() {
    let mut r = fresh_runner();
    let h = r.place_on_field(0, "TEST-008", Some(0));
    let reg = registry(&r);

    // opt_total = 1, opt_used = 0 initially.
    let t = build_tensor(&r.game, 0, &reg);
    let opt_total_off = slot_offset(true, 0, 3);
    let opt_used_off = slot_offset(true, 0, 4);
    assert!((t[opt_total_off] - 1.0).abs() < 1e-6);
    assert!((t[opt_used_off] - 0.0).abs() < 1e-6);

    // Record an activation for slot 0 of source 0.
    let source_handle = r.game.players[0].battle_area[h.index as usize]
        .card_sources[0]
        .handle();
    r.game.players[0].battle_area[h.index as usize]
        .record_activation(source_handle, 0);

    let t = build_tensor(&r.game, 0, &reg);
    assert!((t[opt_total_off] - 1.0).abs() < 1e-6);
    assert!(
        (t[opt_used_off] - 1.0).abs() < 1e-6,
        "after activation expected opt_used=1.0, got {}",
        t[opt_used_off]
    );
}

#[test]
fn source_opt_state_reflects_availability_fraction() {
    let mut r = fresh_runner();
    let h = r.place_on_field(0, "TEST-008", Some(0));
    let reg = registry(&r);

    let opt_state_off = source_offset(true, 0, 0, 1);

    // 1 OPT effect, unused → state = 1.0.
    let t = build_tensor(&r.game, 0, &reg);
    assert!(
        (t[opt_state_off] - 1.0).abs() < 1e-6,
        "initial opt_state expected 1.0, got {}",
        t[opt_state_off]
    );

    // Mark it used → state = 0.0.
    let source_handle = r.game.players[0].battle_area[h.index as usize]
        .card_sources[0]
        .handle();
    r.game.players[0].battle_area[h.index as usize]
        .record_activation(source_handle, 0);
    let t = build_tensor(&r.game, 0, &reg);
    assert!(
        (t[opt_state_off] - 0.0).abs() < 1e-6,
        "after use opt_state expected 0.0, got {}",
        t[opt_state_off]
    );
}

#[test]
fn opp_perspective_swaps_battle_offsets_but_preserves_values() {
    // Player 0 has TEST-007 on field. When we build the tensor from P1's
    // perspective, that permanent should show up in the OPP battle area
    // (their perspective) but NOT the MY battle area of P1.
    let mut r = fresh_runner();
    r.place_on_field(0, "TEST-007", Some(0));
    let reg = registry(&r);

    // Build from P1's perspective.
    let t = build_tensor(&r.game, 1, &reg);

    // P1's MY battle area should be empty (top card id = 0).
    let my_slot0_top = slot_offset(true, 0, 0);
    assert_eq!(t[my_slot0_top], 0.0);

    // P1's OPP battle area (= P0's field) should have the permanent with
    // dp_contribution +0.1 at source 0.
    let opp_dp = source_offset(false, 0, 0, 2);
    assert!(
        (t[opp_dp] - 0.1).abs() < 1e-6,
        "opp perspective dp expected 0.1, got {}",
        t[opp_dp]
    );
}
