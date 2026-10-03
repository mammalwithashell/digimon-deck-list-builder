//! BT26-096 Kosuke Misono — Purple Tamer ([TS]).
//!
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [Main] By returning this Tamer to the bottom of the deck, you may play 1
//! Digimon card with [Chronomon] in its text or 1 Tamer card with the [TS]
//! trait from your hand or trash with the cost reduced by 2.
//! [Security] Play this card without paying the cost.
//!
//! DCGO: BT26/Purple/BT26_096.cs.

use super::support::*;
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS,
};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-096";

fn setup(memory: i16) -> DebugRunner {
    let mut chrono = digimon("CHRONO", "Chronomon", CardColor::Purple, 5, 8, &[]);
    chrono.effect_text = String::new();
    let mut texted = digimon("TEXTED", "Texted", CardColor::Purple, 4, 6, &[]);
    texted.effect_text = "[On Play] If you have [Chronomon] ...".to_string();
    let mut ts_t = tamer("TS-T", "TS Tamer", CardColor::Purple, &["TS"]);
    ts_t.play_cost = 5;
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-096")
        .add_card(filler("FILLER"))
        .add_card(chrono)
        .add_card(texted)
        .add_card(ts_t)
        .add_card(digimon("PLAIN", "Plain", CardColor::Purple, 4, 6, &["TS"]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(memory)
        .start();
    r.set_first_player(0);
    r
}

fn main_bit(idx: usize) -> u16 {
    FIELD_EFFECT_START + idx as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

fn non_pass(r: &DebugRunner) -> usize {
    r.pending_selection_view()
        .map(|v| v.valid_action_ids.iter().filter(|&&a| a != PASS).count())
        .unwrap_or(0)
}

#[test]
fn bt26_096_start_of_turn_sets_memory_to_three() {
    let mut r = setup(1);
    let k = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::StartOfYourTurn, k);
    assert_eq!(r.memory(), 3);
}

#[test]
fn bt26_096_start_of_turn_no_change_above_two() {
    let mut r = setup(4);
    let k = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::StartOfYourTurn, k);
    assert_eq!(r.memory(), 4);
}

#[test]
fn bt26_096_main_bounces_self_and_plays_from_trash_reduced() {
    let mut r = setup(6);
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "CHRONO");
    push_trash(&mut r, 0, "PLAIN"); // [TS] Digimon w/o Chronomon — ineligible
    push_hand(&mut r, 0, "TS-T");
    push_hand(&mut r, 0, "TEXTED");
    r.game.enter_main_phase();
    let _ = r.auto_resolve();
    assert!(build_action_mask(&r.game, 0)[main_bit(0) as usize] > 0.0);
    r.game.decode_action(main_bit(0), 0);
    assert!(field_ids(&r, 0).is_empty(), "Tamer returned as the cost");
    assert_eq!(
        deck_ids(&r, 0).first().map(String::as_str),
        Some(CARD_ID),
        "deck bottom"
    );
    let v = r.pending_selection_view().expect("union pick");
    assert!(v.is_optional, "may");
    assert_eq!(non_pass(&r), 3, "CHRONO (trash), TS-T + TEXTED (hand)");
    // Pick the trash Chronomon (cost 8 → pays 6).
    let a = *r
        .pending_selection_view()
        .unwrap()
        .valid_action_ids
        .iter()
        .find(|&&a| {
            a != PASS && {
                // the trash candidate is the one that is NOT a hand-play bit
                !(digimon_engine::action::space::PLAY_HAND_START
                    ..digimon_engine::action::space::PLAY_HAND_END)
                    .contains(&a)
            }
        })
        .expect("trash candidate");
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["CHRONO".to_string()]);
    assert_eq!(r.memory(), 0, "cost 8 reduced by 2 → 6 paid");
}

#[test]
fn bt26_096_main_plays_ts_tamer_from_hand_reduced() {
    let mut r = setup(6);
    r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "TS-T");
    r.game.enter_main_phase();
    let _ = r.auto_resolve();
    r.game.decode_action(main_bit(0), 0);
    pick_hand(&mut r, 0, "TS-T");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["TS-T".to_string()]);
    assert_eq!(r.memory(), 3, "cost 5 reduced by 2");
    assert!(hand_ids(&r, 0).is_empty());
}

#[test]
fn bt26_096_main_decline_still_returns_tamer() {
    let mut r = setup(6);
    r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "TS-T");
    r.game.enter_main_phase();
    let _ = r.auto_resolve();
    r.game.decode_action(main_bit(0), 0);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
    assert_eq!(hand_ids(&r, 0), vec!["TS-T".to_string()]);
    assert_eq!(r.memory(), 6);
}

#[test]
fn bt26_096_security_plays_self() {
    let r = setup(3);
    let c = r.compiled_card(CARD_ID).unwrap();
    let dbg = format!("{:?}", c.effects);
    assert!(
        dbg.contains("OnSecurity") && dbg.contains("PlayFromSecurity"),
        "{dbg}"
    );
}
