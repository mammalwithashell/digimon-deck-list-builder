//! BT26-092 Shota Kuroi — Tamer, Black, TS.
//!
//! [Start of Your Main Phase] By trashing 1 [TS] trait card from your hand,
//! <Draw 1> and gain 1 memory.
//! [Opponent's Turn] When one of your opponent's Digimon attacks, by returning
//! 1 of your [TS] trait Tamers to the bottom of the deck, you may change the
//! attack target to 1 of your Digimon with the [TS] trait.
//! [Security] Play this card without paying the cost.
//!
//! DCGO: BT26/Black/BT26_092.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-092";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-092")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "TS-CARD",
            "TS Card",
            CardColor::Black,
            3,
            3,
            &["TS"],
        ))
        .add_card(digimon(
            "PLAIN",
            "Plain",
            CardColor::Black,
            3,
            3,
            &["Beast"],
        ))
        .add_card(digimon(
            "TS-D",
            "TS Defender",
            CardColor::Black,
            4,
            5,
            &["TS"],
        ))
        .add_card(digimon("ATK", "Attacker", CardColor::Red, 6, 9, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_092_somp_trash_ts_draws_and_gains_memory() {
    let mut r = setup();
    let s = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "PLAIN");
    push_hand(&mut r, 0, "TS-CARD");
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, s);
    let v = r.pending_selection_view().expect("optional trash pick");
    assert!(v.is_optional);
    assert_eq!(v.valid_action_ids.len(), 1, "only the TS card");
    pick_hand(&mut r, 0, "TS-CARD");
    let _ = r.auto_resolve();
    assert_eq!(trash_ids(&r, 0), vec!["TS-CARD".to_string()]);
    assert_eq!(r.hand_size(0), 2, "PLAIN + Draw 1");
    assert_eq!(r.memory(), mem + 1);
}

#[test]
fn bt26_092_somp_declined() {
    let mut r = setup();
    let s = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "TS-CARD");
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, s);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.memory(), mem);
}

/// Hand the turn to player 1 so player 0's [Opponent's Turn] clause is live.
fn opponents_turn(r: &mut DebugRunner) {
    r.pass_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 1);
}

#[test]
fn bt26_092_redirects_attack_to_ts_digimon_by_bottom_decking_ts_tamer() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "TS-D", Some(0));
    opponents_turn(&mut r);
    let atk = r.place_on_field(1, "ATK", Some(0));
    let deck_before = r.deck_size(0);
    r.attack_player(atk, 0, false);
    // The optional trigger's first prompt is the (declinable) TS Tamer pick.
    let v = r.pending_selection_view().expect("TS Tamer pick");
    assert!(v.is_optional, "Shota's trigger is optional");
    assert_eq!(v.valid_action_ids.len(), 1, "only Shota is a TS Tamer");
    pick_first(&mut r, 0);
    // Pick the TS Digimon as the new attack target.
    let v = r.pending_selection_view().expect("defender pick");
    assert!(v.is_optional);
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck_before + 1, "Shota bottom-decked");
    assert_eq!(
        deck_ids(&r, 0).first().map(String::as_str),
        Some(CARD_ID),
        "returned to the BOTTOM of the deck"
    );
    assert_eq!(r.security_count(0), 3, "the attack never reached security");
    assert!(
        !field_ids(&r, 0).contains(&"TS-D".to_string()),
        "the TS Digimon battled the 6000-DP attacker and was deleted"
    );
}

#[test]
fn bt26_092_declining_the_tamer_return_keeps_original_target() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "TS-D", Some(0));
    opponents_turn(&mut r);
    let atk = r.place_on_field(1, "ATK", Some(0));
    r.attack_player(atk, 0, false);
    pass(&mut r, 0); // decline the Tamer pick
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert!(field_ids(&r, 0).contains(&"TS-D".to_string()));
    assert_eq!(r.security_count(0), 2, "the attack checked security");
}

#[test]
fn bt26_092_does_not_trigger_on_own_turn() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    let atk = r.place_on_field(0, "TS-D", Some(0));
    r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&CARD_ID.to_string()));
}
