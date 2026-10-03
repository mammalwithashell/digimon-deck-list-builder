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

fn inherited_setup() -> (DebugRunner, PermanentHandle) {
    let mut r = builder().deck(0, &["FILL"; 6]).start();
    r.skip_mulligan();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    (r, carrier)
}

#[test]
fn ex13_028_inherited_deletes_other_sukamon_and_carrier_stays() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "OTHER-SUKA", Some(0));
    r.place_on_field(0, "OTHER-PLAIN", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    let accept = r.pending_selection_view().expect("accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    assert!(accept.is_optional);
    r.execute_action(0, accept.valid_action_ids[0]).expect("accept");
    let cost = r.pending_selection_view().expect("cost prompt");
    assert_eq!(cost.kind, SelectionKind::OwnField);
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
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "OTHER-PLAIN", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none());
    assert!(!field_ids(&r, 0).contains(&"CARRIER".to_string()));
}

#[test]
fn ex13_028_inherited_not_offered_for_own_effect_removal() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "OTHER-SUKA", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OwnEffect);
    assert!(r.pending_selection_view().is_none());
    assert!(!field_ids(&r, 0).contains(&"CARRIER".to_string()));
}

#[test]
fn ex13_028_inherited_once_per_turn() {
    let (mut r, carrier) = inherited_setup();
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
