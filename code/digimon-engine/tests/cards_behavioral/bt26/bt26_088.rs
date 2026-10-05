//! BT26-088 Hiroko Sagisaka — Tamer, Red, TS.
//!
//! [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
//! [Your Turn] When any [Boss] or [TS] trait Digimon cards would be played, by
//! suspending this Tamer, reduce the cost by 1. If you have no Digimon,
//! instead reduce the cost by 2.
//! [Security] Play this card without paying the cost.
//!
//! DCGO: BT26/Red/BT26_088.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-088";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-088")
        .add_card(filler("FILLER"))
        .add_card(digimon("TS5", "TS Five", CardColor::Red, 4, 5, &["TS"]))
        .add_card(digimon(
            "BOSS5",
            "Boss Five",
            CardColor::Red,
            4,
            5,
            &["Boss"],
        ))
        .add_card(digimon(
            "PLAIN5",
            "Plain Five",
            CardColor::Red,
            4,
            5,
            &["Beast"],
        ))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 4, 5, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_088_somp_gains_memory_when_opponent_has_digimon() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP", Some(0));
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, h);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem + 1);
}

#[test]
fn bt26_088_somp_no_memory_without_opponent_digimon() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, h);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem);
}

#[test]
fn bt26_088_ts_play_reduced_by_two_with_no_digimon() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "TS5");
    let mem = r.memory();
    r.play(0, 0);
    assert!(r.pending_is_optional(), "the reduction is optional");
    r.accept_optional_trigger().expect("accept");
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
    assert!(field_ids(&r, 0).contains(&"TS5".to_string()));
    assert_eq!(r.memory(), mem - 3, "cost 5 reduced by 2");
}

#[test]
fn bt26_088_boss_play_reduced_by_one_with_a_digimon() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "PLAIN5", Some(0));
    push_hand(&mut r, 0, "BOSS5");
    let mem = r.memory();
    r.play(0, 0);
    r.accept_optional_trigger().expect("accept");
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
    assert_eq!(r.memory(), mem - 4, "cost 5 reduced by 1");
}

#[test]
fn bt26_088_declined_pays_full_cost_and_stays_unsuspended() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "TS5");
    let mem = r.memory();
    r.play(0, 0);
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert!(!r.game.players[0].battle_area[h.index as usize].is_suspended);
    assert_eq!(r.memory(), mem - 5);
}

#[test]
fn bt26_088_non_matching_digimon_not_reduced() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "PLAIN5");
    let mem = r.memory();
    r.play(0, 0);
    assert!(r.game.pending_selection.is_none(), "no reduction offer");
    let _ = r.auto_resolve();
    assert!(!r.game.players[0].battle_area[h.index as usize].is_suspended);
    assert_eq!(r.memory(), mem - 5);
}

// ─── G-ENGINE-PLAY-MASK-IGNORES-WHEN-PLAYING-REDUCTION (field-granted half) ──
// Hiroko's optional "by suspending this Tamer" reduction is a legal choice the
// player can always make while she is unsuspended, so the RL mask offers a
// [TS]/[Boss] play whose cost is payable only after it (rule 1-3-11-1).

fn hand_play_offered(r: &DebugRunner) -> bool {
    let mask = digimon_engine::action::mask::build_action_mask(&r.game, 0);
    mask[digimon_engine::action::space::PLAY_HAND_START as usize] == 1.0
}

#[test]
fn bt26_088_mask_offers_ts_play_payable_only_after_suspending_hiroko() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.game.players[0].hand.clear();
    push_hand(&mut r, 0, "TS5");
    // 5 printed, -2 with no Digimon → 3. From -7: printed → -12 (illegal),
    // reduced → -10 (legal).
    r.game.set_memory(-7);
    assert!(hand_play_offered(&r));
    r.play(0, 0);
    r.accept_optional_trigger().expect("accept");
    assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
    assert!(field_ids(&r, 0).contains(&"TS5".to_string()));
    // Paid 5 - 2 = 3 from -7 → -10; the resolved action's turn-end check then
    // hands the turn over, so the gauge reads +10 from the opponent's side.
    assert_eq!(r.turn_player(), 1, "memory went negative → turn passed");
    assert_eq!(r.memory(), 10, "our -10 is the opponent's 10");

    // Suspended Hiroko can't pay → no reduction → not offered.
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    r.game.players[0].hand.clear();
    push_hand(&mut r, 0, "TS5");
    r.game.set_memory(-7);
    assert!(!hand_play_offered(&r));

    // A non-matching Digimon gets no reduction → not offered.
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    r.game.players[0].hand.clear();
    push_hand(&mut r, 0, "PLAIN5");
    r.game.set_memory(-7);
    assert!(!hand_play_offered(&r));
}

#[test]
fn bt26_088_already_suspended_cannot_pay() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    push_hand(&mut r, 0, "TS5");
    let mem = r.memory();
    r.play(0, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem - 5);
}
