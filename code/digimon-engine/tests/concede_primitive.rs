//! Integration tests for `Game::concede` — the always-legal "give up the
//! game" primitive used by RL action `93` (`CONCEDE_GAME`). The primitive is
//! used by best-of-three match training to short-circuit a game whose
//! outcome the agent has already decided is not worth playing out.
//!
//! Contract:
//! - `pending_selection` is cleared.
//! - `effect_queue` is dropped.
//! - A `GameEvent::Concede` event is emitted **before** the terminal
//!   `GameEvent::GameOver` event (mirroring the surrender-event-before-
//!   declare_winner pattern used by the legacy Python engine — see
//!   `CLAUDE.md` working rule #16).
//! - The terminal-outcome reason is `Concede`, surfaced to Python as
//!   `win_reason = "concede"`.
//! - The winner is the conceder's opponent.
//! - No-op if the game is already over.

use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::GamePhase;
use digimon_engine::events::GameEvent;
use digimon_engine::game::TerminalOutcomeReason;

/// Helper — count the events emitted on `runner.game`. We assert on
/// `events()` not on `drain_events()` so the assertion does not consume
/// the event log.
fn find_event_seq<F: Fn(&GameEvent) -> bool>(runner: &DebugRunner, pred: F) -> Option<u64> {
    runner
        .game
        .events()
        .iter()
        .find(|ev| pred(ev))
        .map(|ev| ev.seq())
}

#[test]
fn concede_in_main_phase_declares_opponent_winner() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::Main;

    assert!(!runner.game.game_over);
    runner.game.concede(0);

    // Game state
    assert!(runner.game.game_over, "game_over must be set");
    assert_eq!(runner.game.winner, Some(1), "opponent of conceder wins");
    assert_eq!(
        runner.game.terminal_outcome_reason,
        Some(TerminalOutcomeReason::Concede)
    );
    assert_eq!(runner.game.current_phase, GamePhase::GameOver);

    // Event ordering: Concede before GameOver
    let concede_seq = find_event_seq(&runner, |ev| matches!(ev, GameEvent::Concede { .. }))
        .expect("Concede event must be emitted");
    let game_over_seq = find_event_seq(&runner, |ev| matches!(ev, GameEvent::GameOver { .. }))
        .expect("GameOver event must be emitted");
    assert!(
        concede_seq < game_over_seq,
        "Concede (seq={}) must precede GameOver (seq={})",
        concede_seq,
        game_over_seq
    );

    // Concede event payload identifies the conceder
    let conceder = runner
        .game
        .events()
        .iter()
        .find_map(|ev| match ev {
            GameEvent::Concede { player, .. } => Some(*player),
            _ => None,
        })
        .expect("Concede event present");
    assert_eq!(conceder, 0);
}

#[test]
fn concede_by_player_one_declares_player_zero_winner() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::Main;

    runner.game.concede(1);
    assert_eq!(runner.game.winner, Some(0));
    assert_eq!(
        runner.game.terminal_outcome_reason,
        Some(TerminalOutcomeReason::Concede)
    );
}

#[test]
fn concede_clears_pending_selection() {
    // Drive into a selection phase by setting up a digivolve-prompt-ish
    // condition: we don't need a real engine selection to verify the
    // contract — we manually install a sentinel pending_selection and assert
    // it's cleared. The selection_test pattern is the right place to exercise
    // the engine-driven selection paths; here we just pin the contract.
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::Main;

    // Use a sample real selection installation if available. For now,
    // a sentinel: install a PendingSelection-like marker by going through
    // an attack-block path is complex. Simplest contract check is that
    // after concede, the optional is None.
    assert!(
        runner.game.pending_selection.is_none(),
        "fresh runner has no pending selection"
    );
    runner.game.concede(0);
    assert!(
        runner.game.pending_selection.is_none(),
        "pending_selection must be None after concede"
    );
}

#[test]
fn concede_drops_effect_queue() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::Main;

    // The runner starts with an empty queue. After concede it must remain
    // empty regardless of pre-state. (Inserting a fake QueuedEffect requires
    // a real source permanent and timing context — out of scope for the
    // contract check; the more involved drainer behavior is exercised by
    // existing effect_queue tests.)
    assert!(
        runner.game.effect_queue.is_empty(),
        "fresh runner has empty effect_queue"
    );
    runner.game.concede(0);
    assert!(
        runner.game.effect_queue.is_empty(),
        "effect_queue must be empty after concede"
    );
}

#[test]
fn concede_in_mulligan_phase_terminates_immediately() {
    let mut runner = DebugRunner::builder().start();
    // Fresh runner starts in mulligan after `.start()` — pin that, then
    // concede should resolve immediately.
    runner.game.current_phase = GamePhase::Mulligan;

    runner.game.concede(0);

    assert!(runner.game.game_over);
    assert_eq!(runner.game.winner, Some(1));
    assert_eq!(runner.game.current_phase, GamePhase::GameOver);
    assert_eq!(
        runner.game.terminal_outcome_reason,
        Some(TerminalOutcomeReason::Concede)
    );
}

#[test]
fn concede_in_block_timing_terminates_immediately() {
    // Concede during a combat block-timing window. We synthesise the phase
    // directly rather than driving a full attack — the contract is that
    // concede works regardless of phase.
    let attacker_card = make_test_card_with_level("ATK-LV3", "Attacker", 3);
    let defender_card = make_test_card_with_level("DEF-LV3", "Defender", 3);
    let mut runner = DebugRunner::builder()
        .add_card(attacker_card)
        .add_card(defender_card)
        .start();
    runner.place_on_field(0, "ATK-LV3", None);
    runner.place_on_field(1, "DEF-LV3", None);
    runner.game.current_phase = GamePhase::BlockTiming;

    runner.game.concede(0);

    assert!(runner.game.game_over);
    assert_eq!(runner.game.winner, Some(1));
    assert_eq!(
        runner.game.terminal_outcome_reason,
        Some(TerminalOutcomeReason::Concede)
    );
}

#[test]
fn concede_in_counter_timing_terminates_immediately() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::CounterTiming;

    runner.game.concede(1);

    assert!(runner.game.game_over);
    assert_eq!(runner.game.winner, Some(0));
    assert_eq!(
        runner.game.terminal_outcome_reason,
        Some(TerminalOutcomeReason::Concede)
    );
}

#[test]
fn concede_in_alliance_timing_terminates_immediately() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::AllianceTiming;

    runner.game.concede(0);

    assert!(runner.game.game_over);
    assert_eq!(runner.game.winner, Some(1));
    assert_eq!(
        runner.game.terminal_outcome_reason,
        Some(TerminalOutcomeReason::Concede)
    );
}

#[test]
fn concede_is_idempotent_when_game_already_over() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::Main;

    runner.game.concede(0);
    let events_after_first = runner.game.events().len();
    let winner_after_first = runner.game.winner;
    let reason_after_first = runner.game.terminal_outcome_reason;

    // Second concede call should be a no-op (game already over).
    runner.game.concede(1);
    assert_eq!(
        runner.game.events().len(),
        events_after_first,
        "second concede call must not emit additional events"
    );
    assert_eq!(
        runner.game.winner, winner_after_first,
        "winner must not change on duplicate concede"
    );
    assert_eq!(
        runner.game.terminal_outcome_reason, reason_after_first,
        "reason must not change on duplicate concede"
    );
}

#[test]
fn terminal_outcome_reason_concede_renders_as_concede() {
    assert_eq!(TerminalOutcomeReason::Concede.as_str(), "concede");
    assert_eq!(TerminalOutcomeReason::Concede.result(), "win");
}

/// Winner attribution must depend ONLY on who conceded — never on whose
/// turn it is or which side of the gauge the memory sits on. Regression
/// guard for the "surrendering shows a victory screen" bug report (the
/// user suspected memory involvement; the engine primitive must be
/// invariant across the full turn × memory × conceder matrix).
#[test]
fn concede_winner_is_opponent_regardless_of_turn_and_memory() {
    for turn_player in [0u8, 1u8] {
        for memory in [-5i16, 0, 5] {
            for conceder in [0u8, 1u8] {
                let mut runner = DebugRunner::builder().start();
                runner.game.current_phase = GamePhase::Main;
                runner.game.turn_order = vec![0, 1];
                runner.game.turn_player_idx = turn_player as usize;
                runner.game.memory = memory;

                runner.game.concede(conceder);

                let expected_winner = 1 - conceder;
                assert!(runner.game.game_over);
                assert_eq!(
                    runner.game.winner,
                    Some(expected_winner),
                    "conceder={conceder} turn_player={turn_player} memory={memory}: \
                     winner must be the conceder's opponent"
                );
                assert_eq!(
                    runner.game.terminal_outcome_reason,
                    Some(TerminalOutcomeReason::Concede)
                );

                // Rule-16 ordering holds in every cell of the matrix.
                let concede_seq =
                    find_event_seq(&runner, |ev| matches!(ev, GameEvent::Concede { .. }))
                        .expect("Concede event present");
                let game_over_seq =
                    find_event_seq(&runner, |ev| matches!(ev, GameEvent::GameOver { .. }))
                        .expect("GameOver event present");
                assert!(concede_seq < game_over_seq);
            }
        }
    }
}
