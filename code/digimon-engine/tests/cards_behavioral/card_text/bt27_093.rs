//! Source: `data/card_bundles/BT27-093.md` (printed text and official Q&A).
//!
//! [Start of Your Main Phase] Place 1 [Chessmon]-in-text hand card face down under this Tamer: Draw 1, +1 memory. [Main] Suspend + trash 2 bottom face-down cards from under your Tamers: 1 Digimon may digivolve into a [Chessmon]-named hand Digimon, cost -2. [Security] Play free.
//! Q&A: the placed card goes to the bottom of the cards under the Tamer.

use digimon_engine::debug_runner::DebugRunner;

use super::bt27_fx::*;
use super::common::{advance_turn, candidates};

const CARD: &str = "BT27-093";

fn runner(hand: &[&str]) -> DebugRunner {
    let mut stock = mon("TEXT-CHESS", "Mystery", 3, 3, &[]); stock.effect_text = "[On Play] Play 1 [Chessmon].".into();
    let evo_card = evo("KNIGHT", "KnightChessmon", 4, 5, 3);
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(stock)
        .add_card(evo_card)
        .add_card(mon("PLAIN", "Agumon", 3, 3, &[]))
        .add_card(mon("BASE", "Base", 3, 3, &[]))
        .add_card(mon("FD", "Fd", 3, 3, &[]))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .hand(0, hand)
        .memory(3)
        .start()
}

fn to_main_phase(r: &mut DebugRunner) {
    advance_turn(r);
    advance_turn(r);
}

#[test]
fn start_of_main_places_a_matching_card_at_the_bottom_face_down_to_draw_and_gain_memory() {
    let mut r = runner(&["TEXT-CHESS", "PLAIN"]);
    let t = stocked(&mut r, CARD, &["FD"]);
    to_main_phase(&mut r);
    let v = r.pending_selection_view().expect("optional placement");
    assert!(v.is_optional);
    assert_eq!(candidates(&r), vec!["TEXT-CHESS"]);
    let (hand, mem) = (r.hand_size(0), r.memory());
    r.choose_prompt_card("TEXT-CHESS", None).unwrap();
    no_prompt(&r);
    assert_eq!(source_ids(&r, t), vec!["TEXT-CHESS", "FD"]);
    assert_eq!(face_down(&r, t), vec![true, true]);
    assert_eq!(r.hand_size(0), hand);
    assert_eq!(r.memory(), mem + 1);
}

#[test]
fn start_of_main_may_be_declined() {
    let mut r = runner(&["TEXT-CHESS"]);
    let t = stocked(&mut r, CARD, &[]);
    to_main_phase(&mut r);
    let mem = r.memory();
    r.decline_prompt().unwrap();
    assert!(source_ids(&r, t).is_empty());
    assert_eq!(r.memory(), mem);
}

#[test]
fn start_of_main_needs_a_matching_hand_card() {
    let mut r = runner(&["PLAIN"]);
    stocked(&mut r, CARD, &[]);
    to_main_phase(&mut r);
    assert!(r.pending_selection().is_none());
}

#[test]
fn main_suspends_trashes_two_bottom_face_down_cards_and_digivolves_reduced_by_two() {
    let mut r = runner(&["KNIGHT", "PLAIN"]);
    let other = stocked(&mut r, "FILL", &["FD"]);
    let t = stocked(&mut r, CARD, &["FD", "PLAIN"]);
    let base = r.place_on_field(0, "BASE", Some(0));
    let mem = r.memory();
    activate_field_main(&mut r, t);
    assert!(r.game.players[0].battle_area[t.index as usize].is_suspended);
    pay_tamer_stock(&mut r);
    let v = r.pending_selection_view().expect("optional digivolve base");
    assert!(v.is_optional);
    r.choose_prompt_card("BASE", None).unwrap();
    assert_eq!(candidates(&r), vec!["KNIGHT"]);
    r.choose_prompt_card("KNIGHT", None).unwrap();
    no_prompt(&r);
    assert_eq!(top_id(&r, base), "KNIGHT");
    assert_eq!(mem - r.memory(), 1, "evo cost 3 reduced by 2");
    assert_eq!(source_ids(&r, t).len() + source_ids(&r, other).len(), 1);
}

#[test]
fn main_needs_two_face_down_cards_and_an_unsuspended_tamer() {
    let mut r = runner(&["KNIGHT"]);
    let t = stocked(&mut r, CARD, &["FD"]);
    r.place_on_field(0, "BASE", Some(0));
    assert!(!main_legal(&mut r, t), "only 1 face-down card");
    let mut r = runner(&["KNIGHT"]);
    let t = stocked(&mut r, CARD, &["FD", "FD"]);
    r.game.players[0].battle_area[t.index as usize].is_suspended = true;
    assert!(!main_legal(&mut r, t), "suspended Tamer");
}

#[test]
fn main_digivolve_may_be_declined_after_paying() {
    let mut r = runner(&["KNIGHT"]);
    let t = stocked(&mut r, CARD, &["FD", "FD"]);
    r.place_on_field(0, "BASE", Some(0));
    activate_field_main(&mut r, t);
    pay_tamer_stock(&mut r);
    r.decline_prompt().unwrap();
    no_prompt(&r);
    assert!(source_ids(&r, t).is_empty(), "cost paid");
    assert_eq!(hand_ids(&r, 0), vec!["KNIGHT"]);
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
    no_prompt(&r);
    assert!(field_ids(&r, 1).contains(&CARD.to_string()));
}
