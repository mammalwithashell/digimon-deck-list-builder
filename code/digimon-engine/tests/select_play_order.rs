//! Integration tests for the `SelectPlayOrder` phase used by best-of-three
//! match training. Between games of a BO3 match the Python `MatchEnv` wrapper
//! calls `Game::request_play_order_selection(loser_id)`, the loser picks
//! `PLAY_FIRST` (action 94) or `PLAY_SECOND` (action 95), and the engine
//! records the choice in `last_play_order_choice` for the wrapper to read.
//!
//! Contract:
//! - `request_play_order_selection` installs a `SelectionKind::PlayOrder`
//!   prompt with `selecting_player = loser_id`, `valid_action_ids = [94, 95]`,
//!   `is_optional = false`, and switches `current_phase` to `SelectPlayOrder`.
//! - The action mask reports 94 and 95 legal for the chooser, and NO other
//!   actions legal except those (no PASS, no concede — wait, concede is
//!   always legal; tested in `concede_primitive.rs`).
//! - Resolution via the action interface (94 → First, 95 → Second) or via
//!   `Game::resolve_play_order_selection(picked)` both write to
//!   `last_play_order_choice`.
//! - Outside the phase, actions 94 and 95 are NOT in the action mask.

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{PLAY_FIRST, PLAY_SECOND};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::GamePhase;
use digimon_engine::selection::{PlayOrder, SelectionKind};

#[test]
fn request_installs_play_order_selection_with_chooser_set() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::GameOver;

    runner.game.request_play_order_selection(1);

    assert_eq!(runner.game.current_phase, GamePhase::SelectPlayOrder);
    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("pending_selection must be installed");
    assert!(matches!(sel.kind, SelectionKind::PlayOrder));
    assert_eq!(sel.selecting_player, 1, "loser is the chooser");
    assert!(!sel.is_optional, "play-order pick is mandatory");
    assert_eq!(
        sel.valid_action_ids,
        vec![PLAY_FIRST, PLAY_SECOND],
        "exactly two legal actions in fixed order"
    );
}

#[test]
fn mask_reports_play_first_and_play_second_legal_for_chooser_only() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::GameOver;
    runner.game.request_play_order_selection(0);

    // Chooser sees the two actions.
    let chooser_mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        chooser_mask[PLAY_FIRST as usize], 1.0,
        "PLAY_FIRST legal for chooser"
    );
    assert_eq!(
        chooser_mask[PLAY_SECOND as usize], 1.0,
        "PLAY_SECOND legal for chooser"
    );

    // Non-chooser sees nothing (selection is gated on selecting_player).
    let other_mask = build_action_mask(&runner.game, 1);
    assert_eq!(
        other_mask[PLAY_FIRST as usize], 0.0,
        "PLAY_FIRST not legal for non-chooser"
    );
    assert_eq!(
        other_mask[PLAY_SECOND as usize], 0.0,
        "PLAY_SECOND not legal for non-chooser"
    );
}

#[test]
fn mask_does_not_expose_play_order_actions_outside_phase() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::Main;

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_FIRST as usize], 0.0,
        "PLAY_FIRST illegal in Main phase"
    );
    assert_eq!(
        mask[PLAY_SECOND as usize], 0.0,
        "PLAY_SECOND illegal in Main phase"
    );
}

#[test]
fn resolve_via_action_first_records_first() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::GameOver;
    runner.game.request_play_order_selection(1);

    runner
        .game
        .resolve_selection(1, PLAY_FIRST)
        .expect("resolve PLAY_FIRST");

    assert_eq!(
        runner.game.last_play_order_choice,
        Some(PlayOrder::First),
        "First action records First"
    );
    assert!(
        runner.game.pending_selection.is_none(),
        "selection cleared after resolution"
    );
}

#[test]
fn resolve_via_action_second_records_second() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::GameOver;
    runner.game.request_play_order_selection(0);

    runner
        .game
        .resolve_selection(0, PLAY_SECOND)
        .expect("resolve PLAY_SECOND");

    assert_eq!(
        runner.game.last_play_order_choice,
        Some(PlayOrder::Second),
        "Second action records Second"
    );
}

#[test]
fn resolve_via_convenience_method_records_choice() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::GameOver;
    runner.game.request_play_order_selection(1);

    runner
        .game
        .resolve_play_order_selection(PlayOrder::Second)
        .expect("convenience resolve");

    assert_eq!(runner.game.last_play_order_choice, Some(PlayOrder::Second));
    assert!(runner.game.pending_selection.is_none());
}

#[test]
fn resolve_convenience_errors_without_pending_play_order() {
    let mut runner = DebugRunner::builder().start();
    // No pending selection installed.
    assert!(runner
        .game
        .resolve_play_order_selection(PlayOrder::First)
        .is_err());
}

#[test]
fn last_play_order_choice_starts_none() {
    let runner = DebugRunner::builder().start();
    assert_eq!(runner.game.last_play_order_choice, None);
}

#[test]
fn resolve_does_not_implicitly_reset_choice_slot() {
    // The wrapper takes the value; the engine does NOT auto-reset between
    // selections. This pins that contract.
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::GameOver;
    runner.game.request_play_order_selection(1);
    runner
        .game
        .resolve_selection(1, PLAY_FIRST)
        .expect("resolve");

    assert_eq!(runner.game.last_play_order_choice, Some(PlayOrder::First));

    // Wrapper "takes" the value by setting None.
    runner.game.last_play_order_choice = None;
    assert_eq!(runner.game.last_play_order_choice, None);
}
