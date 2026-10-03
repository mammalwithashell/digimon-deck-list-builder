//! BT26-055 Giromon — Lv.5 Black, Mine/DM/Ver.3.
//!
//! <Fragment <2>>
//! [On Play] [When Digivolving] [Counter] [Once Per Turn] You may place 1 card
//! in your hand face down as this Digimon's bottom digivolution card. Then,
//! you may delete 1 of your Digimon with the [Ver.3] trait and all of your
//! opponent's Digimon with the lowest play cost.
//! Inherited: [All Turns] [Once Per Turn] When this Digimon would leave the
//! battle area, trash your opponent's top security card.
//!
//! DCGO: BT26/Black/BT26_055.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-055";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-055")
        .add_card(filler("FILLER"))
        .add_card(digimon("VER3", "Gear", CardColor::Black, 4, 5, &["Ver.3"]))
        .add_card(digimon("OPPA", "OppA", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("OPPB", "OppB", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("OPPC", "OppC", CardColor::Red, 5, 7, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(1, &["FILLER"; 3])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

fn own_field_action(h: digimon_engine::permanent::PermanentHandle) -> u16 {
    digimon_engine::action::space::encode_attack(0, h.index as u16)
}

#[test]
fn bt26_055_places_face_down_then_deletes_ver3_and_lowest_cost_batch() {
    let mut r = setup();
    let giro = r.place_on_field(0, CARD_ID, Some(0));
    let ver3 = r.place_on_field(0, "VER3", Some(0));
    r.place_on_field(1, "OPPA", Some(0));
    r.place_on_field(1, "OPPB", Some(0));
    r.place_on_field(1, "OPPC", Some(0));
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::OnPlay, giro);
    let v = r.pending_selection_view().expect("optional hand pick");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "FILLER");
    assert_eq!(sources(&r, giro), 1);
    assert_eq!(face_down_flags(&r, giro), vec![true]);
    r.execute_branch(0).expect("Yes — delete");
    // Giromon itself is [Ver.3] too: 2 candidates.
    assert_eq!(non_pass(&r).len(), 2);
    r.execute_action(0, own_field_action(ver3)).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(
        field_ids(&r, 1),
        vec!["OPPC".to_string()],
        "both cost-3 ties deleted"
    );
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
}

#[test]
fn bt26_055_no_deletes_on_no_and_once_per_turn_holds() {
    let mut r = setup();
    let giro = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPPA", Some(0));
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::WhenDigivolving, giro);
    pick_hand(&mut r, 0, "FILLER");
    r.execute_branch(1).expect("No");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPPA".to_string()]);
    // A card was placed ⇒ the [Once Per Turn] is spent.
    fire(&mut r, EffectTiming::WhenDigivolving, giro);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn bt26_055_doing_nothing_refunds_once_per_turn() {
    let mut r = setup();
    let giro = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPPA", Some(0));
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::OnPlay, giro);
    pass(&mut r, 0); // place nothing
    r.execute_branch(1).expect("No");
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::OnPlay, giro);
    pass(&mut r, 0);
    r.execute_branch(0).expect("Yes this time");
    // Only Giromon is [Ver.3]: it is picked (mandatory) and deleted with OPPA.
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).is_empty());
    assert!(field_ids(&r, 0).is_empty());
}

#[test]
fn bt26_055_counter_timing_fires() {
    let mut r = setup();
    let giro = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::CounterEffect, giro);
    let v = r.pending_selection_view().expect("hand pick at [Counter]");
    assert!(v.is_optional);
}

#[test]
fn bt26_055_has_fragment_2() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let dbg = format!("{card:?}");
    assert!(dbg.contains("Fragment"), "Fragment keyword granted");
}

#[test]
fn bt26_055_inherited_would_leave_trashes_opponent_top_security() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    r.game.set_effect_source_player_for_test(Some(1));
    r.game.delete_permanent_with_effects(carrier);
    r.game.set_effect_source_player_for_test(None);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 2);
    assert!(field_ids(&r, 0).is_empty(), "it still leaves");
}
