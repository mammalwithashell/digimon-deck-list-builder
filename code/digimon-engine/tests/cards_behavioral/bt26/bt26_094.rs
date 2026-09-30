//! BT26-094 Keenan Crier — Tamer, Purple, DATA SQUAD.
//!
//! [Start of Your Main Phase] By placing 1 [DATA SQUAD] trait card from your
//! hand face down under this Tamer, <Draw 1> and gain 1 memory.
//! [Your Turn] When your opponent's hand is trashed from or effects trash cards
//! from under this Tamer, by suspending this Tamer, 1 of your [DATA SQUAD]
//! trait Digimon gains <Execute> for the turn.
//! [Security] Play this card without paying the cost.
//!
//! DCGO: BT26/Purple/BT26_094.cs. The opponent-hand trash is driven by
//! BT26-072 Peckmon's inherited [On Deletion].

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};

const CARD_ID: &str = "BT26-094";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-094")
        .dsl_card("BT26-072")
        .expect("Peckmon")
        .add_card(filler("FILLER"))
        .add_card(digimon("DS-DIGI", "DS", CardColor::Purple, 5, 7, &["DATA SQUAD"]))
        .add_card(digimon("JUNK", "Junk", CardColor::Red, 3, 3, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_094_somp_places_draws_gains() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "DS-DIGI");
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, k);
    pick_hand(&mut r, 0, "DS-DIGI");
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, k), 1);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.memory(), mem + 1);
}

fn opponent_hand_trash(r: &mut DebugRunner) {
    push_hand(r, 1, "JUNK");
    let carrier = r.place_stack(0, &["BT26-072", "FILLER"]);
    fire(r, EffectTiming::OnDeletion, carrier);
    pick_hand(r, 1, "JUNK");
}

#[test]
fn bt26_094_opponent_hand_trash_grants_execute() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    let ds = r.place_on_field(0, "DS-DIGI", Some(0));
    opponent_hand_trash(&mut r);
    r.accept_optional_trigger().expect("Keenan triggers");
    assert!(r.game.players[0].battle_area[k.index as usize].is_suspended);
    let _ = r.auto_resolve(); // the only [DATA SQUAD] Digimon
    assert!(r.game.has_keyword(ds, Keyword::Execute));
}

#[test]
fn bt26_094_decline_keeps_tamer_ready() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    let ds = r.place_on_field(0, "DS-DIGI", Some(0));
    opponent_hand_trash(&mut r);
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert!(!r.game.players[0].battle_area[k.index as usize].is_suspended);
    assert!(!r.game.has_keyword(ds, Keyword::Execute));
}

#[test]
fn bt26_094_own_hand_trash_does_not_trigger() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "DS-DIGI", Some(0));
    // Peckmon's [On Play] trash cost trashes OUR hand.
    push_hand(&mut r, 0, "JUNK");
    let peck = r.place_on_field(0, "BT26-072", Some(0));
    r.fire_on_play(0, peck.index as usize);
    r.execute_branch(0).expect("Keenan present ⇒ cost modal; take 'trash'");
    pick_hand(&mut r, 0, "JUNK");
    let _ = r.auto_resolve();
    assert_eq!(trash_ids(&r, 0), vec!["JUNK".to_string()], "our own hand was trashed");
    assert!(!r.game.players[0].battle_area[k.index as usize].is_suspended);
}

#[test]
fn bt26_094_not_on_opponents_turn() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "DS-DIGI", Some(0));
    r.set_first_player(1);
    opponent_hand_trash(&mut r);
    let _ = r.auto_resolve();
    assert!(!r.game.players[0].battle_area[k.index as usize].is_suspended);
}
