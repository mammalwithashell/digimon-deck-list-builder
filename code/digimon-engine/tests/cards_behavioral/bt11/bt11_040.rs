//! BT11-040 Sukamon — Lv.4 yellow/black Champion, DP 1000, cost 3
//! (digivolve: yellow Lv.3 / 2, black Lv.3 / 2).
//!
//! Printed text (official Bandai DB bundle + DCGO BT11_040.cs):
//! - [On Deletion] Reveal the top 3 cards of your deck. Add 1 card with
//!   [Chuumon], [Sukamon], or [Etemon] in its name among them to your hand.
//!   Trash the rest.
//! - Inherited: [All Turns] When this Digimon would be deleted, by deleting 1
//!   other Digimon with [Sukamon] in its name, prevent that deletion.
//!
//! Synthetic boards only (DCGO replay hunt round 2, cluster reveal-bucket-cards).
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::reveal_bucket_support::*;
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn runner(deck_top_first_ids: &[&str]) -> DebugRunner {
    let r = DebugRunner::builder()
        .dsl_card("BT11-040")
        .expect("BT11-040 loads")
        .add_card(digimon("SUKA", "Sukamon", 4, 4000, 4, vec![CardColor::Black], &[]))
        .add_card(digimon("CHUU", "Chuumon", 3, 3000, 3, vec![CardColor::Black], &[]))
        .add_card(digimon("ETE", "MetalEtemon", 6, 12000, 12, vec![CardColor::Black], &[]))
        .add_card(card("OPT-SUKA", "Sukamon Option", CardKind::Option, vec![CardColor::Black]))
        .add_card(digimon("MON", "Plain Digimon", 3, 3000, 3, vec![CardColor::Red], &[]))
        .add_card(digimon("HOST", "Host Rookie", 3, 2000, 3, vec![CardColor::Yellow], &[]))
        .deck(0, &top_first(deck_top_first_ids))
        .deck(1, &["MON"; 5])
        .memory(10)
        .start();
    into_main(r, 0, 10)
}

fn delete(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle) {
    r.game.delete_permanent_with_effects(h);
    r.game.drain_effect_queue();
}

#[test]
fn bt11_040_on_deletion_adds_one_chuumon_sukamon_or_etemon_name_card_and_trashes_the_rest() {
    let mut r = runner(&["MON", "ETE", "CHUU", "MON"]);
    let suka = r.place_on_field(0, "BT11-040", None);
    delete(&mut r, suka);
    let view = r.pending_selection_view().expect("the name pick is a prompt");
    // Chuumon and MetalEtemon both qualify; the add is mandatory (no PASS).
    assert_eq!(view.valid_action_ids.len(), 2, "[Chuumon] and [Etemon] cards qualify");
    assert!(!view.is_optional, "the add is mandatory (no 'may')");
    pick_reveal(&mut r, 0, "ETE");
    r.auto_resolve().unwrap();
    assert_eq!(hand_ids(&r, 0), vec!["ETE"]);
    let mut trash = trash_ids(&r, 0);
    trash.sort();
    assert_eq!(
        trash,
        vec!["BT11-040", "CHUU", "MON"],
        "the carrier and the two unpicked revealed cards are in the trash"
    );
    assert_eq!(deck_top_first(&r, 0), vec!["MON"], "the 4th card was never revealed");
}

#[test]
fn bt11_040_on_deletion_matches_sukamon_option_cards_too() {
    // "1 card with [..] in its name": any card type qualifies.
    let mut r = runner(&["OPT-SUKA", "MON", "MON"]);
    let suka = r.place_on_field(0, "BT11-040", None);
    delete(&mut r, suka);
    pick_reveal(&mut r, 0, "OPT-SUKA");
    r.auto_resolve().unwrap();
    assert_eq!(hand_ids(&r, 0), vec!["OPT-SUKA"]);
}

#[test]
fn bt11_040_on_deletion_with_no_matching_name_trashes_all_three() {
    let mut r = runner(&["MON", "MON", "MON", "SUKA"]);
    let suka = r.place_on_field(0, "BT11-040", None);
    delete(&mut r, suka);
    r.auto_resolve().unwrap();
    assert!(hand_ids(&r, 0).is_empty());
    assert_eq!(trash_ids(&r, 0).len(), 4, "carrier + 3 revealed cards trashed");
    assert_eq!(deck_top_first(&r, 0), vec!["SUKA"]);
}

#[test]
fn bt11_040_on_deletion_is_not_an_inherited_effect() {
    // As a digivolution SOURCE the [On Deletion] effect does not fire.
    let mut r = runner(&["CHUU", "MON", "MON"]);
    let host = r.place_stack(0, &["BT11-040", "HOST"]);
    delete(&mut r, host);
    // The inherited substitute may prompt (no other Sukamon -> it must not).
    let _ = r.auto_resolve();
    assert!(
        hand_ids(&r, 0).is_empty(),
        "the effect text is on the face-up card only"
    );
}

#[test]
fn bt11_040_inherited_substitute_deletes_another_sukamon_to_prevent_the_deletion() {
    let mut r = runner(&["MON"; 4]);
    let host = r.place_stack(0, &["BT11-040", "HOST"]);
    let other = r.place_on_field(0, "SUKA", None);
    delete(&mut r, host);
    assert!(r.pending_is_optional(), "the substitute is optional ('by deleting ...')");
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    let field = field_ids(&r, 0);
    assert_eq!(field, vec!["HOST"], "the host stayed; the other Sukamon was deleted instead");
    assert!(trash_ids(&r, 0).contains(&"SUKA".to_string()));
}

#[test]
fn bt11_040_inherited_substitute_can_be_declined_and_the_host_is_deleted() {
    let mut r = runner(&["MON"; 4]);
    let host = r.place_stack(0, &["BT11-040", "HOST"]);
    r.place_on_field(0, "SUKA", None);
    delete(&mut r, host);
    r.decline_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["SUKA"], "declined: the host was deleted normally");
}

#[test]
fn bt11_040_inherited_substitute_needs_another_sukamon_name_digimon() {
    let mut r = runner(&["MON"; 4]);
    let host = r.place_stack(0, &["BT11-040", "HOST"]);
    r.place_on_field(0, "MON", None);
    delete(&mut r, host);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["MON"], "no other [Sukamon]: nothing to substitute");
}
