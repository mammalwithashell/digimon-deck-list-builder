//! Staging-setter coverage for the add-ui-scenario-test-substrate change.
//!
//! Pins the `DebugRunner` scenario-staging primitives that `RustDebugGame`
//! orchestrates: `set_phase`, `set_turn`, `set_first_player`,
//! `place_field_stack` (with suspend + turn_played), `inject_trash`, and
//! `validate()`. These mutate engine state directly; the tests assert the
//! derived turn-machine invariants stay consistent.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, GamePhase};

fn lv4_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c
}

fn staged_runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(lv4_digimon("D-A"))
        .add_card(lv4_digimon("D-B"))
        .add_card(lv4_digimon("D-C"))
        .start();
    r.skip_mulligan();
    r
}

#[test]
fn set_phase_moves_into_main() {
    let mut r = staged_runner();
    r.set_phase(GamePhase::Main);
    assert_eq!(r.current_phase(), GamePhase::Main);
}

#[test]
fn set_turn_sets_the_counter() {
    let mut r = staged_runner();
    r.set_turn(7);
    assert_eq!(r.turn_count(), 7);
}

#[test]
fn set_first_player_reorders_and_resets_seesaw() {
    let mut r = staged_runner();
    // Make player 1 (Rust 0-based) the active turn player.
    r.set_first_player(1);
    assert_eq!(r.turn_player(), 1, "player 1 must be the active turn player");
    // memory_pair active side must be the new first player.
    assert_eq!(
        r.game.memory_pair.0, 1,
        "memory seesaw active side must follow the first player"
    );
    // The other player is next.
    assert_eq!(r.game.memory_pair.1, 0);
}

#[test]
fn place_field_stack_applies_suspend_and_turn_played() {
    let mut r = staged_runner();
    let handle = r.place_field_stack(0, &["D-A", "D-B"], true, 3);
    let perm = &r.game.player(0).battle_area[handle.index as usize];
    assert!(perm.is_suspended, "staged permanent must be suspended");
    assert_eq!(perm.turn_played, 3, "turn_played override must apply");
    // Bottom-to-top: D-A under D-B (top).
    assert_eq!(perm.card_sources.len(), 2);
}

#[test]
fn inject_trash_adds_to_trash_pile() {
    let mut r = staged_runner();
    assert_eq!(r.trash_size(0), 0);
    r.inject_trash(0, "D-C");
    assert_eq!(r.trash_size(0), 1, "injected card must land in trash");
}

#[test]
fn validate_accepts_a_clean_staged_board() {
    let mut r = staged_runner();
    r.set_phase(GamePhase::Main);
    r.place_field_stack(0, &["D-A"], false, 0);
    assert!(r.validate().is_ok(), "clean staged board must validate");
}

#[test]
fn validate_rejects_non_mulligan_phase_with_pending_mulligan() {
    // `start_game()` auto-finalizes mulligan, so reach the inconsistent
    // state directly: re-introduce a pending mulligan while the phase is
    // Main. validate() must catch the mismatch (the mask would otherwise
    // offer mulligan actions in a non-mulligan phase).
    let mut r = staged_runner();
    r.set_phase(GamePhase::Main);
    r.game.mulligan_pending.push(0);
    let result = r.validate();
    assert!(
        result.is_err(),
        "Main phase with a pending mulligan must be rejected"
    );
    assert!(
        result.unwrap_err().contains("mulligan"),
        "diagnostic must mention the mulligan inconsistency"
    );
}
