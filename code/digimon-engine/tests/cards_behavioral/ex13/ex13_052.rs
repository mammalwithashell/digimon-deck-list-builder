//! EX13-052 Gladimon — Digimon, Lv.4, Black, 4000 DP, cost 4.
//! Traits: Warrior. Champion / Vaccine. Digivolve: Black Lv.3 cost 2.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-052.md`)
//!
//! <Guard> (When any of your other Digimon would leave the battle area by your
//! opponent's effects, by deleting this Digimon, they don't leave.) [On Play]
//! [On Deletion] <De-Digivolve 1> 1 of your opponent's Digimon.
//!
//! Inherited Effect: [All Turns] [Once Per Turn] When this Digimon would leave
//! the battle area other than by your effects, by deleting 1 of your other
//! Digimon with [Knightmon] in its text, it doesn't leave.
//!
//! # DCGO C# reference
//! None at b9a0638cd (no `EX13_052.cs`); printed text + general_rule.pdf govern.
//! <Guard> is the engine's printed keyword (`cards/keyword_effects.rs`); the
//! inherited replacement mirrors EX13-027 (shipped) with a [Knightmon]-text
//! cost filter.
//!
//! # Patterns
//! - H: printed <Guard>.  - F2: [On Play]/[On Deletion] <De-Digivolve 1>.
//! - F4 + G4: inherited OPT would-leave replacement with a delete cost.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{
    CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-052";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-052 YAML loads")
        .add_card(named("KNIGHT", "Knightmon", CardColor::Black, 5, 7000))
        .add_card(with_text(
            named("KNIGHT-TEXT", "Pawnmon", CardColor::Black, 3, 3000),
            "Digivolve into a card with [Knightmon] in its text.",
        ))
        .add_card(named("PLAIN", "Plainmon", CardColor::Black, 4, 5000))
        .add_card(named("TOP", "Topmon", CardColor::Black, 5, 7000))
        .add_card(named("OPP-LV3", "OppThree", CardColor::Red, 3, 3000))
        .add_card(named("OPP-LV4", "OppFour", CardColor::Red, 4, 5000))
        .add_card(named("FILL", "Filler", CardColor::Black, 3, 3000))
}

fn start(hand: &[&str]) -> DebugRunner {
    builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_052_printed_metadata_guard_and_clause_shape() {
    let mut r = start(&[]);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(4), Some(4000), Some(4)));
    assert_eq!(c.color, vec![CompiledColor::Black]);
    assert_eq!(c.traits, vec!["Warrior".to_string()]);
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 1);
    assert!(t[0].when.contains(&CompiledTiming::OnPlay));
    assert!(t[0].when.contains(&CompiledTiming::OnDeletion));
    let reps: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                scope,
                optional,
                once_per_turn,
                ..
            }) => Some((*scope, *optional, *once_per_turn)),
            _ => None,
        })
        .collect();
    assert_eq!(reps, vec![(CompiledScope::Inherited, true, true)]);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Guard), "<Guard>");
}

// ─── Section 3 — [On Play] / [On Deletion] ───────────────────────────────────

#[test]
fn ex13_052_on_play_de_digivolves_1() {
    let mut r = start(&[CARD_ID]);
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    play_card(&mut r, 0, CARD_ID);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, opp), "OPP-LV3");
}

#[test]
fn ex13_052_on_deletion_de_digivolves_1() {
    let mut r = start(&[]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    delete_by_opponent_effect(&mut r, me);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, opp), "OPP-LV3");
}

#[test]
fn ex13_052_guard_deletes_gladimon_to_save_an_ally_then_on_deletion_fires() {
    let mut r = start(&[]);
    r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "PLAIN", Some(0));
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    delete_with(&mut r, ally, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    accept(&mut r);
    // Gladimon is deleted by <Guard> → its [On Deletion] De-Digivolve.
    if r.pending_kind() == Some(SelectionKind::OppField) {
        pick_field(&mut r, opp);
    }
    let _ = r.auto_resolve();
    let field = field_ids(&r, 0);
    assert!(field.contains(&"PLAIN".to_string()), "ally doesn't leave");
    assert!(!field.contains(&CARD_ID.to_string()), "Gladimon deleted as the <Guard> cost");
    assert_eq!(top_id(&r, opp), "OPP-LV3", "[On Deletion] <De-Digivolve 1>");
}

// ─── Section 2/3/5 — inherited replacement ───────────────────────────────────

fn ex13_052_inherited_setup() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut r = start(&[]);
    let carrier = r.place_stack(0, &[CARD_ID, "TOP"]);
    (r, carrier)
}

#[test]
fn ex13_052_inherited_deletes_other_knightmon_text_digimon_and_carrier_stays() {
    let (mut r, carrier) = ex13_052_inherited_setup();
    let knight = r.place_on_field(0, "KNIGHT-TEXT", Some(0));
    r.place_on_field(0, "PLAIN", Some(0));
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    accept(&mut r);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OwnField));
    assert_eq!(legal(&r).len(), 1, "only the [Knightmon]-text Digimon");
    pick_field(&mut r, knight);
    let _ = r.auto_resolve();
    let field = field_ids(&r, 0);
    assert!(field.contains(&"TOP".to_string()), "carrier stays");
    assert!(!field.contains(&"KNIGHT-TEXT".to_string()), "cost Digimon deleted");
    assert!(field.contains(&"PLAIN".to_string()));
}

#[test]
fn ex13_052_inherited_battle_removal_is_covered() {
    let (mut r, carrier) = ex13_052_inherited_setup();
    r.place_on_field(0, "KNIGHT", Some(0));
    delete_with(&mut r, carrier, ReplacementCause::Battle);
    assert_replacement_prompt(&r);
}

#[test]
fn ex13_052_inherited_not_offered_without_other_knightmon() {
    let (mut r, carrier) = ex13_052_inherited_setup();
    r.place_on_field(0, "PLAIN", Some(0));
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"TOP".to_string()), "carrier leaves");
    assert!(field_ids(&r, 0).contains(&"PLAIN".to_string()));
}

#[test]
fn ex13_052_inherited_not_offered_for_own_effect_removal() {
    let (mut r, carrier) = ex13_052_inherited_setup();
    r.place_on_field(0, "KNIGHT", Some(0));
    delete_with(&mut r, carrier, ReplacementCause::OwnEffect);
    assert!(r.pending_selection().is_none(), "'other than by your effects'");
    assert!(!field_ids(&r, 0).contains(&"TOP".to_string()));
}

#[test]
fn ex13_052_inherited_decline_lets_carrier_leave() {
    let (mut r, carrier) = ex13_052_inherited_setup();
    r.place_on_field(0, "KNIGHT", Some(0));
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    decline(&mut r);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"TOP".to_string()));
    assert!(field_ids(&r, 0).contains(&"KNIGHT".to_string()));
}

#[test]
fn ex13_052_inherited_protects_only_the_carrier() {
    let (mut r, _carrier) = ex13_052_inherited_setup();
    r.place_on_field(0, "KNIGHT", Some(0));
    let other = r.place_on_field(0, "PLAIN", Some(0));
    delete_with(&mut r, other, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection().is_none(), "'this Digimon' only");
}

#[test]
fn ex13_052_inherited_once_per_turn() {
    let (mut r, carrier) = ex13_052_inherited_setup();
    let k1 = r.place_on_field(0, "KNIGHT", Some(0));
    r.place_on_field(0, "KNIGHT-TEXT", Some(0));
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    accept(&mut r);
    pick_field(&mut r, k1);
    let _ = r.auto_resolve();
    let carrier = handle_of(&r, 0, "TOP");
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"TOP".to_string()), "OPT spent → carrier leaves");
}

// ─── Inherited [OPT]: DCGO parity (owner ruling 2026-11-23) ──────────────────
//
// DCGO EX13_052.cs: offered only while another of your [Knightmon]-text
// Digimon exists (`CanActivateCondition`); after the accept, a
// `SelectPermanentEffect` with canNoSelect true. None chosen -> `RemoveUse()`
// and the carrier leaves. One chosen -> spent; the carrier stays only if it
// was deleted. qa/rules/mechanics-gaps-2026-10.md W14 row (f). <Evade> on the
// carrier keeps it after the would-leave window so the use can be observed on
// a later removal the same turn.

fn grant_evade(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle) {
    r.game.modifiers.grant_keyword(
        h,
        Keyword::Evade,
        digimon_engine::enums::Expiry::Permanent,
        0,
    );
}

/// While <Evade> can also answer the deletion, the engine first asks which
/// immediate replacement effect applies next (CR 4.3 15-8-5); pick <Evade>
/// (`evade`) or Gladimon's inherited clause. With one candidate left, its own
/// accept prompt is already pending.
fn pick_replacement(r: &mut DebugRunner, evade: bool) {
    let v = r.pending_selection_view().expect("replacement prompt");
    if v.kind == SelectionKind::EffectChoice && v.prompt.contains("immediate replacement") {
        let row = v
            .effect_choices
            .as_ref()
            .and_then(|rows| rows.iter().find(|c| c.label.contains("Evade") == evade))
            .unwrap_or_else(|| panic!("replacement order rows: {:?}", v.effect_choices))
            .action_id;
        r.execute_action(v.selecting_player, row).expect("order pick");
    }
    assert_replacement_prompt(r);
}

/// PASS is legal at the pending prompt (the action mask, not the raw ids).
fn pass_legal(r: &DebugRunner) -> bool {
    let p = r.pending_selection().expect("prompt").selecting_player;
    digimon_engine::action::build_action_mask(&r.game, p)
        [digimon_engine::action::space::PASS as usize]
        == 1.0
}

fn ex13_052_accept_evade(r: &mut DebugRunner) {
    pick_replacement(r, true);
    ex13_052_accept(r);
    assert!(
        r.pending_selection().is_none(),
        "the same removal does not offer the declined clause again"
    );
}

#[test]
fn ex13_052_inherited_outer_decline_keeps_opt() {
    let (mut r, carrier) = ex13_052_inherited_setup();
    r.place_on_field(0, "KNIGHT", Some(0));
    grant_evade(&mut r, carrier);
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    pick_replacement(&mut r, false);
    ex13_052_decline(&mut r);
    ex13_052_accept_evade(&mut r);
    let carrier = handle_of(&r, 0, "TOP");
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Replacement),
        "declined at the accept prompt: offered again this turn"
    );
}

#[test]
fn ex13_052_inherited_accept_then_decline_cost_keeps_opt() {
    let (mut r, carrier) = ex13_052_inherited_setup();
    r.place_on_field(0, "KNIGHT", Some(0));
    grant_evade(&mut r, carrier);
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    pick_replacement(&mut r, false);
    ex13_052_accept(&mut r);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OwnField), "cost pick");
    assert!(
        pass_legal(&r),
        "DCGO EX13_052.cs canNoSelect: the deletion may be declined after the accept"
    );
    ex13_052_decline(&mut r);
    assert!(field_ids(&r, 0).contains(&"KNIGHT".to_string()), "nothing deleted");
    ex13_052_accept_evade(&mut r);
    let carrier = handle_of(&r, 0, "TOP");
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Replacement),
        "DCGO RemoveUse: the declined cost keeps the [Once Per Turn]"
    );
}

#[test]
fn ex13_052_inherited_declined_cost_lets_carrier_leave() {
    let (mut r, carrier) = ex13_052_inherited_setup();
    r.place_on_field(0, "KNIGHT", Some(0));
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    ex13_052_accept(&mut r);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OwnField), "cost pick");
    assert!(pass_legal(&r), "the cost may be declined");
    ex13_052_decline(&mut r);
    assert!(r.pending_selection().is_none(), "no decision is left");
    assert!(!field_ids(&r, 0).contains(&"TOP".to_string()), "not paid: the carrier leaves");
    assert!(field_ids(&r, 0).contains(&"KNIGHT".to_string()));
}

// ─── Inherited: an undeletable payer is still offered (owner ruling 2026-11-24) ─
//
// DCGO EX13_052.cs offers the clause whenever another of your [Knightmon]-text
// Digimon exists (`CanActivateCondition` = `HasMatchConditionPermanent`, no
// deletability check) and its pick lists every printed match. A chosen payer
// keeps the use spent (`selectedPermanent != null`, no RemoveUse); a payer
// that can't be deleted is filtered out by `DestroyPermanentsClass`, nothing
// is deleted, `SuccessProcess` never runs and the carrier leaves. Owner,
// 2026-11-24: "You can pick a target that is immune from the removal, and it
// will fizzle. Nothing is deleted, right? That is a valid choice."
//
// The payer is your own [Knightmon]-text Digimon under a cause-agnostic
// "can't be deleted by effects" (Craniamon (BT3-075)'s shape).

fn protect_from_all_effects(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle) {
    r.game.modifiers.add(
        h,
        digimon_engine::modifiers::ModifierEntry::simple(
            digimon_engine::enums::ModifierType::CannotBeDestroyedByEffect,
            0,
            digimon_engine::enums::Expiry::Permanent,
            h.player,
        ),
    );
}

fn own_knight_protected(r: &mut DebugRunner) -> digimon_engine::permanent::PermanentHandle {
    let h = r.place_on_field(0, "KNIGHT-TEXT", Some(0));
    protect_from_all_effects(r, h);
    h
}

#[test]
fn ex13_052_inherited_offered_when_the_only_match_is_undeletable() {
    let (mut r, carrier) = ex13_052_inherited_setup();
    let k = own_knight_protected(&mut r);
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    ex13_052_accept(&mut r);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OwnField), "cost pick");
    assert!(field_offered(&r, k), "the undeletable [Knightmon]-text Digimon is listed");
}

#[test]
fn ex13_052_inherited_picking_an_undeletable_payer_spends_opt_and_deletes_nothing() {
    let (mut r, carrier) = ex13_052_inherited_setup();
    let k = own_knight_protected(&mut r);
    grant_evade(&mut r, carrier);
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    pick_replacement(&mut r, false);
    ex13_052_accept(&mut r);
    pick_field(&mut r, k);
    assert!(field_ids(&r, 0).contains(&"KNIGHT-TEXT".to_string()), "nothing was deleted");
    ex13_052_accept_evade(&mut r);
    let carrier = handle_of(&r, 0, "TOP");
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    assert!(
        r.pending_selection().is_none(),
        "DCGO: a Digimon was chosen, so the [Once Per Turn] stays spent"
    );
    assert!(!field_ids(&r, 0).contains(&"TOP".to_string()));
}

#[test]
fn ex13_052_inherited_picking_an_undeletable_payer_lets_the_carrier_leave() {
    let (mut r, carrier) = ex13_052_inherited_setup();
    let k = own_knight_protected(&mut r);
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    ex13_052_accept(&mut r);
    pick_field(&mut r, k);
    let _ = r.auto_resolve();
    let field = field_ids(&r, 0);
    assert!(!field.contains(&"TOP".to_string()), "not paid: the carrier leaves");
    assert!(field.contains(&"KNIGHT-TEXT".to_string()), "the payer was not deleted");
}

#[test]
fn ex13_052_inherited_undeletable_only_pass_keeps_opt() {
    let (mut r, carrier) = ex13_052_inherited_setup();
    own_knight_protected(&mut r);
    grant_evade(&mut r, carrier);
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    pick_replacement(&mut r, false);
    ex13_052_accept(&mut r);
    assert!(pass_legal(&r), "canNoSelect");
    ex13_052_decline(&mut r);
    assert!(field_ids(&r, 0).contains(&"KNIGHT-TEXT".to_string()));
    ex13_052_accept_evade(&mut r);
    let carrier = handle_of(&r, 0, "TOP");
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Replacement),
        "DCGO RemoveUse: nothing chosen keeps the [Once Per Turn]"
    );
}

#[test]
fn ex13_052_inherited_deletable_payer_still_protects_beside_an_undeletable_one() {
    let (mut r, carrier) = ex13_052_inherited_setup();
    let k = own_knight_protected(&mut r);
    let other = r.place_on_field(0, "KNIGHT", Some(0));
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    ex13_052_accept(&mut r);
    assert!(field_offered(&r, k));
    assert!(field_offered(&r, other));
    assert_eq!(legal(&r).len(), 2);
    pick_field(&mut r, other);
    let _ = r.auto_resolve();
    let field = field_ids(&r, 0);
    assert!(field.contains(&"TOP".to_string()), "paid: the carrier doesn't leave");
    assert!(!field.contains(&"KNIGHT".to_string()), "the payer was deleted");
    assert!(field.contains(&"KNIGHT-TEXT".to_string()));
}

/// Accept the pending optional prompt (its one non-PASS action).
fn ex13_052_accept(r: &mut DebugRunner) {
    let view = r.pending_selection_view().expect("optional prompt pending");
    let a = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != digimon_engine::action::space::PASS)
        .expect("an accept action");
    r.execute_action(view.selecting_player, a).expect("accept");
}

/// Decline the pending optional prompt.
fn ex13_052_decline(r: &mut DebugRunner) {
    let view = r.pending_selection_view().expect("optional prompt pending");
    r.execute_action(view.selecting_player, digimon_engine::action::space::PASS)
        .expect("decline");
}
