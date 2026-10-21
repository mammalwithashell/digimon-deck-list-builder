//! BT7-107 Calling From the Darkness — Option, Purple, Cost 1.
//!
//! # Card text (card image + official Bandai DB bundle — authoritative)
//!
//! [Main] Delete 1 of your Digimon. Then, return up to 2 purple Digimon cards
//! from your trash to your hand.
//! Security Effect: [Security] Add this card to its owner's hand.
//!
//! Official Q&A:
//! - Q1673: the purple Digimon deleted by this effect can be returned to the
//!   hand by the same effect.
//! - Q1674: if that Digimon has an [On Deletion] effect and is returned, the
//!   effect triggers but can't activate (the card left the trash first).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT7/Purple/BT7_107.cs
//! - OptionSkill: SelectPermanentEffect (own battle-area Digimon, maxCount 1,
//!   canNoSelect: false, Mode.Destroy); then SelectCardEffect over own purple
//!   Digimon cards in the trash, maxCount 2, Mode.AddHand. (DCGO's
//!   CanEndSelectCondition forces at least 1 pick; the printed "up to 2" and
//!   rule 15-10-2-2 allow 0 — rules govern, see the YAML header.)
//! - SecuritySkill: AddThisCardToHand.
//!
//! # Patterns this test covers
//! - F1 mandatory own-Digimon deletion (effect)
//! - A3 "up to 2" trash → hand, filtered by colour + card kind
//! - Q1674 pending On Deletion of a card that left the trash does not activate
//! - [Security] add-self-to-hand

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::action::space::{PASS, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "BT7-107";
const EYESMON: &str = "BT7-069"; // purple Lv.4, [On Deletion] Draw 3 then trash 2

fn card(id: &str, kind: CardKind, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = kind;
    c.colors = vec![color];
    c.play_cost = 3;
    if kind == CardKind::Digimon {
        c.level = Some(3);
        c.dp = Some(3000);
    } else {
        c.level = None;
        c.dp = None;
    }
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT7-107 YAML parses and compiles")
        .dsl_card(EYESMON)
        .expect("BT7-069 loads")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(card("PURPLE-A", CardKind::Digimon, CardColor::Purple))
        .add_card(card("PURPLE-B", CardKind::Digimon, CardColor::Purple))
        .add_card(card("PURPLE-C", CardKind::Digimon, CardColor::Purple))
        .add_card(card("RED-DIGI", CardKind::Digimon, CardColor::Red))
        .add_card(card("PURPLE-TAMER", CardKind::Tamer, CardColor::Purple))
        .add_card(card("PURPLE-OPTION", CardKind::Option, CardColor::Purple))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
}

fn hand_ids(r: &DebugRunner, p: usize) -> Vec<String> {
    r.game.players[p]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn trash_ids(r: &DebugRunner, p: usize) -> Vec<String> {
    r.game.players[p]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn field_ids(r: &DebugRunner, p: usize) -> Vec<String> {
    r.game.players[p]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

/// The trash-pick action for the first trash card with `id`.
fn trash_action(r: &DebugRunner, id: &str) -> u16 {
    let idx = trash_ids(r, 0)
        .iter()
        .position(|t| t == id)
        .unwrap_or_else(|| panic!("{id} not in trash"));
    let action = TRASH_EFFECT_START + idx as u16;
    let view = r.pending_selection_view().expect("trash pick pending");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert!(
        view.valid_action_ids.contains(&action),
        "{id} must be a legal pick: {:?}",
        view.valid_action_ids
    );
    action
}

/// Use the Option from hand slot 0 with `field` on P0's battle area and
/// `trash` in P0's trash.
fn use_calling(field: &[&str], trash: &[&str]) -> DebugRunner {
    let mut r = base().hand(0, &[CARD_ID]).memory(5).start();
    for f in field {
        r.place_on_field(0, f, Some(0));
    }
    for t in trash {
        r.inject_trash(0, t);
    }
    assert_ne!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid,
        "a purple permanent meets the colour requirement"
    );
    r
}

/// Delete the own Digimon at battle-area index `idx` at the delete prompt.
fn delete_pick(r: &mut DebugRunner, idx: usize) {
    let view = r.pending_selection_view().expect("delete prompt");
    assert!(!view.is_optional, "'Delete 1 of your Digimon' is mandatory");
    assert!(view.valid_action_ids.len() > idx);
    r.execute_action(0, view.valid_action_ids[idx]).unwrap();
}

// ─── Section 1 — Structural ─────────────────────────────────────────────────

#[test]
fn bt7_107_metadata_and_clauses() {
    let r = base().start();
    let c = r.compiled_card(CARD_ID).unwrap();
    assert_eq!(c.kind, CompiledCardKind::Option);
    assert_eq!(c.cost, Some(1));
    let triggered: Vec<_> = c
        .effects
        .iter()
        .filter_map(|cl| match cl {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2);
    let main = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::MainFromHand])
        .expect("[Main]");
    assert!(!main.optional);
    assert!(triggered.iter().any(|t| t.when == vec![CompiledTiming::OnSecurity]));
}

#[test]
fn bt7_107_needs_a_purple_permanent() {
    let mut r = base().hand(0, &[CARD_ID]).memory(5).start();
    r.place_on_field(0, "RED-DIGI", Some(0));
    assert_eq!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Invalid);
    assert_eq!(r.memory(), 5);
}

// ─── Section 2 — [Main] delete 1 of your Digimon ────────────────────────────

#[test]
fn bt7_107_deletes_the_chosen_own_digimon_only() {
    let mut r = base().hand(0, &[CARD_ID]).memory(5).start();
    r.place_on_field(0, "PURPLE-A", Some(0));
    r.place_on_field(0, "RED-DIGI", Some(0));
    r.place_on_field(1, "PURPLE-B", Some(0));
    assert_ne!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Invalid);
    let view = r.pending_selection_view().expect("delete prompt");
    assert_eq!(view.valid_action_ids.len(), 2, "only your own two Digimon are eligible");
    assert!(!view.is_optional);
    delete_pick(&mut r, 1);
    r.auto_resolve().ok();
    assert_eq!(field_ids(&r, 0), vec!["PURPLE-A".to_string()], "RED-DIGI deleted");
    assert_eq!(field_ids(&r, 1), vec!["PURPLE-B".to_string()], "opponent untouched");
    assert!(trash_ids(&r, 0).contains(&"RED-DIGI".to_string()));
    assert_eq!(r.memory(), 4, "cost 1 paid");
}

#[test]
fn bt7_107_no_own_digimon_still_returns_cards() {
    // A purple Tamer meets the colour requirement with no Digimon to delete;
    // the "Then" part still resolves.
    let mut r = use_calling(&["PURPLE-TAMER"], &["PURPLE-A"]);
    let a = trash_action(&r, "PURPLE-A");
    r.execute_action(0, a).unwrap();
    r.auto_resolve().ok();
    assert_eq!(hand_ids(&r, 0), vec!["PURPLE-A".to_string()]);
    assert_eq!(field_ids(&r, 0), vec!["PURPLE-TAMER".to_string()], "Tamers aren't Digimon");
}

// ─── Section 3 — Then, return up to 2 purple Digimon cards ──────────────────

#[test]
fn bt7_107_returns_two_purple_digimon() {
    let mut r = use_calling(&["PURPLE-C"], &["PURPLE-A", "PURPLE-B"]);
    delete_pick(&mut r, 0);
    let a = trash_action(&r, "PURPLE-A");
    assert!(r.pending_is_optional(), "'up to 2' — 0 picks allowed");
    r.execute_action(0, a).unwrap();
    let b = trash_action(&r, "PURPLE-B");
    assert!(r.pending_is_optional());
    r.execute_action(0, b).unwrap();
    r.auto_resolve().ok();
    let mut hand = hand_ids(&r, 0);
    hand.sort();
    assert_eq!(hand, vec!["PURPLE-A".to_string(), "PURPLE-B".to_string()]);
    assert_eq!(trash_ids(&r, 0), vec!["PURPLE-C".to_string(), CARD_ID.to_string()]);
}

#[test]
fn bt7_107_at_most_two_cards_return() {
    let mut r = use_calling(&["PURPLE-C"], &["PURPLE-A", "PURPLE-B"]);
    delete_pick(&mut r, 0);
    r.execute_action(0, trash_action(&r, "PURPLE-A")).unwrap();
    r.execute_action(0, trash_action(&r, "PURPLE-B")).unwrap();
    assert!(
        r.pending_selection().is_none(),
        "after 2 picks no third pick is offered (PURPLE-C is still in the trash)"
    );
    assert!(trash_ids(&r, 0).contains(&"PURPLE-C".to_string()));
}

#[test]
fn bt7_107_may_return_only_one() {
    let mut r = use_calling(&["PURPLE-C"], &["PURPLE-A", "PURPLE-B"]);
    delete_pick(&mut r, 0);
    r.execute_action(0, trash_action(&r, "PURPLE-A")).unwrap();
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().ok();
    assert_eq!(hand_ids(&r, 0), vec!["PURPLE-A".to_string()]);
}

#[test]
fn bt7_107_may_return_none() {
    let mut r = use_calling(&["PURPLE-C"], &["PURPLE-A"]);
    delete_pick(&mut r, 0);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Trash));
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().ok();
    assert!(hand_ids(&r, 0).is_empty());
    assert!(trash_ids(&r, 0).contains(&"PURPLE-A".to_string()));
}

#[test]
fn bt7_107_only_purple_digimon_cards_are_eligible() {
    let mut r = use_calling(
        &["PURPLE-C"],
        &["RED-DIGI", "PURPLE-TAMER", "PURPLE-OPTION", "PURPLE-A"],
    );
    delete_pick(&mut r, 0);
    let view = r.pending_selection_view().unwrap();
    // PURPLE-A and the just-deleted PURPLE-C.
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "red Digimon, purple Tamer and purple Option are not eligible"
    );
}

/// Q1673: the Digimon just deleted can be returned by the same effect.
#[test]
fn bt7_107_the_deleted_digimon_can_be_returned() {
    let mut r = use_calling(&["PURPLE-C"], &[]);
    delete_pick(&mut r, 0);
    r.execute_action(0, trash_action(&r, "PURPLE-C")).unwrap();
    r.auto_resolve().ok();
    assert_eq!(hand_ids(&r, 0), vec!["PURPLE-C".to_string()]);
}

/// Q1674: the returned Digimon's [On Deletion] triggers but can't activate.
#[test]
fn bt7_107_returned_digimon_on_deletion_does_not_activate() {
    let mut r = use_calling(&[EYESMON], &[]);
    let deck_before = r.game.players[0].deck.len();
    delete_pick(&mut r, 0);
    r.execute_action(0, trash_action(&r, EYESMON)).unwrap();
    r.auto_resolve().ok();
    assert_eq!(hand_ids(&r, 0), vec![EYESMON.to_string()]);
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_before,
        "Eyesmon's [On Deletion] <Draw 3> did not activate"
    );
}

/// Control for Q1674: left in the trash, the [On Deletion] does activate.
#[test]
fn bt7_107_on_deletion_activates_when_not_returned() {
    let mut r = use_calling(&[EYESMON], &[]);
    let deck_before = r.game.players[0].deck.len();
    delete_pick(&mut r, 0);
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().ok();
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_before - 3,
        "Eyesmon's [On Deletion] <Draw 3> resolved"
    );
}

// ─── Section 4 — [Security] ─────────────────────────────────────────────────

#[test]
fn bt7_107_security_adds_itself_to_the_owners_hand() {
    let mut attacker = make_test_card("ATK", "ATK");
    attacker.dp = Some(5000);
    let mut r = base().add_card(attacker).security(1, &[CARD_ID]).memory(3).start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let hand_before = r.hand_size(1);
    let _ = r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 0);
    assert_eq!(r.hand_size(1), hand_before + 1);
    assert!(hand_ids(&r, 1).contains(&CARD_ID.to_string()));
    assert!(!trash_ids(&r, 1).contains(&CARD_ID.to_string()));
}

/// "Your Digimon" are those in the battle area: a breeding-area Digimon is not
/// a deletion candidate.
#[test]
fn bt7_107_breeding_area_digimon_is_not_eligible() {
    let mut r = base().hand(0, &[CARD_ID]).memory(5).start();
    r.place_on_field(0, "PURPLE-A", Some(0));
    r.place_in_breeding(0, "PURPLE-B");
    assert_ne!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Invalid);
    let view = r.pending_selection_view().expect("delete prompt");
    assert_eq!(view.valid_action_ids.len(), 1, "only the battle-area Digimon");
}
