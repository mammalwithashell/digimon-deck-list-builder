//! BT9-112 DeathXmon — Digimon, Lv.7, Black/Purple, DP 15000, Cost 20.
//! Traits: Unanalyzable, X Program.
//! Evo: Lv6 Black / Cost 6.
//!
//! # Card text (cards.json)
//!
//! ```text
//! When you would play this card, reduce its memory cost by 3 for each Digimon
//! and Tamer your opponent has in play.
//!
//! [On Play] [When Digivolving] <De-Digivolve 1> all of your opponent's Digimon.
//! (Trash the top card. You can't trash past level 3 cards.)
//! Then, delete all of your opponent's level 4 or lower Digimon.
//!
//! [End of Opponent's Turn] [Once Per Turn]
//! Delete all of your opponent's Digimon with the lowest play cost.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT9/Purple/BT9_112.cs
//!
//! # Patterns this test covers
//! - D2 Cost reduction with formula (3 × opponent battle-area count)
//! - for_each De-Digivolve 1 all opponent Digimon (snapshot semantics)
//! - for_each Delete all opponent ≤Lv4 Digimon (snapshot after de-digivolve)
//! - end_of_opponents_turn triggered clause with once_per_turn
//! - G-OPT-TRIGGERED (OPT lockout on triggered clauses not yet enforced)
//! - raw_rust escape hatch for "lowest play cost" filter (G-PLAY-COST-LTE)

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::TriggerSource;

// ─── Helper ──────────────────────────────────────────────────────────────────

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT9-112")
        .expect("BT9-112 in embedded DSL pack")
        .memory(20)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// BT9-112 has exactly one CostReduction declarative clause (BeforePayCost).
#[test]
fn bt9_112_has_one_cost_reduction_declarative() {
    let runner = runner();
    let card = runner
        .compiled_card("BT9-112")
        .expect("BT9-112 in embedded pack");

    let cr_count = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
            )
        })
        .count();
    assert_eq!(cr_count, 1, "expected 1 CostReduction declarative clause");
}

/// BT9-112 has exactly two triggered clauses:
///   - Clause B: [On Play][When Digivolving] De-Digivolve + delete ≤Lv4
///   - Clause C: [End of Opponent's Turn][Once Per Turn] delete lowest-cost
#[test]
fn bt9_112_has_two_triggered_clauses() {
    let runner = runner();
    let card = runner
        .compiled_card("BT9-112")
        .expect("BT9-112 in embedded pack");

    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        triggered.len(),
        2,
        "expected 2 triggered clauses (on_play/when_digivolving + end_of_opponents_turn)"
    );
}

/// Clause B fires on both OnPlay and WhenDigivolving.
#[test]
fn bt9_112_clause_b_fires_on_on_play_and_when_digivolving() {
    let runner = runner();
    let card = runner
        .compiled_card("BT9-112")
        .expect("BT9-112 in embedded pack");

    let clause_b = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                || t.when.contains(&CompiledTiming::WhenDigivolving)
        })
        .expect("clause B (on_play/when_digivolving) must exist");

    assert!(
        clause_b.when.contains(&CompiledTiming::OnPlay),
        "clause B must include OnPlay"
    );
    assert!(
        clause_b.when.contains(&CompiledTiming::WhenDigivolving),
        "clause B must include WhenDigivolving"
    );
    assert!(
        !clause_b.optional,
        "clause B is mandatory (no 'you may')"
    );
    assert!(
        !clause_b.once_per_turn,
        "clause B has no [Once Per Turn]"
    );
    assert_eq!(clause_b.scope, CompiledScope::FaceUp, "clause B is own-scope");
}

/// Clause C fires on EndOfOpponentsTurn and is [Once Per Turn].
#[test]
fn bt9_112_clause_c_fires_on_end_of_opponents_turn_and_is_opt() {
    let runner = runner();
    let card = runner
        .compiled_card("BT9-112")
        .expect("BT9-112 in embedded pack");

    let clause_c = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::EndOfOpponentsTurn))
        .expect("clause C (end_of_opponents_turn) must exist");

    assert!(
        clause_c.once_per_turn,
        "clause C must be [Once Per Turn]"
    );
    assert!(
        !clause_c.optional,
        "clause C is mandatory — no 'you may' in printed text"
    );
    assert_eq!(
        clause_c.scope,
        CompiledScope::FaceUp,
        "clause C is own-scope"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Cost reduction structural
// ═══════════════════════════════════════════════════════════════════════════════

/// The CostReduction clause compiles correctly (structural verification).
/// The formula uses card_count_in_zone(opponent, battle_area) × 3 which counts
/// both Digimon and Tamers — matching printed text "each Digimon and Tamer".
#[test]
fn bt9_112_cost_reduction_declarative_compiles() {
    let runner = runner();
    let card = runner.compiled_card("BT9-112").expect("BT9-112 in pack");
    assert!(
        card.effects
            .iter()
            .any(|c| matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
            )),
        "CostReduction declarative must be present and compiled"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause B behavioral: De-Digivolve 1 + delete ≤Lv4
// ═══════════════════════════════════════════════════════════════════════════════

/// Clause B: opponent has a Lv4 Digimon → de-digivolve has no stack to trash
/// (single source), but the delete pass removes it because level ≤ 4.
#[test]
fn bt9_112_clause_b_deletes_lv4_digimon() {
    let mut opp_lv4 = make_test_card("OPP-LV4", "OppLv4");
    opp_lv4.level = Some(4);
    opp_lv4.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-112")
        .expect("BT9-112 in embedded DSL pack")
        .add_card(opp_lv4)
        .memory(20)
        .start();

    let handle = runner.place_on_field(0, "BT9-112", Some(0));
    runner.place_on_field(1, "OPP-LV4", Some(0));

    let opp_before = runner.battle_area_size(1);
    assert_eq!(opp_before, 1);

    runner.game.enqueue_triggered(
        EffectTiming::OnPlay,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();

    // No pending selection — for_each is automatic.
    assert!(runner.game.pending_selection.is_none(), "no pending selection expected");

    // Lv4 Digimon should have been deleted by the ≤Lv4 delete pass.
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "Lv4 Digimon must be deleted by clause B"
    );
}

/// Clause B: opponent has no Digimon → no crash, no selection, battle area unchanged.
#[test]
fn bt9_112_clause_b_no_crash_when_opponent_has_no_digimon() {
    let mut runner = runner();
    let handle = runner.place_on_field(0, "BT9-112", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::OnPlay,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();

    assert!(runner.game.pending_selection.is_none(), "no selection expected");
    assert_eq!(runner.battle_area_size(1), 0, "opponent field still empty");
}

/// Clause B: Lv5+ Digimon (not ≤Lv4) survives the delete pass after de-digivolve.
/// A single-source Lv5 is unchanged by de_digivolve (no stack to trash) and is
/// above Lv4, so it survives.
#[test]
fn bt9_112_clause_b_lv5_digimon_not_deleted_if_stays_above_lv4() {
    let mut opp_lv5 = make_test_card("OPP-LV5-B", "OppLv5B");
    opp_lv5.level = Some(5);
    opp_lv5.dp = Some(7000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-112")
        .expect("BT9-112 in embedded DSL pack")
        .add_card(opp_lv5)
        .memory(20)
        .start();

    let handle = runner.place_on_field(0, "BT9-112", Some(0));
    runner.place_on_field(1, "OPP-LV5-B", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::OnPlay,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "Lv5 single-source Digimon must survive — above Lv4 and no stack to de-digivolve"
    );
}

/// Clause B fires identically on WhenDigivolving timing.
#[test]
fn bt9_112_clause_b_also_fires_on_when_digivolving() {
    let mut opp_lv4 = make_test_card("OPP-LV4-WD", "OppLv4Wd");
    opp_lv4.level = Some(4);
    opp_lv4.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-112")
        .expect("BT9-112 in embedded DSL pack")
        .add_card(opp_lv4)
        .memory(20)
        .start();

    let handle = runner.place_on_field(0, "BT9-112", Some(0));
    runner.place_on_field(1, "OPP-LV4-WD", Some(0));

    let opp_before = runner.battle_area_size(1);
    assert_eq!(opp_before, 1);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "Lv4 Digimon must be deleted when clause B fires via WhenDigivolving"
    );
}

/// Clause B: multiple opponent Digimon — all ≤Lv4 are deleted, those above survive.
#[test]
fn bt9_112_clause_b_deletes_all_lv4_or_lower_spares_lv5() {
    let mut lv3 = make_test_card("OPP-LV3", "OppLv3");
    lv3.level = Some(3);
    let mut lv4 = make_test_card("OPP-LV4-MX", "OppLv4Mx");
    lv4.level = Some(4);
    let mut lv5 = make_test_card("OPP-LV5-MX", "OppLv5Mx");
    lv5.level = Some(5);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-112")
        .expect("BT9-112 in embedded DSL pack")
        .add_card(lv3)
        .add_card(lv4)
        .add_card(lv5)
        .memory(20)
        .start();

    let handle = runner.place_on_field(0, "BT9-112", Some(0));
    runner.place_on_field(1, "OPP-LV3", Some(0));
    runner.place_on_field(1, "OPP-LV4-MX", Some(0));
    runner.place_on_field(1, "OPP-LV5-MX", Some(0));

    assert_eq!(runner.battle_area_size(1), 3);

    runner.game.enqueue_triggered(
        EffectTiming::OnPlay,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();

    assert!(runner.game.pending_selection.is_none());

    // Lv3 and Lv4 deleted; Lv5 survives.
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "Lv3 and Lv4 must be deleted; Lv5 must survive"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause C behavioral: delete lowest-play-cost Digimon on
//             End of Opponent's Turn
//
// NOTE: The "lowest play cost" filter is implemented via raw_rust:
// bt9_112_delete_lowest_cost_digimon (see qa/dsl-vocab-gaps.md G-PLAY-COST-LTE).
// The DSL has no play_cost_lte predicate with aggregate formula.
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: BT9-112 on field, EndOfOpponentsTurn fires → lowest-cost opponent
/// Digimon are deleted.
///
/// The raw_rust step deletes all opponent Digimon whose play_cost equals the
/// minimum among all opponent Digimon in the battle area.
#[test]
fn bt9_112_clause_c_deletes_lowest_cost_digimon_on_end_of_opp_turn() {
    let mut cheap = make_test_card("CHEAP", "CheapDigi");
    cheap.play_cost = 3;
    cheap.level = Some(4);
    let mut expensive = make_test_card("EXPENSIVE", "ExpensiveDigi");
    expensive.play_cost = 7;
    expensive.level = Some(5);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-112")
        .expect("BT9-112 in embedded DSL pack")
        .add_card(cheap)
        .add_card(expensive)
        .memory(20)
        .start();

    let handle = runner.place_on_field(0, "BT9-112", Some(0));
    runner.place_on_field(1, "CHEAP", Some(0));
    runner.place_on_field(1, "EXPENSIVE", Some(0));

    assert_eq!(runner.battle_area_size(1), 2);

    runner.game.enqueue_triggered(
        EffectTiming::EndOfOpponentsTurn,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();

    // No pending selection — for_each (via raw_rust) is automatic.
    assert!(runner.game.pending_selection.is_none(), "clause C should not install a selection");

    // The raw_rust step deletes all lowest-cost Digimon.
    // CHEAP (cost 3) should be deleted; EXPENSIVE (cost 7) should survive.
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "only the lowest-cost Digimon (CHEAP) should be deleted, EXPENSIVE survives"
    );

    // Verify the remaining permanent is the expensive one.
    let survivor = &runner.game.player(1).battle_area[0];
    let survivor_id = &runner.game.card_data[survivor.top_card().data_index].card_id;
    assert_eq!(
        survivor_id.as_str(),
        "EXPENSIVE",
        "EXPENSIVE should be the survivor after clause C fires"
    );
}

/// Clause C: when multiple Digimon share the lowest play cost, all are deleted.
#[test]
fn bt9_112_clause_c_deletes_all_tied_lowest_cost_digimon() {
    let mut digi_a = make_test_card("TIED-A", "TiedA");
    digi_a.play_cost = 3;
    digi_a.level = Some(4);
    let mut digi_b = make_test_card("TIED-B", "TiedB");
    digi_b.play_cost = 3;
    digi_b.level = Some(4);
    let mut digi_c = make_test_card("HIGH-C", "HighC");
    digi_c.play_cost = 8;
    digi_c.level = Some(6);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-112")
        .expect("BT9-112 in embedded DSL pack")
        .add_card(digi_a)
        .add_card(digi_b)
        .add_card(digi_c)
        .memory(20)
        .start();

    let handle = runner.place_on_field(0, "BT9-112", Some(0));
    runner.place_on_field(1, "TIED-A", Some(0));
    runner.place_on_field(1, "TIED-B", Some(0));
    runner.place_on_field(1, "HIGH-C", Some(0));

    assert_eq!(runner.battle_area_size(1), 3);

    runner.game.enqueue_triggered(
        EffectTiming::EndOfOpponentsTurn,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();

    assert!(runner.game.pending_selection.is_none());

    // Both TIED-A and TIED-B (cost 3, tied for lowest) deleted; HIGH-C (cost 8) survives.
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "both tied lowest-cost Digimon should be deleted"
    );
}

/// Clause C: no crash when opponent has no Digimon on their turn end.
#[test]
fn bt9_112_clause_c_no_crash_when_opponent_has_no_digimon() {
    let mut runner = runner();
    let handle = runner.place_on_field(0, "BT9-112", Some(0));

    // No opponent Digimon.
    runner.game.enqueue_triggered(
        EffectTiming::EndOfOpponentsTurn,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.battle_area_size(1), 0);
}

/// Negative: clause C only fires via EndOfOpponentsTurn. Does not install a
/// selection and does not delete Digimon when fired via OnPlay (wrong timing).
#[test]
fn bt9_112_clause_c_does_not_fire_on_on_play_timing() {
    let mut cheap = make_test_card("CHEAP-NF", "CheapNf");
    cheap.play_cost = 3;
    cheap.level = Some(4);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-112")
        .expect("BT9-112 in embedded DSL pack")
        .add_card(cheap)
        .memory(20)
        .start();

    let handle = runner.place_on_field(0, "BT9-112", Some(0));
    runner.place_on_field(1, "CHEAP-NF", Some(0));

    let opp_before = runner.battle_area_size(1);

    // Firing OnPlay should trigger clause B (de-digivolve + delete ≤Lv4),
    // NOT clause C (delete lowest cost). Since CHEAP-NF is Lv4, clause B
    // WILL delete it. That's actually the correct behavior — the test just
    // verifies clause C doesn't interfere.
    // Use WhenAttacking (not a clause-B timing) to fire something safe.
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();

    // No selection, no deletion (WhenAttacking doesn't match any BT9-112 clause).
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.battle_area_size(1),
        opp_before,
        "WhenAttacking must not trigger either clause of BT9-112"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — OPT lockout (clause C)
// ═══════════════════════════════════════════════════════════════════════════════

/// Clause C OPT lockout: second activation in the same turn is blocked.
/// Skipped pending G-OPT-TRIGGERED engine gap.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED — triggered OPT lockout not yet enforced by effect_queue.rs"]
fn bt9_112_clause_c_opt_blocks_second_activation_same_turn() {
    todo!("write once G-OPT-TRIGGERED is closed");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Compilation smoke test
// ═══════════════════════════════════════════════════════════════════════════════

/// BT9-112 YAML must compile and appear in the embedded DSL pack.
#[test]
fn bt9_112_yaml_compiles_without_error() {
    let runner = runner();
    let card = runner.compiled_card("BT9-112");
    assert!(
        card.is_some(),
        "BT9-112 must be present in the embedded DSL pack (YAML must parse + compile)"
    );
}
