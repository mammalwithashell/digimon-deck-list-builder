//! Source: `data/card_bundles/BT27-096.md` (printed text and official Q&A).
//!
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [All Turns] When hands are trashed from, by suspending this Tamer, you may
//! place the top 2 cards of your deck face down under this Tamer.
//! [Main] [Once Per Turn] By trashing 5 bottom face-down cards from under any
//! of your Tamers, you may use 1 Option card with [Ravemon] in its text from
//! your trash without paying the cost.
//! [Security] Play this card without paying the cost.

use digimon_engine::debug_runner::DebugRunner;

use super::bt27_fx::*;

const CARD: &str = "BT27-096";
const FALCOMON: &str = "BT27-074";
const YAKSHA: &str = "BT27-101";

fn runner(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .dsl_card(FALCOMON)
        .unwrap()
        .add_card(printed(YAKSHA))
        .dsl_card(YAKSHA)
        .unwrap()
        .add_card(purple(mon("FALCO", "Falcomon", 3, 3, &[])))
        .add_card(mon("PECK", "Peckmon", 4, 5, &[]))
        .add_card(mon("CROW", "Crowmon", 5, 7, &[]))
        .add_card(mon("RAVE", "Ravemon", 6, 12, &[]))
        .add_card(mon("X", "X", 3, 3, &[]))
        .add_card(mon("Y", "Y", 3, 3, &[]))
        .add_card(mon("FD", "Fd", 3, 3, &[]))
        .add_card(mon("D1", "D1", 3, 3, &[]))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["D1"; 10])
        .deck(1, &["FILL"; 10])
        .hand(0, hand)
        .memory(5)
        .start()
}

#[test]
fn start_of_turn_sets_two_or_less_memory_to_three() {
    for (before, after) in [(2, 3), (0, 3), (5, 5)] {
        let mut r = runner(&[]);
        r.place_on_field(0, CARD, Some(0));
        r.game.memory = before;
        r.end_turn();
        r.end_turn();
        assert_eq!(r.memory(), after, "from {before}");
    }
}

/// Falcomon's On Play places 1 card under this Keenan Crier, then trashes 1.
fn trash_from_hand(r: &mut DebugRunner) {
    play_hand(r, FALCOMON);
    r.choose_prompt_card("X", None).unwrap();
    r.choose_prompt_card(CARD, None).unwrap();
    let v = r.pending_selection_view().expect("Falcomon hand trash");
    assert!(!v.is_optional);
    r.choose_prompt_card("Y", None).unwrap();
}

#[test]
fn hand_trash_suspends_this_tamer_to_place_the_top_two_cards_face_down() {
    let mut r = runner(&[FALCOMON, "X", "Y"]);
    let t = r.place_on_field(0, CARD, Some(0));
    trash_from_hand(&mut r);
    let v = r.pending_selection_view().expect("optional trigger");
    assert!(v.is_optional);
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    no_prompt(&r);
    assert!(r.game.players[0].battle_area[t.index as usize].is_suspended);
    let mut s = source_ids(&r, t);
    s.sort();
    assert_eq!(s, vec!["D1", "D1", "X"], "Falcomon's card plus the top 2");
    assert_eq!(face_down(&r, t), vec![true, true, true]);
}

#[test]
fn hand_trash_trigger_may_be_declined_and_needs_an_unsuspended_tamer() {
    let mut r = runner(&[FALCOMON, "X", "Y"]);
    let t = r.place_on_field(0, CARD, Some(0));
    trash_from_hand(&mut r);
    r.decline_prompt().unwrap();
    no_prompt(&r);
    assert!(!r.game.players[0].battle_area[t.index as usize].is_suspended);
    assert_eq!(source_ids(&r, t), vec!["X"]);

    let mut r = runner(&[FALCOMON, "X", "Y"]);
    let t = r.place_on_field(0, CARD, Some(0));
    r.game.players[0].battle_area[t.index as usize].is_suspended = true;
    trash_from_hand(&mut r);
    no_prompt(&r);
    assert!(r.pending_selection().is_none());
    assert_eq!(source_ids(&r, t), vec!["X"]);
}

#[test]
fn main_needs_five_face_down_cards() {
    let mut r = runner(&[]);
    let t = stocked(&mut r, CARD, &["FD"; 4]);
    assert!(!main_legal(&mut r, t));
    let mut r = runner(&[]);
    let t = stocked(&mut r, CARD, &["FD"; 5]);
    assert!(main_legal(&mut r, t));
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

#[test]
fn main_trashes_five_face_down_cards_to_use_a_ravemon_text_option_from_trash_free() {
    let mut r = runner(&[]);
    let t = stocked(&mut r, CARD, &["FD"; 6]);
    let f = r.place_on_field(0, "FALCO", Some(0));
    for id in [YAKSHA, "PECK", "CROW", "RAVE"] {
        r.inject_trash(0, id);
    }
    let mem = r.memory();
    activate_field_main(&mut r, t);
    pay_tamer_stock(&mut r);
    assert_eq!(source_ids(&r, t).len(), 1, "5 bottom face-down cards trashed");
    let v = r.pending_selection_view().expect("optional Option pick");
    assert!(v.is_optional);
    r.choose_prompt_card(YAKSHA, None).unwrap();
    r.choose_prompt_card("FALCO", None).unwrap();
    r.choose_prompt_card("PECK", None).unwrap();
    r.choose_prompt_card("CROW", None).unwrap();
    r.choose_prompt_card("RAVE", None).unwrap();
    no_prompt(&r);
    assert_eq!(top_id(&r, f), "RAVE");
    assert_eq!(r.memory(), mem, "used without paying the cost");
    r.game.players[0].battle_area[t.index as usize].is_suspended = false;
    assert!(!main_legal(&mut r, t), "Once Per Turn");
}

#[test]
fn main_option_use_may_be_declined_after_paying() {
    let mut r = runner(&[]);
    let t = stocked(&mut r, CARD, &["FD"; 5]);
    r.inject_trash(0, YAKSHA);
    activate_field_main(&mut r, t);
    pay_tamer_stock(&mut r);
    r.decline_prompt().unwrap();
    no_prompt(&r);
    assert!(source_ids(&r, t).is_empty());
    assert!(trash_ids(&r, 0).contains(&YAKSHA.to_string()));
}
