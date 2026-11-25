//! EX13-028 Sukamon — Digimon, Lv.4, Yellow, DP 2000, Cost 3.
//! Traits: Abnormal. Form: Champion. Attribute: Virus.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-028.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-028.md` agrees)
//!
//! ```text
//! Digivolve: Yellow Lv.3 / cost 2;  Black Lv.3 / cost 2
//!
//! <Blocker> (This Digimon can block in the blocker timing.)
//! [On Deletion] Reveal the top 3 cards of your deck. You may play 1 play cost
//! 3 or lower Digimon card with [Chuumon] or [Sukamon] in its name among them
//! without paying the cost. Trash the rest.
//!
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When this Digimon would leave the battle area
//! other than by your effects, by deleting 1 other Digimon with [Sukamon] in
//! its name, it doesn't leave.
//! ```
//!
//! # DCGO C# reference
//! None — DCGO has no `EX13_028.cs` at `b9a0638cd`. The [On Deletion] clause
//! is the EX13-059 BigMamemon reveal/may-play/trash-rest shape; the inherited
//! is EX13-027 Chuumon's (identical text).
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - Keyword grant (<Blocker>).
//! - [On Deletion] reveal-3, optional free play from reveal, trash the rest.
//! - F3 inherited leave-prevention replacement, OPT, select-and-delete cost.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::enums::Keyword;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-028";

fn digimon(id: &str, name: &str, level: u8, cost: u16) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = cost;
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-028 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("CHUU-3", "Chuumon", 3, 3))
        .add_card(digimon("SUKA-3", "Sukamon Jr", 4, 3))
        .add_card(digimon("SUKA-4", "Big Sukamon", 4, 4))
        .add_card(digimon("PLAIN-3", "Plainmon", 3, 3))
        .add_card({
            let mut c = make_test_card("SUKA-OPT", "Sukamon Option");
            c.card_kind = CardKind::Option;
            c.level = None;
            c.dp = None;
            c.play_cost = 1;
            c
        })
        .add_card(digimon("FILL", "Filler", 3, 3))
        .add_card(digimon("CARRIER", "Carrier", 5, 5))
        .add_card(digimon("OTHER-SUKA", "Sukamon Ally", 4, 3))
        .add_card(digimon("OTHER-PLAIN", "Plain Ally", 4, 3))
        .deck(1, &["FILL"; 6])
}

fn field_ids(r: &DebugRunner, player: usize) -> Vec<String> {
    r.game.players[player]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn trash_ids(r: &DebugRunner) -> Vec<String> {
    let mut v: Vec<String> = r.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    v.sort();
    v
}

fn handle_of(r: &DebugRunner, id: &str) -> PermanentHandle {
    let i = field_ids(r, 0)
        .iter()
        .position(|c| c == id)
        .unwrap_or_else(|| panic!("{id} on field"));
    PermanentHandle {
        player: 0,
        index: i as u8,
    }
}

fn offered_reveal_ids(r: &DebugRunner) -> Vec<String> {
    let view = r.pending_selection_view().expect("reveal prompt pending");
    let mut ids: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| a.checked_sub(SEL_REVEAL_START))
        .filter_map(|i| r.game.revealed_cards.get(i as usize))
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    ids.sort();
    ids
}

fn pick_revealed(r: &mut DebugRunner, card_id: &str) {
    let view = r.pending_selection_view().expect("reveal prompt pending");
    let want = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .find(|&a| {
            a.checked_sub(SEL_REVEAL_START)
                .and_then(|i| r.game.revealed_cards.get(i as usize))
                .is_some_and(|c| c.card_id(&r.game.card_data) == card_id)
        })
        .unwrap_or_else(|| panic!("{card_id} must be a legal pick: {view:?}"));
    r.execute_action(view.selecting_player, want)
        .expect("pick revealed card");
}

/// Sukamon on the field; deck top 3 = `top3` (last element = top of deck).
fn deletion_setup(top3_bottom_first: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut deck = vec!["FILL", "FILL"];
    deck.extend_from_slice(top3_bottom_first);
    let mut r = builder().deck(0, &deck).memory(0).start();
    r.skip_mulligan();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    (r, h)
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_028_printed_metadata_and_digivolve_paths() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(4), Some(2000), Some(3)));
    assert_eq!(c.color, vec![CompiledColor::Yellow]);
    assert_eq!(c.traits, vec!["Abnormal".to_string()]);
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 2, "Yellow Lv.3 and Black Lv.3 circles");
    for p in digi {
        assert_eq!(p.cost, Some(CompiledCost::Literal(2)));
    }
}

#[test]
fn ex13_028_clause_shape() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    assert_eq!(triggered[0].when, vec![CompiledTiming::OnDeletion]);
    assert!(!triggered[0].optional, "reveal/trash is mandatory; only the play is a may");
    let replacements: Vec<_> = c
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
    assert_eq!(replacements, vec![(CompiledScope::Inherited, true, true)]);
}

#[test]
fn ex13_028_has_blocker_face_up() {
    let mut r = builder().start();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Blocker), "<Blocker>");
}

// ─── Section 2/3 — [On Deletion] reveal / may play / trash rest ──────────────

#[test]
fn ex13_028_on_deletion_offers_cost3_chuumon_or_sukamon_digimon_only() {
    let (mut r, h) = deletion_setup(&["SUKA-4", "PLAIN-3", "CHUU-3"]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
    assert_eq!(
        offered_reveal_ids(&r),
        vec!["CHUU-3".to_string()],
        "cost-4 Sukamon and non-matching names are excluded"
    );
}

#[test]
fn ex13_028_on_deletion_excludes_non_digimon() {
    let (mut r, h) = deletion_setup(&["SUKA-OPT", "SUKA-3", "PLAIN-3"]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
    assert_eq!(offered_reveal_ids(&r), vec!["SUKA-3".to_string()]);
}

#[test]
fn ex13_028_on_deletion_plays_pick_free_and_trashes_rest() {
    let (mut r, h) = deletion_setup(&["PLAIN-3", "SUKA-3", "CHUU-3"]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
    pick_revealed(&mut r, "SUKA-3");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["SUKA-3".to_string()]);
    assert_eq!(r.memory(), 0, "played without paying the cost");
    let trash = trash_ids(&r);
    assert!(trash.contains(&"CHUU-3".to_string()));
    assert!(trash.contains(&"PLAIN-3".to_string()));
    assert!(trash.contains(&CARD_ID.to_string()));
    assert_eq!(r.deck_size(0), 2);
}

#[test]
fn ex13_028_on_deletion_decline_trashes_all_three() {
    let (mut r, h) = deletion_setup(&["PLAIN-3", "SUKA-3", "CHUU-3"]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
    let view = r.pending_selection_view().expect("prompt");
    assert!(view.is_optional, "\"you may\"");
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
    assert_eq!(trash_ids(&r).len(), 4, "3 revealed + Sukamon itself");
    assert_eq!(r.deck_size(0), 2);
}

#[test]
fn ex13_028_on_deletion_no_match_trashes_all() {
    let (mut r, h) = deletion_setup(&["PLAIN-3", "FILL", "SUKA-4"]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
    assert_eq!(trash_ids(&r).len(), 4);
}

#[test]
fn ex13_028_on_play_does_not_reveal() {
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "PLAIN-3", "SUKA-3", "CHUU-3"])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.play(0, 0).expect("Sukamon played");
    assert!(r.pending_selection().is_none());
    assert_eq!(r.deck_size(0), 4);
}

// ─── Section 3/5 — Inherited [All Turns][OPT] leave prevention ───────────────

fn ex13_028_inherited_setup() -> (DebugRunner, PermanentHandle) {
    let mut r = builder().deck(0, &["FILL"; 6]).start();
    r.skip_mulligan();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    (r, carrier)
}

#[test]
fn ex13_028_inherited_deletes_other_sukamon_and_carrier_stays() {
    let (mut r, carrier) = ex13_028_inherited_setup();
    r.place_on_field(0, "OTHER-SUKA", Some(0));
    r.place_on_field(0, "OTHER-PLAIN", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    let accept = r.pending_selection_view().expect("accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    assert!(accept.is_optional);
    r.execute_action(0, accept.valid_action_ids[0]).expect("accept");
    let cost = r.pending_selection_view().expect("cost prompt");
    // "1 other Digimon" (no "your"): either player's Digimon (Q7288), so the
    // pick spans both battle areas.
    assert_eq!(cost.kind, SelectionKind::AnyField);
    let offered: Vec<_> = cost
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .collect();
    assert_eq!(offered.len(), 1);
    r.execute_action(0, *offered[0]).expect("pay");
    let _ = r.auto_resolve();
    let field = field_ids(&r, 0);
    assert!(field.contains(&"CARRIER".to_string()));
    assert!(!field.contains(&"OTHER-SUKA".to_string()));
}

#[test]
fn ex13_028_inherited_not_offered_without_other_sukamon() {
    let (mut r, carrier) = ex13_028_inherited_setup();
    r.place_on_field(0, "OTHER-PLAIN", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none());
    assert!(!field_ids(&r, 0).contains(&"CARRIER".to_string()));
}

#[test]
fn ex13_028_inherited_not_offered_for_own_effect_removal() {
    let (mut r, carrier) = ex13_028_inherited_setup();
    r.place_on_field(0, "OTHER-SUKA", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OwnEffect);
    assert!(r.pending_selection_view().is_none());
    assert!(!field_ids(&r, 0).contains(&"CARRIER".to_string()));
}

#[test]
fn ex13_028_inherited_once_per_turn() {
    let (mut r, carrier) = ex13_028_inherited_setup();
    r.place_on_field(0, "OTHER-SUKA", Some(0));
    r.place_on_field(0, "SUKA-3", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    let accept = r.pending_selection_view().expect("accept prompt");
    r.execute_action(0, accept.valid_action_ids[0]).expect("accept");
    let cost = r.pending_selection_view().expect("cost prompt");
    let pick = *cost
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("a cost");
    r.execute_action(0, pick).expect("pay");
    let _ = r.auto_resolve();
    let carrier = handle_of(&r, "CARRIER");
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none(), "OPT");
    assert!(!field_ids(&r, 0).contains(&"CARRIER".to_string()));
}

// ─── Inherited [OPT]: DCGO parity (owner ruling 2026-11-23) ──────────────────
//
// DCGO EX13_028.cs (same body as EX13_027.cs): offered only while another
// [Sukamon]-named Digimon exists on EITHER battle area (Q7288); after the
// accept, a `SelectPermanentEffect` with canNoSelect true. None chosen ->
// `RemoveUse()` and the carrier leaves. One chosen -> `isUsed` (spent).
// qa/rules/mechanics-gaps-2026-10.md W14 row (f). <Evade> on the carrier keeps
// it after the would-leave window so the use can be observed on a later
// removal the same turn.

fn grant_evade(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.modifiers.grant_keyword(
        h,
        digimon_engine::enums::Keyword::Evade,
        digimon_engine::enums::Expiry::Permanent,
        0,
    );
}

/// While <Evade> can also answer the deletion, the engine first asks which
/// immediate replacement effect applies next (CR 4.3 15-8-5); pick <Evade>
/// (`evade`) or the carrier's inherited clause. With one candidate left, its
/// own accept prompt is already pending.
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
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
}

/// PASS is legal at the pending prompt (the action mask, not the raw ids).
fn pass_legal(r: &DebugRunner) -> bool {
    let p = r.pending_selection().expect("prompt").selecting_player;
    digimon_engine::action::build_action_mask(&r.game, p)[PASS as usize] == 1.0
}

fn ex13_028_accept_evade(r: &mut DebugRunner) {
    pick_replacement(r, true);
    r.execute_action(0, digimon_engine::action::space::REPLACEMENT_ACCEPT)
        .expect("accept <Evade>");
    assert!(
        r.pending_selection().is_none(),
        "the same removal does not offer the declined clause again"
    );
}

#[test]
fn ex13_028_inherited_outer_decline_keeps_opt() {
    let (mut r, carrier) = ex13_028_inherited_setup();
    r.place_on_field(0, "OTHER-SUKA", Some(0));
    grant_evade(&mut r, carrier);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    pick_replacement(&mut r, false);
    r.execute_action(0, PASS).expect("decline the replacement");
    ex13_028_accept_evade(&mut r);
    let carrier = handle_of(&r, "CARRIER");
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Replacement),
        "declined at the accept prompt: offered again this turn"
    );
}

#[test]
fn ex13_028_inherited_accept_then_decline_cost_keeps_opt() {
    let (mut r, carrier) = ex13_028_inherited_setup();
    r.place_on_field(0, "OTHER-SUKA", Some(0));
    grant_evade(&mut r, carrier);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    pick_replacement(&mut r, false);
    r.execute_action(0, digimon_engine::action::space::REPLACEMENT_ACCEPT)
        .expect("accept");
    let cost = r.pending_selection_view().expect("cost pick");
    assert!(
        pass_legal(&r),
        "DCGO EX13_028.cs canNoSelect: the deletion may be declined after the accept"
    );
    r.execute_action(0, PASS).expect("decline the cost");
    assert!(field_ids(&r, 0).contains(&"OTHER-SUKA".to_string()), "nothing deleted");
    ex13_028_accept_evade(&mut r);
    let carrier = handle_of(&r, "CARRIER");
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Replacement),
        "DCGO RemoveUse: the declined cost keeps the [Once Per Turn]"
    );
}

#[test]
fn ex13_028_inherited_declined_cost_lets_carrier_leave() {
    let (mut r, carrier) = ex13_028_inherited_setup();
    r.place_on_field(0, "OTHER-SUKA", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    pick_replacement(&mut r, false);
    r.execute_action(0, digimon_engine::action::space::REPLACEMENT_ACCEPT)
        .expect("accept");
    let cost = r.pending_selection_view().expect("cost pick");
    assert!(pass_legal(&r), "the cost may be declined");
    r.execute_action(0, PASS).expect("decline the cost");
    assert!(r.pending_selection().is_none(), "no decision is left");
    let field = field_ids(&r, 0);
    assert!(!field.contains(&"CARRIER".to_string()), "not paid: the carrier leaves");
    assert!(field.contains(&"OTHER-SUKA".to_string()));
}

/// Q7288: "1 other Digimon with [Sukamon] in its name" may be the opponent's.
#[test]
fn ex13_028_inherited_opponent_sukamon_is_a_legal_cost() {
    let (mut r, carrier) = ex13_028_inherited_setup();
    let opp = r.place_on_field(1, "OTHER-SUKA", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Replacement),
        "an opponent's [Sukamon]-named Digimon can pay the cost (Q7288)"
    );
    pick_replacement(&mut r, false);
    r.execute_action(0, digimon_engine::action::space::REPLACEMENT_ACCEPT)
        .expect("accept");
    let id = digimon_engine::action::space::encode_attack(1, opp.index as u16);
    let cost = r.pending_selection_view().expect("cost pick");
    assert!(cost.valid_action_ids.contains(&id), "{cost:?}");
    r.execute_action(0, id).expect("delete the opponent's Sukamon");
    assert!(r.pending_selection().is_none(), "no decision is left");
    assert!(field_ids(&r, 0).contains(&"CARRIER".to_string()), "the carrier stays");
    assert!(field_ids(&r, 1).is_empty(), "the opponent's Sukamon was deleted");
}

// ─── Inherited: an undeletable payer is still offered (owner ruling 2026-11-24) ─
//
// DCGO EX13_028.cs (the same body as EX13_027.cs) offers the clause whenever a printed match exists
// (`CanActivateCondition` = `HasMatchConditionPermanent(CanSelectPermanentCondition)`,
// no deletability check) and its pick lists every printed match. Picking one
// that can't be deleted sets `isUsed` (spent), `DestroyPermanentsClass` deletes
// nothing, so `SuccessProcess` never runs and the carrier leaves. Owner,
// 2026-11-24: "You can pick a target that is immune from the removal, and it
// will fizzle. Nothing is deleted, right? That is a valid choice."
//
// The payer here is the OPPONENT's [Sukamon]-named Digimon under "can't be
// deleted by your opponent's effects" (to its owner, Chuumon's effect is an
// opponent's effect), e.g. the grant of Gladimon (BT18-062).

fn protect_from_opponent_effects(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.modifiers.add(
        h,
        digimon_engine::modifiers::ModifierEntry::passive_replacement(
            digimon_engine::enums::ModifierType::CannotBeDestroyedByEffect,
            digimon_engine::enums::Expiry::Permanent,
            h.player,
        )
        .opponent_only(),
    );
}

fn opp_sukamon_protected(r: &mut DebugRunner) -> PermanentHandle {
    let h = r.place_on_field(1, "OTHER-SUKA", Some(0));
    protect_from_opponent_effects(r, h);
    h
}

fn any_field_id(h: PermanentHandle) -> u16 {
    digimon_engine::action::space::encode_attack(h.player as u16, h.index as u16)
}

#[test]
fn ex13_028_inherited_offered_when_the_only_match_is_undeletable() {
    let (mut r, carrier) = inherited_setup();
    let opp = opp_sukamon_protected(&mut r);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Replacement),
        "DCGO offers the clause while a printed match exists, deletable or not"
    );
    r.execute_action(0, digimon_engine::action::space::REPLACEMENT_ACCEPT)
        .expect("accept");
    let cost = r.pending_selection_view().expect("cost pick");
    assert!(
        cost.valid_action_ids.contains(&any_field_id(opp)),
        "the undeletable [Sukamon] is listed: {cost:?}"
    );
}

#[test]
fn ex13_028_inherited_picking_an_undeletable_payer_spends_opt_and_deletes_nothing() {
    let (mut r, carrier) = inherited_setup();
    let opp = opp_sukamon_protected(&mut r);
    grant_evade(&mut r, carrier);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    pick_replacement(&mut r, false);
    r.execute_action(0, digimon_engine::action::space::REPLACEMENT_ACCEPT)
        .expect("accept");
    r.execute_action(0, any_field_id(opp)).expect("pick the undeletable Sukamon");
    assert_eq!(r.game.players[1].battle_area.len(), 1, "nothing was deleted");
    accept_evade(&mut r);
    let carrier = handle_of(&r, "CARRIER");
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(
        r.pending_selection().is_none(),
        "DCGO isUsed: a Digimon was chosen, so the [Once Per Turn] is spent"
    );
    assert!(!field_ids(&r, 0).contains(&"CARRIER".to_string()));
}

#[test]
fn ex13_028_inherited_picking_an_undeletable_payer_lets_the_carrier_leave() {
    let (mut r, carrier) = inherited_setup();
    let opp = opp_sukamon_protected(&mut r);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    r.execute_action(0, digimon_engine::action::space::REPLACEMENT_ACCEPT)
        .expect("accept");
    r.execute_action(0, any_field_id(opp)).expect("pick the undeletable Sukamon");
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"CARRIER".to_string()), "not paid: the carrier leaves");
    assert_eq!(r.game.players[1].battle_area.len(), 1, "the Sukamon was not deleted");
}

#[test]
fn ex13_028_inherited_undeletable_only_pass_keeps_opt() {
    let (mut r, carrier) = inherited_setup();
    let opp = opp_sukamon_protected(&mut r);
    grant_evade(&mut r, carrier);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    pick_replacement(&mut r, false);
    r.execute_action(0, digimon_engine::action::space::REPLACEMENT_ACCEPT)
        .expect("accept");
    assert!(pass_legal(&r), "canNoSelect");
    r.execute_action(0, PASS).expect("select nothing");
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    let _ = opp;
    accept_evade(&mut r);
    let carrier = handle_of(&r, "CARRIER");
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Replacement),
        "DCGO RemoveUse: nothing chosen keeps the [Once Per Turn]"
    );
}

#[test]
fn ex13_028_inherited_deletable_payer_still_protects_beside_an_undeletable_one() {
    let (mut r, carrier) = inherited_setup();
    let opp = opp_sukamon_protected(&mut r);
    let own = r.place_on_field(0, "OTHER-SUKA", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    r.execute_action(0, digimon_engine::action::space::REPLACEMENT_ACCEPT)
        .expect("accept");
    let cost = r.pending_selection_view().expect("cost pick");
    assert!(cost.valid_action_ids.contains(&any_field_id(opp)), "{cost:?}");
    assert!(cost.valid_action_ids.contains(&any_field_id(own)), "{cost:?}");
    r.execute_action(0, any_field_id(own)).expect("pay with the deletable Sukamon");
    let _ = r.auto_resolve();
    let field = field_ids(&r, 0);
    assert!(field.contains(&"CARRIER".to_string()), "paid: the carrier doesn't leave");
    assert!(!field.contains(&"OTHER-SUKA".to_string()), "the payer was deleted");
    assert_eq!(r.game.players[1].battle_area.len(), 1);
}
