//! BT26-066 Salamon — Lv.3 Purple, Mammal/Titan/TS.
//!
//! [Start of Your Main Phase] If your hand has 5 or fewer cards, 1 of your
//! Digimon with the [Titan] trait may digivolve into a Digimon card with the
//! [Titan] trait in the trash with the cost reduced by 2.
//! Inherited: [Your Turn] [Once Per Turn] When your hand is trashed from, this
//! [Titan] trait Digimon may digivolve into [Titamon] or a [Titan] trait
//! Digimon card in the trash with the cost reduced by 1.
//!
//! DCGO: BT26/Purple/BT26_066.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-066";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-066")
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
        .add_card(digimon("PLAIN3", "Plain", CardColor::Purple, 3, 3, &[]))
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
        .add_card(digimon_evo(
            "PLAIN4",
            "Plain Four",
            CardColor::Purple,
            4,
            5,
            &[],
            3,
            3,
        ))
        .add_card(digimon_evo(
            "TITAMON",
            "Titamon",
            CardColor::Purple,
            4,
            5,
            &[],
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
fn bt26_066_start_of_main_digivolves_titan_into_trash_titan() {
    let mut r = setup();
    let sal = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TITAN4");
    push_trash(&mut r, 0, "PLAIN4");
    fire(&mut r, EffectTiming::StartOfYourMainPhase, sal);
    let v = r.pending_selection_view().expect("optional Titan pick");
    assert!(v.is_optional);
    pick_first(&mut r, 0);
    let v = r.pending_selection_view().expect("trash pick");
    assert_eq!(non_pass(&r).len(), 1, "only the [Titan] card: {v:?}");
    pick_trash(&mut r, 0, "TITAN4");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, sal), "TITAN4");
    assert_eq!(r.memory(), 5 - (3 - 2));
}

#[test]
fn bt26_066_start_of_main_can_pick_another_titan_digimon() {
    let mut r = setup();
    let other = r.place_on_field(0, "TITAN3", Some(0));
    let plain = r.place_on_field(0, "PLAIN3", Some(0));
    let sal = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TITAN4");
    fire(&mut r, EffectTiming::StartOfYourMainPhase, sal);
    // Salamon and Gazimon are [Titan] (both can take TITAN4); Plain is not.
    assert_eq!(non_pass(&r).len(), 2);
    pick_first(&mut r, 0); // the first legal [Titan] Digimon (Gazimon, index 0)
    pick_trash(&mut r, 0, "TITAN4");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, other), "TITAN4");
    assert_eq!(top_id(&r, plain), "PLAIN3");
}

#[test]
fn bt26_066_start_of_main_needs_hand_of_five_or_fewer() {
    let mut r = setup();
    let sal = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TITAN4");
    for _ in 0..6 {
        push_hand(&mut r, 0, "FILLER");
    }
    fire(&mut r, EffectTiming::StartOfYourMainPhase, sal);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(top_id(&r, sal), CARD_ID);
}

#[test]
fn bt26_066_inherited_hand_trash_digivolves_into_titamon() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "TITAN3"]);
    push_trash(&mut r, 0, "TITAMON");
    push_hand(&mut r, 0, "FILLER");
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    let v = r.pending_selection_view().expect("optional digivolve pick");
    assert!(v.is_optional);
    pick_trash(&mut r, 0, "TITAMON");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, host), "TITAMON");
    assert_eq!(r.memory(), 5 - (3 - 1));
}

#[test]
fn bt26_066_inherited_decline_refunds_once_per_turn() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "TITAN3"]);
    push_trash(&mut r, 0, "TITAN4");
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    pass(&mut r, 0); // decline the digivolve
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, host), "TITAN3");
    // Nothing happened ⇒ the [Once Per Turn] use was refunded.
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    pick_trash(&mut r, 0, "TITAN4");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, host), "TITAN4");
}

#[test]
fn bt26_066_inherited_requires_a_titan_host() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "PLAIN3"]);
    push_trash(&mut r, 0, "TITAN4");
    push_hand(&mut r, 0, "FILLER");
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(top_id(&r, host), "PLAIN3");
}

#[test]
fn bt26_066_inherited_ignores_opponent_hand_trash() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "TITAN3"]);
    push_trash(&mut r, 0, "TITAN4");
    push_hand(&mut r, 1, "FILLER");
    let t = r.place_on_field(1, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 1, "FILLER");
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(top_id(&r, host), "TITAN3");
}
