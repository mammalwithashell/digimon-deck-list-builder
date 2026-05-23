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
//! - F-DELETE-HIGHEST-DP (highest-DP `selector:` target choice — ties exposed)
//! - Delay option as battle-area permanent (implicit Main-clause placement)
//! - All-Turns event-gated Delay activation (on_opponent_security_removed)
//! - inherited Security clause (highest-DP delete, mirrors [Main])
//!
//! # Status — IMPLEMENTED (pure DSL, no raw_rust)
//!
//! Both prior raw_rust stubs are removed:
//! - `bt21_093_cost_reduction_amount` — removed 2026-05-17 (Phase 2 Track G);
//!   clause 0 uses the native `opponent_security_count_lte: 3` predicate.
//! - `bt21_093_delete_highest_dp_opponent` — the [Main] and [Security] clauses
//!   now use `select_opponent_permanent` with `selector: highest_dp`. The
//!   engine pre-filters candidates to the highest-effective-DP opponent
//!   Digimon (`install_select_opponent_permanent` → `select_field_extreme`);
//!   ties expose every tied permanent as a valid action ID so the choice
//!   surfaces to the RL action space. The registered raw_rust function is now
//!   orphaned (registered but unreferenced) — a registry cleanup follow-up,
//!   not a card-file concern.
//!
//! # Delay-trigger modelling note
//!
//! "When your opponent's security stack is removed from" is NOT a native
//! `DelayTrigger` variant — `lower_delay.rs` only maps `OnSuspend` /
//! `OnUnsuspend` to `DelayTrigger::OnEvent`, and
//! `enqueue_event_gated_delayed_options` is not called on `SecurityRemoved`
//! trigger sources. So the card uses a two-clause model:
//!   - a structural `kind: delay` marker (end_of_your_next_turn, empty body)
//!     that makes `classify_option_subtype` place the Option in the battle
//!     area as a Delay permanent, and
//!   - a separate `on_opponent_security_removed` triggered clause (FaceUp
//!     scope, `on_field` gated) that carries the real Delay body and fires
//!     from the battle-area Option permanent via `enqueue_from_permanent`.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledActivationCostKind, CompiledCardKind, CompiledClause, CompiledCostDelta,
    CompiledCountAggregate, CompiledDeclarativeClause, CompiledDpConstraint, CompiledFieldSelector,
    CompiledPlayerRef, CompiledPredicate, CompiledScope, CompiledStep, CompiledTiming,
    CompiledZone,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};
use digimon_engine::trigger_context::EventCause;

/// The production YAML for BT21-093, inlined at compile time from the
/// canonical location under `cards/bt21/`.
const YAML: &str = include_str!("../../../cards/bt21/BT21-093.yaml");

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT21-093")
        .expect("BT21-093 in embedded pack")
        .memory(10)
        .start()
}

/// Build a Red Digimon test card with an explicit DP and trait set.
fn digimon(
    card_id: &str,
    name: &str,
    dp: i32,
    traits: &[&str],
) -> digimon_engine::card_data::CardData {
    let mut d = make_test_card(card_id, name);
    d.dp = Some(dp);
    d.traits = traits.iter().map(|t| t.to_string()).collect();
    d
}

// ─────────────────────────────────────────────────────────────────────────────
// § 1  Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

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
    // Native `opponent_security_count_lte: 3` predicate leaf (Phase 2 Track G,
    // G-OPP-SECURITY-COUNT-LTE RESOLVED 2026-05-17).
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
        "native predicate condition uses a fixed reduction amount"
    );
    assert_eq!(
        *amount_fn, None,
        "opponent security <=3 cost reduction needs no raw formula bridge"
    );
}

/// [Main] clause: pure-DSL highest-DP delete + implicit Delay disposal.
/// The clause now uses `select_opponent_permanent` (selector: highest_dp) +
/// `delete_permanent` — NOT the old `bt21_093_delete_highest_dp_opponent`
/// raw_rust bridge.
#[test]
fn bt21_093_main_clause_is_pure_dsl_highest_dp_delete() {
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
    // No raw_rust escape hatch anywhere in the body.
    assert!(
        !main
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::RawRust { .. })),
        "[Main] clause must be pure DSL — no raw_rust bridge"
    );
    // Step 0: highest-DP opponent selection.
    match &main.process[0] {
        CompiledStep::SelectOpponentPermanent { selector, .. } => {
            assert_eq!(
                *selector,
                Some(CompiledFieldSelector::HighestDp),
                "[Main] delete must restrict to the highest-DP opponent Digimon"
            );
        }
        other => panic!("expected SelectOpponentPermanent as first step, got {other:?}"),
    }
    // Step 1: delete the bound target.
    assert!(
        matches!(
            main.process.get(1),
            Some(CompiledStep::DeletePermanent { .. })
        ),
        "[Main] clause must delete the selected highest-DP Digimon"
    );
    // "Then, place this card in the battle area" — implicit via the `kind:
    // delay` marker; no explicit PlaceSelfAsDelayOption step in the body.
    assert!(
        !main
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlaceSelfAsDelayOption)),
        "Main-clause placement is implicit through Delay option disposal"
    );
}

/// The structural `kind: delay` marker exists with an empty body — its
/// printed trigger is opponent-security removal, modelled by clause 3.
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
        "structural Delay marker; printed trigger is opponent-security removal"
    );
    assert!(
        process.is_empty(),
        "the structural Delay marker must not hide a false end-of-turn behavior body"
    );
}

/// Clause 3 — the event-gated Delay body — fires on
/// `on_opponent_security_removed`, is `[All Turns]`, gates on `on_field`,
/// trashes the source first, and offers a free effect-initiated digivolve.
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
        delay_body.optional,
        "the <Delay> is declinable (Comprehensive Rules 16-16-2) — modelled as \
         optional: true plus a leading trash_self activation_cost"
    );
    assert_eq!(
        delay_body.active_when,
        Some(CompiledPredicate {
            all_turns: Some(true),
            ..CompiledPredicate::default()
        }),
        "printed [All Turns] must remain visible in the compiled active_when predicate"
    );
    // The Delay body gates on (a) on_field and (b) the existence of a
    // Reptile/Dragonkin Digimon (DCGO CanActivateCondition).
    let cond = delay_body
        .condition
        .as_ref()
        .expect("Delay body must carry a condition");
    assert_eq!(
        cond.all_of.len(),
        2,
        "Delay body condition gates on both on_field and a Reptile/Dragonkin Digimon"
    );
    assert!(
        cond.all_of.iter().any(|c| c.on_field == Some(true)),
        "Delay body must require BT21-093 be on field as a Delay option"
    );
    assert!(
        cond.all_of
            .iter()
            .any(|c| c.any_field_permanent.is_some()),
        "Delay body must gate on the existence of a Reptile/Dragonkin Digimon (CanActivateCondition)"
    );
    assert!(
        matches!(
            delay_body.process.first(),
            Some(CompiledStep::ActivationCost {
                kind: CompiledActivationCostKind::TrashSelf
            })
        ),
        "the <Delay> activation cost — 'by trashing this card' — is a declinable \
         activation_cost step (G-ACTIVATION-COST-TRASH-SELF), not a mandatory body step"
    );
    // The free digivolve lives inside the `if { binding_present: evo_target }`
    // gate — recurse into the `If` branches to find it.
    fn has_free_digivolve(steps: &[CompiledStep]) -> bool {
        steps.iter().any(|step| match step {
            CompiledStep::EffectInitiatedDigivolve {
                cost,
                ignore_requirements: true,
                ..
            } => *cost == CompiledCostDelta::Literal(0),
            CompiledStep::If {
                then, else_branch, ..
            } => has_free_digivolve(then) || has_free_digivolve(else_branch),
            _ => false,
        })
    }
    assert!(
        has_free_digivolve(&delay_body.process),
        "Delay body should include a free (cost 0, ignore_requirements) effect-initiated digivolve"
    );
}

/// [Security] clause: pure-DSL highest-DP delete, mirrors [Main].
///
/// FaceUp scope — an Option's `[Security]` effect is dispatched as the
/// Option's own SecuritySkill effect (see BT24-089), not as a digivolution-
/// source inherited clause.
#[test]
fn bt21_093_security_clause_is_pure_dsl_highest_dp_delete() {
    let r = runner();
    let card = r.compiled_card("BT21-093").expect("compiled card present");
    let security = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
        _ => None,
    });
    let Some(security) = security else {
        panic!("expected on_security clause");
    };
    assert_eq!(
        security.scope,
        CompiledScope::FaceUp,
        "an Option's [Security] effect uses FaceUp scope (the Option's own SecuritySkill effect)"
    );

    assert!(
        !security
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::RawRust { .. })),
        "[Security] clause must be pure DSL — no raw_rust bridge"
    );
    match &security.process[0] {
        CompiledStep::SelectOpponentPermanent { selector, .. } => {
            assert_eq!(
                *selector,
                Some(CompiledFieldSelector::HighestDp),
                "[Security] delete must restrict to the highest-DP opponent Digimon"
            );
        }
        other => panic!("expected SelectOpponentPermanent as first step, got {other:?}"),
    }
    assert!(
        matches!(
            security.process.get(1),
            Some(CompiledStep::DeletePermanent { .. })
        ),
        "[Security] clause must delete the selected highest-DP Digimon"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// § 2  Clause 0 — cost-reduction gating (positive + negative)
// ─────────────────────────────────────────────────────────────────────────────

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
        "opponent at 3 security should reduce use cost by 4 (8 - 4)"
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

// ─────────────────────────────────────────────────────────────────────────────
// § 3  Clause 1 — [Main] delete 1 opponent highest-DP Digimon
// ─────────────────────────────────────────────────────────────────────────────

/// With several opponent Digimon at different DP, the [Main] delete prompt
/// must offer ONLY the single highest-DP Digimon, and resolving it deletes
/// that Digimon.
#[test]
fn bt21_093_main_deletes_highest_dp_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-093")
        .expect("BT21-093 in embedded pack")
        .add_card(digimon("OPP-LOW", "OppLow", 3000, &[]))
        .add_card(digimon("OPP-MID", "OppMid", 6000, &[]))
        .add_card(digimon("OPP-HIGH", "OppHigh", 11000, &[]))
        .hand(0, &["BT21-093"])
        .memory(20)
        .start();

    let low = runner.place_on_field(1, "OPP-LOW", None);
    let mid = runner.place_on_field(1, "OPP-MID", None);
    let high = runner.place_on_field(1, "OPP-HIGH", None);
    assert_eq!(runner.battle_area_size(1), 3, "3 opponent Digimon staged");

    runner.play(0, 0);

    // The [Main] highest-DP delete prompt must install as an OppField pick.
    let kind = runner
        .pending_kind()
        .expect("[Main] delete prompt must install with opponent Digimon present");
    assert_eq!(kind, SelectionKind::OppField);

    let view = runner
        .pending_selection_view()
        .expect("pending selection view");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the single highest-DP (11000) opponent Digimon is a legal delete target"
    );

    // Resolve the prompt — the only legal target is OPP-HIGH.
    let (action, player) = (view.valid_action_ids[0], view.selecting_player);
    runner
        .game
        .resolve_selection(player, action)
        .expect("delete resolves");

    assert_eq!(
        runner.battle_area_size(1),
        2,
        "exactly one opponent Digimon deleted"
    );
    // OPP-HIGH (the 11000-DP Digimon) is the one that left.
    let remaining_dp: Vec<i32> = (0..runner.battle_area_size(1))
        .filter_map(|i| {
            runner.game.effective_dp(PermanentHandle {
                player: 1,
                index: i as u8,
            })
        })
        .collect();
    assert!(
        !remaining_dp.contains(&11000),
        "the highest-DP (11000) Digimon must be the deleted one; remaining: {remaining_dp:?}"
    );
    let _ = (low, mid, high);
}

/// Tie-break: when two opponent Digimon share the highest DP, BOTH must be
/// exposed as valid delete targets (the choice surfaces to the action space —
/// no auto-selection).
#[test]
fn bt21_093_main_exposes_all_tied_highest_dp_targets() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-093")
        .expect("BT21-093 in embedded pack")
        .add_card(digimon("OPP-A", "OppA", 9000, &[]))
        .add_card(digimon("OPP-B", "OppB", 9000, &[]))
        .add_card(digimon("OPP-C", "OppC", 4000, &[]))
        .hand(0, &["BT21-093"])
        .memory(20)
        .start();

    runner.place_on_field(1, "OPP-A", None);
    runner.place_on_field(1, "OPP-B", None);
    runner.place_on_field(1, "OPP-C", None);

    runner.play(0, 0);

    let view = runner
        .pending_selection_view()
        .expect("[Main] delete prompt installs");
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both 9000-DP Digimon (tied highest) must be valid targets; the 4000-DP one excluded"
    );
}

/// Negative: with NO opponent Digimon the [Main] delete is a structural
/// no-op — no selection installs, and the play completes without panic.
#[test]
fn bt21_093_main_no_selection_when_opponent_has_no_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-093")
        .expect("BT21-093 in embedded pack")
        .hand(0, &["BT21-093"])
        .memory(20)
        .start();

    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon → highest-DP delete installs no selection"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// § 4  Clause 1 — "Then, place this card in the battle area"
// ─────────────────────────────────────────────────────────────────────────────

/// After the [Main] effect resolves, BT21-093 must be placed in the
/// controller's battle area as a Delay option permanent (implicit disposal
/// via the `kind: delay` marker — `classify_option_subtype` → Delay).
#[test]
fn bt21_093_main_places_self_in_battle_area_as_delay() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-093")
        .expect("BT21-093 in embedded pack")
        .hand(0, &["BT21-093"])
        .memory(20)
        .start();

    assert_eq!(runner.battle_area_size(0), 0, "battle area starts empty");
    runner.play(0, 0);
    // No opponent Digimon → no delete prompt → the Main effect drains and the
    // Option is disposed straight into the battle area as a Delay permanent.
    assert!(
        runner.pending_selection().is_none(),
        "play should fully resolve with no opponent Digimon"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "Raging Serpentine must be placed in its controller's battle area"
    );
    assert_eq!(runner.hand_size(0), 0, "the Option leaves hand once placed");
}

// ─────────────────────────────────────────────────────────────────────────────
// § 5  Clause 3 — [All Turns] Delay activation on opponent-security removal
// ─────────────────────────────────────────────────────────────────────────────

/// Place BT21-093 in P0's battle area as a Delay option, then fire the
/// `OnOpponentSecurityRemoved` observer. The Delay body must: (1) trash
/// BT21-093 (Delay activation cost), then (2) offer the optional free
/// digivolve of a Reptile/Dragonkin Digimon into a Reptile/Dragonkin hand
/// card. Driving the digivolve makes the chosen base evolve into the chosen
/// hand card without paying the cost.
#[test]
fn bt21_093_delay_activates_on_opp_security_removed() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-093")
        .expect("BT21-093 in embedded pack")
        .add_card(digimon("MY-REPTILE", "MyReptile", 4000, &["Reptile"]))
        .add_card(digimon(
            "EVO-DRAGONKIN",
            "EvoDragonkin",
            9000,
            &["Dragonkin"],
        ))
        .hand(0, &["EVO-DRAGONKIN"])
        .memory(20)
        .start();

    // BT21-093 on P0's field (the Delay option) + a Reptile base to digivolve.
    let option = runner.place_on_field(0, "BT21-093", Some(0));
    let base = runner.place_on_field(0, "MY-REPTILE", Some(0));
    assert_eq!(runner.battle_area_size(0), 2);
    let base_stack_before = runner
        .game
        .player(0)
        .battle_area
        .get(base.index as usize)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);
    assert_eq!(base_stack_before, 1, "base starts as a single card");

    // Fire the opponent-security-removal observer: P1 (the opponent) loses a
    // security card; P0's battle area is scanned for opponent-side observers.
    let observed_card = runner.game.player(1).security.first().map(|c| c.handle());
    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 1,
            observer_player: 0,
            source_player: 0,
            card: observed_card.unwrap_or(digimon_engine::card_source::CardHandle(0)),
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();

    // The <Delay> is declinable — an accept/decline pre-cost prompt installs
    // first (G-ACTIVATION-COST-TRASH-SELF). Accept it: that pays the
    // trash_self activation cost, then runs the body.
    let gate = runner
        .pending_selection_view()
        .expect("the <Delay> accept/decline prompt must install");
    assert!(
        gate.is_optional,
        "the <Delay> prompt is declinable (PASS legal, Comprehensive Rules 16-16-2)"
    );
    runner
        .game
        .resolve_selection(gate.selecting_player, gate.valid_action_ids[0])
        .expect("accept the <Delay>");

    // Accepting paid the trash_self cost: BT21-093 left the battle area into
    // the trash.
    assert_eq!(
        runner.trash_size(0),
        1,
        "accepting the <Delay> trashes BT21-093 (the activation cost)"
    );
    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "BT21-093"),
        "BT21-093 must no longer be a battle-area permanent after the <Delay> cost"
    );

    // The body then installs the optional digivolve-base selection.
    let kind = runner
        .pending_kind()
        .expect("Delay body must install the optional digivolve-base selection");
    assert_eq!(kind, SelectionKind::OwnField);
    assert!(
        runner.pending_is_optional(),
        "the digivolve sub-step is optional (\"1 of your Digimon MAY digivolve\")"
    );
    let _ = option;

    // The base Reptile is the only valid digivolve target.
    let view = runner.pending_selection_view().expect("pending view");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "the lone Reptile Digimon is the only digivolve base"
    );
    let (action, player) = (view.valid_action_ids[0], view.selecting_player);
    runner
        .game
        .resolve_selection(player, action)
        .expect("digivolve base selected");

    // Next: the hand-card selection (Reptile/Dragonkin Digimon to evolve into).
    let kind = runner
        .pending_kind()
        .expect("hand-card selection must follow the base pick");
    assert_eq!(kind, SelectionKind::Hand);
    let view = runner.pending_selection_view().expect("hand pending view");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "EvoDragonkin is the only Reptile/Dragonkin hand card"
    );
    let (action, player) = (view.valid_action_ids[0], view.selecting_player);
    runner
        .game
        .resolve_selection(player, action)
        .expect("evolution hand card selected");

    // The digivolve happened for free: the base now carries 2 cards in its
    // stack (the original Reptile + EvoDragonkin on top), and the hand card
    // was consumed.
    let base_after = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| {
            p.card_sources
                .iter()
                .any(|cs| cs.card_id(&runner.game.card_data) == "MY-REPTILE")
        })
        .expect("digivolved base permanent");
    assert_eq!(
        base_after.card_sources.len(),
        2,
        "base must have digivolved (EvoDragonkin stacked on the Reptile)"
    );
    assert_eq!(
        base_after.top_card().card_id(&runner.game.card_data),
        "EVO-DRAGONKIN",
        "the chosen hand card is now the top card"
    );
    assert_eq!(
        runner.hand_size(0),
        0,
        "the evolution hand card was consumed"
    );

    // The Delay activation observably mutated game state: BT21-093 left the
    // battle area into the trash (the mandatory Delay cost) and a digivolve
    // resolved without a cost being paid. The effect-context delete path does
    // not emit a discrete `GameEvent::Trash`, so the trash-as-cost evidence is
    // the trash/battle-area state asserted above.
}

/// Once the <Delay> is accepted (trash-self cost paid) the digivolve sub-step
/// is still independently optional: declining the base selection (PASS)
/// leaves BT21-093 trashed but performs no digivolve and consumes no hand card.
#[test]
fn bt21_093_delay_digivolve_is_optional_decline_keeps_trash() {
    use digimon_engine::action::space::PASS;

    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-093")
        .expect("BT21-093 in embedded pack")
        .add_card(digimon("MY-REPTILE", "MyReptile", 4000, &["Reptile"]))
        .add_card(digimon(
            "EVO-DRAGONKIN",
            "EvoDragonkin",
            9000,
            &["Dragonkin"],
        ))
        .hand(0, &["EVO-DRAGONKIN"])
        .memory(20)
        .start();

    runner.place_on_field(0, "BT21-093", Some(0));
    runner.place_on_field(0, "MY-REPTILE", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 1,
            observer_player: 0,
            source_player: 0,
            card: digimon_engine::card_source::CardHandle(0),
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();

    // Accept the <Delay> — pays the trash_self activation cost.
    let gate = runner
        .pending_selection_view()
        .expect("the <Delay> accept/decline prompt installs");
    runner
        .game
        .resolve_selection(gate.selecting_player, gate.valid_action_ids[0])
        .expect("accept the <Delay>");
    assert_eq!(
        runner.trash_size(0),
        1,
        "accepting the <Delay> trashes BT21-093 (the activation cost)"
    );

    // The digivolve base pick is independently optional — decline it (PASS).
    assert!(
        runner.pending_is_optional(),
        "the digivolve base pick must be optional"
    );
    let player = runner
        .pending_selection_view()
        .expect("pending view")
        .selecting_player;
    runner
        .game
        .resolve_selection(player, PASS)
        .expect("decline digivolve");

    // The trash-self cost was already paid; declining the digivolve keeps it.
    assert_eq!(
        runner.trash_size(0),
        1,
        "BT21-093 stays trashed when the digivolve sub-step is declined"
    );
    // No digivolve: the hand card is untouched, no further selection installs.
    assert!(
        runner.pending_selection().is_none(),
        "declining the base pick skips the hand selection entirely"
    );
    assert_eq!(
        runner.hand_size(0),
        1,
        "the evolution hand card is NOT consumed when the digivolve is declined"
    );
    // The base Reptile is unchanged (single-card stack).
    let base = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "MY-REPTILE")
        .expect("base still on field");
    assert_eq!(
        base.card_sources.len(),
        1,
        "the base Reptile did not digivolve"
    );
}

/// G-ACTIVATION-COST-TRASH-SELF: the <Delay> itself is declinable. Declining
/// the accept/decline prompt does NOT pay the trash-self cost — BT21-093 stays
/// on the field as a Delay option and no digivolve sub-flow runs. (Comprehensive
/// Rules 16-16-2: the processing of a <Delay> is optional.)
#[test]
fn bt21_093_delay_can_be_declined() {
    use digimon_engine::action::space::PASS;

    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-093")
        .expect("BT21-093 in embedded pack")
        .add_card(digimon("MY-REPTILE", "MyReptile", 4000, &["Reptile"]))
        .add_card(digimon(
            "EVO-DRAGONKIN",
            "EvoDragonkin",
            9000,
            &["Dragonkin"],
        ))
        .hand(0, &["EVO-DRAGONKIN"])
        .memory(20)
        .start();

    runner.place_on_field(0, "BT21-093", Some(0));
    runner.place_on_field(0, "MY-REPTILE", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 1,
            observer_player: 0,
            source_player: 0,
            card: digimon_engine::card_source::CardHandle(0),
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();

    // The <Delay> accept/decline prompt installs — decline it (PASS).
    let gate = runner
        .pending_selection_view()
        .expect("the <Delay> accept/decline prompt must install");
    assert!(gate.is_optional, "the <Delay> prompt is declinable");
    runner
        .game
        .resolve_selection(gate.selecting_player, PASS)
        .expect("decline the <Delay>");

    // Declined: the trash-self cost was NOT paid; BT21-093 stays on the field.
    assert_eq!(
        runner.trash_size(0),
        0,
        "declining the <Delay> must NOT trash BT21-093 (the cost is declinable)"
    );
    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT21-093"),
        "BT21-093 stays on the field as a Delay option when the <Delay> is declined"
    );
    assert!(
        runner.pending_selection().is_none(),
        "declining the <Delay> skips the whole clause — no digivolve sub-flow"
    );
    assert_eq!(
        runner.hand_size(0),
        1,
        "the evolution hand card is untouched when the <Delay> is declined"
    );
}

/// Negative: with NO Reptile/Dragonkin Digimon on the controller's field the
/// Delay does NOT activate — its clause condition (DCGO `CanActivateCondition`
/// → `HasMatchConditionOwnersPermanent`) fails, so BT21-093 is NOT trashed and
/// no selection installs.
#[test]
fn bt21_093_delay_does_not_activate_without_reptile_or_dragonkin_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-093")
        .expect("BT21-093 in embedded pack")
        .add_card(digimon("MY-PLAIN", "MyPlain", 4000, &[]))
        .hand(0, &["BT21-093"])
        .memory(20)
        .start();

    runner.place_on_field(0, "BT21-093", Some(0));
    // A non-Reptile/Dragonkin Digimon — not a valid digivolve base, so the
    // Delay's CanActivateCondition gate fails.
    runner.place_on_field(0, "MY-PLAIN", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 1,
            observer_player: 0,
            source_player: 0,
            card: digimon_engine::card_source::CardHandle(0),
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.trash_size(0),
        0,
        "Delay must NOT activate (no trash) when no Reptile/Dragonkin Digimon exists"
    );
    assert_eq!(
        runner.battle_area_size(0),
        2,
        "BT21-093 stays on field as a Delay option when the Delay does not activate"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no Reptile/Dragonkin Digimon → the Delay does not activate, no selection installs"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// § 6  Clause 4 — inherited [Security] highest-DP delete (behavioral)
// ─────────────────────────────────────────────────────────────────────────────

/// The inherited [Security] clause deletes the opponent's highest-DP Digimon.
/// Driven clause-isolated through the effect queue with an `OnSecurity`
/// trigger so the same `select_opponent_permanent` (selector: highest_dp) +
/// `delete_permanent` body runs as for [Main].
#[test]
fn bt21_093_inherited_security_deletes_highest_dp_opponent_digimon() {
    // P1 plays BT21-093 as a battle-area Option permanent so its inherited
    // security body can be exercised through the OnSecurity timing; the
    // opponent of the BT21-093 controller (P0) holds the Digimon to delete.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-093")
        .expect("BT21-093 in embedded pack")
        .add_card(digimon("FOE-LOW", "FoeLow", 2000, &[]))
        .add_card(digimon("FOE-HIGH", "FoeHigh", 12000, &[]))
        .memory(20)
        .start();

    // BT21-093 carrier on P1's field; P0 holds the opponent Digimon.
    let carrier = runner.place_on_field(1, "BT21-093", Some(0));
    runner.place_on_field(0, "FOE-LOW", None);
    runner.place_on_field(0, "FOE-HIGH", None);
    assert_eq!(runner.battle_area_size(0), 2, "2 opponent Digimon staged");

    // Fire the inherited [Security] body from the carrier.
    // DSL `on_security` lowers to `EffectTiming::SecuritySkill`.
    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    // Highest-DP selection installs against P1's opponent (P0).
    let view = runner
        .pending_selection_view()
        .expect("[Security] highest-DP delete prompt installs");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the single highest-DP (12000) Digimon is a legal [Security] delete target"
    );
    let (action, player) = (view.valid_action_ids[0], view.selecting_player);
    runner
        .game
        .resolve_selection(player, action)
        .expect("[Security] delete resolves");

    assert_eq!(
        runner.battle_area_size(0),
        1,
        "exactly one opponent Digimon deleted by the [Security] effect"
    );
    let remaining_dp: Vec<i32> = (0..runner.battle_area_size(0))
        .filter_map(|i| {
            runner.game.effective_dp(PermanentHandle {
                player: 0,
                index: i as u8,
            })
        })
        .collect();
    assert!(
        !remaining_dp.contains(&12000),
        "[Security] must delete the highest-DP (12000) Digimon; remaining: {remaining_dp:?}"
    );
}
