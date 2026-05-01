//! Hygiene tests for the BeforePayCost battle-area scan.
//!
//! Verifies the critical invariant (Python Issue 24 avoidance): only
//! effects from permanents currently in battle_area with timing ==
//! BeforePayCost and condition passing are included in the cost reduction.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind};
use std::sync::Arc;

fn plain_digimon(card_id: &str, play_cost: u16) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(2000),
        play_cost,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// A permanent with BeforePayCost effect reducing cost by 3.
struct BeforePayCostReducer3;
impl CardEffect for BeforePayCostReducer3 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("reduce by 3")
            .cost_reduction(3)
            .build()]
    }
}

/// A permanent with BeforePayCost effect but condition always false.
struct AlwaysFalseConditionReducer;
impl CardEffect for AlwaysFalseConditionReducer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("always-false condition reducer")
            .condition(|_| false)
            .cost_reduction(3)
            .build()]
    }
}

/// A permanent with OnPlay timing but cost_reduction = 99.
/// This is the Python Issue 24 regression test: OnPlay timing must NOT
/// contribute to play-cost reduction.
struct OnPlayWithCostReduction;
impl CardEffect for OnPlayWithCostReduction {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("OnPlay with cost_reduction=99 — should NOT reduce play cost")
            .cost_reduction(99)
            .build()]
    }
}

/// A permanent with BeforePayCost effect reducing cost by 1.
struct BeforePayCostReducer1;
impl CardEffect for BeforePayCostReducer1 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("reduce by 1")
            .cost_reduction(1)
            .build()]
    }
}

#[test]
fn scan_excludes_trashed_sources() {
    // Permanent A has BeforePayCost effect with cost_reduction(3).
    // Trash A (delete it so it leaves the battle_area).
    // Play another Digimon with printed_cost = 5.
    // Expected: no reduction applied; effective cost = 5.
    let target = plain_digimon("TARGET", 5);
    let perm_a = make_test_card("PERM-A", "PermA");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(perm_a)
        .hand(0, &["TARGET"])
        .memory(10)
        .start();

    r.register_effect("PERM-A", Arc::new(BeforePayCostReducer3));

    // Place A on field then delete it (simulate "trashed" state).
    let handle = r.place_on_field(0, "PERM-A", Some(0));
    r.game_mut().players[handle.player as usize].delete_permanent(handle.index as usize);

    // PERM-A is now in trash; battle_area is empty.
    assert_eq!(r.game.players[0].battle_area.len(), 0);

    let memory_before = r.memory(); // 10
    r.play(0, 0);

    assert_eq!(
        r.memory(),
        memory_before - 5,
        "trashed source must not contribute cost reduction; effective cost = 5"
    );
}

#[test]
fn scan_excludes_failed_condition() {
    // Permanent A has BeforePayCost effect with condition(|_| false), cost_reduction(3).
    // Play a Digimon with printed_cost = 5.
    // Expected: no reduction; effective cost = 5.
    let target = plain_digimon("TARGET", 5);
    let perm_a = make_test_card("PERM-A", "PermA");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(perm_a)
        .hand(0, &["TARGET"])
        .memory(10)
        .start();

    r.register_effect("PERM-A", Arc::new(AlwaysFalseConditionReducer));
    r.place_on_field(0, "PERM-A", Some(0));

    let memory_before = r.memory(); // 10
    r.play(0, 0);

    assert_eq!(
        r.memory(),
        memory_before - 5,
        "failed condition must not contribute cost reduction; effective cost = 5"
    );
}

#[test]
fn scan_excludes_out_of_scope_timing() {
    // Permanent A has an OnPlay effect with static cost_reduction = 99.
    // This is the Python Issue 24 regression test: OnPlay timing should
    // NOT contribute to play-cost reduction. Only BeforePayCost timing does.
    // Play a Digimon with printed_cost = 5.
    // Expected: effective cost = 5 (99-reduction NOT applied).
    let target = plain_digimon("TARGET", 5);
    let perm_a = make_test_card("PERM-A", "PermA");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(perm_a)
        .hand(0, &["TARGET"])
        .memory(10)
        .start();

    r.register_effect("PERM-A", Arc::new(OnPlayWithCostReduction));
    r.place_on_field(0, "PERM-A", Some(0));

    let memory_before = r.memory(); // 10
    r.play(0, 0);

    assert_eq!(
        r.memory(),
        memory_before - 5,
        "OnPlay timing must not contribute to cost reduction (Python Issue 24 regression); effective cost = 5"
    );
}

#[test]
fn scan_applies_to_both_players_battle_areas() {
    // Both players have a permanent with BeforePayCost effect reducing cost by 1.
    // Player 0 plays a Digimon with printed_cost = 4.
    // Expected: effective cost = 4 - 2 = 2.
    //
    // This test validates scan symmetry (both players' battle areas are inspected).
    // Real card text always uses the condition closure to scope to the controller's
    // own Digimon; this test explicitly does NOT use such scoping to exercise the
    // infrastructure.
    let target = plain_digimon("TARGET", 4);
    let perm_p0 = make_test_card("PERM-P0", "PermP0");
    let perm_p1 = make_test_card("PERM-P1", "PermP1");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(perm_p0)
        .add_card(perm_p1)
        .hand(0, &["TARGET"])
        .memory(10)
        .start();

    r.register_effect("PERM-P0", Arc::new(BeforePayCostReducer1));
    r.register_effect("PERM-P1", Arc::new(BeforePayCostReducer1));

    r.place_on_field(0, "PERM-P0", Some(0));
    r.place_on_field(1, "PERM-P1", Some(0));

    let memory_before = r.memory(); // 10
    r.play(0, 0);

    assert_eq!(
        r.memory(),
        memory_before - 2,
        "both players' battle area BeforePayCost reducers should stack; effective cost = 4 - 2 = 2"
    );
}
