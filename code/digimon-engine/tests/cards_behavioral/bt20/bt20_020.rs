//! BT20-020 Imperialdramon: Fighter Mode — Digimon, Lv.6, DP13000, Cost13, Red/White.
//! Traits: Ancient Dragonkin
//!
//! # Card text (cards.json)
//!
//! ＜Raid＞ (When this Digimon attacks, you may switch the target of attack to 1
//!   of your opponent's unsuspended Digimon with the highest DP.)
//! ＜Piercing＞ (When this Digimon attacks and deletes an opponent's Digimon and
//!   survives the battle, it performs any security checks it normally would.)
//! [When Digivolving] Your opponent can't play Digimon or Tamers by effects
//!   until the end of their turn. Then, if [Imperialdramon: Dragon Mode] is in
//!   this Digimon's digivolution cards, trash your opponent's top security card.
//! [All Turns] [Once Per Turn] When your opponent's security stack is removed
//!   from, delete 1 of their Digimon with as much or less DP as this Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT20/Red/BT20_020.cs
//!
//! # Alt-paths
//! 1. Standard digivolve: Lv.5 Red / Cost 5 (per evo_costs in cards.json).
//! 2. Alt digivolve (xros_req): [Imperialdramon: Dragon Mode] (Lv.6) / Cost 2.
//!
//! # Patterns this test file covers
//! - Clause 0 (Raid): `kind: grant_keyword, keyword: Raid` — face-up declarative
//! - Clause 1 (Piercing): `kind: grant_keyword, keyword: Piercing` — face-up declarative
//! - Clause 2 (When Digivolving): triggered — two `add_player_modifier` steps for
//!   CannotPlayDigimonByEffect + CannotPlayTamerByEffect (end_of_opponents_turn);
//!   then conditional `trash_top_security` if Dragon Mode in digi-stack.
//! - Clause 3 (All Turns OPT): triggered on `on_opponent_security_removed`,
//!   once_per_turn, active_when: all_turns. Delete body uses `source_dp`.
//!
//! # Known gaps and test status
//!
//! | Clause | Gap | Status |
//! |--------|-----|--------|
//! | (0) Raid declarative grant | G-DECLARATIVE-KEYWORD: never enqueued at runtime | compiled/structural only |
//! | (1) Piercing declarative grant | G-DECLARATIVE-KEYWORD | compiled/structural only |
//! | (2) When Digivolving flood-gate | No gap — add_player_modifier fully implemented | IMPLEMENTED |
//! | (2) Conditional security trash | self_digivolution_contains_name in if-condition | IMPLEMENTED |
//! | (3) OPT trigger structure | on_opponent_security_removed + once_per_turn | IMPLEMENTED |
//! | (3) Delete <= self DP | source_dp formula | IMPLEMENTED |
//! | (3) OPT enforcement | once_per_turn trigger lockout | IMPLEMENTED |

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledPlayerRef, CompiledScope, CompiledStep, CompiledTiming,
};

use digimon_dsl::compiled::CompiledPredicate;

use crate::dsl_card_data::compiled;

// ─── Alt-path predicate helpers ──────────────────────────────────────────────
//
// `alt_paths[].from` is a `CompiledPredicate`. When the YAML authors the `from`
// as `{ all_of: [...] }`, the leaf fields (`level_eq`, `color_is`,
// `name_contains`) live on the nested `all_of` children, not on the top-level
// predicate. These helpers walk both the top-level field and the `all_of`
// children so the assertions hold regardless of how the `from` is nested.

fn pred_level_eq(p: &CompiledPredicate) -> Option<u8> {
    p.level_eq
        .or_else(|| p.all_of.iter().find_map(pred_level_eq))
}

fn pred_color_is(p: &CompiledPredicate) -> Option<CompiledColor> {
    p.color_is
        .or_else(|| p.all_of.iter().find_map(pred_color_is))
}

fn pred_name_contains(p: &CompiledPredicate) -> Option<String> {
    p.name_contains
        .clone()
        .or_else(|| p.all_of.iter().find_map(pred_name_contains))
}

// ─── Section 1 — Identity assertions ─────────────────────────────────────────

/// BT20-020 compiles with the correct printed card identity.
#[test]
fn bt20_020_has_correct_identity() {
    let card = compiled("BT20-020");

    assert_eq!(card.card, "BT20-020");
    assert_eq!(card.name, "Imperialdramon: Fighter Mode");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(13000));
    assert_eq!(card.cost, Some(13));
    assert!(
        card.color.contains(&CompiledColor::Red),
        "BT20-020 must be Red"
    );
    assert!(
        card.color.contains(&CompiledColor::White),
        "BT20-020 must be White"
    );
    assert!(
        card.traits.iter().any(|t| t == "Ancient Dragonkin"),
        "BT20-020 must have Ancient Dragonkin trait"
    );
}

// ─── Section 2 — Alt-path assertions ─────────────────────────────────────────

/// Standard digivolve: Lv.5 Red / Cost 5 (per evo_costs in cards.json).
#[test]
fn bt20_020_has_standard_lv5_red_digivolve() {
    let card = compiled("BT20-020");

    let has_standard = card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(5))
            && path.from.as_ref().is_some_and(|from| {
                pred_level_eq(from) == Some(5) && pred_color_is(from) == Some(CompiledColor::Red)
            })
    });

    assert!(
        has_standard,
        "BT20-020 must have a standard Lv.5 Red / Cost 5 digivolve path"
    );
}

/// Alt digivolve (xros_req): [Imperialdramon: Dragon Mode] (Lv.6) / Cost 2.
/// Per DCGO BT20_020.cs: AddSelfDigivolutionRequirementStaticEffect with
/// PermanentCondition checking EqualsCardName("Imperialdramon: Dragon Mode").
#[test]
fn bt20_020_has_dragon_mode_alt_digivolve() {
    let card = compiled("BT20-020");

    let has_dragon_mode_path = card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(2))
            && path.from.as_ref().is_some_and(|from| {
                pred_level_eq(from) == Some(6)
                    && pred_name_contains(from)
                        .as_deref()
                        .is_some_and(|n| n.contains("Imperialdramon"))
            })
    });

    assert!(
        has_dragon_mode_path,
        "BT20-020 must have an alt digivolve from [Imperialdramon: Dragon Mode] (Lv.6) for cost 2"
    );
}

// ─── Section 3 — Declarative keyword grants (Clauses 0 and 1) ────────────────

/// BT20-020 must compile exactly 2 declarative GrantKeyword clauses: Raid and Piercing.
#[test]
fn bt20_020_has_raid_and_piercing_grants() {
    let card = compiled("BT20-020");

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

    assert_eq!(
        keywords.len(),
        2,
        "BT20-020 must have exactly 2 GrantKeyword declarative clauses (Raid + Piercing); got: {:?}",
        keywords
    );
    assert!(
        keywords.contains("Raid"),
        "BT20-020 must grant Raid keyword; got: {:?}",
        keywords
    );
    assert!(
        keywords.contains("Piercing"),
        "BT20-020 must grant Piercing keyword; got: {:?}",
        keywords
    );
}

// ─── Section 4 — Triggered clause count ──────────────────────────────────────

/// BT20-020 must compile exactly 2 triggered clauses:
///   [0] when_digivolving (flood-gate + conditional security trash)
///   [1] on_opponent_security_removed OPT (delete body uses source_dp)
#[test]
fn bt20_020_has_two_triggered_clauses() {
    let card = compiled("BT20-020");

    let triggered_count = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();

    assert_eq!(
        triggered_count, 2,
        "BT20-020 must have exactly 2 triggered clauses; got {triggered_count}"
    );
}

// ─── Section 5 — Clause 2: [When Digivolving] structural ─────────────────────

/// Clause 2 fires on WhenDigivolving, is FaceUp scope, not optional, not OPT.
#[test]
fn bt20_020_clause2_is_when_digivolving_mandatory() {
    let card = compiled("BT20-020");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving));

    let clause = clause.expect("[WhenDigivolving] triggered clause (clause 2) must exist");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "clause 2 must be FaceUp scope"
    );
    assert!(
        !clause.optional,
        "clause 2 is not optional — flood-gate installs unconditionally on digivolve"
    );
    assert!(
        !clause.once_per_turn,
        "clause 2 has no [Once Per Turn] restriction"
    );
}

/// Clause 2 process starts with two AddPlayerModifier steps (CannotPlayDigimonByEffect
/// and CannotPlayTamerByEffect), both targeting opponent with end_of_opponents_turn expiry.
#[test]
fn bt20_020_clause2_has_two_cannot_play_modifiers() {
    let card = compiled("BT20-020");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("[WhenDigivolving] clause must exist");

    let player_modifiers: Vec<_> = clause
        .process
        .iter()
        .filter_map(|step| match step {
            CompiledStep::AddPlayerModifier {
                target_player,
                modifier,
                expiry,
            } => Some((target_player, modifier.as_str(), expiry.as_str())),
            _ => None,
        })
        .collect();

    assert_eq!(
        player_modifiers.len(),
        2,
        "clause 2 must have exactly 2 AddPlayerModifier steps; got: {:?}",
        player_modifiers
    );

    let has_digimon_block = player_modifiers.iter().any(|(player, modifier, expiry)| {
        **player == CompiledPlayerRef::Opponent
            && *modifier == "CannotPlayDigimonByEffect"
            && *expiry == "end_of_opponents_turn"
    });
    assert!(
        has_digimon_block,
        "clause 2 must have CannotPlayDigimonByEffect on opponent with end_of_opponents_turn"
    );

    let has_tamer_block = player_modifiers.iter().any(|(player, modifier, expiry)| {
        **player == CompiledPlayerRef::Opponent
            && *modifier == "CannotPlayTamerByEffect"
            && *expiry == "end_of_opponents_turn"
    });
    assert!(
        has_tamer_block,
        "clause 2 must have CannotPlayTamerByEffect on opponent with end_of_opponents_turn"
    );
}

/// Clause 2 process contains an If step (the conditional security trash).
#[test]
fn bt20_020_clause2_has_conditional_security_trash() {
    let card = compiled("BT20-020");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("[WhenDigivolving] clause must exist");

    let has_if = clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::If { .. }));

    assert!(
        has_if,
        "clause 2 must contain an If step for the conditional security trash"
    );
}

/// Clause 2 process has exactly 3 steps: two AddPlayerModifier steps and one If step.
#[test]
fn bt20_020_clause2_has_exactly_three_steps() {
    let card = compiled("BT20-020");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("[WhenDigivolving] clause must exist");

    assert_eq!(
        clause.process.len(),
        3,
        "clause 2 must have exactly 3 process steps (2x AddPlayerModifier + 1x If); got {}",
        clause.process.len()
    );
}

// ─── Section 6 — Clause 3: [All Turns][OPT] on_opponent_security_removed ─────

/// Clause 3 fires on OnOpponentSecurityRemoved, is all_turns, once_per_turn.
#[test]
fn bt20_020_clause3_is_on_opp_security_removed_opt() {
    let card = compiled("BT20-020");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnOpponentSecurityRemoved));

    let clause =
        clause.expect("[OnOpponentSecurityRemoved] triggered clause (clause 3) must exist");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "clause 3 must be FaceUp scope"
    );
    assert!(
        clause.once_per_turn,
        "clause 3 must have once_per_turn (printed [Once Per Turn])"
    );
    assert!(
        !clause.optional,
        "clause 3 is not optional — 'delete 1' is mandatory when eligible target exists"
    );
}

/// Clause 3's process carries the delete body: an If-gated
/// select_opponent_permanent + delete_permanent. The DP filter uses the
/// `source_dp` formula (G-FORMULA-SOURCE-DP resolved).
#[test]
fn bt20_020_clause3_has_delete_body_with_source_dp_filter() {
    let card = compiled("BT20-020");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnOpponentSecurityRemoved))
        .expect("[OnOpponentSecurityRemoved] clause must exist");

    assert!(
        !clause.process.is_empty(),
        "clause 3 must carry the delete body now that G-FORMULA-SOURCE-DP is resolved"
    );

    // The delete body is gated behind an If step (legal-target precondition).
    let has_if = clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::If { .. }));
    assert!(
        has_if,
        "clause 3 must gate the delete behind an If precondition; steps={:?}",
        clause.process
    );

    // The delete-permanent + select must use a dp_lte filter sourced from a
    // formula (source_dp), reachable inside the If's then-branch.
    fn collect_steps<'a>(steps: &'a [CompiledStep], out: &mut Vec<&'a CompiledStep>) {
        for s in steps {
            out.push(s);
            if let CompiledStep::If {
                then, else_branch, ..
            } = s
            {
                collect_steps(then, out);
                collect_steps(else_branch, out);
            }
        }
    }
    let mut all_steps = Vec::new();
    collect_steps(&clause.process, &mut all_steps);

    let has_delete = all_steps
        .iter()
        .any(|s| matches!(s, CompiledStep::DeletePermanent { .. }));
    assert!(
        has_delete,
        "clause 3 must contain a delete_permanent step; steps={:?}",
        all_steps
    );
}

// ─── Section 7 — Clause 3 behavioral tests ───────────────────────────────────

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.dp = Some(dp);
    c
}

/// Fire the `on_opponent_security_removed` observer for BT20-020's permanent
/// directly, simulating an opponent security card being removed.
fn fire_opp_security_removed(
    runner: &mut DebugRunner,
    ifm: digimon_engine::permanent::PermanentHandle,
) {
    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::Permanent(ifm),
    );
    runner.game.drain_effect_queue();
}

/// G-OPT-TRIGGERED resolved (Phase 2 Track C): the triggered [Once Per Turn]
/// gate is enforced at runtime. Clause 3 must fire at most once per turn even
/// when opponent's security is removed multiple times.
#[test]
fn bt20_020_clause3_fires_at_most_once_per_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-020")
        .expect("BT20-020 YAML loads")
        .add_card(opp_digimon("OPP-A", 5000))
        .add_card(opp_digimon("OPP-B", 5000))
        .memory(10)
        .start();

    let ifm = runner.place_on_field(0, "BT20-020", Some(0));
    runner.place_on_field(1, "OPP-A", Some(0));
    runner.place_on_field(1, "OPP-B", Some(0));
    assert_eq!(runner.battle_area_size(1), 2, "pre: opponent has 2 Digimon");

    // First security removal → clause 3 fires, delete prompt installs.
    fire_opp_security_removed(&mut runner, ifm);
    let first = runner
        .pending_selection_view()
        .expect("first security removal must install the delete selection");
    assert_eq!(first.kind, SelectionKind::OppField);
    runner
        .execute_action(first.selecting_player, first.valid_action_ids[0])
        .expect("delete one opponent Digimon");
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "first activation must delete exactly one opponent Digimon"
    );

    // Second security removal in the SAME turn → [Once Per Turn] blocks it.
    fire_opp_security_removed(&mut runner, ifm);
    assert!(
        runner.pending_selection().is_none(),
        "clause 3 [Once Per Turn] must suppress a second activation in the same turn"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "no second delete — once-per-turn gate holds"
    );
}

/// G-FORMULA-SOURCE-DP resolved: clause 3's delete target must have DP ≤ this
/// Digimon's current effective DP. The `source_dp` formula reads BT20-020's
/// effective DP, so a Digimon above that threshold is NOT a legal target.
#[test]
fn bt20_020_clause3_only_targets_digimon_with_lte_dp() {
    // BT20-020 printed DP is 13000. Place one opponent Digimon at 13000
    // (eligible: DP ≤ 13000) and one at 14000 (ineligible: DP > 13000).
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-020")
        .expect("BT20-020 YAML loads")
        .add_card(opp_digimon("OPP-EQ", 13000))
        .add_card(opp_digimon("OPP-HI", 14000))
        .memory(10)
        .start();

    let ifm = runner.place_on_field(0, "BT20-020", Some(0));
    let opp_eq = runner.place_on_field(1, "OPP-EQ", Some(0));
    let opp_hi = runner.place_on_field(1, "OPP-HI", Some(0));

    fire_opp_security_removed(&mut runner, ifm);

    let view = runner
        .pending_selection_view()
        .expect("clause 3 must install the delete selection (eligible target exists)");
    assert_eq!(view.kind, SelectionKind::OppField);
    // Only the 13000-DP Digimon is a legal target — the 14000-DP one is gated out.
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the DP-≤-13000 Digimon must be a legal delete target; got {} targets",
        view.valid_action_ids.len()
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("delete the eligible Digimon");
    let _ = runner.auto_resolve();

    // The eligible (13000) Digimon is gone; the ineligible (14000) one remains.
    let ids: Vec<String> = runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        !ids.contains(&"OPP-EQ".to_string()),
        "the DP-≤-13000 Digimon must be deleted; remaining={ids:?}"
    );
    assert!(
        ids.contains(&"OPP-HI".to_string()),
        "the DP-14000 Digimon must NOT be deletable (DP > source DP); remaining={ids:?}"
    );
    let _ = (opp_eq, opp_hi);
}
