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
//! # Patterns this audit covers
//! - D2 cost reduction (opponent-conditioned, before_pay_cost)
//! - F-DELETE-HIGHEST-DP (highest-DP aggregate target choice)
//! - Delay option as battle-area permanent
//! - All-Turns event-gated Delay activation
//! - inherited Security clause
//!
//! # Status — AUDIT-PARTIAL
//!
//! Current YAML compiles, but it is still only structurally faithful for the
//! highest-DP deletion and event-gated Delay flow. This audit intentionally
//! pins the remaining raw_rust escape hatches and the stale comments that would
//! otherwise make the card look more complete than it is.
//!
//! Active gaps / drift:
//! - G-HIGHEST-DP-TARGET-SELECTION (engine/dsl) — clauses 1 and 4 still route
//!   through `bt21_093_delete_highest_dp_opponent`, which is currently a no-op
//!   raw_rust step. A faithful implementation must expose tied highest-DP
//!   choices through pending selection/action masks.
//! - G-DELAY-ON-OPPONENT-SECURITY-REMOVED (engine/dsl) — YAML uses a structural
//!   `kind: delay` marker with an empty end-of-your-next-turn body plus a
//!   separate `on_opponent_security_removed` triggered clause. This is not a
//!   native Delay trigger and remains behaviorally unverified.
//!
//! Resolved/stale gap note:
//! - Phase 2 Track G (2026-05-17) wired the native
//!   `opponent_security_count_lte` predicate, so clause 0 now reads
//!   `condition: { opponent_security_count_lte: 3 }, amount: 4` instead of
//!   delegating to a `bt21_093_cost_reduction_amount` raw_rust formula.
//!   The raw_rust shim has been removed.
//! - `place_self_as_delay_option: {}` now exists for explicit source-option
//!   placement. BT21-093's main-use placement currently relies on standard
//!   Delay option disposal from hand, so `G-PLACE-SELF-AS-OPTION-PERMANENT` is
//!   not the blocker for the main clause in this audit.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledCostDelta, CompiledCountAggregate,
    CompiledDeclarativeClause, CompiledDpConstraint, CompiledPlayerRef, CompiledPredicate,
    CompiledScope, CompiledStep, CompiledTiming, CompiledZone,
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
    let card = r.compiled_card("BT21-093").expect("compiled card present");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(8));
    assert_eq!(
        card.effects.len(),
        5,
        "expected cost reduction, main, delay marker, event-gated Delay body, and inherited security clauses"
    );
}

#[test]
fn bt21_093_cost_reduction_uses_opponent_security_count_predicate() {
    let r = runner();
    let card = r.compiled_card("BT21-093").expect("compiled card present");
    let cost_reduction = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
            reduction_timing,
            when_playing_this,
            condition,
            amount,
            amount_fn,
            ..
        }) => Some((
            reduction_timing,
            when_playing_this,
            condition,
            amount,
            amount_fn,
        )),
        _ => None,
    });
    let Some((reduction_timing, when_playing_this, condition, amount, amount_fn)) = cost_reduction
    else {
        panic!("expected one cost_reduction declarative clause");
    };

    assert_eq!(reduction_timing.as_deref(), Some("before_pay_cost"));
    assert!(
        *when_playing_this,
        "cost reduction must only apply when BT21-093 itself would be used"
    );
    // After Phase 2 Track G the YAML uses the native
    // `opponent_security_count_lte: 3` predicate leaf rather than the
    // `count_lte` aggregate over `{ zone: [security], owner: opponent }`.
    // Both forms compile to a CompiledPredicate that gates on the same
    // opponent-security count threshold.
    assert_eq!(
        *condition,
        Some(CompiledPredicate {
            opponent_security_count_lte: Some(CompiledDpConstraint::Literal(3)),
            ..CompiledPredicate::default()
        }),
        "cost reduction must gate on opponent security count <= 3"
    );
    assert_eq!(
        *amount,
        Some(4),
        "native count_lte condition should use a fixed reduction amount"
    );
    assert_eq!(
        *amount_fn, None,
        "opponent security <=3 cost reduction no longer needs a raw formula bridge"
    );
}

#[test]
fn bt21_093_main_clause_is_raw_rust_highest_dp_delete_then_implicit_delay_disposal() {
    let r = runner();
    let card = r.compiled_card("BT21-093").expect("compiled card present");
    let main = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => Some(t),
        _ => None,
    });
    let Some(main) = main else {
        panic!("expected one main_from_hand triggered clause");
    };

    assert_eq!(main.scope, CompiledScope::FaceUp);
    assert!(
        !main.optional,
        "[Main] delete highest-DP opponent Digimon is mandatory when a target exists"
    );
    assert_eq!(
        main.process,
        vec![CompiledStep::RawRust {
            fn_name: "bt21_093_delete_highest_dp_opponent".to_string(),
            consumes: vec![],
            binds: vec![],
        }],
        "main clause still depends on the no-op highest-DP raw_rust bridge; no native selector is wired here"
    );
    assert!(
        !main
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlaceSelfAsDelayOption)),
        "BT21-093 main-use placement is currently implicit through Delay option disposal, not the explicit inherited-security placement step"
    );
}

#[test]
fn bt21_093_has_structural_delay_marker_but_not_native_event_gated_delay() {
    let r = runner();
    let card = r.compiled_card("BT21-093").expect("compiled card present");
    let delay = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
            trigger, process, ..
        }) => Some((trigger, process)),
        _ => None,
    });
    let Some((trigger, process)) = delay else {
        panic!("expected declarative Delay marker clause");
    };

    assert_eq!(
        *trigger,
        CompiledTiming::EndOfYourNextTurn,
        "current YAML uses a structural Delay marker; printed trigger is opponent-security removal"
    );
    assert!(
        process.is_empty(),
        "the structural Delay marker must not hide a false end-of-turn behavior body"
    );
}

#[test]
fn bt21_093_event_gated_delay_clause_is_all_turns_on_opponent_security_removed() {
    let r = runner();
    let card = r.compiled_card("BT21-093").expect("compiled card present");
    let delay_body = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::OnOpponentSecurityRemoved) =>
        {
            Some(t)
        }
        _ => None,
    });
    let Some(delay_body) = delay_body else {
        panic!("expected on_opponent_security_removed triggered clause for Delay body");
    };

    assert_eq!(delay_body.scope, CompiledScope::FaceUp);
    assert!(
        !delay_body.optional,
        "Delay cost/trash is mandatory once the event-gated Delay is activated"
    );
    assert_eq!(
        delay_body.active_when,
        Some(CompiledPredicate {
            all_turns: Some(true),
            ..CompiledPredicate::default()
        }),
        "printed [All Turns] must remain visible in the compiled active_when predicate"
    );
    assert_eq!(
        delay_body.condition,
        Some(CompiledPredicate {
            on_field: Some(true),
            ..CompiledPredicate::default()
        }),
        "Delay body must only fire while BT21-093 is already a battle-area Delay option"
    );
    assert!(
        matches!(
            delay_body.process.first(),
            Some(CompiledStep::DeletePermanent { .. })
        ),
        "Delay activation must trash BT21-093 before offering the free digivolve"
    );
    assert!(
        delay_body.process.iter().any(|step| matches!(
            step,
            CompiledStep::EffectInitiatedDigivolve {
                cost,
                ignore_requirements: true,
                ..
            } if *cost == CompiledCostDelta::Literal(0)
        )),
        "Delay body should include free effect-initiated digivolve"
    );
}

#[test]
fn bt21_093_inherited_security_clause_reuses_highest_dp_raw_rust_delete() {
    let r = runner();
    let card = r.compiled_card("BT21-093").expect("compiled card present");
    let security = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::OnSecurity) =>
        {
            Some(t)
        }
        _ => None,
    });
    let Some(security) = security else {
        panic!("expected inherited on_security clause");
    };

    assert_eq!(
        security.process,
        vec![CompiledStep::RawRust {
            fn_name: "bt21_093_delete_highest_dp_opponent".to_string(),
            consumes: vec![],
            binds: vec![],
        }],
        "security clause still depends on the same highest-DP raw_rust bridge as [Main]"
    );
}

#[test]
#[ignore = "pending: G-HIGHEST-DP-TARGET-SELECTION — raw_rust bridge is currently a no-op; needs pending selection/action-mask coverage for tied highest DP"]
fn bt21_093_main_deletes_highest_dp_opponent_digimon() {
    todo!("activate from hand, opponent has multi-DP digimon, assert highest is deleted");
}

#[test]
fn bt21_093_cost_reduced_when_opp_has_3_or_fewer_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-093")
        .expect("BT21-093 in embedded pack")
        .hand(0, &["BT21-093"])
        .security(1, &["BT21-093", "BT21-093", "BT21-093"])
        .memory(10)
        .start();

    let mem_before = runner.memory();
    runner.play(0, 0);
    let cost = mem_before - runner.memory();
    assert_eq!(
        cost, 4,
        "opponent at 3 security should reduce use cost by 4"
    );
}

#[test]
fn bt21_093_cost_not_reduced_when_opp_has_4_or_more_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-093")
        .expect("BT21-093 in embedded pack")
        .hand(0, &["BT21-093"])
        .security(1, &["BT21-093", "BT21-093", "BT21-093", "BT21-093"])
        .memory(10)
        .start();

    let mem_before = runner.memory();
    runner.play(0, 0);
    let cost = mem_before - runner.memory();
    assert_eq!(cost, 8, "opponent at 4 security should pay full use cost");
}

#[test]
#[ignore = "pending: G-DELAY-ON-OPPONENT-SECURITY-REMOVED — placement should be tested through normal option play after highest-DP raw_rust no-op is removed or isolated"]
fn bt21_093_main_places_self_in_battle_area_as_delay() {
    todo!("after main resolves, assert option is in battle area as Delayed");
}

#[test]
#[ignore = "pending: G-DELAY-ON-OPPONENT-SECURITY-REMOVED + G-HIGHEST-DP-TARGET-SELECTION — event-gated Delay activation and optional free digivolve need behavioral coverage"]
fn bt21_093_delay_activates_on_opp_security_removed() {
    todo!("place self via Main, then opp loses security, assert Delay body fires");
}
