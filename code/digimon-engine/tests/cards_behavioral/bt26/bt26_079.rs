//! BT26-079 ZombiePlutomon — Lv.6 Purple, Undead/Titan/TS.
//!
//! {Trash} [Main] If your hand has 5 or fewer cards, play this card with the
//! cost reduced by 4.
//! <Security A. +1> <Decode ([Plutomon])> <Retaliation>
//! [On Play] [When Digivolving] [When Attacking] By trashing 1 card in your
//! hand, delete 1 of your opponent's level 6 or lower Digimon.
//! [All Turns] [Once Per Turn] When any of your opponent's Digimon are played
//! or digivolve, both players trash cards in their hand so that they have 4
//! left.
//!
//! DCGO: BT26/Purple/BT26_079.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-079";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-079")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "PLUTO",
            "Plutomon",
            CardColor::Purple,
            6,
            13,
            &["Titan"],
        ))
        .add_card(digimon("OPP6", "Opp6", CardColor::Red, 6, 12, &[]))
        .add_card(digimon("OPP7", "Opp7", CardColor::Red, 7, 14, &[]))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

fn trash_index(r: &DebugRunner, p: u8, id: &str) -> usize {
    r.game.players[p as usize]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .expect("in trash")
}

#[test]
fn bt26_079_trash_main_plays_itself_at_minus_four() {
    let mut r = setup();
    push_trash(&mut r, 0, CARD_ID);
    let idx = trash_index(&r, 0, CARD_ID);
    assert!(
        r.game.activate_trash_main(0, idx),
        "[Main] from trash offered"
    );
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
    assert_eq!(r.memory(), 5 - (12 - 4));
    assert!(trash_ids(&r, 0).is_empty());
}

#[test]
fn bt26_079_trash_main_needs_hand_of_five_or_fewer() {
    let mut r = setup();
    push_trash(&mut r, 0, CARD_ID);
    for _ in 0..6 {
        push_hand(&mut r, 0, "FILLER");
    }
    let idx = trash_index(&r, 0, CARD_ID);
    assert!(!r.game.activate_trash_main(0, idx));
    assert!(field_ids(&r, 0).is_empty());
}

#[test]
fn bt26_079_on_play_trash_cost_deletes_level_six_or_lower() {
    let mut r = setup();
    let z = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP6", Some(0));
    r.place_on_field(1, "OPP7", Some(0));
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::OnPlay, z);
    let v = r.pending_selection_view().expect("optional hand cost");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "FILLER");
    let v = r.pending_selection_view().expect("delete pick");
    assert!(!v.is_optional);
    assert_eq!(non_pass(&r).len(), 1);
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP7".to_string()]);
}

#[test]
fn bt26_079_when_attacking_has_no_once_per_turn() {
    let mut r = setup();
    let z = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::WhenAttacking, z);
    pick_hand(&mut r, 0, "FILLER");
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::WhenAttacking, z);
    assert!(r.pending_selection_view().is_some());
}

#[test]
fn bt26_079_opponent_play_makes_both_hands_four() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    for _ in 0..6 {
        push_hand(&mut r, 0, "FILLER");
    }
    for _ in 0..5 {
        push_hand(&mut r, 1, "FILLER");
    }
    let opp = r.place_on_field(1, "OPP6", Some(0));
    r.fire_play_event_triggers(1, opp.index as usize, false, false);
    assert_eq!(
        r.pending_selection_view()
            .expect("own discard")
            .selecting_player,
        0
    );
    drain_first(&mut r);
    assert_eq!(r.hand_size(0), 4);
    assert_eq!(r.hand_size(1), 4);
    // [Once Per Turn]
    for _ in 0..3 {
        push_hand(&mut r, 1, "FILLER");
    }
    let opp2 = r.place_on_field(1, "OPP7", Some(0));
    r.fire_play_event_triggers(1, opp2.index as usize, false, false);
    drain_first(&mut r);
    assert_eq!(r.hand_size(1), 7);
}

#[test]
fn bt26_079_own_play_does_not_trigger_hand_trim() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    for _ in 0..6 {
        push_hand(&mut r, 0, "FILLER");
    }
    let own = r.place_on_field(0, "OPP6", Some(0));
    r.fire_play_event_triggers(0, own.index as usize, false, false);
    drain_first(&mut r);
    assert_eq!(r.hand_size(0), 6);
}

#[test]
fn bt26_079_decode_plays_plutomon_source() {
    let mut r = setup();
    let z = r.place_stack(0, &["PLUTO", CARD_ID]);
    r.game.set_effect_source_player_for_test(Some(1));
    r.game.delete_permanent_with_effects(z);
    r.game.set_effect_source_player_for_test(None);
    let v = r.pending_selection_view().expect("<Decode> offered");
    assert!(v.is_optional);
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["PLUTO".to_string()]);
}
