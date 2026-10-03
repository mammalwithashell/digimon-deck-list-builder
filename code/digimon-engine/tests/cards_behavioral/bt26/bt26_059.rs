//! BT26-059 Plutomon — Lv.6 Black/Purple, Shaman/Titan/TS.
//!
//! When this card would be played, if your hand has fewer cards than your
//! opponent's, reduce the cost by 6.
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] By trashing 1
//! card in your hand, if it's your turn, you may play 1 [Titan] trait Digimon
//! card from your trash with the cost reduced by 7. This effect can't play
//! [Plutomon].
//! [All Turns] [Once Per Turn] When hands are trashed from, you may delete all
//! of your opponent's lowest level Digimon.
//!
//! DCGO: BT26/Purple/BT26_059.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-059";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-059")
        .from_dsl_yaml(HAND_TRASHER_YAML)
        .expect("trasher")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "TITAN5",
            "Big Titan",
            CardColor::Purple,
            5,
            9,
            &["Titan"],
        ))
        .add_card(digimon(
            "PLUTO",
            "Plutomon",
            CardColor::Purple,
            6,
            9,
            &["Titan"],
        ))
        .add_card(digimon("OPP3", "Opp3", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("OPP3B", "Opp3b", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("OPP5", "Opp5", CardColor::Red, 5, 7, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_059_play_cost_reduced_with_smaller_hand() {
    let mut r = setup();
    let idx = r.add_to_hand(0, CARD_ID);
    for _ in 0..2 {
        push_hand(&mut r, 1, "FILLER");
    }
    // Hand: just Plutomon (1) < opponent 2.
    r.play(0, idx).expect("played");
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 5 - (13 - 6));
}

#[test]
fn bt26_059_play_cost_full_with_equal_hands() {
    let mut r = setup();
    let idx = r.add_to_hand(0, CARD_ID);
    push_hand(&mut r, 1, "FILLER");
    r.play(0, idx).expect("played");
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 5 - 13);
}

#[test]
fn bt26_059_trash_cost_plays_titan_from_trash_minus_seven_not_plutomon() {
    let mut r = setup();
    let pluto = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TITAN5");
    push_trash(&mut r, 0, "PLUTO");
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::WhenAttacking, pluto);
    let v = r.pending_selection_view().expect("optional hand cost");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "FILLER");
    // Hand trash fires Plutomon's own [All Turns] observer only if the
    // opponent has a Digimon — none here.
    let v = r.pending_selection_view().expect("trash pick");
    assert!(v.is_optional);
    assert_eq!(non_pass(&r).len(), 1, "[Plutomon] excluded");
    pick_trash(&mut r, 0, "TITAN5");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"TITAN5".to_string()));
    assert_eq!(r.memory(), 5 - (9 - 7));
}

#[test]
fn bt26_059_declined_cost_refunds_once_per_turn() {
    let mut r = setup();
    let pluto = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::OnPlay, pluto);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::OnPlay, pluto);
    assert!(r.pending_selection_view().is_some(), "refunded");
}

#[test]
fn bt26_059_hand_trash_may_delete_all_lowest_level() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP3", Some(0));
    r.place_on_field(1, "OPP3B", Some(0));
    r.place_on_field(1, "OPP5", Some(0));
    // Opponent's own effect trashes from the opponent's hand — "hands" = any.
    push_hand(&mut r, 1, "FILLER");
    let t = r.place_on_field(1, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 1, "FILLER");
    r.accept_optional_trigger()
        .expect("optional delete offered");
    let _ = r.auto_resolve();
    // T-HANDTRASH is level 3 too: the opponent's lowest level is 3.
    assert_eq!(field_ids(&r, 1), vec!["OPP5".to_string()]);
}

#[test]
fn bt26_059_hand_trash_delete_can_be_declined() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP3", Some(0));
    push_hand(&mut r, 0, "FILLER");
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    r.decline_optional_trigger().expect("declined");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP3".to_string()]);
}
