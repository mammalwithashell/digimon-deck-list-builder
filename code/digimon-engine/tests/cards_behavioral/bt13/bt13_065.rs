//! BT13-065 PlatinumSukamon — Lv.4 black Champion, DP 2000, cost 3
//! (digivolve: black Lv.3 / 2).
//!
//! Printed text (official Bandai DB bundle + DCGO BT13_065.cs):
//! - [On Deletion] <De-Digivolve 1> 1 of your opponent's Digimon. (Trash 1 card
//!   from the top of 1 of your opponent's Digimon. Stop trashing when you would
//!   trash a level 3 card or the Digimon's last card.)
//! - Inherited: [All Turns] When this Digimon would be deleted, by deleting 1
//!   other Digimon with [Sukamon] in its name, prevent it from being deleted.
//!
//! Synthetic boards only (DCGO replay hunt round 2, cluster reveal-bucket-cards).
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::reveal_bucket_support::*;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn runner() -> DebugRunner {
    let r = DebugRunner::builder()
        .dsl_card("BT13-065")
        .expect("BT13-065 loads")
        .add_card(digimon("SUKA", "Sukamon", 4, 4000, 4, vec![CardColor::Black], &[]))
        .add_card(digimon("L3", "Level Three", 3, 3000, 3, vec![CardColor::Red], &[]))
        .add_card(digimon("L3B", "Level Three B", 3, 3000, 3, vec![CardColor::Red], &[]))
        .add_card(digimon("L4", "Level Four", 4, 5000, 5, vec![CardColor::Red], &[]))
        .add_card(digimon("L5", "Level Five", 5, 7000, 7, vec![CardColor::Red], &[]))
        .add_card(digimon("HOST", "Host Rookie", 3, 2000, 3, vec![CardColor::Black], &[]))
        .deck(0, &["L3"; 6])
        .deck(1, &["L3"; 6])
        .memory(10)
        .start();
    into_main(r, 0, 10)
}

fn delete(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle) {
    r.game.delete_permanent_with_effects(h);
    r.game.drain_effect_queue();
}

#[test]
fn bt13_065_on_deletion_de_digivolves_one_opponent_digimon_by_one() {
    let mut r = runner();
    let plat = r.place_on_field(0, "BT13-065", None);
    let victim = r.place_stack(1, &["L3", "L4", "L5"]);
    delete(&mut r, plat);
    // One opponent Digimon: the mandatory pick resolves (single candidate may
    // still be a prompt); resolve everything.
    r.auto_resolve().unwrap();
    assert_eq!(top_id(&r, victim), "L4", "<De-Digivolve 1> trashed the Lv.5 top card");
    assert!(trash_ids(&r, 1).contains(&"L5".to_string()));
}

#[test]
fn bt13_065_de_digivolve_stops_at_a_level_3_card() {
    let mut r = runner();
    let plat = r.place_on_field(0, "BT13-065", None);
    let victim = r.place_stack(1, &["L3B", "L3"]);
    delete(&mut r, plat);
    r.auto_resolve().unwrap();
    assert_eq!(top_id(&r, victim), "L3", "a level 3 card is never trashed by <De-Digivolve>");
    assert!(trash_ids(&r, 1).is_empty());
}

#[test]
fn bt13_065_on_deletion_chooses_among_several_opponent_digimon() {
    let mut r = runner();
    let plat = r.place_on_field(0, "BT13-065", None);
    let a = r.place_stack(1, &["L3", "L4"]);
    let b = r.place_stack(1, &["L3B", "L5"]);
    delete(&mut r, plat);
    let view = r.pending_selection_view().expect("a target prompt with two candidates");
    assert_eq!(view.valid_action_ids.len(), 2);
    assert!(!r.pending_is_optional(), "the target is mandatory");
    r.auto_resolve().unwrap();
    // Exactly one of the two lost its top card.
    let tops = (top_id(&r, a), top_id(&r, b));
    assert!(
        tops == ("L3".to_string(), "L5".to_string())
            || tops == ("L4".to_string(), "L3B".to_string()),
        "exactly one Digimon was de-digivolved, got {tops:?}"
    );
}

#[test]
fn bt13_065_on_deletion_with_no_opponent_digimon_does_nothing() {
    let mut r = runner();
    let plat = r.place_on_field(0, "BT13-065", None);
    delete(&mut r, plat);
    assert!(r.pending_selection().is_none());
}

#[test]
fn bt13_065_inherited_substitute_deletes_another_sukamon_to_prevent_the_deletion() {
    let mut r = runner();
    let host = r.place_stack(0, &["BT13-065", "HOST"]);
    r.place_on_field(0, "SUKA", None);
    delete(&mut r, host);
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["HOST"], "the host stays, the other Sukamon is deleted");
}

#[test]
fn bt13_065_inherited_substitute_is_not_offered_without_another_sukamon() {
    let mut r = runner();
    let host = r.place_stack(0, &["BT13-065", "HOST"]);
    delete(&mut r, host);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty(), "the host was deleted normally");
}
