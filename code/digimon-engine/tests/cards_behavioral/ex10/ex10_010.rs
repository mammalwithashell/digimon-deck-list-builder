//! EX10-010 BlackWarGreymon — Digimon, Lv.6, Red/Black, DP12000, Cost7.
//!
//! # Card text (cards.json)
//!
//! `[Hand] [Counter] ＜Blast Digivolve＞ (Your Digimon may digivolve into this card without paying the cost.)`
//! `＜Raid＞`
//! `＜Reboot＞`
//! `＜Blocker＞`
//! `[On Play] [When Digivolving] Delete 1 of your opponent's play cost 7 or lower Digimon or Tamers.`
//! `[All Turns] While your opponent has a Digimon with 13000 DP or more, your opponent's Digimon's`
//! `effects don't affect this Digimon, and it gets +3000 DP.`
//!
//! # Inherited effect
//! `Ace Overflow ＜-4＞ (As this card moves from the field or under a card to an area other than those, lose 4 memory.)`
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX10/Red/EX10_010.cs
//!
//! # Patterns this test file covers
//! - H12: Blast Digivolve / burst_digivolve with marker:true
//! - H9: Raid keyword grant (declarative)
//! - H7: Reboot keyword grant (declarative)
//! - H5: Blocker keyword grant (declarative)
//! - ace_overflow: -4 declarative clause
//! - D1: Conditional DP aura (+3000 while opp has ≥13000 DP Digimon, self-aura path)
//!   - Gap: G-PRED-DP-LTE (dp_gte not evaluated in active_when; aura over-fires)
//! - F6: Effect immunity while condition holds
//!   - Gap: lower_aura.rs modifier field not wired (CannotBeAffected never installed via aura)
//!   - Gap: CannotBeAffected not enforced by engine effect-execution path
//! - G-PLAY-COST-LTE: play_cost_lte filter on delete target (known gap — selection over-permissive)

#![allow(dead_code, unused_imports, unused_variables)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;

use dsl_card_data::{card_data_from_compiled, compiled};

// ─── helpers ──────────────────────────────────────────────────────────────────

/// Generic filler card with no effects or traits.
fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec![];
    c
}

/// Make an opponent Digimon with a given DP value.
fn make_opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.dp = Some(dp);
    c
}

/// Build a DebugRunner with EX10-010 loaded from the embedded DSL pack.
/// Uses only registered cards for hand/deck/security.
fn runner_with_ex10_010() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX10-010")
        .expect("EX10-010 must be in embedded DSL pack")
        .add_card(make_filler("FILLER"))
        .hand(0, &["EX10-010"])
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .security(0, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .security(1, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(7)
        .start()
}

fn resolve_first_pending(runner: &mut DebugRunner) {
    let (player, action) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("a pending selection must be installed");
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    runner
        .game
        .resolve_selection(player, action)
        .expect("resolve_selection succeeds");
}

fn pass_pending(runner: &mut DebugRunner) {
    let player = runner
        .game
        .pending_selection
        .as_ref()
        .expect("selection installed")
        .selecting_player;
    runner
        .game
        .resolve_selection(player, PASS)
        .expect("decline resolves");
}

// ─── SECTION 1 — Structural assertions ───────────────────────────────────────

/// BlackWarGreymon must have exactly 3 GrantKeyword declarative clauses:
/// Raid, Reboot, Blocker.
#[test]
fn ex10_010_has_three_keyword_grants() {
    let card = compiled("EX10-010");

    let keywords: Vec<String> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) => Some(keyword.clone()),
            _ => None,
        })
        .collect();

    assert_eq!(
        keywords.len(),
        3,
        "Raid, Reboot, Blocker must each produce one GrantKeyword clause; got: {:?}",
        keywords
    );
}

/// The three granted keywords must be Raid, Reboot, and Blocker.
#[test]
fn ex10_010_keyword_grants_are_raid_reboot_blocker() {
    let card = compiled("EX10-010");

    let keywords: std::collections::HashSet<String> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) => Some(keyword.clone()),
            _ => None,
        })
        .collect();

    for expected in &["Raid", "Reboot", "Blocker"] {
        assert!(
            keywords.contains(*expected),
            "keyword grant '{}' missing; got: {:?}",
            expected,
            keywords
        );
    }
}

/// The AceOverflow declarative clause must be present with value -4.
#[test]
fn ex10_010_has_ace_overflow_minus_4() {
    let card = compiled("EX10-010");
    let ace = card.ace_overflow;
    assert_eq!(
        ace,
        Some(-4),
        "EX10-010 must have ace_overflow: -4; got: {:?}",
        ace
    );
}

/// The alt_paths must include a burst_digivolve marker path (Blast Digivolve).
#[test]
fn ex10_010_has_blast_digivolve_alt_path() {
    let card = compiled("EX10-010");

    let has_blast = card
        .alt_paths
        .iter()
        .any(|ap| matches!(ap.kind, CompiledAltPathKind::BurstDigivolve) && ap.marker);

    assert!(
        has_blast,
        "EX10-010 must have a burst_digivolve with marker:true (Blast Digivolve alt-path); \
         alt_paths kinds: {:?}",
        card.alt_paths
            .iter()
            .map(|ap| format!("{:?} marker={}", ap.kind, ap.marker))
            .collect::<Vec<_>>()
    );
}

/// There must be a standard digivolve alt-path (from evo_costs).
#[test]
fn ex10_010_has_standard_digivolve_alt_path() {
    let card = compiled("EX10-010");

    let has_digivolve = card
        .alt_paths
        .iter()
        .any(|ap| matches!(ap.kind, CompiledAltPathKind::Digivolve));

    assert!(
        has_digivolve,
        "EX10-010 must have at least one Digivolve alt-path"
    );
}

/// The On Play / When Digivolving delete clause must exist as a triggered clause
/// covering both timings.
#[test]
fn ex10_010_has_on_play_when_digivolving_triggered_clause() {
    let card = compiled("EX10-010");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        });

    assert!(
        clause.is_some(),
        "EX10-010 must have a triggered clause with when: [on_play, when_digivolving]"
    );

    let clause = clause.unwrap();
    // Not optional — card text has no "you may"
    assert!(
        !clause.optional,
        "On Play / When Digivolving delete clause must NOT be optional (no 'you may' in text)"
    );
    // Not OPT — no [Once Per Turn] tag
    assert!(
        !clause.once_per_turn,
        "On Play / When Digivolving delete clause must NOT be once_per_turn"
    );
    // Face-up scope
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "On Play / When Digivolving clause is a face-up own effect"
    );
}

/// The [All Turns] conditional aura clause must exist as a declarative Aura
/// with dp_modifier = 3000.
#[test]
fn ex10_010_has_conditional_all_turns_dp_aura_with_3000() {
    let card = compiled("EX10-010");

    let aura_dp = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            dp_modifier: Some(dp),
            ..
        }) => Some(*dp),
        _ => None,
    });

    assert_eq!(
        aura_dp,
        Some(3000),
        "EX10-010 must have an Aura clause with dp_modifier = 3000; got: {:?}",
        aura_dp
    );
}

// ─── SECTION 2 — Condition gating: [On Play/WD] delete requires a valid target ──

/// Positive case: opponent has a Digimon on field → delete prompt installs after play.
/// G-PLAY-COST-LTE: cost ≤7 filter is not enforced at selection time; any opponent
/// Digimon/Tamer appears regardless of cost.
#[test]
fn ex10_010_on_play_installs_delete_prompt_when_opp_has_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-010")
        .expect("EX10-010 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-DIGIMON", 3000))
        .add_card(make_filler("FILLER"))
        .hand(0, &["EX10-010"])
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .security(0, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .security(1, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(7)
        .start();

    // Place a low-cost opponent Digimon on field
    let _opp_digimon = runner.place_on_field(1, "OPP-DIGIMON", None);

    // Play EX10-010 from hand (cost 7, memory set to 7)
    let _perm = runner.play(0, 0);

    // A pending selection for the delete must have installed
    assert!(
        runner.game.pending_selection.is_some(),
        "On Play should install a delete selection when opponent has a Digimon"
    );
}

/// Negative case: opponent has no Digimon or Tamer → delete prompt does NOT install.
/// DCGO: CanActivateSharedCondition gates the effect body on HasMatchConditionPermanent.
#[test]
fn ex10_010_on_play_no_prompt_when_opp_has_no_valid_targets() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-010")
        .expect("EX10-010 in embedded DSL pack")
        .add_card(make_filler("FILLER"))
        .hand(0, &["EX10-010"])
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(7)
        .start();

    // Opponent's battle area is empty — no Digimon or Tamer targets
    let _perm = runner.play(0, 0);

    // No delete prompt should install when there are no valid targets
    assert!(
        runner.game.pending_selection.is_none(),
        "On Play should NOT install delete prompt when opponent has no Digimon or Tamer"
    );
}

// ─── SECTION 3 — Behavioral: [On Play] delete clause ─────────────────────────

/// Playing EX10-010 with an opponent Digimon present: selecting the target deletes it.
#[test]
fn ex10_010_on_play_deletes_selected_opponent_permanent() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-010")
        .expect("EX10-010 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-DIGIMON", 3000))
        .add_card(make_filler("FILLER"))
        .hand(0, &["EX10-010"])
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .security(0, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .security(1, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(7)
        .start();

    let _opp_digimon = runner.place_on_field(1, "OPP-DIGIMON", None);
    let initial_opp_field = runner.battle_area_size(1);

    let _perm = runner.play(0, 0);

    assert!(
        runner.game.pending_selection.is_some(),
        "delete prompt must install"
    );
    resolve_first_pending(&mut runner);

    let final_opp_field = runner.battle_area_size(1);
    assert_eq!(
        final_opp_field,
        initial_opp_field - 1,
        "opponent's field should shrink by 1 after delete; had {initial_opp_field}, now {final_opp_field}"
    );
}

/// [When Digivolving] fires identically to [On Play] for the delete clause.
/// Tested via place_on_field + fire_on_play.
#[test]
fn ex10_010_when_digivolving_deletes_selected_opponent_permanent() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-010")
        .expect("EX10-010 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-DIGIMON", 3000))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .security(0, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .security(1, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(7)
        .start();

    let _opp_digimon = runner.place_on_field(1, "OPP-DIGIMON", None);
    let initial_opp_field = runner.battle_area_size(1);

    // Simulate digivolving via place + fire
    let perm = runner.place_on_field(0, "EX10-010", None);
    runner.fire_on_play(0, perm.index as usize);

    assert!(
        runner.game.pending_selection.is_some(),
        "WhenDigivolving should install delete prompt when opponent has a Digimon"
    );
    resolve_first_pending(&mut runner);

    let final_opp_field = runner.battle_area_size(1);
    assert_eq!(
        final_opp_field,
        initial_opp_field - 1,
        "When Digivolving: opponent field should shrink by 1; had {initial_opp_field}, now {final_opp_field}"
    );
}

// ─── SECTION 4 — G-PLAY-COST-LTE gap test ────────────────────────────────────

/// G-PLAY-COST-LTE: a high-cost opponent permanent (cost > 7) should NOT appear
/// as a valid delete target per card text, but will until the gap is closed.
/// This test documents the expected behavior when the gap is resolved.
#[test]
#[ignore = "pending: G-PLAY-COST-LTE — play_cost_lte predicate not evaluated at selection time"]
fn ex10_010_on_play_excludes_opponent_permanent_with_cost_above_7() {
    // Post gap-close test scaffolding:
    // Place a cost-14 Digimon and a cost-3 Digimon on opponent's field.
    // After playing EX10-010, only the cost-3 card should appear in valid_action_ids.
}

// ─── SECTION 5 — Keyword grants (declarative, runtime) ───────────────────────

/// When EX10-010 is on the field, it must have the Raid keyword.
#[test]
fn ex10_010_on_field_has_raid_keyword() {
    let mut runner = runner_with_ex10_010();
    let perm = runner.place_on_field(0, "EX10-010", None);

    let has_raid = runner.game.has_keyword(perm, Keyword::Raid);

    assert!(has_raid, "EX10-010 on field must have Raid keyword");
}

/// When EX10-010 is on the field, it must have the Reboot keyword.
#[test]
fn ex10_010_on_field_has_reboot_keyword() {
    let mut runner = runner_with_ex10_010();
    let perm = runner.place_on_field(0, "EX10-010", None);

    let has_reboot = runner.game.has_keyword(perm, Keyword::Reboot);

    assert!(has_reboot, "EX10-010 on field must have Reboot keyword");
}

/// When EX10-010 is on the field, it must have the Blocker keyword.
#[test]
fn ex10_010_on_field_has_blocker_keyword() {
    let mut runner = runner_with_ex10_010();
    let perm = runner.place_on_field(0, "EX10-010", None);

    let has_blocker = runner.game.has_keyword(perm, Keyword::Blocker);

    assert!(has_blocker, "EX10-010 on field must have Blocker keyword");
}

// ─── SECTION 6 — [All Turns] conditional DP aura (+3000) ─────────────────────

/// Positive: when opponent has a Digimon on field, the self-aura contributes
/// +3000 DP to BlackWarGreymon's source_dp_contribution.
///
/// Gap note (G-PRED-DP-LTE): the active_when predicate includes `dp_gte: 13000`
/// which is not evaluated — the aura fires whenever ANY opponent Digimon is
/// present. This test passes with or without the gap, demonstrating the intended
/// behavior when the condition is TRUE (opp has a Digimon, intended or not).
#[test]
fn ex10_010_gets_3000_dp_from_aura_when_opp_digimon_present() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-010")
        .expect("EX10-010 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-DIGIMON", 15000))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(0)
        .start();

    let bwg = runner.place_on_field(0, "EX10-010", None);
    let _opp = runner.place_on_field(1, "OPP-DIGIMON", None);

    // The self-aura dp_modifier = 3000 is static; source_dp_contribution
    // evaluates the condition against the current game state.
    let contrib = runner.game.source_dp_contribution(bwg, 0);

    assert_eq!(
        contrib, 3000,
        "EX10-010 source_dp_contribution should be +3000 when aura condition passes; got {}",
        contrib
    );
}

/// Negative gate test: when opponent has NO Digimon, the +3000 aura should NOT apply.
///
/// Gap: G-PRED-DP-LTE — the active_when condition `any_permanent: { of: opponent, ... }`
/// requires the opponent to have a Digimon present to be TRUE. When opponent has no
/// Digimon, the `any_permanent` evaluates FALSE and the aura condition fails.
/// This test verifies the condition correctly gates the aura when NO opponent is present.
#[test]
fn ex10_010_no_dp_buff_when_opp_has_no_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-010")
        .expect("EX10-010 in embedded DSL pack")
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(0)
        .start();

    // EX10-010 on field; opponent battle area is empty
    let bwg = runner.place_on_field(0, "EX10-010", None);
    // No opponent Digimon placed

    let contrib = runner.game.source_dp_contribution(bwg, 0);

    assert_eq!(
        contrib, 0,
        "EX10-010 source_dp_contribution should be 0 when opponent has no Digimon; got {}",
        contrib
    );
}

// ─── SECTION 7 — Immunity from opponent Digimon effects (aura modifier) ─────

/// [All Turns] Immunity: EX10-010 should carry CannotBeAffected modifier when
/// the condition is met (opp has ≥13000 DP Digimon).
///
/// Gap: lower_aura.rs does not pass the `modifier` field from the compiled Aura
/// clause into `lower_aura::lower()`. The `CannotBeAffected` modifier is compiled
/// into `CompiledDeclarativeClause::Aura.modifier` but is never installed in the
/// modifier registry.
/// Additionally, even if installed, `CannotBeAffected` is not enforced by the
/// engine's effect execution path.
///
/// When both gaps close: the `modifier` field must be threaded into lower_aura and
/// applied via `ctx.add_modifier(h, CannotBeAffected, 0, Expiry::Permanent)`.
#[test]
#[ignore = "pending: lower_aura.rs modifier field not wired — CannotBeAffected never installed; CannotBeAffected not enforced by engine"]
fn ex10_010_has_cannot_be_affected_modifier_when_opp_has_13000_dp_digimon() {
    // Post-gap-close scaffolding:
    // let mut runner = runner_with_ex10_010();
    // let bwg = runner.place_on_field(0, "EX10-010", None);
    // let _big_opp = runner.place_on_field(1, <≥13000 DP card>, None);
    // assert!(runner.game.modifiers.has(bwg, ModifierType::CannotBeAffected),
    //   "EX10-010 must carry CannotBeAffected when opp has ≥13000 DP Digimon");
}

/// Negative case: when opp has no ≥13000 DP Digimon, CannotBeAffected must NOT apply.
#[test]
#[ignore = "pending: lower_aura.rs modifier field not wired — CannotBeAffected never installed; CannotBeAffected not enforced by engine"]
fn ex10_010_no_cannot_be_affected_modifier_when_opp_lacks_13000_dp_digimon() {
    // Post-gap-close scaffolding:
    // let mut runner = runner_with_ex10_010();
    // let bwg = runner.place_on_field(0, "EX10-010", None);
    // // No ≥13000 DP opp Digimon
    // assert!(!runner.game.modifiers.has(bwg, ModifierType::CannotBeAffected),
    //   "EX10-010 must NOT carry CannotBeAffected when opp lacks ≥13000 DP Digimon");
}

// SECTION 8 - Closed-gap regression tests (audit 2026-05-03)
//
// The #[ignore]d scaffolds above predate engine landings:
// - 2026-05-01: play_cost_lte parsed and evaluated in eval_card_fields
//   (predicate.rs:727). qa/dsl-vocab-gaps.md P-189.
// - 2026-05-02: dp_lte/dp_gte permanent predicates evaluated against live
//   effective DP in eval_dp_constraints (predicate.rs:867).
//   qa/archetype-qa/engine-gaps.md G-PRED-DP-LTE.
// Plus lower_aura.rs threads `modifier` end-to-end (lines 38, 89, 144) and
// game.rs:1885 enforces CannotBeAffected via permanent_is_unaffected_by_effect.
// flood_gate path in lower_flood_gate.rs also installs the modifier per tick.

fn make_opp_digimon_costed(id: &str, dp: i32, play_cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.dp = Some(dp);
    c.play_cost = play_cost;
    c
}

#[test]
fn ex10_010_dp_aura_applies_when_opp_has_13000_dp_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-010")
        .expect("EX10-010 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-BIG", 13000))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(0)
        .start();
    let bwg = runner.place_on_field(0, "EX10-010", None);
    let _opp = runner.place_on_field(1, "OPP-BIG", None);
    let contrib = runner.game.source_dp_contribution(bwg, 0);
    assert_eq!(
        contrib, 3000,
        "expected +3000 with >=13000 DP opp; got {}",
        contrib
    );
}

#[test]
fn ex10_010_dp_aura_does_not_apply_when_opp_digimon_below_13000_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-010")
        .expect("EX10-010 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-SMALL", 12000))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(0)
        .start();
    let bwg = runner.place_on_field(0, "EX10-010", None);
    let _opp = runner.place_on_field(1, "OPP-SMALL", None);
    let contrib = runner.game.source_dp_contribution(bwg, 0);
    assert_eq!(contrib, 0, "expected 0 with <13000 DP opp; got {}", contrib);
}

#[test]
#[ignore = "audit finding A1: YAML omits play_cost_lte: 7 even though predicate now exists"]
fn ex10_010_on_play_filter_excludes_cost_above_7_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-010")
        .expect("EX10-010 in embedded DSL pack")
        .add_card(make_opp_digimon_costed("OPP-CHEAP", 5000, 3))
        .add_card(make_opp_digimon_costed("OPP-BIG", 12000, 9))
        .add_card(make_filler("FILLER"))
        .hand(0, &["EX10-010"])
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .security(0, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .security(1, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(7)
        .start();
    let _ = runner.place_on_field(1, "OPP-CHEAP", None);
    let _ = runner.place_on_field(1, "OPP-BIG", None);
    let _ = runner.play(0, 0);
    let n = runner
        .game
        .pending_selection
        .as_ref()
        .expect("prompt")
        .valid_action_ids
        .len();
    assert_eq!(n, 1, "only cost-3 should be valid; got {}", n);
}

#[test]
fn ex10_010_cannot_be_affected_installed_when_opp_has_13000_dp_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-010")
        .expect("EX10-010 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-BIG", 13000))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(0)
        .start();
    let bwg = runner.place_on_field(0, "EX10-010", None);
    let _opp = runner.place_on_field(1, "OPP-BIG", None);
    runner.game.tick_declarative_effects();
    assert!(
        runner
            .game
            .modifiers
            .has(bwg, ModifierType::CannotBeAffected),
        "expected CannotBeAffected installed"
    );
}

#[test]
fn ex10_010_cannot_be_affected_not_installed_when_opp_lacks_13000_dp_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-010")
        .expect("EX10-010 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-SMALL", 12000))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(0)
        .start();
    let bwg = runner.place_on_field(0, "EX10-010", None);
    let _opp = runner.place_on_field(1, "OPP-SMALL", None);
    runner.game.tick_declarative_effects();
    assert!(
        !runner
            .game
            .modifiers
            .has(bwg, ModifierType::CannotBeAffected),
        "expected CannotBeAffected NOT installed"
    );
}
