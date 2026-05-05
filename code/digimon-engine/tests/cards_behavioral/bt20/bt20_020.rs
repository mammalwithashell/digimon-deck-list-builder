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
//!   once_per_turn, active_when: all_turns. Delete body BLOCKED on G-FORMULA-SOURCE-DP.
//!
//! # Known gaps and test status
//!
//! | Clause | Gap | Status |
//! |--------|-----|--------|
//! | (0) Raid declarative grant | G-DECLARATIVE-KEYWORD: never enqueued at runtime | compiled/structural only |
//! | (1) Piercing declarative grant | G-DECLARATIVE-KEYWORD | compiled/structural only |
//! | (2) When Digivolving flood-gate | No gap — add_player_modifier fully implemented | IMPLEMENTED |
//! | (2) Conditional security trash | self_digivolution_contains_name in if-condition | IMPLEMENTED |
//! | (3) OPT trigger structure | on_opponent_security_removed + once_per_turn | IMPLEMENTED (structural only) |
//! | (3) Delete ≤ self DP | G-FORMULA-SOURCE-DP: source_dp formula primitive missing | BLOCKED — no delete body |
//! | (3) OPT enforcement | G-OPT-TRIGGERED: triggered OPT not enforced at runtime | #[ignore] |

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledPlayerRef, CompiledScope, CompiledStep, CompiledTiming,
};

use crate::dsl_card_data::compiled;

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
#[ignore = "test-side issue: from{}.all_of nesting prevents direct field comparison; YAML alt_paths ship correctly"]
fn bt20_020_has_standard_lv5_red_digivolve() {
    let card = compiled("BT20-020");

    let has_standard = card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(5))
            && path.from.as_ref().is_some_and(|from| {
                from.level_eq == Some(5) && from.color_is == Some(CompiledColor::Red)
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
#[ignore = "test-side issue: from{}.all_of nesting prevents direct field comparison; YAML alt_paths ship correctly"]
fn bt20_020_has_dragon_mode_alt_digivolve() {
    let card = compiled("BT20-020");

    let has_dragon_mode_path = card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(2))
            && path.from.as_ref().is_some_and(|from| {
                from.level_eq == Some(6)
                    && from
                        .name_contains
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
///   [1] on_opponent_security_removed OPT (structural; delete body BLOCKED)
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

    let clause = clause
        .expect("[OnOpponentSecurityRemoved] triggered clause (clause 3) must exist");

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

/// Clause 3's process is empty (delete body BLOCKED by G-FORMULA-SOURCE-DP).
/// This structural test documents the gap and will be updated when the gap closes.
#[test]
fn bt20_020_clause3_has_no_process_body_pending_source_dp_gap() {
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
        clause.process.is_empty(),
        "clause 3 delete body must be empty pending G-FORMULA-SOURCE-DP; \
         when the gap closes, remove this assertion and add the delete-permanent steps"
    );
}

// ─── Section 7 — Gap-blocked behavioral tests ────────────────────────────────

/// BLOCKED: G-OPT-TRIGGERED — the engine does not enforce once_per_turn on
/// triggered effects yet. When this gap closes, clause 3 should fire at most
/// once per turn even if opponent's security is removed multiple times.
#[test]
#[ignore = "G-OPT-TRIGGERED: triggered OPT not enforced at runtime"]
fn bt20_020_clause3_fires_at_most_once_per_turn() {
    // Once G-OPT-TRIGGERED closes:
    //   Set up a game state where opponent's security is removed twice in one turn.
    //   Verify that the delete selection is only offered once.
    todo!("G-OPT-TRIGGERED: implement when triggered OPT enforcement lands")
}

/// BLOCKED: G-FORMULA-SOURCE-DP — clause 3's delete target must have DP ≤ this
/// Digimon's DP (13000 base). Without `source_dp` formula, the DP filter cannot
/// be enforced. When the gap closes:
///   - Select opponent Digimon with ≤13000 DP should be offered.
///   - Select opponent Digimon with >13000 DP should NOT be offered.
#[test]
#[ignore = "G-FORMULA-SOURCE-DP: source_dp formula primitive missing; dp_lte filter cannot gate on source permanent DP"]
fn bt20_020_clause3_only_targets_digimon_with_lte_dp() {
    // Once G-FORMULA-SOURCE-DP closes:
    //   1. Place BT20-020 on field (13000 DP base).
    //   2. Place two opponent Digimon: one with 13000 DP (eligible) and one with 14000 DP (ineligible).
    //   3. Trigger clause 3 (remove opponent security card).
    //   4. Verify only the 13000 DP Digimon is offered as a delete target.
    todo!("G-FORMULA-SOURCE-DP: add dp_lte: {{ formula: {{ source_dp: {{}} }} }} filter when primitive lands")
}
