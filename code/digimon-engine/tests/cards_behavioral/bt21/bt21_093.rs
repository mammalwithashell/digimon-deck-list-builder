//! BT21-093 Raging Serpentine — Option, Cost 8, Red, LIBERATOR.
//!
//! # Card text (cards.json)
//!
//! When this card would be used, if your opponent has 3 or fewer security
//! cards, reduce the use cost by 4.
//! [Main] Delete 1 of your opponent's highest DP Digimon. Then, place this
//! card in the battle area.
//! [All Turns] When your opponent's security stack is removed from, ＜Delay＞
//! (By trashing this card after the placing turn, activate the effect below.)
//! ・1 of your [Reptile] or [Dragonkin] trait Digimon may digivolve into a
//!   [Reptile] or [Dragonkin] trait Digimon card in the hand without paying
//!   the cost.
//! Inherited: [Security] Delete 1 of your opponent's Digimon with the highest DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Red/BT21_093.cs
//!
//! # Patterns this test covers
//! - D2 cost reduction (opponent-conditioned, before_pay_cost)
//! - F-DELETE-HIGHEST-DP (highest-DP aggregate filter)
//! - Delay option as battle-area permanent
//! - All-Turns triggered Delay activation
//! - inherited Security clause
//!
//! # Status — PARTIAL (orchestrator-salvaged after agent stall)
//!
//! YAML written by the original Batch 13 worker; the agent stalled mid-test,
//! so the orchestrator wrote this structural-only test file and registered
//! the two raw_rust stubs the YAML references. All behavioral tests are
//! gap-blocked.
//!
//! Active gaps:
//! - G-OPP-SECURITY-COUNT-LTE (dsl) — clause-0 cost reduction predicate
//!   modeled via `bt21_093_cost_reduction_amount` raw_rust formula stub.
//! - G-PRED-DP-LTE family (engine) — "highest DP" aggregate filter not
//!   evaluated; clauses 1+4 wrap deletion in
//!   `bt21_093_delete_highest_dp_opponent` no-op stub.
//! - G-PLACE-SELF-AS-OPTION-PERMANENT (dsl) — "place this card in the
//!   battle area" sub-step relies on `kind: delay` classification.
//! - G-ALL-TURNS-FILTER (engine) — `active_when: { all_turns: true }`
//!   firing on opponent's turn unverified.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::debug_runner::DebugRunner;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT21-093")
        .expect("BT21-093 in embedded pack")
        .memory(10)
        .start()
}

#[test]
fn bt21_093_compiles_as_option_card() {
    let r = runner();
    let card = r
        .compiled_card("BT21-093")
        .expect("compiled card present");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(8));
}

#[test]
fn bt21_093_has_cost_reduction_clause() {
    let r = runner();
    let card = r
        .compiled_card("BT21-093")
        .expect("compiled card present");
    let has_cr = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )
    });
    assert!(has_cr, "expected one cost_reduction declarative clause");
}

#[test]
fn bt21_093_has_main_from_hand_clause() {
    let r = runner();
    let card = r
        .compiled_card("BT21-093")
        .expect("compiled card present");
    let has_main = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand)
        )
    });
    assert!(has_main, "expected one main_from_hand triggered clause");
}

#[test]
fn bt21_093_has_inherited_security_clause() {
    let r = runner();
    let card = r
        .compiled_card("BT21-093")
        .expect("compiled card present");
    let has_security = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited && t.when.contains(&CompiledTiming::OnSecurity)
        )
    });
    assert!(has_security, "expected inherited on_security clause");
}

#[test]
#[ignore = "pending: G-PRED-DP-LTE — highest-DP aggregate filter not evaluated"]
fn bt21_093_main_deletes_highest_dp_opponent_digimon() {
    todo!("activate from hand, opponent has multi-DP digimon, assert highest is deleted");
}

#[test]
#[ignore = "pending: G-OPP-SECURITY-COUNT-LTE — opponent_security_count predicate not in DSL"]
fn bt21_093_cost_reduced_when_opp_has_3_or_fewer_security() {
    todo!("set opp security to 3, attempt to play, assert cost was 4 not 8");
}

#[test]
#[ignore = "pending: G-OPP-SECURITY-COUNT-LTE — negative branch"]
fn bt21_093_cost_not_reduced_when_opp_has_4_or_more_security() {
    todo!("set opp security to 4+, attempt to play, assert cost was 8");
}

#[test]
#[ignore = "pending: G-PLACE-SELF-AS-OPTION-PERMANENT — Delay placement path"]
fn bt21_093_main_places_self_in_battle_area_as_delay() {
    todo!("after main resolves, assert option is in battle area as Delayed");
}

#[test]
#[ignore = "pending: G-ALL-TURNS-FILTER + G-PLACE-SELF-AS-OPTION-PERMANENT"]
fn bt21_093_delay_activates_on_opp_security_removed() {
    todo!("place self via Main, then opp loses security, assert Delay body fires");
}
