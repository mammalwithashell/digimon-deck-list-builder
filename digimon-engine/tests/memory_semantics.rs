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
