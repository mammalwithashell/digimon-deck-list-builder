//! Integration test: a full offline game vs a null-agent policy (stub) drives
//! to legal completion via the Tauri layer's `run_agent_steps` dispatch.
//!
//! # Approach: stub policy (no ONNX fixture)
//!
//! This test does NOT exercise ONNX inference. It uses `PlayerKind::Greedy`
//! for both seats, which routes through the production `run_agent_steps` path
//! including the `GameSession` + `PlayerKind` dispatch logic. ONNX loading is
//! already covered by `it_model_download.rs` (Task 16). The scope here is:
//!
//!   "does a complete game driven by the Tauri agent loop reach a legal
//!    terminal state without panicking or looping forever?"
//!
//! This is the minimal test seam that proves the desktop game-play path (the
//! same code `rust_step_game` calls) drives games to completion. A follow-up
//! test can layer in a real ONNX fixture once one is available in the repo.
//!
//! # Determinism
//!
//! The greedy policy is deterministic (no randomness). `Game::new` uses
//! `Some(42)` as the RNG seed. The game will always end with the same winner
//! from the same initial state, so the test is not flaky.

use digimon_tcg::engine_commands::{
    run_agent_steps, test_card_db, test_deck, GameSession, PlayerKind,
};
use digimon_tcg::inference_state::InferenceState;
use digimon_engine::{
    card_registry::CardRegistry,
    game::Game,
    rules::Rules,
};

// ─── helpers ──────────────────────────────────────────────────────────────────

/// Build a freshly-started game, past mulligan, using the standard test
/// card database and deck lists. Seeds the RNG at 42 for determinism.
fn build_started_game() -> (Game, CardRegistry) {
    let db = test_card_db();
    let decks = vec![test_deck(), test_deck()];
    let mut game = Game::new(&decks, &db, Rules::standard(), Some(42))
        .expect("Game::new must succeed with valid test decks");
    game.start_game();

    // Drive through mulligan: both players keep their opening hands.
    while let Some(p) = game.mulligan_current_player() {
        game.accept_mulligan(p, /* keep */ true)
            .expect("accept_mulligan must succeed during mulligan phase");
    }

    let registry = CardRegistry::from_cards(&db);
    (game, registry)
}

// ─── core test ────────────────────────────────────────────────────────────────

/// A full offline game with both seats as `Greedy` agents reaches a legal
/// terminal state. This exercises `run_agent_steps` → `GameSession` →
/// `PlayerKind::Greedy` dispatch — the same code path that `rust_step_game`
/// uses at runtime.
///
/// Assertions:
/// - `game.game_over` is `true` (loop terminated normally, not via cap).
/// - `game.winner` is `Some(_)` (a real winner was declared, not a timeout).
/// - No panics (mask violations, illegal actions, etc.).
#[test]
fn test_offline_game_vs_greedy_null_agent_completes_legally() {
    let (mut game, registry) = build_started_game();

    let session = GameSession {
        registry: Some(registry),
        player_kinds: vec![PlayerKind::Greedy, PlayerKind::Greedy],
        player_model_ids: vec![None, None],
    };
    let inference = InferenceState::default();

    // `run_agent_steps` drives until game_over (both seats are non-human) or
    // MAX_AGENT_STEPS is hit (returns Err in that case — we treat it as a
    // test failure below).
    let result = run_agent_steps(&mut game, &session, &inference);

    assert!(
        result.is_ok(),
        "run_agent_steps must not error (possible mask bug or infinite loop): {:?}",
        result.err()
    );
    assert!(
        game.game_over,
        "game must be over after two greedy agents run to completion"
    );
    assert!(
        game.winner.is_some(),
        "a winner must be declared (got game_over with no winner — unexpected draw/timeout)"
    );
}

// ─── additional coverage ──────────────────────────────────────────────────────

/// Verify that a game driven only from the human seat (no agent) does NOT
/// auto-advance — `run_agent_steps` must stop immediately and leave the game
/// state unchanged. Guards against accidental regressions where the agent
/// loop advances human turns.
#[test]
fn test_offline_human_game_does_not_auto_advance() {
    let (mut game, registry) = build_started_game();

    let session = GameSession {
        registry: Some(registry),
        player_kinds: vec![PlayerKind::Human, PlayerKind::Human],
        player_model_ids: vec![None, None],
    };
    let inference = InferenceState::default();

    let turn_before = game.turn_count;
    let phase_before = game.current_phase;

    run_agent_steps(&mut game, &session, &inference)
        .expect("run_agent_steps must not error for a human session");

    assert_eq!(
        game.turn_count, turn_before,
        "turn count must not advance when both seats are human"
    );
    assert_eq!(
        game.current_phase, phase_before,
        "phase must not change when both seats are human"
    );
    assert!(
        !game.game_over,
        "game must not be over after a human session with no moves"
    );
}

/// Smoke-check that the action mask produced after game start is the correct
/// engine size (2168). Guards against shape mismatches that would cause the
/// Tauri frontend to misread the mask array.
#[test]
fn test_action_mask_has_correct_size_after_game_start() {
    use digimon_tcg::engine_commands::GameSession as Gs;
    use digimon_engine::action::{build_action_mask, space::ACTION_SPACE_SIZE};

    let (game, _registry) = build_started_game();

    // The turn player is the current decision-maker post-mulligan.
    let pid = game.turn_player();
    let mask = build_action_mask(&game, pid);

    assert_eq!(
        mask.len(),
        ACTION_SPACE_SIZE,
        "action mask must have exactly ACTION_SPACE_SIZE ({}) entries, got {}",
        ACTION_SPACE_SIZE,
        mask.len()
    );

    let legal_count = mask.iter().filter(|&&v| v > 0.0).count();
    assert!(
        legal_count > 0,
        "at least one action must be legal at the start of the game"
    );

    // Suppress unused-import warning.
    let _ = Gs::default();
}
