//! BT3-103 Hidden Potential Discovered! — Option, Green, Cost 4.
//!
//! # Card text (cards.json)
//!
//! [Main] For the turn, when one of your green Digimon would next digivolve,
//! by suspending 1 of your Digimon, reduce the digivolution cost by 5.
//!
//! Inherited: Security Effect [Security] Add this card to the hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT3/Green/BT3_103.cs
//!
//! # Implementation status: PARTIAL
//!
//! | Clause | Description                                       | Status      |
//! |--------|---------------------------------------------------|-------------|
//! | 0      | [Main] one-shot green-Digimon digivolve cost -5   | BLOCKED     |
//! | 1      | [Security] Add this card to the hand              | IMPLEMENTED |
//!
//! # Clause 0 — BLOCKED
//!
//! The main effect arms a turn-scoped BeforePayCost observer that fires once
//! when any green Digimon would digivolve, allowing the player to suspend any
//! 1 of their Digimon to reduce the digivolution cost by 5.
//!
//! Three DSL vocab gaps block this clause:
//!
//! * **G-COST-REDUCE-ALLY-DIGIVOLVE** — `CostReductionBody` has no
//!   `when_any_ally_would_digivolve` trigger form. The engine's
//!   `scan_before_pay_cost_reduction` fires only during play-cost scans,
//!   not during digivolve-cost scans. The analogous gap for digivolve-INTO
//!   (BT23-005, BT5-092) is tracked in qa/dsl-vocab-gaps.md; this is the
//!   SOURCE-color form of the same missing primitive.
//!
//! * **G-COST-REDUCE-NEXT-SINGLE-FIRE** — the "next digivolve" wording means
//!   the observer should fire exactly once (DCGO achieves this via a "Remove
//!   Effect" background coroutine that unregisters the ChangeCostClass after
//!   the first application). No one-shot/self-removing variant exists for
//!   CostReductionBody.
//!
//! * **G-PAY-COST-SELECT-ARBITRARY-SUSPEND** — the suspension cost in
//!   `pay_cost:` step lists is currently only verified for `suspend: { target:
//!   source }` (the source card itself). BT3-103 requires selecting any 1 of
//!   the controller's own Digimon to suspend — a `select_own_permanent` inside
//!   `pay_cost:`, which is unimplemented in the BeforePayCost scanning flow.
//!
//! Clause 0 is intentionally omitted from BT3-103.yaml. The `#[ignore]`'d
//! tests below document the expected behavior for when these gaps close.
//!
//! # Clause 1 — IMPLEMENTED
//!
//! The inherited security clause uses `add_this_option_to_hand: {}`, resolved
//! 2026-05-01. `EffectContext::add_pending_security_to_hand()` moves the card
//! from `Game.pending_security` to the defender/controller hand.
//!
//! # Test coverage
//!
//! Active tests (structural):
//!   - `bt3_103_compiles_as_option_card` — card parses + compiles, cost 4, green
//!   - `bt3_103_has_exactly_one_inherited_security_clause` — clause count assertion
//!   - `bt3_103_security_clause_uses_add_this_option_to_hand` — step-kind assertion
//!   - `bt3_103_security_clause_is_not_optional` — mandatory per rules
//!
//! Ignored (blocked):
//!   - `bt3_103_main_arms_digivolve_cost_reduction_for_turn` (G-COST-REDUCE-ALLY-DIGIVOLVE)
//!   - `bt3_103_cost_reduction_fires_on_green_digimon_only` (G-COST-REDUCE-ALLY-DIGIVOLVE)
//!   - `bt3_103_cost_reduction_does_not_fire_on_non_green_digimon` (G-COST-REDUCE-ALLY-DIGIVOLVE)
//!   - `bt3_103_cost_reduction_requires_player_to_suspend_one_digimon` (G-PAY-COST-SELECT-ARBITRARY-SUSPEND)
//!   - `bt3_103_cost_reduction_fires_at_most_once_per_play` (G-COST-REDUCE-NEXT-SINGLE-FIRE)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

// ─── Helper builders ─────────────────────────────────────────────────────────

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn make_green_digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Green];
    c
}

fn make_red_digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Red];
    c
}

/// Base runner: BT3-103 registered in hand index 0, filler decks on both
/// sides so no deck-out on draw, filler security for P1.
fn runner() -> DebugRunner {
    let filler = make_filler("FILL");
    DebugRunner::builder()
        .dsl_card("BT3-103")
        .expect("BT3-103 YAML must parse and compile")
        .add_card(filler.clone())
        .hand(0, &["BT3-103"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
        .memory(4)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions (active)
// ═══════════════════════════════════════════════════════════════════════════

/// BT3-103 compiles as a green Option with cost 4.
#[test]
fn bt3_103_compiles_as_option_card() {
    let r = runner();
    let card = r
        .compiled_card("BT3-103")
        .expect("BT3-103 must be in compiled_cards");

    use digimon_dsl::compiled::CompiledCardKind;
    assert_eq!(card.kind, CompiledCardKind::Option, "must be an Option card");
    assert_eq!(card.cost, Some(4), "play cost must be 4");

    // Color is not stored in CompiledCard but we can verify via the raw
    // card registry instead.
    // The YAML specifies color: [green]; the card registry does not surface
    // CompiledCard.colors — just assert the cost and kind are correct here.
}

/// The YAML omits Clause 0 (BLOCKED main effect) so only the inherited
/// security clause should be present.
#[test]
fn bt3_103_has_exactly_one_inherited_security_clause() {
    let r = runner();
    let card = r
        .compiled_card("BT3-103")
        .expect("BT3-103 must be in compiled_cards");

    assert_eq!(
        card.effects.len(),
        1,
        "BT3-103 must have exactly 1 clause (inherited security only; \
         main clause is BLOCKED on G-COST-REDUCE-ALLY-DIGIVOLVE)"
    );
}

/// The single clause must be the inherited security clause with
/// `OnSecurity` timing.
#[test]
fn bt3_103_security_clause_is_inherited_with_on_security_timing() {
    let r = runner();
    let card = r
        .compiled_card("BT3-103")
        .expect("BT3-103 compiled card present");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("must have a clause with OnSecurity timing");

    assert_eq!(
        clause.scope,
        CompiledScope::Inherited,
        "security clause must have Inherited scope"
    );
}

/// The security clause must NOT be optional — [Security] effects are
/// mandatory per the rules manual and DCGO.
#[test]
fn bt3_103_security_clause_is_not_optional() {
    let r = runner();
    let card = r
        .compiled_card("BT3-103")
        .expect("BT3-103 compiled card present");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("security clause must exist");

    assert!(
        !clause.optional,
        "security clause must not be optional — [Security] effects are mandatory"
    );
}

/// The security clause process must contain exactly one step and that step
/// must be `AddThisOptionToHand` (the resolved G-ADD-OPTION-SELF-TO-HAND
/// primitive).
#[test]
fn bt3_103_security_clause_uses_add_this_option_to_hand() {
    let r = runner();
    let card = r
        .compiled_card("BT3-103")
        .expect("BT3-103 compiled card present");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("security clause must exist");

    assert_eq!(
        clause.process.len(),
        1,
        "security clause must have exactly 1 process step"
    );

    let is_add_to_hand = matches!(
        &clause.process[0],
        CompiledStep::AddThisOptionToHand
    );
    assert!(
        is_add_to_hand,
        "security clause process step must be AddThisOptionToHand, got: {:?}",
        &clause.process[0]
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 0 (Main) — BLOCKED behavioral tests
// ═══════════════════════════════════════════════════════════════════════════
//
// All tests below are #[ignore]'d. They document the expected behavioral
// contract for Clause 0 and will be un-ignored once the blocking gaps close.
//
// Gap tags:
//   G-COST-REDUCE-ALLY-DIGIVOLVE — no when_any_ally_would_digivolve trigger
//   G-COST-REDUCE-NEXT-SINGLE-FIRE — no one-shot self-removing observer
//   G-PAY-COST-SELECT-ARBITRARY-SUSPEND — no select_own_permanent in pay_cost

/// Playing BT3-103 from hand should arm a BeforePayCost observer for the
/// current turn. The armed observer should be visible in engine state (as
/// a modifier or queued effect) so that RL agents can observe it in the
/// action mask.
///
/// Blocked: G-COST-REDUCE-ALLY-DIGIVOLVE — the DSL has no
/// `when_any_ally_would_digivolve` trigger form and
/// `scan_before_pay_cost_reduction` does not intercept digivolve-cost scans.
#[test]
#[ignore = "BLOCKED: G-COST-REDUCE-ALLY-DIGIVOLVE (qa/dsl-vocab-gaps.md)"]
fn bt3_103_main_arms_digivolve_cost_reduction_for_turn() {
    let mut r = runner();
    let _fired = r.game.activate_hand_main(0, 0);
    // Expected: engine has a turn-scoped observer registered that will
    // intercept the next green-Digimon digivolve BeforePayCost event.
    // Assertion shape (TBD once primitive lands):
    //   assert!(r.game.has_pending_cost_reduction_observer_for_player(0));
    todo!("implement once G-COST-REDUCE-ALLY-DIGIVOLVE closes")
}

/// The cost reduction should trigger only when a GREEN Digimon would
/// digivolve. If the digivolving Digimon is a different color (e.g. red),
/// the observer should not fire.
///
/// Blocked: G-COST-REDUCE-ALLY-DIGIVOLVE — source-color predicate (green)
/// on the digivolving permanent is also missing from the trigger form.
#[test]
#[ignore = "BLOCKED: G-COST-REDUCE-ALLY-DIGIVOLVE (source-color predicate)"]
fn bt3_103_cost_reduction_fires_on_green_digimon_only() {
    todo!("implement once G-COST-REDUCE-ALLY-DIGIVOLVE closes (green source predicate)")
}

/// Negative test: a non-green (e.g. red) Digimon digivolving should not
/// receive the cost discount.
///
/// Blocked: G-COST-REDUCE-ALLY-DIGIVOLVE.
#[test]
#[ignore = "BLOCKED: G-COST-REDUCE-ALLY-DIGIVOLVE"]
fn bt3_103_cost_reduction_does_not_fire_on_non_green_digimon() {
    todo!("implement once G-COST-REDUCE-ALLY-DIGIVOLVE closes")
}

/// When the observer fires, the player must be prompted to suspend any 1
/// of their Digimon (not the Option itself). If the player declines (passes)
/// the digivolution cost is unmodified.
///
/// Blocked: G-PAY-COST-SELECT-ARBITRARY-SUSPEND — `select_own_permanent`
/// inside pay_cost steps is unimplemented for BeforePayCost scanning.
#[test]
#[ignore = "BLOCKED: G-PAY-COST-SELECT-ARBITRARY-SUSPEND"]
fn bt3_103_cost_reduction_requires_player_to_suspend_one_digimon() {
    todo!("implement once G-PAY-COST-SELECT-ARBITRARY-SUSPEND closes")
}

/// The observer must be self-removing after the first firing ("NEXT
/// digivolve"). Playing BT3-103 and then having two green Digimon digivolve
/// in the same turn should only grant the cost reduction on the first
/// digivolve; the second digivolve cost is unaffected.
///
/// Blocked: G-COST-REDUCE-NEXT-SINGLE-FIRE — no one-shot/self-removing
/// qualifier on CostReductionBody.
#[test]
#[ignore = "BLOCKED: G-COST-REDUCE-NEXT-SINGLE-FIRE"]
fn bt3_103_cost_reduction_fires_at_most_once_per_play() {
    todo!("implement once G-COST-REDUCE-NEXT-SINGLE-FIRE closes")
}
