//! Tests for `pay_cost_fn` wiring at the BeforePayCost dispatch site.
//!
//! Verifies that a `pay_cost_fn` closure on a BeforePayCost effect acts as the
//! "cost of the discount" — e.g., "trash 2 cards from top of your deck to
//! reduce play cost by 2." If the closure returns false (e.g. deck too small),
//! the reduction is skipped but the play itself still proceeds at full cost.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind};
use std::sync::Arc;

// ─── Test card data helpers ───────────────────────────────────────────────────

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

fn filler_digimon(card_id: &str) -> CardData {
    plain_digimon(card_id, 3)
}

// ─── CardEffect structs ───────────────────────────────────────────────────────

/// BeforePayCost: trash 2 from top of P0's deck and reduce play cost by 2.
/// pay_cost_fn returns true (always, regardless of deck size, for the
/// "sufficient deck" tests). Distinct from the "insufficient" variant below.
struct TrashTwoForDiscountTwo;
impl CardEffect for TrashTwoForDiscountTwo {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("trash 2 → reduce 2")
            .pay_cost_fn(|ctx| {
                // Trash 2 from top of player 0's deck (always succeeds here).
                ctx.trash_from_top(0, 2);
                true
            })
            .cost_reduction_fn(|_| 2)
            .build()]
    }
}

/// BeforePayCost: trash 2 from top of P0's deck — but only if P0 has >= 2
/// cards. Returns false if not enough cards; does NOT trash anything.
struct TrashTwoIfEnoughElseFalse;
impl CardEffect for TrashTwoIfEnoughElseFalse {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("trash 2 if possible → reduce 2")
            .pay_cost_fn(|ctx| {
                if ctx.player(0).deck.len() < 2 {
                    return false; // cannot pay; skip reduction
                }
                ctx.trash_from_top(0, 2);
                true
            })
            .cost_reduction_fn(|_| 2)
            .build()]
    }
}

/// BeforePayCost: condition always false; pay_cost_fn panics if called.
/// Validates that a failing condition gates pay_cost_fn.
struct ConditionFalsePayCostPanics;
impl CardEffect for ConditionFalsePayCostPanics {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("condition false → pay_cost never fires")
            .condition(|_| false)
            .pay_cost_fn(|_ctx| panic!("pay_cost_fn must not be called when condition is false"))
            .cost_reduction_fn(|_| 2)
            .build()]
    }
}

/// OnPlay effect (NOT BeforePayCost) with pay_cost_fn that panics.
/// Validates scan-scope hygiene: the BeforePayCost scan must NOT touch
/// non-BeforePayCost timing effects.
struct OnPlayPayCostPanics;
impl CardEffect for OnPlayPayCostPanics {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("OnPlay pay_cost panic — must not fire in scan")
            .pay_cost_fn(|_ctx| {
                panic!("OnPlay pay_cost_fn must not fire during BeforePayCost scan")
            })
            .cost_reduction_fn(|_| 2)
            .build()]
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn pay_cost_fn_gates_reduction_in_play_path() {
    // Battle-area permanent has a BeforePayCost effect:
    //   .pay_cost_fn(|ctx| { trash 2 from top; true })
    //   .cost_reduction_fn(|_| 2)
    //
    // Player's deck has 5 cards. Play a Digimon with printed_cost = 6.
    // Memory starts at 10. Expected:
    //   - 2 cards moved from deck to trash (pay_cost_fn fired)
    //   - effective cost = 6 - 2 = 4
    //   - memory = 10 - 4 = 6
    let target = plain_digimon("TARGET-PC", 6);
    let reducer = make_test_card("REDUCER-PC", "ReducerPc");
    let deck_filler = filler_digimon("DECK-CARD");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(reducer.clone())
        .add_card(deck_filler)
        // Give P0 5 deck cards — enough to pay the trash cost.
        .deck(
            0,
            &[
                "DECK-CARD",
                "DECK-CARD",
                "DECK-CARD",
                "DECK-CARD",
                "DECK-CARD",
            ],
        )
        .hand(0, &["TARGET-PC"])
        .memory(10)
        .start();

    r.register_effect("REDUCER-PC", Arc::new(TrashTwoForDiscountTwo));
    r.place_on_field(0, "REDUCER-PC", Some(0));

    let deck_before = r.game.players[0].deck.len();
    let trash_before = r.game.players[0].trash.len();
    let memory_before = r.memory();

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

    // pay_cost_fn fired: 2 cards moved deck → trash
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_before - 2,
        "pay_cost_fn should have trashed 2 cards from top of deck"
    );
    assert_eq!(
        r.game.players[0].trash.len(),
        trash_before + 2,
        "trashed cards should appear in P0's trash"
    );
    // effective cost = 6 - 2 = 4
    assert_eq!(
        r.memory(),
        memory_before - 4,
        "effective cost should be 6 - 2 = 4; memory should move from {} to {}",
        memory_before,
        memory_before - 4,
    );
}

#[test]
fn pay_cost_fn_returning_false_skips_reduction_but_play_proceeds() {
    // Same setup but with only 1 card in the player's deck.
    // pay_cost_fn checks deck.len() >= 2; returns false (can't pay).
    //   - 0 cards trashed (closure returned false)
    //   - effective cost = 6 (NO reduction applied)
    //   - memory = 10 - 6 = 4
    //   - play STILL SUCCEEDED (card is in battle area)
    let target = plain_digimon("TARGET-PF", 6);
    let reducer = make_test_card("REDUCER-PF", "ReducerPf");
    let deck_filler = filler_digimon("DECK-CARD-PF");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(reducer)
        .add_card(deck_filler)
        // Only 1 card in deck — not enough to pay the trash cost.
        .deck(0, &["DECK-CARD-PF"])
        .hand(0, &["TARGET-PF"])
        .memory(10)
        .start();

    r.register_effect("REDUCER-PF", Arc::new(TrashTwoIfEnoughElseFalse));
    r.place_on_field(0, "REDUCER-PF", Some(0));

    let deck_before = r.game.players[0].deck.len(); // 1
    let trash_before = r.game.players[0].trash.len();
    let memory_before = r.memory(); // 10

    assert!(
        r.play(0, 0).is_none(),
        "paid reducer should park even when its pay_cost_fn may fail"
    );
    r.execute_branch(0).expect("accept paid reducer");

    // 0 cards trashed — pay_cost_fn returned false, no mutation
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_before,
        "pay_cost_fn returned false; no cards should be trashed"
    );
    assert_eq!(
        r.game.players[0].trash.len(),
        trash_before,
        "trash should be unchanged when pay_cost_fn returns false"
    );
    // effective cost = 6 (no reduction applied)
    assert_eq!(
        r.memory(),
        memory_before - 6,
        "no reduction should apply when pay_cost_fn returns false; effective cost = 6"
    );
    // Play must still have succeeded
    assert_eq!(
        r.game.players[0].battle_area.len(),
        2, // REDUCER-PF (field[0]) + TARGET-PF (field[1])
        "play should still succeed when pay_cost_fn returns false"
    );
}

#[test]
fn pay_cost_fn_at_before_pay_does_not_fire_when_condition_fails() {
    // BeforePayCost effect with .condition(|_| false).pay_cost_fn(|_| panic!())
    // Play a card. Assert no panic (condition gated pay_cost_fn).
    let target = plain_digimon("TARGET-CF", 4);
    let reducer = make_test_card("REDUCER-CF", "ReducerCf");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(reducer)
        .hand(0, &["TARGET-CF"])
        .memory(10)
        .start();

    r.register_effect("REDUCER-CF", Arc::new(ConditionFalsePayCostPanics));
    r.place_on_field(0, "REDUCER-CF", Some(0));

    let memory_before = r.memory();
    // Would panic inside pay_cost_fn if condition didn't gate it.
    r.play(0, 0);

    // No panic = condition correctly prevented pay_cost_fn from firing.
    // No reduction: effective cost = 4.
    assert_eq!(
        r.memory(),
        memory_before - 4,
        "condition(false) should gate pay_cost_fn in BeforePayCost scan"
    );
}

#[test]
fn pay_cost_fn_at_before_pay_does_not_fire_for_out_of_scope_timing() {
    // OnPlay effect (NOT BeforePayCost) with pay_cost_fn(|_| panic!()).
    // The BeforePayCost scan must NOT touch OnPlay timing effects.
    // Play a card. Assert no panic.
    let target = plain_digimon("TARGET-OS", 4);
    let reducer = make_test_card("REDUCER-OS", "ReducerOs");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(reducer)
        .hand(0, &["TARGET-OS"])
        .memory(10)
        .start();

    r.register_effect("REDUCER-OS", Arc::new(OnPlayPayCostPanics));
    r.place_on_field(0, "REDUCER-OS", Some(0));

    let memory_before = r.memory();
    // Would panic if BeforePayCost scan accidentally touched OnPlay effects.
    r.play(0, 0);

    // No panic = scan correctly filtered to BeforePayCost timing only.
    // No reduction (OnPlay effects don't participate in cost reduction scan).
    assert_eq!(
        r.memory(),
        memory_before - 4,
        "OnPlay timing effects must not fire in BeforePayCost scan"
    );
}
