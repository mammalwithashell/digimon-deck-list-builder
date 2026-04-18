//! Game recorder — captures post-shuffle initial state and per-action
//! deltas. Mirrors `digimon_gym/engine/recording.py::GameRecorder`.
//!
//! The recorder is a standalone struct, **not** embedded in `Game`. A
//! runner (future `HeadlessRunner` or a test harness) owns the recorder
//! and wraps `Game::decode_action` with `record_action` /
//! `finalize_action`. This mirrors Python's `HeadlessGame` pattern at
//! `digimon_gym/engine/runners/headless_game.py:46-54`.
//!
//! Deferred: tensor snapshotting, structured events, ISO-8601
//! timestamp (needs a date/time dep we don't currently pull in).

use serde::{Deserialize, Serialize};

use crate::enums::PlayerId;
use crate::game::Game;

/// Post-shuffle deck/hand/security ordering for one player. Captured
/// once, right after `Game::start_game` completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInitialState {
    pub player_id: PlayerId,
    /// Original deck list (pre-shuffle). Backfilled by the runner; the
    /// recorder itself only sees the shuffled `deck` on the `Player`.
    pub deck_list: Vec<String>,
    /// Card IDs in the main library after shuffle. Index 0 is the top.
    pub library_order: Vec<String>,
    /// Card IDs in the egg library after shuffle.
    pub digitama_library_order: Vec<String>,
    /// Security stack ordering (index 0 = bottom).
    pub security_order: Vec<String>,
    /// Opening hand card IDs.
    pub initial_hand: Vec<String>,
}

/// Complete initial game state after `start_game()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialState {
    pub first_player_id: PlayerId,
    pub players: Vec<PlayerInitialState>,
}

/// A single recorded action with before/after memory and turn metadata.
/// Produced by `record_action` (pre-execution) and finalized by
/// `finalize_action` (post-execution).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedAction {
    pub step_number: u32,
    pub player_id: PlayerId,
    pub action_id: u16,
    pub phase: String,
    pub memory_before: i16,
    pub memory_after: i16,
    pub turn_number: u16,
    pub is_game_over: bool,
    pub winner_id: Option<PlayerId>,
}

/// Captures game actions and initial state for replay / debugging.
#[derive(Debug, Default)]
pub struct GameRecorder {
    initial: Option<InitialState>,
    actions: Vec<RecordedAction>,
    step_counter: u32,
}

impl GameRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Capture post-shuffle initial state. Must be called AFTER
    /// `Game::start_game` so libraries are shuffled and hands dealt.
    pub fn capture_initial_state(&mut self, game: &Game) {
        let players: Vec<PlayerInitialState> = game
            .players
            .iter()
            .map(|p| PlayerInitialState {
                player_id: p.id,
                deck_list: Vec::new(), // Backfilled by runner
                library_order: p
                    .deck
                    .iter()
                    .map(|c| c.card_id(&game.card_data).to_string())
                    .collect(),
                digitama_library_order: p
                    .digitama_deck
                    .iter()
                    .map(|c| c.card_id(&game.card_data).to_string())
                    .collect(),
                security_order: p
                    .security
                    .iter()
                    .map(|c| c.card_id(&game.card_data).to_string())
                    .collect(),
                initial_hand: p
                    .hand
                    .iter()
                    .map(|c| c.card_id(&game.card_data).to_string())
                    .collect(),
            })
            .collect();

        self.initial = Some(InitialState {
            first_player_id: game.turn_player(),
            players,
        });
    }

    pub fn initial_state(&self) -> Option<&InitialState> {
        self.initial.as_ref()
    }

    /// Record an action. Call BEFORE `Game::decode_action`. The returned
    /// value is a snapshot of the stored entry; pass it to
    /// `finalize_action` after execution to update the post-state fields.
    pub fn record_action(
        &mut self,
        game: &Game,
        action_id: u16,
        player_id: PlayerId,
    ) -> RecordedAction {
        self.step_counter += 1;
        let entry = RecordedAction {
            step_number: self.step_counter,
            player_id,
            action_id,
            phase: format!("{:?}", game.current_phase),
            memory_before: game.memory,
            memory_after: game.memory,
            turn_number: game.turn_count,
            is_game_over: false,
            winner_id: None,
        };
        self.actions.push(entry.clone());
        entry
    }

    /// Finalize the most recently recorded action with post-execution
    /// state. Call AFTER `Game::decode_action`.
    pub fn finalize_action(&mut self, game: &Game, entry: &mut RecordedAction) {
        entry.memory_after = game.memory;
        entry.is_game_over = game.game_over;
        entry.winner_id = game.winner;
        if let Some(last) = self.actions.last_mut() {
            last.memory_after = entry.memory_after;
            last.is_game_over = entry.is_game_over;
            last.winner_id = entry.winner_id;
        }
    }

    pub fn actions(&self) -> &[RecordedAction] {
        &self.actions
    }

    /// Serialize the full recording to a JSON value with `initial_state`
    /// and `actions` top-level keys.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "initial_state": self.initial,
            "actions": self.actions,
        })
    }
}
