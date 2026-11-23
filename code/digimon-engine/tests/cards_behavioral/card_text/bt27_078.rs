//! Source: `data/card_bundles/BT27-078.md` (printed text).
//!
//! [On Play] [When Digivolving] If you have 1 or fewer Tamers, you may play 1
//! [Keenan Crier] from your hand without paying the cost.
//! Inherited: [On Deletion] Your opponent trashes 1 card in their hand.

use digimon_engine::debug_runner::DebugRunner;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-078";

fn runner(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(tamer("KEENAN", "Keenan Crier", &["DATA SQUAD"]))
        .add_card(tamer("T1", "Tamer One", &[]))
        .add_card(tamer("T2", "Tamer Two", &[]))
        .add_card(mon("DS3", "Ds", 3, 3, &["DATA SQUAD"]))
        .add_card(mon("TOP", "Top", 5, 7, &[]))
        .add_card(mon("O1", "O1", 3, 3, &[]))
        .add_card(mon("O2", "O2", 3, 3, &[]))
        .add_card(wall())
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .hand(0, hand)
        .hand(1, &["O1", "O2"])
        .memory(10)
        .start()
}

#[test]
fn on_play_with_one_tamer_plays_keenan_free() {
    let mut r = runner(&[CARD, "KEENAN"]);
    r.place_on_field(0, "T1", Some(0));
    play_hand(&mut r, CARD);
    let v = r.pending_selection_view().expect("optional Keenan play");
    assert!(v.is_optional);
    assert_eq!(candidates(&r), vec!["KEENAN"]);
    let mem = r.memory();
    r.choose_prompt_card("KEENAN", None).unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"KEENAN".to_string()));
    assert_eq!(r.memory(), mem);
}

#[test]
fn when_digivolving_with_no_tamers_offers_keenan_and_may_decline() {
    let mut r = runner(&[CARD, "KEENAN"]);
    let base = r.place_on_field(0, "DS3", Some(0));
    assert!(digivolve_hand(&mut r, CARD, base));
    let v = r.pending_selection_view().expect("optional Keenan play");
    assert!(v.is_optional);
    r.decline_prompt().unwrap();
    assert!(hand_ids(&r, 0).contains(&"KEENAN".to_string()));
}

#[test]
fn two_tamers_block_the_play() {
    let mut r = runner(&[CARD, "KEENAN"]);
    r.place_on_field(0, "T1", Some(0));
    r.place_on_field(0, "T2", Some(0));
    play_hand(&mut r, CARD);
    assert!(r.pending_selection().is_none());
    assert!(hand_ids(&r, 0).contains(&"KEENAN".to_string()));
}

#[test]
fn inherited_on_deletion_opponent_chooses_and_trashes_one_hand_card() {
    let mut r = runner(&[]);
    let a = r.place_stack(0, &[CARD, "TOP"]).index;
    lose_battle(&mut r, a);
    let v = r.pending_selection_view().expect("opponent hand trash");
    assert_eq!(v.selecting_player, 1);
    assert!(!v.is_optional);
    r.choose_prompt_card("O2", None).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 1), vec!["O1"]);
    assert_eq!(trash_ids(&r, 1), vec!["O2"]);
}
