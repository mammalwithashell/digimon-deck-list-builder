//! P-181 Royal Base — Option, cost 2, green/black, trait [Royal Base].
//!
//! Printed text (official Bandai DB bundle + DCGO P_181.cs):
//! - {Security} [Your Turn] [Once Per Turn] When any of your Digimon would
//!   digivolve into a Digimon card with the [Royal Base] trait, reduce the
//!   digivolution cost by 1.
//! - [Main] Add your top security card to the hand. Then, place this card face
//!   up as your bottom security card.
//! - Security Effect [Security] You may play 1 level 5 or lower Digimon card
//!   with the [Royal Base] trait from your hand without paying the cost.
//! - Official Q&A: with no security card, [Main] adds nothing and the card is
//!   still placed in the security stack.
//!
//! CR 4.3 15-14-5-1: a {Security} effect works while the card is FACE UP in the
//! security stack.
//!
//! Synthetic boards only (DCGO replay hunt round 2, cluster reveal-bucket-cards).
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::reveal_bucket_support::*;
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::OptionPlayResult;

fn royal_digimon(id: &str, level: u8) -> CardData {
    let mut c = digimon(id, "Royal Digimon", level, 4000, 4, vec![CardColor::Green], &["Royal Base"]);
    if level == 4 {
        c = with_evo(c, CardColor::Green, 3, 3);
    }
    if level == 3 {
        c = with_evo(c, CardColor::Green, 2, 1);
    }
    c
}

fn runner(hand: &[&str]) -> DebugRunner {
    let r = DebugRunner::builder()
        .dsl_card("P-181")
        .expect("P-181 loads")
        .add_card(royal_digimon("ROY4", 4))
        .add_card(royal_digimon("ROY4B", 4))
        .add_card(royal_digimon("ROY3", 3))
        .add_card(royal_digimon("ROY6", 6))
        .add_card(with_evo(
            digimon("PLAIN4", "Plain Four", 4, 4000, 4, vec![CardColor::Green], &[]),
            CardColor::Green,
            3,
            3,
        ))
        .add_card(digimon("BASE", "Base Digimon", 3, 3000, 3, vec![CardColor::Green], &[]))
        .add_card(digimon("BASE-B", "Black Base Digimon", 3, 3000, 3, vec![CardColor::Black], &[]))
        .add_card(digimon("BASE2", "Base Digimon 2", 3, 3000, 3, vec![CardColor::Green], &[]))
        .add_card(digimon("SEC1", "Security One", 3, 1000, 3, vec![CardColor::Red], &[]))
        .add_card(digimon("SEC2", "Security Two", 3, 1000, 3, vec![CardColor::Red], &[]))
        .add_card(digimon("ATK", "Attacker", 4, 5000, 5, vec![CardColor::Red], &[]))
        .deck(0, &["SEC1"; 8])
        .deck(1, &["SEC1"; 8])
        .hand(0, hand)
        .memory(10)
        .start();
    into_main(r, 0, 10)
}

#[test]
fn p_181_main_adds_top_security_to_hand_and_places_itself_face_up_at_the_bottom() {
    let mut r = runner(&["P-181"]);
    // CR 4.3 4-22-3: a green/black Option needs BOTH colours on your field.
    r.place_on_field(0, "BASE", Some(0));
    r.place_on_field(0, "BASE-B", Some(0));
    put_in_security(&mut r, 0, "SEC1", false); // bottom of the 2-card stack
    put_in_security(&mut r, 0, "SEC2", false); // top
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).contains(&"SEC2".to_string()), "the TOP security card was added");
    let stack = security_top_first(&r, 0);
    assert_eq!(stack, vec!["SEC1", "P-181"], "Royal Base is the new bottom security card");
    let p181_index = r.game.players[0].security[0].card_index;
    assert!(
        r.game.players[0].face_up_security.contains(&p181_index),
        "placed FACE UP"
    );
    assert!(!field_ids(&r, 0).contains(&"P-181".to_string()), "not a Delay Option");
}

#[test]
fn p_181_main_with_no_security_only_places_the_card() {
    let mut r = runner(&["P-181"]);
    r.place_on_field(0, "BASE", Some(0));
    r.place_on_field(0, "BASE-B", Some(0));
    r.game.players[0].security.clear();
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert_eq!(security_top_first(&r, 0), vec!["P-181"], "official Q&A: only placed");
    assert!(hand_ids(&r, 0).is_empty(), "nothing was added to the hand");
}

#[test]
fn p_181_face_up_in_security_reduces_royal_base_digivolution_by_1_once_per_turn() {
    let mut r = runner(&[]);
    put_in_security(&mut r, 0, "P-181", true);
    let a = r.place_on_field(0, "BASE", Some(0));
    let b = r.place_on_field(0, "BASE2", Some(0));
    let before = r.memory();
    digivolve_via_action(&mut r, 0, "ROY4", a);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, a), "ROY4");
    assert_eq!(before - r.memory(), 2, "printed cost 3 reduced by 1");
    // [Once Per Turn]: the second Royal Base digivolve is not reduced.
    let mid = r.memory();
    digivolve_via_action(&mut r, 0, "ROY4B", b);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, b), "ROY4B");
    assert_eq!(mid - r.memory(), 3, "once per turn: the second digivolve pays full price");
}

#[test]
fn p_181_does_not_reduce_a_digivolution_into_a_non_royal_base_card() {
    let mut r = runner(&[]);
    put_in_security(&mut r, 0, "P-181", true);
    let a = r.place_on_field(0, "BASE", Some(0));
    let before = r.memory();
    digivolve_via_action(&mut r, 0, "PLAIN4", a);
    let _ = r.auto_resolve();
    assert_eq!(before - r.memory(), 3, "no [Royal Base] trait: full cost");
}

#[test]
fn p_181_face_down_in_security_has_no_active_effect() {
    let mut r = runner(&[]);
    put_in_security(&mut r, 0, "P-181", false);
    let a = r.place_on_field(0, "BASE", Some(0));
    let before = r.memory();
    digivolve_via_action(&mut r, 0, "ROY4", a);
    let _ = r.auto_resolve();
    assert_eq!(before - r.memory(), 3, "CR 15-14-5-1: face-down {{Security}} effects are inactive");
}

#[test]
fn p_181_reduction_applies_only_on_your_turn() {
    let mut r = runner(&[]);
    put_in_security(&mut r, 0, "P-181", true);
    let a = r.place_on_field(0, "BASE", Some(0));
    // Seat 1's turn: seat 0's reducer is a [Your Turn] effect.
    let mut r = into_main(r, 1, 10);
    let b = r.place_on_field(1, "BASE2", Some(0));
    let before = r.memory();
    digivolve_via_action(&mut r, 1, "ROY4", b);
    let _ = r.auto_resolve();
    assert_eq!(before - r.memory(), 3, "the opponent's Royal Base is not reduced by my security card");
}

#[test]
fn p_181_security_effect_plays_a_level_5_or_lower_royal_base_digimon_free() {
    let mut r = runner(&[]);
    let attacker = r.place_on_field(0, "ATK", Some(0));
    put_in_security(&mut r, 1, "P-181", false);
    r.add_to_hand(1, "ROY4");
    r.add_to_hand(1, "ROY6");
    r.attack_player(attacker, 1, false);
    let view = r.pending_selection_view().expect("the [Security] play pick");
    assert!(view.is_optional, "'you may'");
    let pick = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != PASS)
        .unwrap();
    assert_eq!(
        view.valid_action_ids.iter().filter(|a| **a != PASS).count(),
        1,
        "only the level <= 5 [Royal Base] Digimon (not the level 6)"
    );
    r.execute_action(view.selecting_player, pick).unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&"ROY4".to_string()));
}
