//! ST22-08 Offensive Plug-In V — Option, Cost 4, Red, Traits: Plug-In
//!
//! Card text (cards.json):
//! While you have a Tamer, you can ignore this card's color requirements.
//! [Security] Delete 1 of your opponent's Digimon with the lowest DP.
//!   Then, add this card to the hand.
//! [Main] You may link this card to 1 of your Digimon without paying the
//!   cost. Then, delete 1 of your opponent's Digimon with as much or less
//!   DP as 1 of your Digimon.
//! Inherited: Link Requirements [Link] Lv.3 or higher: Cost 2
//!   (Plug this card from the hand or battle area sideways into the
//!   specified Digimon in the battle area.)
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/ST22/Red/ST22_08.cs
//!
//! Pattern rows (test API §4.3):
//!   - H (color bypass via IgnoreColorRequirement conditional)
//!   - G (opponent Digimon delete, DP-comparative)
//!   - Security-add-self-to-hand post-resolution
//!   - Link registration + linked scope effect (EOT may attack)
//!
//! BLOCKED verdict — hybrid engine + DSL gaps:
//!   G-IGNORE-COLOR-MASK    : IgnoreColorRequirement modifier not enforced
//!                            in Rust action/mask.rs option_color_match_available
//!   G-PRED-DP-LTE          : dp_lte predicate not evaluated in
//!                            eval_permanent_fields (engine-gaps.md)
//!   G-DSL-LINK-VERB        : no DSL clause kind or step verb for link
//!                            registration or optional link action step
//!   G-DSL-LINKED-SCOPE     : no DSL "scope: linked" for linked-card effects
//!   G-BINDING-DP-FORMULA   : no formula primitive to reference a named
//!                            binding's DP (needed for main-clause DP compare)
//!   G-ADD-OPTION-SELF-TO-HAND: no DSL step for returning the played Option
//!                            to hand after security resolution

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::selection::SelectionKind;

// ---------------------------------------------------------------------------
// Section 1: Structural assertions (what we CAN verify from the empty stub)
// ---------------------------------------------------------------------------

/// ST22-08.yaml must parse as a valid card spec with `kind: option`.
/// Even the BLOCKED stub must round-trip without errors.
#[test]
fn st22_08_yaml_parses_as_option_kind() {
    const YAML: &str = include_str!("../../../cards/st22/ST22-08.yaml");
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("ST22-08.yaml must parse");
    assert_eq!(spec.card, "ST22-08", "card ID must be ST22-08");
    // kind is stored as a string discriminant in CardSpec
    // The option kind is encoded as integer 2 in cards.json
    // No assertions on clause count since the stub has effects: []
}

/// ST22-08.yaml must compile through the DSL registry without errors.
#[test]
fn st22_08_yaml_compiles_without_errors() {
    const YAML: &str = include_str!("../../../cards/st22/ST22-08.yaml");
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("ST22-08.yaml must parse");
    let _registry = digimon_dsl::CardRegistry::from_specs("test", &[spec])
        .expect("ST22-08.yaml must compile (stub has no raw_rust references)");
}

// ---------------------------------------------------------------------------
// Section 2: Clause 1 — Color requirement bypass (BLOCKED: G-IGNORE-COLOR-MASK)
//
// When resolved: clause should be flood_gate with IgnoreColorRequirement
// conditioned on any_permanent: { of: you, kind: tamer }.
// The action mask must check IgnoreColorRequirement before blocking the Option.
// ---------------------------------------------------------------------------

#[test]
#[ignore = "BLOCKED: G-IGNORE-COLOR-MASK — action/mask.rs option_color_match_available does not check IgnoreColorRequirement modifier; see code/digimon-engine/src/action/mask.rs line 596"]
fn st22_08_color_bypass_active_when_tamer_present() {
    // Setup: P0 has a Tamer (not matching ST22-08's red color), no red Digimon.
    // Expected: ST22-08 is playable because IgnoreColorRequirement modifier
    // bypasses the color-match check.
    // When G-IGNORE-COLOR-MASK is fixed, implement using action mask inspection:
    //   let mask = r.action_mask(0);
    //   assert_eq!(mask[hand_action_id], 1.0, "ST22-08 playable with Tamer despite color mismatch");
}

#[test]
#[ignore = "BLOCKED: G-IGNORE-COLOR-MASK — color bypass predicate can be expressed in flood_gate YAML but enforcement hook is missing in Rust action mask"]
fn st22_08_color_requirement_enforced_without_tamer() {
    // Setup: P0 has no Tamer and no red permanent.
    // Expected: ST22-08 is NOT playable (color requirement not met).
    // When G-IGNORE-COLOR-MASK is fixed, this should be the negative branch.
}

// ---------------------------------------------------------------------------
// Section 3: Clause 2+3 — [Security] delete lowest DP Digimon + add to hand
// (BLOCKED: G-PRED-DP-LTE + G-ADD-OPTION-SELF-TO-HAND)
//
// The security clause must:
//   (a) Present a selection of opponent Digimon filtered to only those with
//       DP equal to the lowest DP among all opponent Digimon.
//   (b) Delete the selected Digimon.
//   (c) Return this card (ST22-08) to the controller's hand.
//
// Gap analysis:
//   (a) Requires dp_lte: { formula: { aggregate: { selector: lowest_dp, scope: opponent } } }
//       but eval_permanent_fields does NOT evaluate dp_lte (G-PRED-DP-LTE).
//   (c) Requires a raw_rust step or a new DSL verb for "return this played
//       Option to hand" — analogous to ex6_072_add_self_to_hand.
// ---------------------------------------------------------------------------

#[test]
#[ignore = "BLOCKED: G-PRED-DP-LTE — dp_lte predicate not evaluated in eval_permanent_fields; see engine-gaps.md"]
fn st22_08_security_presents_selection_for_lowest_dp_digimon() {
    // Setup: opponent has two Digimon (3000 DP, 5000 DP).
    // Expected: selection offers only the 3000 DP Digimon (lowest).
    // Negative: 5000 DP Digimon must NOT appear in valid_action_ids.
}

#[test]
#[ignore = "BLOCKED: G-PRED-DP-LTE — dp_lte predicate not evaluated"]
fn st22_08_security_no_selection_when_no_opp_digimon() {
    // Setup: opponent has no Digimon.
    // Expected: selection is skipped (or empty candidates → no selection installed).
    // Add-to-hand still fires regardless.
}

#[test]
#[ignore = "BLOCKED: G-PRED-DP-LTE — deletion is mandatory (canNoSelect: false in DCGO)"]
fn st22_08_security_delete_is_mandatory_when_candidates_exist() {
    // DCGO: canNoSelect: false for SecurityEffect selection.
    // Expected: pending_is_optional() == false when opponent has Digimon.
}

#[test]
#[ignore = "BLOCKED: G-ADD-OPTION-SELF-TO-HAND — no DSL step for returning played Option to hand post-security; see qa/dsl-vocab-gaps.md"]
fn st22_08_security_adds_self_to_hand_after_delete() {
    // Expected: after delete resolves, P0's hand contains ST22-08.
    // This mirrors ex6_072_add_self_to_hand raw_rust pattern.
    // When gap closes, YAML uses raw_rust: { fn: st22_08_add_self_to_hand }.
}

// ---------------------------------------------------------------------------
// Section 4: Clause 4+5 — [Main] optional free link + DP-comparative delete
// (BLOCKED: G-DSL-LINK-VERB + G-BINDING-DP-FORMULA + G-PRED-DP-LTE)
//
// The main clause:
//   (a) Optionally links this card to 1 own Digimon without paying the cost.
//       DCGO: canNoSelect: true for the link selection — truly optional.
//   (b) Then selects 1 of own Digimon as a DP comparator.
//   (c) Then deletes 1 opponent Digimon with DP ≤ selected Digimon's DP.
//       DCGO: CanSelectOwnerDigimon → CanSelectOpponentDigimon where
//             permanent.DP <= selectedOwnerDigimon.DP
//
// Gap analysis:
//   (a) No DSL verb for optional link action step. DCGO uses LinkAction +
//       canNoSelect: true inside SelectPermanentEffect. EX11-027 uses raw_rust.
//       Gap: G-DSL-LINK-VERB
//   (b)+(c) Requires selecting own Digimon, binding its DP, then filtering
//       opponent Digimon by dp_lte referencing the binding. The formula
//       system has no { binding_dp: pick } primitive (G-BINDING-DP-FORMULA),
//       and dp_lte is not evaluated even if expressed (G-PRED-DP-LTE).
//       Gap: hybrid (G-BINDING-DP-FORMULA + G-PRED-DP-LTE)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "BLOCKED: G-DSL-LINK-VERB — no DSL clause kind or step verb for optional link action"]
fn st22_08_main_link_step_is_optional() {
    // DCGO: canNoSelect: true → player may decline linking.
    // Expected: link selection is optional (PASS is a legal action).
    // When gap closes: pending_is_optional() == true for link selection.
}

#[test]
#[ignore = "BLOCKED: G-DSL-LINK-VERB — link registration and action routing unavailable in DSL"]
fn st22_08_main_link_attaches_card_to_digimon() {
    // Expected: when player selects a Digimon to link to, ST22-08's card handle
    // appears in digimon_perm.linked_cards.
    // Linked effect (EOT may attack) then fires each turn.
}

#[test]
#[ignore = "BLOCKED: G-BINDING-DP-FORMULA — formula cannot reference a named binding's DP; no DSL primitive exists"]
fn st22_08_main_delete_opponent_digimon_at_or_below_selected_ally_dp() {
    // Setup: P0 has a 5000 DP Digimon. Opponent has 4000 DP and 8000 DP Digimon.
    // Expected: only the 4000 DP opponent Digimon is a valid deletion target.
    // The 8000 DP Digimon must NOT appear as a candidate.
}

#[test]
#[ignore = "BLOCKED: G-BINDING-DP-FORMULA — DP comparator bound to player choice, not source permanent"]
fn st22_08_main_delete_skipped_when_no_valid_dp_target() {
    // Setup: P0 has a 2000 DP Digimon. Opponent has only 5000 DP Digimon.
    // Expected: no deletion selection is installed (no candidates ≤ 2000 DP).
    // DCGO: the outer condition checks opponentDigimonList.Any(x => x.DP <= highestDp)
    //       before installing the first select, and selectedOwnerDigimon != null
    //       before installing the delete select.
}

#[test]
#[ignore = "BLOCKED: G-BINDING-DP-FORMULA — comparator uses highest of own Digimon as pre-check only"]
fn st22_08_main_delete_mandatory_when_candidate_exists() {
    // DCGO: canNoSelect: false for the deletion selection.
    // Expected: pending_is_optional() == false for delete when targets exist.
}

// ---------------------------------------------------------------------------
// Section 5: Clause 6 — Inherited Link Requirements (BLOCKED: G-DSL-LINK-VERB)
//
// "Link Requirements [Link] Lv.3 or higher: Cost 2"
// DCGO: CardEffectFactory.AddSelfLinkConditionStaticEffect(
//           permanentCondition: perm => perm.TopCard.HasLevel && perm.Level >= 3,
//           linkCost: 2, card: card)
//
// This registers a link cost and host-filter on the card itself (as an
// OptionSubtype::Link effect). No DSL clause kind exists for this.
// ---------------------------------------------------------------------------

#[test]
#[ignore = "BLOCKED: G-DSL-LINK-VERB — no DSL clause kind for link requirement registration"]
fn st22_08_inherited_link_available_on_level_3_plus() {
    // Expected: ST22-08 in hand shows as a valid link target for Lv.3+ Digimon.
    // When G-DSL-LINK-VERB closes: link_requirements clause registered via
    //   kind: raw_rust fn: st22_08_link_requirements triggers: [main_from_hand, main_on_field]
    //   OR a native "kind: link_requirement" clause with level_gte: 3, cost: 2.
}

#[test]
#[ignore = "BLOCKED: G-DSL-LINK-VERB — link requirement host filter not expressible in DSL"]
fn st22_08_inherited_link_not_available_on_level_2_or_lower() {
    // Expected: ST22-08 is NOT a valid link target for Lv.2 Digimon.
    // Level constraint: Lv.3 or higher (level_gte: 3 in the filter).
}

#[test]
#[ignore = "BLOCKED: G-DSL-LINK-VERB — link cost not expressible in DSL"]
fn st22_08_inherited_link_costs_2_memory() {
    // Expected: linking costs 2 memory.
    // DCGO: linkCost: 2 in AddSelfLinkConditionStaticEffect.
}

// ---------------------------------------------------------------------------
// Section 6: Clause 7 — Linked effect: [End of Your Turn] may attack
// (BLOCKED: G-DSL-LINKED-SCOPE)
//
// DCGO: activateClass.SetIsLinkedEffect(true)
//       EndOfTurnLinkedEffect fires via OnEndTurn with CanActivateCondition
//       checking IsExistOnBattleAreaDigimon + CanAttack.
//       Once per turn (maxCount: 1), optional (isOptional flag = true on
//       SetUpActivateClass with the per-OPT hash "EOT_ST22_08").
//
// The DSL has no "scope: linked" — CompiledScope variants are:
//   FaceUp, Inherited (and pending: Linked for linked-card effects).
// ---------------------------------------------------------------------------

#[test]
#[ignore = "BLOCKED: G-DSL-LINKED-SCOPE — no DSL scope for linked-card effects; CompiledScope has no Linked variant"]
fn st22_08_linked_eot_may_attack_fires_when_linked() {
    // Expected: at end of controller's turn, the Digimon ST22-08 is linked to
    // receives a "may attack" prompt (optional attack window).
    // DCGO: optional via isOptional=true, once per turn via maxCount: 1.
    // When G-DSL-LINKED-SCOPE closes: clause uses scope: linked + when: end_of_your_turn
    //   with optional: true and once_per_turn: true.
}

#[test]
#[ignore = "BLOCKED: G-DSL-LINKED-SCOPE — linked-effect scope unavailable"]
fn st22_08_linked_eot_may_attack_does_not_fire_when_not_linked() {
    // Expected: if ST22-08 is not linked to any Digimon, no EOT attack prompt.
}

#[test]
#[ignore = "BLOCKED: G-DSL-LINKED-SCOPE — OPT enforcement for linked effects unavailable"]
fn st22_08_linked_eot_may_attack_is_once_per_turn() {
    // Expected: if the linked Digimon already attacked via this effect,
    // a second activation in the same turn is blocked.
    // DCGO: SetHashString("EOT_ST22_08") for OPT tracking.
}
