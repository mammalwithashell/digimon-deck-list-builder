//! BT26-093 Reina Sakuya — Tamer, Black, Cost 3. Traits: Glowing Dawn, BEATBREAK.
//!
//! [Start of Your Main Phase] By placing 1 [BEATBREAK] trait card from your hand
//! face down under this Tamer, <Draw 1> and gain 1 memory.
//! [All Turns] When a Digimon attacks, by suspending this Tamer, place the top
//! card of your deck face down under this Tamer. After, 1 of your [BEATBREAK]
//! trait Digimon gains <Collision> and <Blocker> for the turn.
//! [Security] Play this card without paying the cost.
//!
//! DCGO: BT26/Black/BT26_093.cs (OnAllyAttack + CanTriggerOnPermanentAttack(any)).

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};

const CARD_ID: &str = "BT26-093";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-093")
        .add_card(filler("FILLER"))
        .add_card(tamer("SECT", "SecTamer", CardColor::Red, &[]))
        .add_card(digimon(
            "BB",
            "Beatbreak",
            CardColor::Black,
            3,
            3,
            &["BEATBREAK"],
        ))
        .add_card(digimon("PLAIN", "Plain", CardColor::Black, 3, 3, &[]))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("DECKTOP", "DeckTop", CardColor::Red, 3, 3, &[]))
        .deck(0, &["FILLER", "FILLER", "DECKTOP"])
        .deck(1, &["FILLER"; 6])
        .security(0, &["SECT", "SECT", "SECT"])
        .security(1, &["SECT", "SECT", "SECT"])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_093_somp_places_beatbreak_card_draws_and_gains_memory() {
    let mut r = setup();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "BB");
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, t);
    let v = r.pending_selection_view().expect("optional hand pick");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "BB");
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, t), vec!["BB".to_string()]);
    assert_eq!(face_down_flags(&r, t), vec![true]);
    assert_eq!(hand_ids(&r, 0), vec!["DECKTOP".to_string()], "<Draw 1>");
    assert_eq!(r.memory(), mem + 1);
}

#[test]
fn bt26_093_somp_declined_does_nothing() {
    let mut r = setup();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "BB");
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, t);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 0);
    assert_eq!(r.memory(), mem);
}

#[test]
fn bt26_093_somp_needs_a_beatbreak_card_in_hand() {
    let mut r = setup();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "PLAIN");
    fire(&mut r, EffectTiming::StartOfYourMainPhase, t);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(sources(&r, t), 0);
}

#[test]
fn bt26_093_own_attack_places_deck_top_and_grants_collision_blocker() {
    let mut r = setup();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    let bb = r.place_on_field(0, "BB", Some(0));
    r.attack_player(bb, 1, false);
    r.accept_optional_trigger()
        .expect("Reina triggers when a Digimon attacks");
    assert!(
        r.game.players[0].battle_area[t.index as usize].is_suspended,
        "suspend cost"
    );
    assert_eq!(source_ids(&r, t), vec!["DECKTOP".to_string()]);
    assert_eq!(face_down_flags(&r, t), vec![true]);
    let v = r.pending_selection_view().expect("BEATBREAK Digimon pick");
    assert!(!v.is_optional, "the grant pick is mandatory");
    pick_first(&mut r, 0);
    assert!(r.game.has_keyword(bb, Keyword::Collision));
    assert!(r.game.has_keyword(bb, Keyword::Blocker));
}

#[test]
fn bt26_093_opponent_attack_also_triggers() {
    let mut r = setup();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    let bb = r.place_on_field(0, "BB", Some(0));
    r.pass_turn(); // → P1's turn
    let atk = r.place_on_field(1, "OPP", Some(0));
    r.attack_player(atk, 0, false);
    r.accept_optional_trigger()
        .expect("Reina triggers on the opponent's attack");
    pick_first(&mut r, 0);
    assert!(r.game.players[0].battle_area[t.index as usize].is_suspended);
    assert_eq!(sources(&r, t), 1);
    assert!(r.game.has_keyword(bb, Keyword::Blocker));
    assert!(r.game.has_keyword(bb, Keyword::Collision));
}

#[test]
fn bt26_093_only_beatbreak_digimon_are_eligible() {
    let mut r = setup();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    let plain = r.place_on_field(0, "PLAIN", Some(0));
    r.attack_player(plain, 1, false);
    r.accept_optional_trigger().expect("trigger");
    // No [BEATBREAK] Digimon ⇒ the deck card is still placed, no grant pick.
    assert!(r.game.players[0].battle_area[t.index as usize].is_suspended);
    assert_eq!(sources(&r, t), 1);
    assert!(!r.game.has_keyword(plain, Keyword::Blocker));
    assert!(!r.game.has_keyword(plain, Keyword::Collision));
}

#[test]
fn bt26_093_trigger_is_declinable() {
    let mut r = setup();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    let bb = r.place_on_field(0, "BB", Some(0));
    r.attack_player(bb, 1, false);
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert!(!r.game.players[0].battle_area[t.index as usize].is_suspended);
    assert_eq!(sources(&r, t), 0);
    assert!(!r.game.has_keyword(bb, Keyword::Collision));
}

#[test]
fn bt26_093_suspended_reina_cannot_pay_the_cost() {
    let mut r = setup();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    r.game.players[0].battle_area[t.index as usize].is_suspended = true;
    let bb = r.place_on_field(0, "BB", Some(0));
    r.attack_player(bb, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 0);
    assert!(!r.game.has_keyword(bb, Keyword::Collision));
}

#[test]
fn bt26_093_grant_expires_at_end_of_turn() {
    let mut r = setup();
    let _t = r.place_on_field(0, CARD_ID, Some(0));
    let bb = r.place_on_field(0, "BB", Some(0));
    r.attack_player(bb, 1, false);
    r.accept_optional_trigger().expect("trigger");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(r.game.has_keyword(bb, Keyword::Blocker));
    r.pass_turn();
    assert!(!r.game.has_keyword(bb, Keyword::Blocker), "for the turn");
    assert!(!r.game.has_keyword(bb, Keyword::Collision), "for the turn");
}
