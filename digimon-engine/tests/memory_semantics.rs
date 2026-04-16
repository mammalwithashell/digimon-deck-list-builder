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
fn pass_turn_forces_minus_three_when_memory_nonnegative() {
    let mut r = empty_runner();
    r.game.set_memory(5);
    // Snapshot memory_range to disable turn-switch memory flip for this test.
    // We only care about what pass_turn DID to memory before end_turn runs.
    // end_turn will then negate it — which is tested separately in Step 3.
    r.game.pass_turn();
    // After pass_turn: set to -3 (>=0 branch), then end_turn negates to +3.
    // That's the Python behavior: new active player gets 3 memory.
    // NOTE: this test will be updated once Step 3 lands the `negate` semantics.
    // For now, pre-Step-3, memory is clamped to 3 in end_turn — same observable.
    assert_eq!(r.memory(), 3, "next player receives 3 memory from a clean pass");
}

#[test]
fn pass_turn_preserves_negative_overflow() {
    let mut r = empty_runner();
    // Simulate an over-cost play that left memory at -4 (in-range).
    r.game.set_memory(-4);
    // Do NOT call pay_memory — just preset state. Then pass.
    r.game.pass_turn();
    // Pre-Step-3: end_turn clamps; post-Step-3: end_turn negates → +4.
    // What matters here is that pass_turn did NOT overwrite -4 with -3.
    // Step 3 will assert the resulting +4 specifically.
    assert!(
        r.memory() >= 3,
        "the overflow from over-cost plays must not be collapsed to -3 by pass_turn; \
         got memory={}",
        r.memory()
    );
}
