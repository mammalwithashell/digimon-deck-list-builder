//! LM-021 Agumon - Bond of Bravery — Digimon, Lv.7, Red, DP 14000, Cost 8.
//! Traits: Unknown
//! Evo: Lv6 Red / Cost 3
//! Alt-digi (xros_req): While you have 2 or fewer security cards, [Agumon]: Cost 3
//!
//! # Card text (cards.json)
//!
//! [Hand] [Counter] <Blast Digivolve>
//! [On Play] [When Digivolving] Delete any number of your opponent's Digimon
//!   whose total DP adds up to equal or less than this Digimon's DP.
//! [When Attacking] [Once Per Turn] If you have a Tamer, trash the top card
//!   of your opponent's security stack.
//!
//! Inherited: Ace Overflow <-5>
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/LM/Red/LM_021.cs
//!
//! # Patterns this test covers
//! - H12: Blast Digivolve (grant_keyword: BlastDigivolve, ACE card)
//! - H13: ACE (ace_overflow: -5)
//! - Clause (a): [On Play][When Digivolving] multi-select opponent Digimon, running DP-sum ≤ self.DP
//!   → PARTIAL (G-MULTI-SELECT-OPP-DP-SUM): uses raw_rust placeholder
//! - Clause (b): [When Attacking][Once Per Turn] tamer condition → trash_top_security
//!   (fully implemented)
//! - G-OPT-TRIGGERED: OPT lockout not enforced for triggered effects
//! - Alt-path: standard Lv6 digivolve cost 3
//!
//! # Known gaps
//! - **G-MULTI-SELECT-OPP-DP-SUM**: Engine lacks multi-select of opponent battle-area
//!   permanents with a running DP-sum cap. DCGO uses `SelectPermanentEffect` with
//!   `canTargetConditionByPreSelectedList` (running sum per pick) + `canEndSelectCondition`
//!   (total ≤ MaxDP). `select_count_capped_multi` only targets Hand/Trash/Material.
//!   `select_opponent_permanent` is single-target only. Clause (a) uses `raw_rust`
//!   placeholder until this gap is resolved.
//! - **G-OPT-TRIGGERED**: OPT lockout not enforced for triggered effects in queue.

#![allow(unused_imports, dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const LM_021_YAML: &str = include_str!("../../../cards/lm/LM-021.yaml");

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_filler(id: &str) -> digimon_engine::card_data::CardData {
    make_test_card(id, id)
}

fn make_digimon_dp(id: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

fn make_tamer(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c
}

/// Build the base runner for structural / compilation tests.
fn lm_021_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(LM_021_YAML)
        .expect("LM-021 YAML parses")
        .memory(10)
        .start()
}

/// Fire OnPlay for the given permanent handle.
fn fire_on_play(runner: &mut DebugRunner, handle: digimon_engine::permanent::PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

/// Fire WhenDigivolving for the given permanent handle.
fn fire_when_digivolving(
    runner: &mut DebugRunner,
    handle: digimon_engine::permanent::PermanentHandle,
) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

/// Fire WhenAttacking for the given permanent handle.
fn fire_when_attacking(
    runner: &mut DebugRunner,
    handle: digimon_engine::permanent::PermanentHandle,
) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn lm_021_yaml_parses_without_error() {
    let _runner = lm_021_runner();
}

#[test]
fn lm_021_has_ace_overflow_minus_5() {
    let runner = lm_021_runner();
    let compiled = runner
        .compiled_card("LM-021")
        .expect("LM-021 must be in compiled_cards");

    assert!(
        compiled.ace_overflow.is_some(),
        "LM-021 must have ace_overflow set (Ace Overflow <-5>)"
    );
    assert_eq!(
        compiled.ace_overflow,
        Some(-5),
        "Ace Overflow value must be -5"
    );
}

#[test]
fn lm_021_has_blast_digivolve_keyword_clause() {
    let runner = lm_021_runner();
    let compiled = runner
        .compiled_card("LM-021")
        .expect("LM-021 in compiled_cards");

    // BlastDigivolve is a grant_keyword declarative clause.
    let has_blast = compiled.effects.iter().any(|c| {
        if let CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            ..
        }) = c
        {
            keyword.to_lowercase().contains("blast")
        } else {
            false
        }
    });
    assert!(
        has_blast,
        "LM-021 must have a BlastDigivolve grant_keyword declarative clause for [Hand][Counter] <Blast Digivolve>"
    );
}

#[test]
fn lm_021_has_on_play_when_digivolving_triggered_clause() {
    let runner = lm_021_runner();
    let compiled = runner
        .compiled_card("LM-021")
        .expect("LM-021 in compiled_cards");

    let has_op_wd = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .any(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        });

    assert!(
        has_op_wd,
        "LM-021 must have a triggered clause with both on_play and when_digivolving timings"
    );
}

#[test]
fn lm_021_has_when_attacking_once_per_turn_clause() {
    let runner = lm_021_runner();
    let compiled = runner
        .compiled_card("LM-021")
        .expect("LM-021 in compiled_cards");

    let wa_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenAttacking));

    assert!(
        wa_clause.is_some(),
        "LM-021 must have a WhenAttacking triggered clause"
    );

    let wa = wa_clause.unwrap();
    assert!(
        wa.once_per_turn,
        "[Once Per Turn] must be set on the WhenAttacking clause"
    );
    assert!(
        !wa.optional,
        "WhenAttacking clause is mandatory (not 'you may') once condition is met"
    );
    assert_eq!(
        wa.scope,
        CompiledScope::FaceUp,
        "WhenAttacking is a face-up (own) scope clause"
    );
}

#[test]
fn lm_021_has_standard_digivolve_alt_path_cost_3() {
    let runner = lm_021_runner();
    let compiled = runner
        .compiled_card("LM-021")
        .expect("LM-021 in compiled_cards");

    let has_standard = compiled.alt_paths.iter().any(|ap| {
        ap.kind == CompiledAltPathKind::Digivolve
            && matches!(ap.cost, Some(CompiledCost::Literal(3)))
    });
    assert!(
        has_standard,
        "LM-021 must have a standard digivolve alt-path (Lv6 / Cost 3); alt_paths: {:?}",
        compiled
            .alt_paths
            .iter()
            .map(|ap| (&ap.kind, &ap.cost))
            .collect::<Vec<_>>()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating: [When Attacking] tamer check
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: P0 has a Tamer and opponent has security → WhenAttacking fires
/// and trashes top security.
#[test]
fn lm_021_when_attacking_fires_with_tamer_present() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_021_YAML)
        .expect("LM-021 parses")
        .add_card(make_tamer("TAMER-1"))
        .add_card(make_filler("OPP-SEC-1"))
        .add_card(make_filler("FILLER"))
        .security(1, &["OPP-SEC-1"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();

    let lm_handle = runner.place_on_field(0, "LM-021", Some(0));
    let _ = runner.place_on_field(0, "TAMER-1", Some(0));

    let security_before = runner.security_count(1);
    fire_when_attacking(&mut runner, lm_handle);
    let _ = runner.auto_resolve();

    let security_after = runner.security_count(1);
    assert!(
        security_after < security_before,
        "WhenAttacking must trash top security when P0 has a Tamer; before={security_before}, after={security_after}"
    );
}

/// Negative: P0 has NO Tamer → `any_permanent { kind: tamer }` condition fails,
/// WhenAttacking does NOT trash security.
#[test]
fn lm_021_when_attacking_does_not_fire_without_tamer() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_021_YAML)
        .expect("LM-021 parses")
        .add_card(make_filler("OPP-SEC-1"))
        .add_card(make_filler("FILLER"))
        .security(1, &["OPP-SEC-1"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();

    // LM-021 on field but NO Tamer for P0.
    let lm_handle = runner.place_on_field(0, "LM-021", Some(0));

    let security_before = runner.security_count(1);
    fire_when_attacking(&mut runner, lm_handle);
    let _ = runner.auto_resolve();

    let security_after = runner.security_count(1);
    assert_eq!(
        security_after, security_before,
        "WhenAttacking must NOT trash security when P0 has no Tamer"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral outcome: [When Attacking] trashes exactly 1 security
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn lm_021_when_attacking_trashes_exactly_one_security() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_021_YAML)
        .expect("LM-021 parses")
        .add_card(make_tamer("TAMER-1"))
        .add_card(make_filler("OPP-SEC-A"))
        .add_card(make_filler("OPP-SEC-B"))
        .add_card(make_filler("OPP-SEC-C"))
        .add_card(make_filler("FILLER"))
        .security(1, &["OPP-SEC-A", "OPP-SEC-B", "OPP-SEC-C"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();

    let lm_handle = runner.place_on_field(0, "LM-021", Some(0));
    let _ = runner.place_on_field(0, "TAMER-1", Some(0));

    let security_before = runner.security_count(1);
    let trash_before = runner.trash_size(1);

    fire_when_attacking(&mut runner, lm_handle);
    let _ = runner.auto_resolve();

    let security_after = runner.security_count(1);
    let trash_after = runner.trash_size(1);

    assert_eq!(
        security_before - security_after,
        1,
        "Exactly 1 security card must be trashed; before={security_before}, after={security_after}"
    );
    assert_eq!(
        trash_after - trash_before,
        1,
        "Trashed card must appear in opponent's trash; trash_before={trash_before}, trash_after={trash_after}"
    );
}

/// When opponent has no security, trash_top_security is a no-op.
/// (DCGO CanActivateCondition: `SecurityCards.Count >= 1`)
#[test]
fn lm_021_when_attacking_no_op_with_empty_opponent_security() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_021_YAML)
        .expect("LM-021 parses")
        .add_card(make_tamer("TAMER-1"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();

    // Ensure P1 starts with no security.
    runner.game.players[1].security.clear();

    let lm_handle = runner.place_on_field(0, "LM-021", Some(0));
    let _ = runner.place_on_field(0, "TAMER-1", Some(0));

    fire_when_attacking(&mut runner, lm_handle);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        0,
        "No security to trash; count stays 0"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — OPT enforcement: [When Attacking] [Once Per Turn]
// ═══════════════════════════════════════════════════════════════════════════════

/// Second WhenAttacking in the same turn must NOT trash security again.
///
/// BLOCKED: G-OPT-TRIGGERED — `max_per_turn = 1` is stored on the Effect but
/// `run_queued_effect_inner` in effect_queue.rs does not check it for triggered
/// effects. Effect fires twice until gap is closed.
#[test]
fn lm_021_when_attacking_opt_blocks_second_trigger_same_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_021_YAML)
        .expect("LM-021 parses")
        .add_card(make_tamer("TAMER-1"))
        .add_card(make_filler("OPP-SEC-A"))
        .add_card(make_filler("OPP-SEC-B"))
        .add_card(make_filler("OPP-SEC-C"))
        .add_card(make_filler("FILLER"))
        .security(1, &["OPP-SEC-A", "OPP-SEC-B", "OPP-SEC-C"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();

    let lm_handle = runner.place_on_field(0, "LM-021", Some(0));
    let _ = runner.place_on_field(0, "TAMER-1", Some(0));

    // First trigger — fires, trashes 1.
    fire_when_attacking(&mut runner, lm_handle);
    let _ = runner.auto_resolve();
    let security_after_first = runner.security_count(1);

    // Second trigger in the same turn — OPT must block.
    fire_when_attacking(&mut runner, lm_handle);
    let _ = runner.auto_resolve();
    let security_after_second = runner.security_count(1);

    assert_eq!(
        security_after_second, security_after_first,
        "OPT must block second WhenAttacking trigger in the same turn"
    );
}

/// OPT lockout must clear after end_turn.
#[test]
fn lm_021_when_attacking_opt_clears_after_end_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_021_YAML)
        .expect("LM-021 parses")
        .add_card(make_tamer("TAMER-1"))
        .add_card(make_filler("OPP-SEC-A"))
        .add_card(make_filler("OPP-SEC-B"))
        .add_card(make_filler("OPP-SEC-C"))
        .add_card(make_filler("OPP-SEC-D"))
        .add_card(make_filler("FILLER"))
        .security(1, &["OPP-SEC-A", "OPP-SEC-B", "OPP-SEC-C", "OPP-SEC-D"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();

    let lm_handle = runner.place_on_field(0, "LM-021", Some(0));
    let _ = runner.place_on_field(0, "TAMER-1", Some(0));

    // Fire once on P0's turn.
    fire_when_attacking(&mut runner, lm_handle);
    let _ = runner.auto_resolve();
    let security_t1 = runner.security_count(1);

    // Advance turn cycle back to P0.
    runner.end_turn(); // → P1's turn
    runner.end_turn(); // → P0's turn again

    // Fire on new turn — must not be blocked by prior turn's OPT.
    fire_when_attacking(&mut runner, lm_handle);
    let _ = runner.auto_resolve();
    let security_t2 = runner.security_count(1);

    assert!(
        security_t2 < security_t1,
        "OPT must clear after end_turn; P1 security must decrease again; t1={security_t1}, t2={security_t2}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — On Play / When Digivolving delete clause (PARTIAL)
// ═══════════════════════════════════════════════════════════════════════════════

/// PARTIAL (G-MULTI-SELECT-OPP-DP-SUM):
/// The [On Play][When Digivolving] clause deletes "any number of your opponent's
/// Digimon whose total DP ≤ this Digimon's DP (14000)".
///
/// This requires multi-select of opponent battle-area permanents with a running
/// DP-sum cap. DCGO uses `SelectPermanentEffect` with:
///   - `canTargetConditionByPreSelectedList` (running sum per pick)
///   - `canEndSelectCondition` (validates total ≤ MaxDP / self.DP)
///   - `canNoSelect: false` (at least 1 deletion when ≥1 eligible target exists)
///   - `canEndNotMax: true` (player can stop early)
///
/// Gap: `select_count_capped_multi` only covers Hand/Trash/Material zones.
/// `select_opponent_permanent` is single-target only. No verb for running-DP-sum
/// multi-select on opponent battle area.
///
/// Workaround: clause (a) is implemented as `raw_rust: lm_021_delete_dp_sum`
/// placeholder. This test confirms the placeholder runs without panic.
#[test]
fn lm_021_on_play_does_not_panic_with_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_021_YAML)
        .expect("LM-021 parses")
        .add_card(make_digimon_dp("OPP-DIGI-1", 4000))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();

    let _ = runner.place_on_field(1, "OPP-DIGI-1", Some(0));
    let lm_handle = runner.place_on_field(0, "LM-021", Some(0));

    // Must not panic.
    fire_on_play(&mut runner, lm_handle);
    let _ = runner.auto_resolve();
}

/// Negative: On Play with NO opponent Digimon → `any_permanent` condition fails,
/// no effect fires, no selection installs.
#[test]
fn lm_021_on_play_no_effect_when_no_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_021_YAML)
        .expect("LM-021 parses")
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();

    // P1 has no Digimon on field.
    let lm_handle = runner.place_on_field(0, "LM-021", Some(0));
    fire_on_play(&mut runner, lm_handle);

    // Condition (`any_permanent` opponent digimon) fails → no selection.
    let pending = runner.pending_selection();
    assert!(
        pending.is_none(),
        "On Play must not install selection when opponent has no Digimon"
    );
}

/// WhenDigivolving fires the same delete clause as OnPlay.
/// Mirror of the negative OnPlay test for the WhenDigivolving timing.
#[test]
fn lm_021_when_digivolving_no_effect_when_no_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_021_YAML)
        .expect("LM-021 parses")
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();

    let lm_handle = runner.place_on_field(0, "LM-021", Some(0));
    fire_when_digivolving(&mut runner, lm_handle);

    let pending = runner.pending_selection();
    assert!(
        pending.is_none(),
        "When Digivolving must not install selection when opponent has no Digimon"
    );
}
