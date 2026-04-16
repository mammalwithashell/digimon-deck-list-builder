//! Memory-seesaw semantics parity tests.
//!
//! These lock in behavior that matches Python's `digimon_gym/engine/game/__init__.py`:
//! - `pay_memory` is a pure deduction; it never ends the turn (§1.4 in parity doc).
//! - `pass_turn` preserves overflow when memory is already negative (§1.3).
//! - `end_turn` flips the seesaw via `memory = -memory` — no clamp (§1.2).
//! - Memory swing-back: an OnEndTurn effect that restores memory to ≥ 0 keeps the turn (§1.5).

use digimon_engine::debug_runner::{make_test_card, DebugRunner};

fn empty_runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(make_test_card("FILLER", "Filler"))
        .start()
}

// ── §1.4 — pay_memory must NOT end the turn ─────────────────────────

#[test]
fn pay_memory_does_not_end_turn_on_negative() {
    let mut r = empty_runner();
    r.game.set_memory(3);
    let tp_before = r.turn_player();
    let turn_before = r.turn_count();

    // Pay 5 — memory goes to -2 but turn must continue.
    let ok = r.game.pay_memory(5);
    assert!(ok, "5 should be affordable (memory_range is -10..10)");
    assert_eq!(r.memory(), -2);
    assert_eq!(r.turn_player(), tp_before, "pay_memory must not switch turn");
    assert_eq!(r.turn_count(), turn_before, "pay_memory must not bump turn count");
}

#[test]
fn pay_memory_rejects_unaffordable_cost() {
    let mut r = empty_runner();
    r.game.set_memory(3);
    // memory_range.0 = -10 for standard. From 3, paying 14 goes to -11 (out of range).
    let ok = r.game.pay_memory(14);
    assert!(!ok, "14 should exceed memory_range.0 from memory 3");
    assert_eq!(r.memory(), 3, "memory unchanged when cost is unaffordable");
}

#[test]
fn check_turn_end_ends_when_memory_negative() {
    let mut r = empty_runner();
    r.game.set_memory(3);
    let tp_before = r.turn_player();
    r.game.pay_memory(5); // memory = -2, turn not advanced
    assert_eq!(r.turn_player(), tp_before);

    r.game.check_turn_end();
    // After check_turn_end, memory was negative, so turn advanced.
    assert_ne!(r.turn_player(), tp_before);
}

#[test]
fn check_turn_end_no_op_when_memory_nonnegative() {
    let mut r = empty_runner();
    r.game.set_memory(3);
    let tp_before = r.turn_player();
    let turn_before = r.turn_count();

    r.game.check_turn_end();
    assert_eq!(r.turn_player(), tp_before);
    assert_eq!(r.turn_count(), turn_before);
}

// ── §1.3 — pass_turn preserves overflow ──────────────────────────────

#[test]
fn pass_turn_gives_next_player_three_from_clean_pass() {
    let mut r = empty_runner();
    r.game.set_memory(5);
    let tp_before = r.turn_player();
    r.game.pass_turn();
    assert_ne!(r.turn_player(), tp_before, "turn switched");
    // pass_turn clamped to -3, then end_turn negated to +3.
    assert_eq!(r.memory(), 3);
}

#[test]
fn pass_turn_preserves_negative_overflow() {
    let mut r = empty_runner();
    // Simulate an over-cost play that left memory at -4 (in-range).
    r.game.set_memory(-4);
    r.game.pass_turn();
    // pass_turn must NOT overwrite -4 with -3; end_turn negates -4 → +4.
    assert_eq!(r.memory(), 4, "overflow must carry through as +4 for the next player");
}

// ── §1.2 — end_turn negates memory; no clamp ─────────────────────────

#[test]
fn end_turn_negates_positive_memory() {
    let mut r = empty_runner();
    r.game.set_memory(7);
    r.game.end_turn();
    assert_eq!(r.memory(), -7, "+7 on active player's side becomes -7 from new perspective");
}

#[test]
fn end_turn_negates_negative_memory() {
    let mut r = empty_runner();
    r.game.set_memory(-5);
    r.game.end_turn();
    assert_eq!(r.memory(), 5, "-5 (on next player's side) becomes +5 from their perspective");
}

#[test]
fn end_turn_at_zero_stays_zero() {
    let mut r = empty_runner();
    r.game.set_memory(0);
    r.game.end_turn();
    assert_eq!(r.memory(), 0, "0 memory flips to 0");
}

// ── §1.5 — Memory swing-back ─────────────────────────────────────────

#[test]
fn end_turn_swing_back_keeps_turn_with_same_player() {
    // TEST-006 is "End of your turn: Gain 5 memory." If the active player
    // ends their turn at -3 and TEST-006 swings memory back to +2, the turn
    // continues and returns to Main phase with the same player still active.
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-006", "SwingBack"))
        .start();
    let tp_before = r.turn_player();
    let turn_before = r.turn_count();

    r.place_on_field(tp_before, "TEST-006", Some(0));
    r.game.set_memory(-3);
    r.game.end_turn();

    // Swing-back triggered: memory went from -3 to +2 during OnEndTurn.
    assert_eq!(r.memory(), 2, "TEST-006 should have added 5 to -3 → +2");
    assert_eq!(r.turn_player(), tp_before, "turn must stay with the same player");
    assert_eq!(r.turn_count(), turn_before, "turn count must not advance");
    assert_eq!(
        r.current_phase(),
        digimon_engine::enums::GamePhase::Main,
        "phase should return to Main after swing-back"
    );
}

#[test]
fn end_turn_no_swing_back_when_memory_stays_negative() {
    // Memory starts at -10 (min). TEST-006 adds 5 → -5 (still negative).
    // No swing-back; turn advances normally and memory flips to +5 for next player.
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-006", "SwingBack"))
        .start();
    let tp_before = r.turn_player();

    r.place_on_field(tp_before, "TEST-006", Some(0));
    r.game.set_memory(-10);
    r.game.end_turn();

    assert_ne!(r.turn_player(), tp_before, "turn must advance");
    // -10 + 5 = -5 at end of effects, negate = +5.
    assert_eq!(r.memory(), 5);
}

#[test]
fn end_turn_no_swing_back_when_memory_already_nonnegative() {
    // Memory starts at 0. TEST-006 adds 5 → 5. memory_before wasn't negative,
    // so swing-back does NOT fire — turn advances and memory negates to -5.
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-006", "SwingBack"))
        .start();
    let tp_before = r.turn_player();

    r.place_on_field(tp_before, "TEST-006", Some(0));
    r.game.set_memory(0);
    r.game.end_turn();

    assert_ne!(r.turn_player(), tp_before, "turn must advance");
    assert_eq!(r.memory(), -5);
}
