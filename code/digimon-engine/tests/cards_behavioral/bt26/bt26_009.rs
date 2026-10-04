//! BT26-009 Hyokomon — Lv.3 Red (Bird / Iliad / TS).
//!
//! [Start of Your Main Phase] By trashing 1 card with [Chronomon] in its text
//! or the [Shaman] trait from your hand, <Draw 1> and gain 1 memory.
//! Inherited: [When Attacking] [Once Per Turn] <Draw 1>. Then, if your hand has
//! 6 or more cards, return 1 card in your hand to the bottom of the deck.
//!
//! DCGO: BT26/Red/BT26_009.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-009";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-009")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "SHAMAN",
            "Shamanmon",
            CardColor::Red,
            5,
            7,
            &["Shaman"],
        ))
        .add_card(digimon(
            "CHRONO",
            "Chronomon Kid",
            CardColor::Red,
            4,
            5,
            &[],
        ))
        .add_card(digimon("PLAIN", "Plainmon", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("EGG", "Egg", CardColor::Red, 2, 0, &["TS"]))
        .add_card(digimon("HOST", "Host", CardColor::Red, 4, 5, &[]))
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
fn bt26_009_alt_digivolve_from_lv2_ts() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).unwrap();
    let alt = format!("{:?}", card.alt_paths);
    assert!(
        alt.contains("TS") && alt.contains("level_eq: Some(2)"),
        "{alt}"
    );
}

#[test]
fn bt26_009_somp_trash_chronomon_text_card_draws_and_gains_memory() {
    let mut r = setup();
    push_hand(&mut r, 0, "PLAIN");
    push_hand(&mut r, 0, "CHRONO");
    let h = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::StartOfYourMainPhase, h);
    let v = r.pending_selection_view().expect("hand pick");
    assert!(v.is_optional, "'by trashing' is optional");
    let plain = digimon_engine::action::space::PLAY_HAND_START;
    assert!(!v.valid_action_ids.contains(&plain), "PLAIN not eligible");
    pick_hand(&mut r, 0, "CHRONO");
    assert_eq!(trash_ids(&r, 0), vec!["CHRONO"]);
    assert_eq!(hand_ids(&r, 0).len(), 2, "PLAIN + 1 drawn");
    assert_eq!(r.memory(), 4);
}

#[test]
fn bt26_009_somp_accepts_shaman_trait() {
    let mut r = setup();
    push_hand(&mut r, 0, "SHAMAN");
    let h = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::StartOfYourMainPhase, h);
    pick_hand(&mut r, 0, "SHAMAN");
    assert_eq!(trash_ids(&r, 0), vec!["SHAMAN"]);
    assert_eq!(r.memory(), 4);
}

#[test]
fn bt26_009_somp_decline_costs_nothing() {
    let mut r = setup();
    push_hand(&mut r, 0, "CHRONO");
    let h = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::StartOfYourMainPhase, h);
    pass(&mut r, 0);
    assert_eq!(hand_ids(&r, 0), vec!["CHRONO"]);
    assert_eq!(r.memory(), 3);
}

#[test]
fn bt26_009_somp_no_candidate_no_prompt() {
    let mut r = setup();
    push_hand(&mut r, 0, "PLAIN");
    let h = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::StartOfYourMainPhase, h);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), 3);
}

#[test]
fn bt26_009_inherited_draw_small_hand_keeps_all() {
    let mut r = setup();
    push_hand(&mut r, 0, "PLAIN");
    let h = r.place_stack(0, &[CARD_ID, "HOST"]);
    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(hand_ids(&r, 0).len(), 2);
}

#[test]
fn bt26_009_inherited_draw_then_bottom_one_at_six() {
    let mut r = setup();
    for _ in 0..4 {
        push_hand(&mut r, 0, "PLAIN");
    }
    push_hand(&mut r, 0, "CHRONO");
    let h = r.place_stack(0, &[CARD_ID, "HOST"]);
    fire(&mut r, EffectTiming::WhenAttacking, h);
    let v = r
        .pending_selection_view()
        .expect("mandatory hand pick at 6");
    assert!(!v.is_optional);
    pick_hand(&mut r, 0, "CHRONO");
    assert_eq!(hand_ids(&r, 0).len(), 5);
    assert_eq!(deck_ids(&r, 0)[0], "CHRONO", "bottom of deck = index 0");
    // Once per turn.
    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert_eq!(hand_ids(&r, 0).len(), 5);
}
