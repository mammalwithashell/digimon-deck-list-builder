//! Tests for closure-valued cost reductions via BeforePayCost scan.
//!
//! These verify that cost_reduction_fn closures attached to battle-area
//! permanents correctly reduce play costs, stack with static reductions,
//! and are clamped appropriately.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{DebugRunner, make_test_card};
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
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// CardEffect that reduces cost by the size of player 0's trash (live state).
struct TrashSizeCostReduction;
impl CardEffect for TrashSizeCostReduction {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("reduce by trash size")
            .condition(|_| true)
            .cost_reduction_fn(|ctx| ctx.player(0).trash.len() as i32)
            .build()]
    }
}

/// CardEffect with a static cost_reduction of 1.
struct StaticReductionOne;
impl CardEffect for StaticReductionOne {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("static -1")
            .cost_reduction(1)
            .build()]
    }
}

/// CardEffect with a closure that returns 2.
struct ClosureReductionTwo;
impl CardEffect for ClosureReductionTwo {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("closure -2")
            .cost_reduction_fn(|_| 2)
            .build()]
    }
}

/// CardEffect whose closure returns a negative value (should clamp to 0).
struct NegativeReturnClosure;
impl CardEffect for NegativeReturnClosure {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("negative closure")
            .cost_reduction_fn(|_| -5)
            .build()]
    }
}

/// CardEffect whose closure returns an oversized reduction (99).
struct HugeCostReduction;
impl CardEffect for HugeCostReduction {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("huge reduction")
            .cost_reduction_fn(|_| 99)
            .build()]
    }
}

/// No-effect card for the permanent providing no reduction (control group).
struct NoEffect;
impl CardEffect for NoEffect {
    fn effects(&self, _card: CardHandle) -> Vec<Effect> {
        vec![]
    }
}

#[test]
fn closure_valued_cost_reduction_reads_live_state() {
    // Player 0 has 2 cards in trash — we set this up by pre-loading trash via
    // place_on_field then deleting them, but it's simpler to just push cards
    // into trash directly via game_mut().
    //
    // Setup: P0 has a reducer permanent on field; P0 has 2 trash cards.
    // Play a Digimon from hand with printed_cost = 7.
    // Memory starts at 10 (so play is affordable).
    // Expected: effective cost = 7 - 2 = 5; memory = 10 - 5 = 5.
    let target = plain_digimon("TARGET", 7);
    let reducer = make_test_card("REDUCER", "Reducer");
    let filler = make_test_card("FILLER", "Filler");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(reducer.clone())
        .add_card(filler.clone())
        .hand(0, &["TARGET"])
        .memory(10)
        .start();

    r.register_effect("REDUCER", Arc::new(TrashSizeCostReduction));
    r.register_effect("FILLER", Arc::new(NoEffect));

    // Place the reducer on the field.
    r.place_on_field(0, "REDUCER", Some(0));

    // Add 2 cards to player 0's trash directly.
    let game = r.game_mut();
    let filler_card1 = {
        let idx = game.card_data.iter().position(|c| c.card_id == "FILLER").unwrap();
        let ci = game.next_card_index();
        digimon_engine::card_source::CardSource::new(idx, 0, ci)
    };
    let filler_card2 = {
        let idx = game.card_data.iter().position(|c| c.card_id == "FILLER").unwrap();
        let ci = game.next_card_index();
        digimon_engine::card_source::CardSource::new(idx, 0, ci)
    };
    game.players[0].trash.push(filler_card1);
    game.players[0].trash.push(filler_card2);

    assert_eq!(r.game.players[0].trash.len(), 2);

    let memory_before = r.memory(); // 10
    r.play(0, 0);

    // effective cost = 7 - 2 = 5; memory = 10 - 5 = 5
    assert_eq!(
        r.memory(),
        memory_before - 5,
        "cost should be 7 - 2 (trash size) = 5"
    );
}

#[test]
fn static_cost_reduction_stacks_with_closure_reduction() {
    // Two separate battle-area permanents:
    //   A: static cost_reduction(1)
    //   B: closure cost_reduction_fn(|_| 2)
    // Play a Digimon with printed_cost = 5.
    // Expected: effective cost = 5 - 3 = 2.
    let target = plain_digimon("TARGET", 5);
    let perm_a = make_test_card("PERM-A", "PermA");
    let perm_b = make_test_card("PERM-B", "PermB");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(perm_a)
        .add_card(perm_b)
        .hand(0, &["TARGET"])
        .memory(10)
        .start();

    r.register_effect("PERM-A", Arc::new(StaticReductionOne));
    r.register_effect("PERM-B", Arc::new(ClosureReductionTwo));

    r.place_on_field(0, "PERM-A", Some(0));
    r.place_on_field(0, "PERM-B", Some(0));

    let memory_before = r.memory(); // 10
    r.play(0, 0);

    // effective cost = 5 - 1 - 2 = 2; memory = 10 - 2 = 8
    assert_eq!(
        r.memory(),
        memory_before - 2,
        "static 1 + closure 2 = total reduction 3; cost 5 - 3 = 2"
    );
}

#[test]
fn cost_reduction_fn_returning_negative_does_not_increase_cost() {
    // Closure returns -5. Per-effect clamp: max(0, -5) = 0.
    // No reduction applied.
    // Play a Digimon with printed_cost = 3.
    // Expected: effective cost = 3 (no reduction).
    let target = plain_digimon("TARGET", 3);
    let reducer = make_test_card("NEG-REDUCER", "NegReducer");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(reducer)
        .hand(0, &["TARGET"])
        .memory(10)
        .start();

    r.register_effect("NEG-REDUCER", Arc::new(NegativeReturnClosure));
    r.place_on_field(0, "NEG-REDUCER", Some(0));

    let memory_before = r.memory(); // 10
    r.play(0, 0);

    assert_eq!(
        r.memory(),
        memory_before - 3,
        "negative closure return should not increase cost; effective cost = 3"
    );
}

#[test]
fn effective_cost_floors_at_zero() {
    // Closure returns 99. Printed cost = 3.
    // Total reduction = 99, but effective_cost = max(0, 3 - 99) = 0.
    // Expected: effective cost = 0 (not negative); memory unchanged.
    let target = plain_digimon("TARGET", 3);
    let reducer = make_test_card("HUGE-REDUCER", "HugeReducer");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(reducer)
        .hand(0, &["TARGET"])
        .memory(10)
        .start();

    r.register_effect("HUGE-REDUCER", Arc::new(HugeCostReduction));
    r.place_on_field(0, "HUGE-REDUCER", Some(0));

    let memory_before = r.memory(); // 10
    r.play(0, 0);

    assert_eq!(
        r.memory(),
        memory_before, // cost = 0, no memory change
        "effective cost should floor at 0 when reduction exceeds printed cost"
    );
}
