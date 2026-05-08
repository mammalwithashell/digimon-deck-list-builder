//! EX4-073 Omnimon Alter-B — Digimon, Lv.7, Black, DP 15000, Cost 15.
//! Traits: Holy Warrior. Attribute: Virus.
//! Evo: Lv6 Black / Cost 5. Special evo: Lv7 w/[Omnimon] in name / Cost 2.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon.
//!   (Trash up to 3 cards from the top. You can't trash past level 3 cards.)
//!   Then, delete up to 6 play cost's total worth of their Digimon.
//! [When Attacking] By trashing up to 3 level 6 or higher cards in this
//!   Digimon's digivolution cards, for each card trashed, activate the effect
//!   below. Then, if you trashed 3 cards, trash the top 2 cards of your
//!   opponent's security stack.
//!   - Delete 1 of your opponent's Digimon or Tamers with the lowest play cost.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX4/Black/EX4_073.cs
//!
//! # Patterns this test covers
//! - Two alt-paths (standard digivolve + Omnimon-name special digivolve at cost 2)
//! - [When Digivolving] arm 1: select_opponent_permanent + de_digivolve(3, stop_at:3)
//! - [When Digivolving] arm 2: BLOCKED — G-MULTI-SELECT-OPP-PLAY-COST-SUM
//!   (no play_cost-budget multi-select in DSL or engine; only DP-budget exists)
//! - [When Attacking] entire clause: BLOCKED — G-DSL-SELECT-OWN-SOURCES-FILTER
//!   (no `filter:` field on `select_own_sources`) AND G-PLAY-COST-AGGREGATE
//!   (no `lowest_play_cost` predicate; sibling of literal P-189 G-PLAY-COST-LTE)

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::TriggerSource;

// ─── Helper ──────────────────────────────────────────────────────────────────

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX4-073")
        .expect("EX4-073 in embedded DSL pack")
        .memory(20)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Compilation smoke test
// ═══════════════════════════════════════════════════════════════════════════════

/// EX4-073 YAML must compile and appear in the embedded DSL pack.
#[test]
fn ex4_073_yaml_compiles_without_error() {
    let runner = runner();
    let card = runner.compiled_card("EX4-073");
    assert!(
        card.is_some(),
        "EX4-073 must be present in the embedded DSL pack (YAML must parse + compile)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// EX4-073 has exactly one triggered clause authored today (clause B partial —
/// arm 1 only). Clause C ([When Attacking]) is OMITTED entirely under blocking
/// gaps; arm 2 of clause B (delete-up-to-6-cost) is OMITTED under
/// G-MULTI-SELECT-OPP-PLAY-COST-SUM.
///
/// When G-MULTI-SELECT-OPP-PLAY-COST-SUM closes, arm 2 is appended to the
/// existing when_digivolving clause (still 1 triggered clause).
/// When the [When Attacking] gaps close, a second triggered clause is added
/// (then 2 triggered clauses).
#[test]
fn ex4_073_has_one_triggered_clause_authored() {
    let runner = runner();
    let card = runner
        .compiled_card("EX4-073")
        .expect("EX4-073 in embedded pack");

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
        1,
        "expected 1 triggered clause authored today (clause B arm 1 only); \
         clause B arm 2 + clause C are BLOCKED — see YAML header comments"
    );
}

/// The single authored triggered clause fires on WhenDigivolving and is
/// mandatory (no "you may"), face-up scope, no [Once Per Turn].
#[test]
fn ex4_073_clause_b_fires_on_when_digivolving_mandatory_face_up() {
    let runner = runner();
    let card = runner
        .compiled_card("EX4-073")
        .expect("EX4-073 in embedded pack");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("clause B must include WhenDigivolving");

    assert!(
        !clause.optional,
        "clause B is mandatory — printed text has no 'you may' on the outer trigger"
    );
    assert!(!clause.once_per_turn, "clause B has no [Once Per Turn]");
    assert_eq!(clause.scope, CompiledScope::FaceUp, "clause B is own-scope");
}

/// EX4-073 has two alt-paths: standard digivolve (Lv6 Black / 5) and
/// Omnimon-name special digivolve (Lv7 / 2).
#[test]
fn ex4_073_has_two_alt_paths() {
    let runner = runner();
    let card = runner
        .compiled_card("EX4-073")
        .expect("EX4-073 in embedded pack");

    assert_eq!(
        card.alt_paths.len(),
        2,
        "EX4-073 has standard digivolve (Lv6 Black) + Omnimon-name special (Lv7) = 2 alt-paths"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause B arm 1 behavioral: <De-Digivolve 3> on 1 opp Digimon
// ═══════════════════════════════════════════════════════════════════════════════

/// Clause B arm 1: opponent has 1 Lv5 Digimon with stacked sources →
/// fire WhenDigivolving → selection is installed for the de-digivolve target.
/// (No multi-pick budget — clause B arm 2 is BLOCKED.)
#[test]
fn ex4_073_clause_b_installs_selection_for_de_digivolve_target() {
    let mut opp_lv5 = make_test_card("OPP-LV5", "OppLv5");
    opp_lv5.level = Some(5);
    opp_lv5.dp = Some(7000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX4-073")
        .expect("EX4-073 in embedded DSL pack")
        .add_card(opp_lv5)
        .memory(20)
        .start();

    let handle = runner.place_on_field(0, "EX4-073", Some(0));
    runner.place_on_field(1, "OPP-LV5", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();

    // A pending selection must be installed for the de-digivolve pick.
    assert!(
        runner.game.pending_selection.is_some(),
        "clause B arm 1 must install a pending selection for the de-digivolve target"
    );
}

/// Clause B arm 1: opponent has no Digimon → outer condition gate prevents
/// firing; no selection installed.
#[test]
fn ex4_073_clause_b_no_fire_when_opponent_has_no_digimon() {
    let mut runner = runner();
    let handle = runner.place_on_field(0, "EX4-073", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "no opponent Digimon → outer existential condition fails → no selection"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause B arm 2 (BLOCKED): delete up to 6 play-cost worth
// ═══════════════════════════════════════════════════════════════════════════════

/// Clause B arm 2 — "delete up to 6 play cost's total worth of their Digimon".
///
/// BLOCKED on G-MULTI-SELECT-OPP-PLAY-COST-SUM: the engine has only the
/// DP-budget multi-select primitive (`select_opponent_permanents_by_dp_budget`)
/// and the matching DSL step `select_opponent_dp_budget`. There is no
/// play-cost-budget sibling. Authored as #[ignore] until the gap closes;
/// when it does, the test exercises a multi-select that:
///   - lets the player pick 0..N opp Digimon,
///   - enforces running play_cost sum ≤ 6 after each pick,
///   - filters out any single Digimon whose individual play_cost > 6,
///   - deletes all picked permanents on resolution.
#[test]
#[ignore = "pending: G-MULTI-SELECT-OPP-PLAY-COST-SUM — no play-cost-budget multi-select API in engine or DSL (only DpBudget exists)"]
fn ex4_073_clause_b_arm2_deletes_opp_digimon_within_play_cost_sum_6() {
    todo!("write once G-MULTI-SELECT-OPP-PLAY-COST-SUM is closed");
}

/// Clause B arm 2: a single opp Digimon with play_cost > 6 must be ineligible
/// (individual cap, mirrors DCGO `permanent.TopCard.GetCostItself <= 6`).
/// BLOCKED — same gap.
#[test]
#[ignore = "pending: G-MULTI-SELECT-OPP-PLAY-COST-SUM — per-card cap requires the play-cost-budget primitive's candidate filter"]
fn ex4_073_clause_b_arm2_excludes_opp_digimon_with_play_cost_above_6() {
    todo!("write once G-MULTI-SELECT-OPP-PLAY-COST-SUM is closed");
}

/// Clause B arm 2: when no opp Digimon are eligible (all cost > 6 or none on
/// field), the arm silently no-ops and arm 1 still resolved. BLOCKED — same gap.
#[test]
#[ignore = "pending: G-MULTI-SELECT-OPP-PLAY-COST-SUM — silent no-op behavior depends on the primitive's existential gate"]
fn ex4_073_clause_b_arm2_silent_noop_when_no_eligible_targets() {
    todo!("write once G-MULTI-SELECT-OPP-PLAY-COST-SUM is closed");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Clause C (BLOCKED): [When Attacking] trash sources + lowest-cost
// ═══════════════════════════════════════════════════════════════════════════════

/// Clause C outer step — "trash up to 3 level 6 or higher cards in this
/// Digimon's digivolution cards" — BLOCKED on G-DSL-SELECT-OWN-SOURCES-FILTER:
/// `select_own_sources` exists (added when G-ROCKS-SOURCE-SELECTION-DSL closed)
/// but exposes no `filter:` field, so the level_gte: 6 per-source filter is
/// unrepresentable. The lowering at selections.rs:1330 passes
/// `|_game, _source| true` (accept-all).
#[test]
#[ignore = "pending: G-DSL-SELECT-OWN-SOURCES-FILTER — select_own_sources lacks a filter field; level_gte: 6 per-source filter is unrepresentable"]
fn ex4_073_clause_c_trashes_up_to_3_lv6_or_higher_sources() {
    todo!("write once G-DSL-SELECT-OWN-SOURCES-FILTER is closed");
}

/// Clause C inner per-trash effect — "Delete 1 of your opponent's Digimon or
/// Tamers with the lowest play cost." BLOCKED on G-PLAY-COST-AGGREGATE: the
/// DSL has the LITERAL `play_cost_lte` form (closed by P-189 on 2026-05-01)
/// but no aggregate `lowest_play_cost: bool` predicate analogous to
/// `lowest_dp` / `highest_dp` / `lowest_level`. BT9-112 hit the same gap and
/// uses a per-card raw_rust escape hatch; new authorings should not extend
/// that pattern.
#[test]
#[ignore = "pending: G-PLAY-COST-AGGREGATE — no `lowest_play_cost` predicate (sibling of `lowest_dp`); BT9-112 raw_rust hatch is grandfathered, not extended"]
fn ex4_073_clause_c_inner_deletes_opp_lowest_play_cost_per_trash() {
    todo!("write once G-PLAY-COST-AGGREGATE is closed");
}

/// Clause C tail — "if you trashed 3 cards, trash the top 2 cards of your
/// opponent's security stack" — depends on a count binding from the blocked
/// outer source-trash step, so it's transitively BLOCKED.
#[test]
#[ignore = "pending: G-DSL-SELECT-OWN-SOURCES-FILTER (parent gap) — count binding from source-trash step is unavailable while the outer step itself is blocked"]
fn ex4_073_clause_c_trashes_2_opp_security_when_3_sources_trashed() {
    todo!("write once G-DSL-SELECT-OWN-SOURCES-FILTER closes (and a count binding from select_own_sources is plumbed)");
}

/// Clause C is OPTIONAL ("By trashing ... up to 3" — DCGO isOptional: true,
/// canNoSelect: () => true on the source-select). When the gaps close, the
/// player should be able to pass on the entire clause. BLOCKED.
#[test]
#[ignore = "pending: G-DSL-SELECT-OWN-SOURCES-FILTER — clause body cannot be authored until source-select filter is wired"]
fn ex4_073_clause_c_player_may_decline_to_trash_any_sources() {
    todo!("write once G-DSL-SELECT-OWN-SOURCES-FILTER closes");
}
