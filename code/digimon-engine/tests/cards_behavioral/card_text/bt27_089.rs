//! Source: `data/card_bundles/BT27-089.md` (printed text and official Q&A).
//!
//! [Start of Your Main Phase] By placing 1 Digimon card with [Gaomon] or
//! [Gaogamon] in its name from your hand face down under this Tamer, <Draw 1>
//! and gain 1 memory.
//! [All Turns] When effects add to your opponent's hand, by suspending this
//! Tamer, you may place the top card of your deck face down under this Tamer.
//! After, 1 of your Digimon with [Gaogamon] in its name can't be deleted in
//! battle for the turn.
//! [Security] Play this card without paying the cost.
//! Q&A: if this Tamer isn't suspended, the part after "After" isn't processed.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::ModifierType;

use super::bt27_fx::*;
use super::common::{candidates, next_turn};

const CARD: &str = "BT27-089";

fn runner(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(mon("GAOMON", "Gaomon", 3, 3, &[]))
        .add_card(mon("GAOGA", "Gaogamon", 4, 5, &[]))
        .add_card(mon("PLAIN", "Plain", 3, 3, &[]))
        .add_card(mon("OPP", "Opp", 3, 3, &[]))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .add_card(mon("TOPD", "TopD", 3, 3, &[]))
        .deck(0, &["TOPD"; 10])
        .deck(1, &["FILL"; 10])
        .hand(0, hand)
        .memory(3)
        .start()
}

#[test]
fn start_of_main_places_a_gaomon_card_face_down_to_draw_and_gain_memory() {
    let mut r = runner(&["GAOMON", "GAOGA", "PLAIN"]);
    let t = r.place_on_field(0, CARD, Some(0));
    next_turn(&mut r);
    next_turn(&mut r);
    let v = r.pending_selection_view().expect("optional placement");
    assert!(v.is_optional);
    let mut c = candidates(&r);
    c.sort();
    assert_eq!(c, vec!["GAOGA", "GAOMON"]);
    let (hand, mem) = (r.hand_size(0), r.memory());
    r.choose_prompt_card("GAOGA", None).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, t), vec!["GAOGA"]);
    assert_eq!(face_down(&r, t), vec![true]);
    assert_eq!(r.hand_size(0), hand);
    assert_eq!(r.memory(), mem + 1);
}

#[test]
fn start_of_main_may_be_declined_and_needs_a_matching_card() {
    let mut r = runner(&["GAOMON"]);
    let t = r.place_on_field(0, CARD, Some(0));
    next_turn(&mut r);
    next_turn(&mut r);
    r.decline_prompt().unwrap();
    assert!(source_ids(&r, t).is_empty());

    let mut r = runner(&["PLAIN"]);
    r.place_on_field(0, CARD, Some(0));
    next_turn(&mut r);
    next_turn(&mut r);
    assert!(r.pending_selection().is_none());
}

fn bounce_opponent(r: &mut DebugRunner) {
    let opp = r.place_on_field(1, "OPP", Some(0));
    let src = r.place_on_field(0, "PLAIN", Some(0));
    effect_bounce(r, 0, src, opp);
}

#[test]
fn opponent_hand_add_suspends_places_deck_top_and_protects_a_gaogamon() {
    let mut r = runner(&[]);
    let t = r.place_on_field(0, CARD, Some(0));
    let gao = r.place_on_field(0, "GAOGA", Some(0));
    r.place_on_field(0, "GAOMON", Some(0));
    bounce_opponent(&mut r);
    let v = r.pending_selection_view().expect("optional trigger");
    assert!(v.is_optional);
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    assert!(r.game.players[0].battle_area[t.index as usize].is_suspended);
    choose_branch(&mut r, 0);
    let v = r.pending_selection_view().expect("mandatory Gaogamon pick");
    assert!(!v.is_optional);
    assert_eq!(candidates(&r), vec!["GAOGA"], "[Gaomon] is not [Gaogamon]");
    r.choose_prompt_card("GAOGA", None).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, t), vec!["TOPD"]);
    assert_eq!(face_down(&r, t), vec![true]);
    assert!(r.game.modifiers.has(gao, ModifierType::CannotBeDestroyedByBattle));
}

#[test]
fn placement_is_optional_but_protection_still_follows_the_suspend() {
    let mut r = runner(&[]);
    let t = r.place_on_field(0, CARD, Some(0));
    let gao = r.place_on_field(0, "GAOGA", Some(0));
    bounce_opponent(&mut r);
    let v = r.pending_selection_view().unwrap();
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    choose_branch(&mut r, 1);
    let _ = r.auto_resolve();
    assert!(source_ids(&r, t).is_empty());
    assert!(r.game.modifiers.has(gao, ModifierType::CannotBeDestroyedByBattle));
}

#[test]
fn declining_skips_the_after_part() {
    let mut r = runner(&[]);
    let t = r.place_on_field(0, CARD, Some(0));
    let gao = r.place_on_field(0, "GAOGA", Some(0));
    bounce_opponent(&mut r);
    r.decline_prompt().unwrap();
    let _ = r.auto_resolve();
    assert!(!r.game.players[0].battle_area[t.index as usize].is_suspended);
    assert!(!r.game.modifiers.has(gao, ModifierType::CannotBeDestroyedByBattle));
}

#[test]
fn a_suspended_tamer_cannot_pay_and_own_hand_adds_do_not_trigger() {
    let mut r = runner(&[]);
    let t = r.place_on_field(0, CARD, Some(0));
    r.place_on_field(0, "GAOGA", Some(0));
    r.game.players[0].battle_area[t.index as usize].is_suspended = true;
    bounce_opponent(&mut r);
    assert!(r.pending_selection().is_none());

    let mut r = runner(&[]);
    r.place_on_field(0, CARD, Some(0));
    let mine = r.place_on_field(0, "GAOGA", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    effect_bounce(&mut r, 1, opp, mine);
    assert!(r.pending_selection().is_none(), "my hand, not my opponent's");
}

#[test]
fn security_plays_this_tamer_free() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(mon("ATK", "Atk", 5, 7, &[]))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .security(1, &[CARD])
        .start();
    let a = r.place_on_field(0, "ATK", Some(0)).index;
    attack_security(&mut r, a);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&CARD.to_string()));
}
