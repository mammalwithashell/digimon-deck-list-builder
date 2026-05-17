//! BT24-016 Lamiamon — Lv5, Red, Dragonkin / LIBERATOR.
//!
//! # Card text (cards.json / printed)
//!
//! **[Hand] [Main]** If you have [Owen Dreadnought], by placing 1
//! [Dimetromon] from your trash as any of your [Elizamon]'s bottom
//! digivolution card, it digivolves into this card for a digivolution
//! cost of 3, ignoring digivolution requirements.
//!
//! **[When Digivolving] [When Attacking] [Once Per Turn]** Your
//! opponent places 1 card from their hand as the bottom security card.
//! Then, trash their top security card.
//!
//! **Inherited — [All Turns] [Once Per Turn]** When your opponent's
//! security stack is removed from, you may play 1 5000 DP or lower
//! [Reptile] or [Dragonkin] Digimon card from your hand without paying
//! the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Red/BT24_016.cs
//!
//! # Patterns this test covers
//! - Structural: 2 alt_paths (Digivolve + ActivatedDigivolve), 2 triggered
//!   clauses (own OPT + inherited OPT)
//! - Alt-path gating (activated path): Elizamon on field + Dimetromon in trash
//! - Activated-digivolve extra_cost: select Dimetromon from trash →
//!   place_as_bottom_source under Elizamon → digivolve fires
//! - Clause 3 [WhenDigivolving/WhenAttacking] OPT: as_selecting_player →
//!   place_on_security bottom → trash_top_security
//! - Clause 4 inherited OnOpponentSecurityRemoved OPT: play Reptile/Dragonkin
//!   from hand free
//!
//! # Known gaps
//! - **G-INHERITED-DISPATCH**: inherited `on_opponent_security_removed`
//!   firing is not wired up; behavioral tests for clause 4 are `#[ignore]`.
//! - **G-OPT-TRIGGERED**: OPT lockout enforcement on clause 3 not wired;
//!   lockout tests are `#[ignore]`.
//!
//! # Known card-local gaps
//! - **Owen Dreadnought gate not wired.** `G-ALT-PATH-CONDITION` is
//!   RESOLVED (2026-05-15) — `AltPathSpec.condition` exists and is
//!   consumed by the Digivolve route. BT24-016's YAML does not yet
//!   populate `condition:` on the activated_digivolve path, so the path
//!   is currently available whenever an Elizamon and a Dimetromon-in-
//!   trash are present, regardless of Owen presence. Card-local
//!   authoring follow-up.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPath, CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};

use super::super::dsl_card_data::compiled;

// ── §1 Structural assertions ────────────────────────────────────────────────

/// Lamiamon has exactly two alt_paths: a standard digivolve (Lv4, cost 3) and
/// an activated_digivolve path (Elizamon source, cost 3, ignore_requirements).
#[test]
fn bt24_016_has_two_alt_paths() {
    let card = compiled("BT24-016");
    assert_eq!(
        card.alt_paths.len(),
        2,
        "expected exactly 2 alt_paths (Digivolve + ActivatedDigivolve), got {}",
        card.alt_paths.len()
    );
}

#[test]
fn bt24_016_first_alt_path_is_digivolve_lv4_cost3() {
    let card = compiled("BT24-016");
    let path = &card.alt_paths[0];
    assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(
        path.cost,
        Some(digimon_dsl::compiled::CompiledCost::Literal(3)),
        "standard digivolve path must cost 3"
    );
    assert!(
        path.ignore_requirements == false,
        "standard digivolve path must NOT ignore requirements"
    );
}

#[test]
fn bt24_016_second_alt_path_is_activated_digivolve() {
    let card = compiled("BT24-016");
    let path = &card.alt_paths[1];
    assert_eq!(path.kind, CompiledAltPathKind::ActivatedDigivolve);
    assert_eq!(
        path.cost,
        Some(digimon_dsl::compiled::CompiledCost::Literal(3)),
        "activated_digivolve path must cost 3"
    );
    assert!(
        path.ignore_requirements,
        "activated_digivolve path must ignore digivolution requirements"
    );
    assert!(
        !path.extra_cost.is_empty(),
        "activated_digivolve path must carry an extra_cost block (select Dimetromon + place_as_bottom_source)"
    );
}

/// Card has exactly 2 triggered effects: one own-scope OPT on
/// [WhenDigivolving, WhenAttacking] and one inherited OPT on
/// OnOpponentSecurityRemoved.
#[test]
fn bt24_016_has_exactly_two_triggered_clauses() {
    let card = compiled("BT24-016");
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
        "expected 2 triggered clauses, got {}: {:?}",
        triggered.len(),
        triggered.iter().map(|t| &t.when).collect::<Vec<_>>()
    );
}

#[test]
fn bt24_016_own_triggered_clause_has_correct_shape() {
    let card = compiled("BT24-016");
    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    // Clause 3 is the first (index 0) triggered clause.
    let clause3 = triggered
        .iter()
        .find(|t| {
            t.when.contains(&CompiledTiming::WhenDigivolving)
                && t.when.contains(&CompiledTiming::WhenAttacking)
        })
        .expect("clause 3 must be present: [WhenDigivolving, WhenAttacking]");
    assert_eq!(
        clause3.scope,
        CompiledScope::FaceUp,
        "clause 3 is own-scope"
    );
    assert!(clause3.once_per_turn, "clause 3 must be OPT");
    assert!(
        !clause3.optional,
        "clause 3 is NOT optional (opponent is forced to place a card)"
    );
}

#[test]
fn bt24_016_inherited_clause_has_correct_shape() {
    let card = compiled("BT24-016");
    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    let clause4 = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnOpponentSecurityRemoved))
        .expect("clause 4 must be present: OnOpponentSecurityRemoved");
    assert_eq!(
        clause4.scope,
        CompiledScope::Inherited,
        "clause 4 must be inherited scope"
    );
    assert!(clause4.once_per_turn, "clause 4 must be OPT");
    assert!(clause4.optional, "clause 4 is optional (\"you may play\")");
    // active_when must be set (all_turns: true)
    assert!(
        clause4.active_when.is_some(),
        "clause 4 active_when must be set to all_turns"
    );
}

// ── §2 Alt-path gating ──────────────────────────────────────────────────────

/// The `activated_digivolve` `from:` filter targets `name_contains: "Elizamon"`.
/// When no Elizamon is on the controller's field, no valid source exists and
/// the path cannot be offered. (Behavioral gating test — asserts the `from`
/// predicate carries the name filter, not the engine gating itself.)
#[test]
fn bt24_016_activated_path_targets_elizamon_by_name() {
    let card = compiled("BT24-016");
    let path = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::ActivatedDigivolve)
        .expect("ActivatedDigivolve path must be present");
    let from_pred = path
        .from
        .as_ref()
        .expect("activated_digivolve path must carry a `from:` predicate");
    assert_eq!(
        from_pred.name_contains.as_deref(),
        Some("Elizamon"),
        "from filter must target name_contains: Elizamon"
    );
}

/// The `extra_cost` block has exactly 2 steps (select_trash → place_as_bottom_source).
#[test]
fn bt24_016_activated_path_extra_cost_has_two_steps() {
    let card = compiled("BT24-016");
    let path = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::ActivatedDigivolve)
        .expect("ActivatedDigivolve path must be present");
    assert_eq!(
        path.extra_cost.len(),
        2,
        "extra_cost must have exactly 2 steps: SelectTrash + PlaceAsBottomSource"
    );
}

// ── §3 Clause 3 behavioral (WhenDigivolving / WhenAttacking OPT) ────────────
//
// Full behavioral engine tests require:
//   - WhenAttacking timing dispatch (G-WHEN-ATTACKING)
//   - as_selecting_player scope routing (G-AS-SELECTING-PLAYER)
//   - place_on_security primitive (G-PLACE-ON-SECURITY)
//
// Until those gaps close, the engine-level tests are `#[ignore]`.

/// Positive: after digivolving into Lamiamon the opponent is prompted to
/// place a card, and their top security is trashed. Requires WhenDigivolving
/// dispatch, as_selecting_player, place_on_security + trash_top_security.
#[test]
#[ignore = "pending: G-AS-SELECTING-PLAYER (test body not written; OPT closed in Phase 2 Track C)"]
fn bt24_016_when_digivolving_opponent_places_and_top_security_trashed() {
    // Scaffolding (to be filled once the gaps close):
    //
    //   let mut runner = DebugRunner::builder()
    //       .dsl_card("BT24-016")   // Lamiamon — in _examples pack
    //       .dsl_card("BT21-008")   // Elizamon (Lv4) as digivolve base
    //       .memory(20)
    //       .hand(0, &["BT24-016"])
    //       .start();
    //   let elizamon_idx = runner.place_on_field(0, "BT21-008", Some(0));
    //   // Opponent has 3 security + 1 hand card.
    //   runner.game.players[1].security = vec![/* 3 cards */];
    //   let opp_hand_before = runner.hand_size(1);
    //   let sec_count_before = runner.security_count(1);
    //
    //   // Digivolve Lamiamon onto Elizamon.
    //   runner.digivolve(0, elizamon_idx, 0).expect("digivolve succeeds");
    //
    //   // Engine must install a pending_selection on the OPPONENT (place card).
    //   let pending = runner.pending_selection().expect("placement prompt installs");
    //   assert_eq!(pending.selecting_player, 1, "opponent selects");
    //   runner.execute_action(pending.valid_action_ids[0]); // pick first hand card
    //
    //   // After resolving placement the engine must trash top security automatically.
    //   assert_eq!(runner.security_count(1), sec_count_before, "net security unchanged
    //       (added bottom, trashed top => same count)");
    //   assert_eq!(runner.hand_size(1), opp_hand_before - 1, "opponent lost 1 hand card");
    todo!("implement once G-OPT-TRIGGERED + G-AS-SELECTING-PLAYER gaps close");
}

/// OPT lockout: clause 3 must not fire twice in the same turn even if both
/// WhenDigivolving and WhenAttacking trigger in the same turn.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED"]
fn bt24_016_clause3_opt_fires_at_most_once_per_turn() {
    todo!("implement once G-OPT-TRIGGERED OPT lockout enforcement lands");
}

// ── §4 Clause 4 behavioral (Inherited OnOpponentSecurityRemoved OPT) ────────
//
// Requires G-INHERITED-DISPATCH (inherited clause wiring) +
// OnOpponentSecurityRemoved observer firing. Both are currently gaps.

/// Positive: when the opponent's security is removed from, the controller may
/// play a qualifying Reptile or Dragonkin Digimon (DP ≤ 5000) free from hand.
#[test]
#[ignore = "pending: G-INHERITED-DISPATCH"]
fn bt24_016_inherited_on_security_removed_play_free_positive() {
    // Scaffolding:
    //
    //   let mut runner = DebugRunner::builder()
    //       .dsl_card("BT24-016")
    //       .dsl_card("SMALL-REPTILE")  // Lv3, Reptile, DP 3000
    //       .memory(20)
    //       .hand(0, &["SMALL-REPTILE"])
    //       .start();
    //   runner.place_on_field(0, "BT24-016", Some(0)); // Lamiamon on field
    //   // Trigger the inherited: remove an opponent security card.
    //   runner.fire_security_loss(1); // helper fires OnOpponentSecurityRemoved
    //
    //   // Optional prompt should install.
    //   assert!(runner.pending_selection().is_some(), "free-play prompt installs");
    //   runner.execute_action(/* accept action */);
    //
    //   assert_eq!(runner.battle_area_size(0), 2, "Reptile played to field free");
    //   assert_eq!(runner.hand_size(0), 0, "hand card consumed");
    todo!("implement once G-INHERITED-DISPATCH + OnOpponentSecurityRemoved fire");
}

/// Negative: if no qualifying Digimon (Reptile/Dragonkin, DP ≤ 5000) is in
/// hand, the optional prompt must still install but offer no valid targets
/// (or not install if the engine skips empty selections).
#[test]
#[ignore = "pending: G-INHERITED-DISPATCH"]
fn bt24_016_inherited_on_security_removed_no_target_no_selection() {
    // Scaffolding:
    //
    //   // Hand has only a high-DP Digimon or a non-qualifying card.
    //   // After security removal, the optional prompt either does not install
    //   // (empty target set) or installs with PASS-only.
    //   assert!(
    //       runner.pending_selection().is_none()
    //           || runner.pending_is_optional(),
    //       "no valid target → no forced selection"
    //   );
    todo!("implement once G-INHERITED-DISPATCH + OnOpponentSecurityRemoved fire");
}

/// The inherited clause must not fire when the Digimon carrying this inherited
/// effect has left the field.
#[test]
#[ignore = "pending: G-INHERITED-DISPATCH"]
fn bt24_016_inherited_does_not_fire_after_source_leaves_field() {
    todo!("implement once G-INHERITED-DISPATCH lands");
}

/// OPT lockout on inherited clause: must not fire a second time in the same
/// turn even if the opponent loses multiple security cards.
#[test]
#[ignore = "pending: G-INHERITED-DISPATCH (test body not written; OPT closed in Phase 2 Track C)"]
fn bt24_016_inherited_clause_opt_fires_at_most_once_per_turn() {
    todo!("implement once G-INHERITED-DISPATCH + G-OPT-TRIGGERED land");
}
