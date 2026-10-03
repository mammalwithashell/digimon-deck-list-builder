//! BT26-069 Dobermon — Lv.4 Purple, Dark Animal/Titan/TS.
//!
//! When this card is trashed from the hand, if your hand has 5 or fewer
//! cards, <Draw 1>.
//! [On Play] [When Digivolving] By trashing 1 card in your hand, delete 1
//! level 4 or lower Digimon.
//! Inherited: [Your Turn] [Once Per Turn] When your hand is trashed from, this
//! [Titan] trait Digimon may digivolve into [Titamon] or a [Titan] trait
//! Digimon card in the trash with the cost reduced by 1.
//!
//! DCGO: BT26/Purple/BT26_069.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-069";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-069")
        .from_dsl_yaml(HAND_TRASHER_YAML)
        .expect("trasher")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "TITAN3",
            "Gazimon",
            CardColor::Purple,
            3,
            3,
            &["Titan"],
        ))
        .add_card(digimon("OPP4", "Opp4", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("OPP5", "Opp5", CardColor::Red, 5, 7, &[]))
        .add_card(digimon_evo(
            "TITAN4",
            "Big Titan",
            CardColor::Purple,
            4,
            5,
            &["Titan"],
            3,
            3,
        ))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_069_trashed_from_hand_draws_one() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FILLER");
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    let deck_before = r.deck_size(0);
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, CARD_ID);
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck_before - 1, "<Draw 1>");
    assert_eq!(r.hand_size(0), 2);
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn bt26_069_trashed_from_hand_with_big_hand_does_not_draw() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    for _ in 0..6 {
        push_hand(&mut r, 0, "FILLER");
    }
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    let deck_before = r.deck_size(0);
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, CARD_ID);
    let _ = r.auto_resolve();
    assert_eq!(
        r.deck_size(0),
        deck_before,
        "6 cards left in hand ⇒ no draw"
    );
}

#[test]
fn bt26_069_already_in_trash_does_not_fire_on_other_discards() {
    let mut r = setup();
    push_trash(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FILLER");
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    let deck_before = r.deck_size(0);
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck_before);
}

#[test]
fn bt26_069_opponent_effect_trashing_it_still_draws_for_owner() {
    let mut r = setup();
    push_hand(&mut r, 1, CARD_ID);
    let t = r.place_on_field(1, "T-HANDTRASH", Some(0));
    let deck_before = r.deck_size(1);
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 1, CARD_ID);
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(1), deck_before - 1);
    assert_eq!(r.hand_size(1), 1);
}

#[test]
fn bt26_069_on_play_trash_cost_deletes_level_four_or_lower_either_side() {
    let mut r = setup();
    let dob = r.place_on_field(0, CARD_ID, Some(0));
    let opp4 = r.place_on_field(1, "OPP4", Some(0));
    r.place_on_field(1, "OPP5", Some(0));
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::OnPlay, dob);
    let v = r.pending_selection_view().expect("optional hand cost");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "FILLER");
    let v = r.pending_selection_view().expect("delete pick");
    assert!(!v.is_optional, "the deletion is mandatory once paid");
    // Dobermon itself (Lv.4) and OPP4 — OPP5 is too high.
    assert_eq!(non_pass(&r).len(), 2);
    pick_any_field(&mut r, 0, opp4);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP5".to_string()]);
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
}

#[test]
fn bt26_069_when_digivolving_decline_cost_deletes_nothing() {
    let mut r = setup();
    let dob = r.place_stack(0, &["TITAN3", CARD_ID]);
    r.place_on_field(1, "OPP4", Some(0));
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::WhenDigivolving, dob);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP4".to_string()]);
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn bt26_069_inherited_digivolves_on_own_hand_trash() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "TITAN3"]);
    push_trash(&mut r, 0, "TITAN4");
    push_hand(&mut r, 0, "FILLER");
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    pick_trash(&mut r, 0, "TITAN4");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, host), "TITAN4");
    assert_eq!(r.memory(), 5 - 2);
}
