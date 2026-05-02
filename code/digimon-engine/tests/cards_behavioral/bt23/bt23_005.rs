//! BT23-005 Elizamon — Digimon, Lv.3, Red.
//!
//! # Card text (cards.json)
//!
//! **Own effect:**
//! [Your Turn] When this Digimon would digivolve into a Digimon card with the
//! [Reptile] or [Dragonkin] trait, reduce the digivolution cost by 1.
//!
//! **Inherited effect:**
//! [Your Turn] This Digimon gets +2000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT23/Red/BT23_005.cs
//!
//! # Patterns this test covers
//! - D2 (partial) — cost reduction with BeforePayCost (Clause 1 BLOCKED; see below)
//! - D4 — declarative aura inherited DP modifier (Clause 2 IMPLEMENTED)
//! - §6 Aura "inherited self-DP" pattern: scope: inherited, active_when: your_turn, target: {}
//!
//! # Clause 1 — cost reduction BLOCKED (DSL vocab gap)
//!
//! The DSL `CostReductionBody` only supports:
//!   - `when_playing_this: true` — reduce cost when playing this card from hand
//!   - `when_any_ally_played: { ... }` — reduce cost when an ally matching a
//!     predicate enters the field
//!
//! It has no `when_this_digivolves_into` + `target_trait_has` trigger form:
//! "when THIS permanent (BT23-005) is the digivolution source and the card
//! being digivolved INTO has Reptile or Dragonkin trait."
//!
//! Furthermore, `scan_before_pay_cost_reduction` constructs `EffectReadContext`
//! from the source card (Elizamon), not from the target hand card, so the
//! condition closure cannot inspect the target's traits.
//!
//! Logged in qa/dsl-vocab-gaps.md. Affected tests use `#[ignore]`.
//!
//! # Clause 2 — inherited [Your Turn] +2000 DP IMPLEMENTED
//!
//! The DSL self-aura (empty target `{}`) with `scope: inherited` sets the
//! static `dp_modifier` field on the Effect. `Game::source_dp_contribution`
//! reads this field for tensor-layer DP computation, gated by the condition
//! (which wraps `active_when: { your_turn: true }`).

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::permanent::PermanentHandle;

use crate::dsl_card_data::card_data_from_compiled;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Build a runner with BT23-005 (Elizamon) registered and stacked under a Lv4
/// dummy Digimon on player 0's battle area. Also registers a second non-Reptile
/// Lv4 for negative-condition tests.
///
/// Stack layout after `elizamon_under_carrier()`:
///   P0 battle_area[0]: [elizamon (source 0), carrier_lv4 (source 1/top)]
fn runner_with_elizamon_as_source() -> (DebugRunner, PermanentHandle) {
    let elizamon_data = card_data_from_compiled("BT23-005");
    let mut carrier = make_test_card("CARRIER-LV4", "CarrierLv4");
    carrier.level = Some(4);
    carrier.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .add_card(elizamon_data)
        .add_card(carrier)
        .memory(10)
        .start();

    // Place Elizamon on field as the base (source 0).
    let elizamon_handle = runner.place_on_field(0, "BT23-005", Some(0));

    // Simulate digivolving the carrier on top of Elizamon by directly pushing
    // the carrier CardSource into the Elizamon permanent's stack.
    // This mimics `digivolve_from_hand` without the memory/evo-cost machinery.
    let carrier_data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "CARRIER-LV4")
        .expect("CARRIER-LV4 in card_data");
    let next_idx = runner.game.next_card_index();
    let carrier_source = digimon_engine::card_source::CardSource::new(
        carrier_data_idx,
        0, // player 0
        next_idx,
    );
    let turn = runner.game.turn_count;
    runner.game.players[0].battle_area[elizamon_handle.index as usize]
        .digivolve(carrier_source, turn);

    // After digivolving, the permanent's handle is unchanged (same field index).
    // elizamon_handle still points to the correct battle_area slot.
    (runner, elizamon_handle)
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

#[test]
fn bt23_005_compiles_with_exactly_one_inherited_aura_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("BT23-005")
        .expect("BT23-005 found in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT23-005")
        .expect("BT23-005 in compiled_cards");

    // Exactly one clause in the YAML (the inherited aura).
    // Clause 1 (cost reduction) is BLOCKED and absent from the YAML.
    assert_eq!(
        compiled.effects.len(),
        1,
        "BT23-005 should have exactly 1 compiled clause (inherited aura only; \
         cost-reduction clause is BLOCKED on DSL vocab gap)"
    );
}

#[test]
fn bt23_005_inherited_aura_clause_has_correct_scope() {
    let runner = DebugRunner::builder()
        .dsl_card("BT23-005")
        .expect("BT23-005 found in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT23-005")
        .expect("BT23-005 in compiled_cards");

    let aura = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope, dp_modifier, ..
        }) => Some((*scope, *dp_modifier)),
        _ => None,
    });

    let (scope, dp) = aura.expect("BT23-005 should have a Declarative::Aura clause");
    assert_eq!(
        scope,
        CompiledScope::Inherited,
        "the aura clause must have Inherited scope (active as a digivolution source)"
    );
    assert_eq!(dp, Some(2000), "the aura dp_modifier must be +2000 DP");
}

// ─── Section 2: Behavioral — inherited DP positive gate (your turn) ──────────

/// When BT23-005 is a digivolution source under a carrier, on the controller's
/// own turn, `source_dp_contribution` must return +2000 for Elizamon's slot.
#[test]
fn bt23_005_inherited_dp_active_on_your_turn() {
    let (runner, carrier_handle) = runner_with_elizamon_as_source();

    // Source index 0 = Elizamon (the base), index 1 = carrier (top card).
    let elizamon_src_idx = 0;

    // It is currently player 0's turn (turn 1 starts with player 0).
    assert_eq!(
        runner.turn_player(),
        0,
        "precondition: it is player 0's turn"
    );

    let contribution = runner
        .game
        .source_dp_contribution(carrier_handle, elizamon_src_idx);

    assert_eq!(
        contribution, 2000,
        "Elizamon's inherited +2000 DP must contribute on the controller's own turn; \
         got {} instead",
        contribution
    );
}

/// On the opponent's turn, the [Your Turn] gate must suppress the DP buff.
/// Source index 0 (Elizamon) should contribute 0 during player 1's turn.
#[test]
fn bt23_005_inherited_dp_inactive_on_opponents_turn() {
    let (mut runner, carrier_handle) = runner_with_elizamon_as_source();

    // End player 0's turn → player 1's turn begins.
    runner.end_turn();
    assert_eq!(
        runner.turn_player(),
        1,
        "precondition: it is now player 1's turn"
    );

    let elizamon_src_idx = 0;
    let contribution = runner
        .game
        .source_dp_contribution(carrier_handle, elizamon_src_idx);

    assert_eq!(
        contribution, 0,
        "Elizamon's inherited +2000 DP must NOT contribute on the opponent's turn \
         ([Your Turn] gate); got {} instead",
        contribution
    );
}

/// Confirm that the carrier's TOP card (index 1) does NOT contribute the +2000 DP
/// even on the controller's turn, since it is NOT the Elizamon source.
#[test]
fn bt23_005_top_card_does_not_contribute_elizamon_dp() {
    let (runner, carrier_handle) = runner_with_elizamon_as_source();

    // Stack: [elizamon (0), carrier (1/top)]
    // The top-card slot is not inherited, so its dp contribution comes from its
    // own non-inherited effects. CARRIER-LV4 has no effects, so should be 0.
    let top_src_idx = 1;
    let contribution = runner
        .game
        .source_dp_contribution(carrier_handle, top_src_idx);

    assert_eq!(
        contribution, 0,
        "the carrier's own slot (top card) should not contribute Elizamon's buff"
    );
}

// ─── Section 3: Behavioral — cost reduction BLOCKED ──────────────────────────
//
// Clause 1 cannot be expressed in the current DSL:
//   "When THIS Digimon would digivolve into a card with [Reptile] or [Dragonkin]
//    trait, reduce the digivolution cost by 1."
//
// `CostReductionBody` only supports `when_playing_this` and `when_any_ally_played`.
// There is no `when_this_digivolves_into` + `target_trait_has` trigger variant.
// The `scan_before_pay_cost_reduction` engine path also lacks a mechanism to
// thread the target card's traits into the condition closure.
//
// Tests for the positive and negative branches of this clause are listed below
// with `#[ignore]` pointing to the dsl-vocab-gaps.md entry so they are
// mechanically unblockable once the gap closes.

/// Positive branch: digivolving FROM Elizamon INTO a Reptile/Dragonkin Lv4
/// should reduce the printed evo cost by 1.
///
/// Expected behavior once gap closes:
///   - Place Elizamon on field.
///   - Lv4 Reptile in hand, printed evo cost = 2.
///   - Memory before digivolve = 2.
///   - After digivolve: memory should be 2 − (2 − 1) = 1 (reduction applies).
#[test]
#[ignore = "pending: DSL vocab gap — no when_this_digivolves_into + target_trait_has in \
             CostReductionBody (see qa/dsl-vocab-gaps.md)"]
fn bt23_005_cost_reduction_fires_digivolving_into_reptile() {
    todo!("implement once DSL adds when_this_digivolves_into + target_trait_has trigger")
}

/// Negative branch 1: digivolving FROM Elizamon INTO a non-Reptile / non-Dragonkin
/// Lv4 must NOT trigger the cost reduction.
#[test]
#[ignore = "pending: DSL vocab gap — same as bt23_005_cost_reduction_fires_digivolving_into_reptile"]
fn bt23_005_cost_reduction_does_not_fire_for_non_trait_target() {
    todo!("implement once DSL adds when_this_digivolves_into + target_trait_has trigger")
}

/// Negative branch 2: the [Your Turn] gate must suppress the cost reduction on
/// the opponent's turn.
#[test]
#[ignore = "pending: DSL vocab gap — same as bt23_005_cost_reduction_fires_digivolving_into_reptile"]
fn bt23_005_cost_reduction_inactive_on_opponents_turn() {
    todo!("implement once DSL adds when_this_digivolves_into + target_trait_has trigger")
}

/// Negative branch 3: the "THIS Digimon" gate — if a different Lv3 (not BT23-005)
/// is the digivolution source, no cost reduction applies.
#[test]
#[ignore = "pending: DSL vocab gap — same as bt23_005_cost_reduction_fires_digivolving_into_reptile"]
fn bt23_005_cost_reduction_does_not_fire_for_different_source() {
    todo!("implement once DSL adds when_this_digivolves_into + target_trait_has trigger")
}
