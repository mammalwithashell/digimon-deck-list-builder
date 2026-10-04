//! BT26-087 Toya Kuga — Tamer, Red, TS.
//!
//! [Start of Your Main Phase] By returning 1 [TS] trait Digimon card from your
//! trash to the bottom of the deck, gain 1 memory. After, you may return 1
//! [Giant Slayer] from your trash to the hand.
//! [On Play] By trashing 1 [TS] trait card from your hand, <Draw 2>.
//! [Security] Play this card without paying the cost.
//!
//! DCGO: BT26/Red/BT26_087.cs.

use super::support::*;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};

const CARD_ID: &str = "BT26-087";

fn giant_slayer() -> CardData {
    let mut c = make_test_card("SLAYER", "Giant Slayer");
    c.card_kind = CardKind::Option;
    c.play_cost = 2;
    c.colors = vec![CardColor::Red];
    c
}

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-087")
        .add_card(filler("FILLER"))
        .add_card(digimon("TS-D", "TS Digi", CardColor::Red, 3, 3, &["TS"]))
        .add_card(tamer("TS-T", "TS Tamer", CardColor::Red, &["TS"]))
        .add_card(digimon("PLAIN", "Plain", CardColor::Red, 3, 3, &["Beast"]))
        .add_card(giant_slayer())
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_087_somp_returns_ts_digimon_gains_memory_and_returns_slayer() {
    let mut r = setup();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TS-T");
    push_trash(&mut r, 0, "TS-D");
    push_trash(&mut r, 0, "SLAYER");
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, t);
    let v = r.pending_selection_view().expect("trash pick");
    assert!(v.is_optional);
    assert_eq!(v.valid_action_ids.len(), 1, "only the TS DIGIMON card");
    pick_first(&mut r, 0);
    assert_eq!(deck_ids(&r, 0)[0], "TS-D", "bottom of the deck");
    assert_eq!(r.memory(), mem + 1);
    let v = r.pending_selection_view().expect("Giant Slayer pick");
    assert!(v.is_optional);
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["SLAYER".to_string()]);
    assert_eq!(trash_ids(&r, 0), vec!["TS-T".to_string()]);
}

#[test]
fn bt26_087_somp_slayer_return_is_optional() {
    let mut r = setup();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TS-D");
    push_trash(&mut r, 0, "SLAYER");
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, t);
    pick_first(&mut r, 0);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem + 1);
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(trash_ids(&r, 0), vec!["SLAYER".to_string()]);
}

#[test]
fn bt26_087_somp_declined_skips_after_clause() {
    let mut r = setup();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TS-D");
    push_trash(&mut r, 0, "SLAYER");
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, t);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), mem);
    assert_eq!(r.hand_size(0), 0, "no Giant Slayer return without the cost");
    assert_eq!(r.trash_size(0), 2);
}

#[test]
fn bt26_087_somp_needs_ts_digimon_in_trash() {
    let mut r = setup();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TS-T");
    push_trash(&mut r, 0, "SLAYER");
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, t);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), mem);
    assert_eq!(r.hand_size(0), 0);
}

#[test]
fn bt26_087_on_play_trash_ts_draws_two() {
    let mut r = setup();
    push_hand(&mut r, 0, "PLAIN");
    push_hand(&mut r, 0, "TS-T");
    let t = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    let v = r.pending_selection_view().expect("optional trash pick");
    assert!(v.is_optional);
    assert_eq!(v.valid_action_ids.len(), 1);
    pick_hand(&mut r, 0, "TS-T");
    let _ = r.auto_resolve();
    assert_eq!(trash_ids(&r, 0), vec!["TS-T".to_string()]);
    assert_eq!(r.hand_size(0), 3);
}

#[test]
fn bt26_087_on_play_declined() {
    let mut r = setup();
    push_hand(&mut r, 0, "TS-T");
    let t = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.trash_size(0), 0);
}
