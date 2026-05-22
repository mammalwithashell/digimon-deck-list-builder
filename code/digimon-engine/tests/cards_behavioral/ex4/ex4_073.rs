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
//! - [When Attacking] entire clause: BLOCKED — G-PLAY-COST-AGGREGATE
//!   (no `lowest_play_cost` predicate in DSL; sibling of literal P-189
//!   G-PLAY-COST-LTE). G-DSL-SELECT-OWN-SOURCES-FILTER was closed 2026-05-08
//!   (select_own_sources now accepts filter: + evaluates against source cards),
//!   so the outer source-trash step is no longer blocked; only the inner
//!   per-trash delete (lowest_play_cost aggregate) and the count-binding
//!   for the conditional security-trash remain blocked on G-PLAY-COST-AGGREGATE.

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::{SelectionKind, TriggerSource};

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

/// Clause B arm 1: the installed selection must be `SelectionKind::OppField`
/// and mandatory (`is_optional == false`).
///
/// DCGO: `canNoSelect: false` — printed text has no "you may" on the outer
/// select; if a legal target exists the player must pick one.
#[test]
fn ex4_073_clause_b_selection_is_oppfield_and_mandatory() {
    let mut opp_lv5 = make_test_card("OPP-LV5B", "OppLv5B");
    opp_lv5.level = Some(5);
    opp_lv5.dp = Some(7000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX4-073")
        .expect("EX4-073 in embedded DSL pack")
        .add_card(opp_lv5)
        .memory(20)
        .start();

    let handle = runner.place_on_field(0, "EX4-073", Some(0));
    runner.place_on_field(1, "OPP-LV5B", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();

    let kind = runner
        .pending_kind()
        .expect("a pending selection must be installed when opp has a Digimon");
    assert_eq!(
        kind,
        SelectionKind::OppField,
        "select_opponent_permanent lowers to SelectionKind::OppField"
    );
    assert!(
        !runner.pending_is_optional(),
        "De-Digivolve target selection must be mandatory (DCGO canNoSelect: false)"
    );
}

/// Clause B arm 1 behavioral outcome: resolving the de-digivolve target
/// trashes up to 3 cards from the top of the selected Digimon's stack.
///
/// Setup: opponent has a Lv6 Digimon with 3 sources stacked below it
/// (total 4 cards in stack). After de_digivolve(3, stop_at:3), the
/// stack should have been reduced by 3 (to 1 base card, since all
/// intermediate sources are above the Lv3 floor).
///
/// The Lv3 base card itself is never trashed (stop_at_level: 3 prevents
/// popping when the NEXT top would be below Lv3).
#[test]
fn ex4_073_clause_b_de_digivolve_reduces_opp_stack_by_up_to_3() {
    // Opponent: a Lv6 top card stacked on Lv5 / Lv4 / Lv3 sources.
    // Stack (bottom→top): Lv3_base | Lv4_src | Lv5_src | Lv6_top
    // De-Digivolve 3 with stop_at:3:
    //   Pop Lv6 → next top would be Lv5 (≥3) → OK.
    //   Pop Lv5 → next top would be Lv4 (≥3) → OK.
    //   Pop Lv4 → next top would be Lv3 (≥3) → OK.
    //   amount cap reached (3 pops) → done.
    // Remaining stack: Lv3_base (1 card).
    let mut opp_top = make_test_card("OPP-LV6-TOP", "OppLv6Top");
    opp_top.level = Some(6);
    opp_top.dp = Some(9000);

    let mut src_lv5 = make_test_card("OPP-SRC-LV5", "OppSrcLv5");
    src_lv5.level = Some(5);

    let mut src_lv4 = make_test_card("OPP-SRC-LV4", "OppSrcLv4");
    src_lv4.level = Some(4);

    let mut src_lv3 = make_test_card("OPP-SRC-LV3", "OppSrcLv3");
    src_lv3.level = Some(3);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX4-073")
        .expect("EX4-073 in embedded DSL pack")
        .add_card(opp_top)
        .add_card(src_lv5.clone())
        .add_card(src_lv4.clone())
        .add_card(src_lv3.clone())
        .memory(20)
        .start();

    let owner_handle = runner.place_on_field(0, "EX4-073", Some(0));
    let opp_perm = runner.place_on_field(1, "OPP-LV6-TOP", Some(0));

    // Stack 3 sources below the top card (bottom→top order: lv3 / lv4 / lv5).
    runner.push_source(opp_perm, "OPP-SRC-LV3");
    runner.push_source(opp_perm, "OPP-SRC-LV4");
    runner.push_source(opp_perm, "OPP-SRC-LV5");

    // Verify setup: 4 cards in stack before the trigger.
    let stack_before = runner.game.player(1).battle_area[opp_perm.index as usize]
        .card_sources
        .len();
    assert_eq!(stack_before, 4, "setup: opp permanent must have 4 cards (3 sources + top)");

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(owner_handle),
    );
    runner.game.drain_effect_queue();

    // Resolve the OppField de-digivolve target pick.
    let view = runner
        .pending_selection_view()
        .expect("OppField selection must be installed");
    let sel_player = view.selecting_player;
    let action_id = view.valid_action_ids[0];
    runner
        .game
        .resolve_selection(sel_player, action_id)
        .expect("target pick resolves cleanly");

    // No further selections after de_digivolve (clause B arm 2 is BLOCKED).
    assert!(
        runner.game.pending_selection.is_none(),
        "after de_digivolve resolves, no further selections should remain \
         (clause B arm 2 is BLOCKED on G-MULTI-SELECT-OPP-PLAY-COST-SUM)"
    );

    // Stack must now be reduced by 3 (3 sources popped; Lv3 base remains).
    let stack_after = runner.game.player(1).battle_area[opp_perm.index as usize]
        .card_sources
        .len();
    assert_eq!(
        stack_after,
        stack_before - 3,
        "de_digivolve(3, stop_at:3) must trash exactly 3 sources from the top \
         (Lv6, Lv5, Lv4 popped; Lv3 base remains)"
    );
}

/// Clause B arm 1 stop-at-Lv3: when the next card below the top is Lv2
/// (i.e. below the floor), de_digivolve stops after 0 pops even though
/// the amount cap allows up to 3.
///
/// Setup: opponent has a Lv4 Digimon with a Lv2 source below it
/// (total 2 cards). De-Digivolve 3 with stop_at:3: popping the Lv4
/// would leave the Lv2 as top — which is < 3, so it immediately stops.
/// Stack remains at 2 cards (unchanged).
///
/// Note: the target selection still fires (the outer existential
/// condition only checks that an opp Digimon exists); it's the
/// de_digivolve step itself that evaluates the floor.
#[test]
fn ex4_073_clause_b_de_digivolve_stops_at_lv3_floor_when_next_top_below_3() {
    let mut opp_lv4 = make_test_card("OPP-LV4-STOP", "OppLv4Stop");
    opp_lv4.level = Some(4);
    opp_lv4.dp = Some(6000);

    let mut src_lv2 = make_test_card("OPP-SRC-LV2", "OppSrcLv2");
    src_lv2.level = Some(2);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX4-073")
        .expect("EX4-073 in embedded DSL pack")
        .add_card(opp_lv4)
        .add_card(src_lv2)
        .memory(20)
        .start();

    let owner_handle = runner.place_on_field(0, "EX4-073", Some(0));
    let opp_perm = runner.place_on_field(1, "OPP-LV4-STOP", Some(0));
    runner.push_source(opp_perm, "OPP-SRC-LV2");

    // Verify setup: 2 cards (Lv2 source at bottom, Lv4 top).
    let stack_before = runner.game.player(1).battle_area[opp_perm.index as usize]
        .card_sources
        .len();
    assert_eq!(stack_before, 2, "setup: opp permanent must have 2 cards");

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(owner_handle),
    );
    runner.game.drain_effect_queue();

    // Resolve the OppField target pick.
    let view = runner
        .pending_selection_view()
        .expect("OppField selection must install — the existential condition passes");
    let sel_player = view.selecting_player;
    let action_id = view.valid_action_ids[0];
    runner
        .game
        .resolve_selection(sel_player, action_id)
        .expect("target pick resolves");

    // de_digivolve must have popped 0 cards: next top would be Lv2 < 3.
    let stack_after = runner.game.player(1).battle_area[opp_perm.index as usize]
        .card_sources
        .len();
    assert_eq!(
        stack_after, stack_before,
        "stop_at_level:3 prevents trashing when next top would be Lv2 (< 3); \
         stack must remain unchanged"
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
/// Digimon's digivolution cards".
///
/// G-DSL-SELECT-OWN-SOURCES-FILTER is CLOSED (2026-05-08): `select_own_sources`
/// now accepts `filter:` and evaluates it against each source card via
/// `eval_predicate_with_bindings(PredicateSubject::Card(source.card))`.
/// The `level_gte: 6` filter is now representable.
///
/// The clause body remains BLOCKED on G-PLAY-COST-AGGREGATE: the inner
/// per-trash delete ("delete 1 opp Digimon/Tamer with the lowest play cost")
/// requires a `lowest_play_cost` aggregate predicate that does not yet exist
/// in the DSL (only literal `play_cost_lte` is available). Until the inner
/// body is writable, the clause cannot be authored faithfully.
#[test]
#[ignore = "pending: G-PLAY-COST-AGGREGATE — inner per-trash delete requires `lowest_play_cost` aggregate predicate (no `LowestPlayCost` in AggregateSelector; DSL has only literal play_cost_lte). Outer source-trash step is unblocked (G-DSL-SELECT-OWN-SOURCES-FILTER closed 2026-05-08)."]
fn ex4_073_clause_c_trashes_up_to_3_lv6_or_higher_sources() {
    todo!("write once G-PLAY-COST-AGGREGATE is closed");
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
/// opponent's security stack" — transitively BLOCKED on G-PLAY-COST-AGGREGATE:
/// the count binding comes from the inner for-each / source-trash loop, which
/// cannot be authored until the inner delete (lowest_play_cost) is closed.
/// G-DSL-SELECT-OWN-SOURCES-FILTER is CLOSED (2026-05-08) so the outer
/// source-trash step itself is no longer the bottleneck.
#[test]
#[ignore = "pending: G-PLAY-COST-AGGREGATE (parent gap) — count binding for the 3-trash threshold depends on the for-each source-trash body, which requires the `lowest_play_cost` aggregate predicate to be representable"]
fn ex4_073_clause_c_trashes_2_opp_security_when_3_sources_trashed() {
    todo!("write once G-PLAY-COST-AGGREGATE is closed");
}

/// Clause C is OPTIONAL ("By trashing ... up to 3" — DCGO isOptional: true,
/// canNoSelect: () => true on the source-select). When the gaps close, the
/// player should be able to pass on the entire clause.
///
/// G-DSL-SELECT-OWN-SOURCES-FILTER is CLOSED (2026-05-08). The clause remains
/// BLOCKED solely on G-PLAY-COST-AGGREGATE (inner delete body).
#[test]
#[ignore = "pending: G-PLAY-COST-AGGREGATE — clause body requires `lowest_play_cost` predicate; clause optionality test is writable once the body is writable. G-DSL-SELECT-OWN-SOURCES-FILTER closed 2026-05-08."]
fn ex4_073_clause_c_player_may_decline_to_trash_any_sources() {
    todo!("write once G-PLAY-COST-AGGREGATE is closed");
}
