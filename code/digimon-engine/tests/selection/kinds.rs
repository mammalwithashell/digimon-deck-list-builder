//! Integration tests for the PR5 selection helpers: `select_hand`,
//! `select_trash`, `select_effect_choice`, and the two pilot cards
//! TEST-011 / TEST-012.

use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::GamePhase;
use digimon_engine::selection::{SelectionError, SelectionKind};

#[test]
fn test_011_select_hand_installs_prompt_and_trashes_then_draws() {
    // P0 plays TEST-011 with two other hand cards to trash from.
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-011", "HandSelectPilot"))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["TEST-011", "FILLER", "FILLER"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .memory(3) // pre-fund play cost
        .start();

    let hand_before = r.hand_size(0);
    let deck_before = r.deck_size(0);
    let trash_before = r.trash_size(0);

    r.play(0, 0);
    // Post-play: TEST-011 went to field (hand.len() = 2), selection installed.
    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("TEST-011 must install a hand-selection on play");
    assert_eq!(sel.kind, SelectionKind::Hand);
    assert!(!sel.is_optional, "TEST-011's selection is mandatory");
    assert_eq!(r.current_phase(), GamePhase::SelectHand);
    // Both remaining hand cards are legal picks.
    assert_eq!(
        sel.valid_action_ids,
        vec![PLAY_HAND_START, PLAY_HAND_START + 1]
    );

    // Pick hand index 0 — the FILLER we trash.
    r.game
        .resolve_selection(0, PLAY_HAND_START)
        .expect("valid hand pick resolves");

    assert!(r.game.pending_selection.is_none());
    // Trash gained one card. Hand: started with 3, played TEST-011, trashed 1, drew 2 → 3.
    assert_eq!(r.trash_size(0), trash_before + 1);
    assert_eq!(r.hand_size(0), hand_before - 1 - 1 + 2);
    assert_eq!(r.deck_size(0), deck_before - 2);
}

#[test]
fn test_011_mandatory_select_hand_rejects_pass() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-011", "HandSelectPilot"))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["TEST-011", "FILLER"])
        .deck(0, &["FILLER", "FILLER"])
        .memory(3)
        .start();
    r.play(0, 0);

    let err = r
        .game
        .resolve_selection(0, PASS)
        .expect_err("mandatory select_hand rejects PASS");
    assert_eq!(err, SelectionError::InvalidAction);
    assert!(r.game.pending_selection.is_some());
}

#[test]
fn test_011_select_hand_noop_when_hand_empty() {
    // Only TEST-011 in hand → after playing, hand is empty → select_hand
    // sees no candidates → no-op (no selection installed, OnPlay completes
    // without firing the trash/draw branch).
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-011", "HandSelectPilot"))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["TEST-011"])
        .deck(0, &["FILLER", "FILLER"])
        .memory(3)
        .start();
    let hand_before_effect = r.hand_size(0);
    r.play(0, 0);
    assert!(r.game.pending_selection.is_none());
    // Hand went 1 → 0 via play, and select_hand filter found nothing to
    // pick, so no draw / no trash. Hand still 0.
    assert_eq!(r.hand_size(0), hand_before_effect - 1);
}

#[test]
fn test_012_effect_choice_installs_prompt() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-012", "ChoicePilot"))
        .hand(0, &["TEST-012"])
        .memory(3)
        .start();
    r.play(0, 0);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("TEST-012 installs an EffectChoice");
    assert_eq!(sel.kind, SelectionKind::EffectChoice);
    assert_eq!(r.current_phase(), GamePhase::EffectChoice);
    assert!(!sel.is_optional, "effect choice must pick a branch");
    assert_eq!(
        sel.valid_action_ids,
        vec![HAND_EFFECT_START, HAND_EFFECT_START + 1]
    );
    let choices = sel.effect_choices.as_ref().expect("choices populated");
    assert_eq!(choices.len(), 2);
    assert_eq!(choices[0].label, "Gain 2 memory");
    assert_eq!(choices[1].label, "Draw 2 cards");
}

#[test]
fn test_012_effect_choice_gain_memory_branch() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-012", "ChoicePilot"))
        .hand(0, &["TEST-012"])
        .memory(3)
        .start();
    let mem_after_cost = 3 - 3; // cost 3
    r.play(0, 0);
    r.game
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("pick gain-memory");
    assert_eq!(r.memory(), mem_after_cost + 2, "memory branch grants +2");
}

#[test]
fn test_012_effect_choice_draw_branch() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-012", "ChoicePilot"))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["TEST-012"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .memory(3)
        .start();
    r.play(0, 0);
    r.game
        .resolve_selection(0, HAND_EFFECT_START + 1)
        .expect("pick draw");
    // Post-play: hand 0, drew 2 = hand 2. Deck: 3 - 2 = 1.
    assert_eq!(r.hand_size(0), 2);
    assert_eq!(r.deck_size(0), 1);
}

#[test]
fn test_012_effect_choice_rejects_invalid_branch() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-012", "ChoicePilot"))
        .hand(0, &["TEST-012"])
        .memory(3)
        .start();
    r.play(0, 0);
    // HAND_EFFECT_START + 5 is outside the 2 legal branches.
    let err = r
        .game
        .resolve_selection(0, HAND_EFFECT_START + 5)
        .expect_err("out-of-range branch rejected");
    assert_eq!(err, SelectionError::InvalidAction);
}
