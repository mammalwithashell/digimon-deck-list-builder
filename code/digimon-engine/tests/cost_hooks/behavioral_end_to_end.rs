//! End-to-end behavioral test: static + closure cost reduction + pay_cost_fn stacking.
//!
//! Scenario (synthesized from Rocks archetype patterns):
//!
//! - Permanent A: unconditional static cost_reduction(1) — "your Digimon cost 1 less to play."
//! - Permanent B: condition-gated BeforePayCost with pay_cost_fn + cost_reduction_fn:
//!     "When pre-pay memory >= 5, trash 2 cards from top of your deck to reduce cost by 2."
//!
//! Two sub-tests:
//!   1. `static_plus_closure_reduction_and_pay_cost_stack_when_condition_passes`
//!      — memory = 5 (passes B's condition) → both effects fire, stacked reduction = 3,
//!        effective cost = 6 - 3 = 3, memory ends at 2, 2 cards trashed.
//!   2. `closure_reduction_and_pay_cost_skipped_when_condition_fails`
//!      — memory = 4 (fails B's condition) → only A fires, reduction = 1,
//!        effective cost = 6 - 1 = 5, memory ends at -1, deck unchanged.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind};
use std::sync::Arc;

// ─── Card data helpers ────────────────────────────────────────────────────────

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
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn filler_card(card_id: &str) -> CardData {
    plain_digimon(card_id, 3)
}

// ─── CardEffect structs ───────────────────────────────────────────────────────

/// Permanent A: unconditional static cost_reduction(1).
/// Models "your Digimon cost 1 less to play."
struct StaticReductionOne;
impl CardEffect for StaticReductionOne {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("static -1 unconditional")
            .cost_reduction(1)
            .build()]
    }
}

/// Permanent B: condition-gated BeforePayCost.
/// Condition: pre-pay memory >= 5.
/// pay_cost_fn: trash 2 cards from top of player 0's deck; always returns true
///   (non-optional — if the condition passes, the trash cost is always paid).
/// cost_reduction_fn: returns 2.
struct MemoryGatedTrashTwoForDiscountTwo;
impl CardEffect for MemoryGatedTrashTwoForDiscountTwo {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("memory>=5 → trash 2 → reduce 2")
            .condition(|ctx| ctx.memory() >= 5)
            .pay_cost_fn(|ctx| {
                ctx.trash_from_top(0, 2);
                true
            })
            .cost_reduction_fn(|_| 2)
            .build()]
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn static_plus_closure_reduction_and_pay_cost_stack_when_condition_passes() {
    // Setup:
    //   - Permanent A (PERM-A) on P0's battle area: static cost_reduction(1)
    //   - Permanent B (PERM-B) on P0's battle area: memory>=5 → trash 2 → reduce 2
    //   - P0's deck has 5 known filler cards (so trash cost can be paid)
    //   - P0's hand has TARGET with printed_cost = 6
    //   - Memory starts at 5 (passes B's condition: memory >= 5)
    //
    // Expected:
    //   - B's condition passes → pay_cost_fn fires → 2 cards trashed from deck
    //   - A's static reduction = 1; B's closure reduction = 2; total = 3
    //   - effective cost = 6 - 1 - 2 = 3
    //   - memory = 5 - 3 = 2
    //   - Digimon entered battle area

    let target = plain_digimon("TARGET-ETE", 6);
    let perm_a = make_test_card("PERM-A-ETE", "PermAEte");
    let perm_b = make_test_card("PERM-B-ETE", "PermBEte");
    let deck_card = filler_card("DECK-ETE");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(perm_a)
        .add_card(perm_b)
        .add_card(deck_card)
        .deck(
            0,
            &["DECK-ETE", "DECK-ETE", "DECK-ETE", "DECK-ETE", "DECK-ETE"],
        )
        .hand(0, &["TARGET-ETE"])
        .memory(5)
        .start();

    r.register_effect("PERM-A-ETE", Arc::new(StaticReductionOne));
    r.register_effect("PERM-B-ETE", Arc::new(MemoryGatedTrashTwoForDiscountTwo));

    r.place_on_field(0, "PERM-A-ETE", Some(0));
    r.place_on_field(0, "PERM-B-ETE", Some(0));

    let deck_before = r.game.players[0].deck.len(); // 5
    let trash_before = r.game.players[0].trash.len(); // 0
    let memory_before = r.memory(); // 5

    assert!(
        r.play(0, 0).is_none(),
        "paid reducer should park until its cost is explicitly accepted"
    );
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_before,
        "pay_cost_fn must not run before acceptance"
    );
    r.execute_branch(0).expect("accept paid reducer");

    // B's pay_cost_fn fired: 2 cards moved deck → trash
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_before - 2,
        "condition passes (memory=5 >= 5): pay_cost_fn should trash 2 from top of deck"
    );
    assert_eq!(
        r.game.players[0].trash.len(),
        trash_before + 2,
        "trashed cards should appear in P0's trash"
    );

    // Stacked reductions: A(-1) + B(-2) = 3; effective cost = 6 - 3 = 3
    assert_eq!(
        r.memory(),
        memory_before - 3,
        "static(-1) + closure(-2) = total reduction 3; effective cost = 6 - 3 = 3; \
         memory should be {} - 3 = {}",
        memory_before,
        memory_before - 3,
    );

    // Digimon entered battle area (PERM-A + PERM-B already at indices 0/1)
    assert!(
        r.game.players[0].battle_area.len() >= 3,
        "TARGET-ETE should be in battle area after play"
    );
}

#[test]
fn closure_reduction_and_pay_cost_skipped_when_condition_fails() {
    // Same setup as above BUT memory starts at 4 (fails B's condition: memory < 5).
    //
    // Expected:
    //   - B's condition fails → pay_cost_fn does NOT fire → deck unchanged
    //   - Only A's static reduction = 1 applies; B's closure is skipped entirely
    //   - effective cost = 6 - 1 = 5
    //   - memory = 4 - 5 = -1
    //   - Digimon entered battle area

    let target = plain_digimon("TARGET-CF2", 6);
    let perm_a = make_test_card("PERM-A-CF2", "PermACf2");
    let perm_b = make_test_card("PERM-B-CF2", "PermBCf2");
    let deck_card = filler_card("DECK-CF2");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(perm_a)
        .add_card(perm_b)
        .add_card(deck_card)
        .deck(
            0,
            &["DECK-CF2", "DECK-CF2", "DECK-CF2", "DECK-CF2", "DECK-CF2"],
        )
        .hand(0, &["TARGET-CF2"])
        .memory(4) // fails B's condition (memory < 5)
        .start();

    r.register_effect("PERM-A-CF2", Arc::new(StaticReductionOne));
    r.register_effect("PERM-B-CF2", Arc::new(MemoryGatedTrashTwoForDiscountTwo));

    r.place_on_field(0, "PERM-A-CF2", Some(0));
    r.place_on_field(0, "PERM-B-CF2", Some(0));

    let deck_before = r.game.players[0].deck.len(); // 5
    let trash_before = r.game.players[0].trash.len(); // 0
    let memory_before = r.memory(); // 4

    r.play(0, 0);

    // B's condition failed → pay_cost_fn did NOT fire → deck unchanged
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_before,
        "condition fails (memory=4 < 5): pay_cost_fn must not fire; deck should be unchanged"
    );
    assert_eq!(
        r.game.players[0].trash.len(),
        trash_before,
        "condition fails: trash should be unchanged"
    );

    // Only A's static reduction = 1; B's closure skipped → effective cost = 6 - 1 = 5
    assert_eq!(
        r.memory(),
        memory_before - 5,
        "only static(-1) applies when condition fails; effective cost = 6 - 1 = 5; \
         memory should be {} - 5 = {}",
        memory_before,
        memory_before - 5,
    );

    // Digimon still entered battle area (play succeeds regardless)
    assert!(
        r.game.players[0].battle_area.len() >= 3,
        "TARGET-CF2 should be in battle area after play"
    );
}
