//! BT26-072 Peckmon — Lv.4 Purple, Avian/DATA SQUAD.
//!
//! <Blocker>
//! [On Play] [When Digivolving] By trashing 1 card in your hand or placing it
//! face down under any of your [Keenan Crier]s, delete 1 of your opponent's
//! level 4 or lower Digimon.
//! Inherited: [On Deletion] Your opponent trashes 1 card in their hand.
//!
//! DCGO: BT26/Purple/BT26_072.cs (3-way cost modal only when a Keenan exists).

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-072";

fn setup() -> (DebugRunner, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-072")
        .add_card(filler("FILLER"))
        .add_card(tamer("KEENAN", "Keenan Crier", CardColor::Purple, &["DATA SQUAD"]))
        .add_card(digimon("OPP4", "Opp4", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("OPP5", "Opp5", CardColor::Red, 5, 7, &[]))
        .add_card(digimon("JUNK", "Junk", CardColor::Red, 3, 3, &[]))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(3)
        .start();
    r.set_first_player(0);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    (r, h)
}

#[test]
fn bt26_072_has_blocker() {
    let (r, _) = setup();
    let card = r.compiled_card(CARD_ID).unwrap();
    let dbg = format!("{:?}", card.effects);
    assert!(dbg.contains("Blocker"), "<Blocker> granted");
    let _ = Keyword::Blocker;
}

#[test]
fn bt26_072_trash_cost_then_delete_lv4() {
    let (mut r, h) = setup();
    push_hand(&mut r, 0, "JUNK");
    r.place_on_field(1, "OPP4", Some(0));
    r.place_on_field(1, "OPP5", Some(0));
    r.fire_on_play(0, h.index as usize);
    // No Keenan ⇒ straight to the declinable trash pick.
    let v = r.pending_selection_view().expect("hand pick");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "JUNK");
    let _ = r.auto_resolve();
    assert_eq!(trash_ids(&r, 0), vec!["JUNK".to_string()]);
    assert_eq!(field_ids(&r, 1), vec!["OPP5".to_string()], "Lv.4 deleted, Lv.5 untouched");
}

#[test]
fn bt26_072_declining_cost_deletes_nothing() {
    let (mut r, h) = setup();
    push_hand(&mut r, 0, "JUNK");
    r.place_on_field(1, "OPP4", Some(0));
    r.fire_on_play(0, h.index as usize);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(field_ids(&r, 1), vec!["OPP4".to_string()]);
}

#[test]
fn bt26_072_place_under_keenan_branch() {
    let (mut r, h) = setup();
    let keenan = r.place_on_field(0, "KEENAN", Some(0));
    push_hand(&mut r, 0, "JUNK");
    r.place_on_field(1, "OPP4", Some(0));
    r.fire_on_play(0, h.index as usize);
    // Keenan present ⇒ the 3-way cost modal.
    r.execute_branch(1).expect("place under Keenan");
    pick_first(&mut r, 0); // Keenan
    pick_hand(&mut r, 0, "JUNK");
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, keenan), 1);
    assert!(r.game.players[0].battle_area[keenan.index as usize].card_sources[0].face_down);
    assert!(r.trash_size(0) == 0, "placed, not trashed");
    assert!(field_ids(&r, 1).is_empty(), "the Lv.4 was deleted");
}

#[test]
fn bt26_072_modal_dont_pay() {
    let (mut r, h) = setup();
    r.place_on_field(0, "KEENAN", Some(0));
    push_hand(&mut r, 0, "JUNK");
    r.place_on_field(1, "OPP4", Some(0));
    r.fire_on_play(0, h.index as usize);
    r.execute_branch(2).expect("don't pay");
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(field_ids(&r, 1), vec!["OPP4".to_string()]);
}

#[test]
fn bt26_072_no_activation_with_empty_hand() {
    let (mut r, h) = setup();
    r.place_on_field(1, "OPP4", Some(0));
    r.fire_on_play(0, h.index as usize);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(field_ids(&r, 1), vec!["OPP4".to_string()]);
}

#[test]
fn bt26_072_inherited_on_deletion_opponent_discards() {
    let (mut r, _) = setup();
    push_hand(&mut r, 1, "JUNK");
    push_hand(&mut r, 1, "OPP4");
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    fire(&mut r, EffectTiming::OnDeletion, carrier);
    let v = r.pending_selection_view().expect("opponent chooses");
    assert!(!v.is_optional);
    assert_eq!(r.game.pending_selection.as_ref().unwrap().selecting_player, 1);
    pick_hand(&mut r, 1, "OPP4");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 1), vec!["JUNK".to_string()]);
    assert_eq!(trash_ids(&r, 1), vec!["OPP4".to_string()]);
}
