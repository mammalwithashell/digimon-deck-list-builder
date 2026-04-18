//! Game recorder — captures post-shuffle initial state and per-action
//! deltas. Mirrors `digimon_gym/engine/recording.py::GameRecorder`.
//!
//! The recorder is a standalone struct, **not** embedded in `Game`. A
//! runner (`HeadlessRunner` or a test harness) owns the recorder and wraps
//! `Game::decode_action` with `record_action` / `finalize_action`. This
//! mirrors Python's `HeadlessGame` pattern at
//! `digimon_gym/engine/runners/headless_game.py:46-54`.
//!
//! Player IDs are stored internally as Rust `PlayerId` (0/1). The
//! `to_json()` method translates to Python's 1/2 convention at
//! serialization time — matching the CLAUDE.md rule #20 binding boundary.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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
    /// Player at Rust index 0 (player 1 in Python convention).
    pub player1: PlayerInitialState,
    /// Player at Rust index 1 (player 2 in Python convention).
    pub player2: PlayerInitialState,
    /// Capture timestamp — seconds since UNIX epoch. Matches Python's
    /// `datetime.now(timezone.utc).isoformat()` for replay identification.
    pub timestamp: String,
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

/// Optional tensor + action mask snapshot captured at a given step.
#[derive(Debug, Clone)]
pub struct TensorSnapshot {
    pub step: u32,
    pub player_id: PlayerId,
    pub tensor: Vec<f32>,
    pub action_mask: Vec<f32>,
}

/// Captures game actions and initial state for replay / debugging.
/// Mirrors `digimon_gym/engine/recording.py::GameRecorder`.
#[derive(Debug, Default)]
pub struct GameRecorder {
    pub record_tensors: bool,
    initial: Option<InitialState>,
    actions: Vec<RecordedAction>,
    tensor_snapshots: Vec<TensorSnapshot>,
    step_counter: u32,
}

impl GameRecorder {
    pub fn new(record_tensors: bool) -> Self {
        Self {
            record_tensors,
            initial: None,
            actions: Vec::new(),
            tensor_snapshots: Vec::new(),
            step_counter: 0,
        }
    }

    /// Capture post-shuffle initial state. Must be called AFTER
    /// `Game::start_game` so libraries are shuffled and hands dealt.
    ///
    /// `deck_lists` is `(deck1_card_ids, deck2_card_ids)` — the original
    /// pre-shuffle submission lists. The runner passes these in because
    /// the game only retains the shuffled order.
    pub fn capture_initial_state(
        &mut self,
        game: &Game,
        deck_lists: (&[String], &[String]),
    ) {
        let p1 = &game.players[0];
        let p2 = &game.players[1];

        let to_ids =
            |v: &[crate::card_source::CardSource]| -> Vec<String> {
                v.iter()
                    .map(|c| c.card_id(&game.card_data).to_string())
                    .collect()
            };

        self.initial = Some(InitialState {
            first_player_id: game.turn_order[0],
            timestamp: unix_secs_string(),
            player1: PlayerInitialState {
                player_id: p1.id,
                deck_list: deck_lists.0.to_vec(),
                library_order: to_ids(&p1.deck),
                digitama_library_order: to_ids(&p1.digitama_deck),
                security_order: to_ids(&p1.security),
                initial_hand: to_ids(&p1.hand),
            },
            player2: PlayerInitialState {
                player_id: p2.id,
                deck_list: deck_lists.1.to_vec(),
                library_order: to_ids(&p2.deck),
                digitama_library_order: to_ids(&p2.digitama_deck),
                security_order: to_ids(&p2.security),
                initial_hand: to_ids(&p2.hand),
            },
        });
    }

    pub fn initial_state(&self) -> Option<&InitialState> {
        self.initial.as_ref()
    }

    /// Record an action. Call BEFORE `Game::decode_action`. Returns the
    /// index of the pushed entry so `finalize_action` can update it.
    pub fn record_action(
        &mut self,
        game: &Game,
        action_id: u16,
        player_id: PlayerId,
    ) -> usize {
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
        self.actions.push(entry);
        self.actions.len() - 1
    }

    /// Finalize the action at `idx` with post-execution state. Call AFTER
    /// `Game::decode_action`.
    pub fn finalize_action(&mut self, idx: usize, game: &Game) {
        let rec = &mut self.actions[idx];
        rec.memory_after = game.memory;
        rec.is_game_over = game.game_over;
        rec.winner_id = game.winner;
    }

    /// Capture tensor snapshot. No-op when `record_tensors` is false.
    pub fn record_tensor(
        &mut self,
        player_id: PlayerId,
        tensor: Vec<f32>,
        action_mask: Vec<f32>,
    ) {
        if !self.record_tensors {
            return;
        }
        self.tensor_snapshots.push(TensorSnapshot {
            step: self.step_counter,
            player_id,
            tensor,
            action_mask,
        });
    }

    pub fn actions(&self) -> &[RecordedAction] {
        &self.actions
    }

    /// Serialize to JSON matching Python's `GameRecorder.to_dict` shape:
    ///
    /// ```json
    /// {
    ///   "initial_state": { "first_player_id": 1, "timestamp": "...",
    ///                       "player1": {...}, "player2": {...} },
    ///   "actions": [...],
    ///   "total_actions": N,
    ///   "tensor_snapshots_count": M,
    ///   "tensor_snapshots": [...]
    /// }
    /// ```
    ///
    /// Player IDs are translated to the Python 1/2 convention at this
    /// layer (CLAUDE.md rule #20).
    pub fn to_json(&self) -> Value {
        let py_pid = |p: PlayerId| -> i64 { (p as i64) + 1 };
        let py_opt_pid = |p: Option<PlayerId>| -> Value {
            match p {
                None => Value::Null,
                Some(pid) => json!(py_pid(pid)),
            }
        };

        let initial_state = match &self.initial {
            None => Value::Null,
            Some(is) => {
                json!({
                    "first_player_id": py_pid(is.first_player_id),
                    "timestamp": is.timestamp,
                    "player1": player_initial_json(&is.player1, &py_pid),
                    "player2": player_initial_json(&is.player2, &py_pid),
                })
            }
        };

        let actions: Vec<Value> = self
            .actions
            .iter()
            .map(|a| {
                json!({
                    "step": a.step_number,
                    "player_id": py_pid(a.player_id),
                    "action_id": a.action_id,
                    "phase": a.phase,
                    "memory_before": a.memory_before,
                    "memory_after": a.memory_after,
                    "turn": a.turn_number,
                    "is_game_over": a.is_game_over,
                    "winner_id": py_opt_pid(a.winner_id),
                })
            })
            .collect();

        let tensors: Vec<Value> = self
            .tensor_snapshots
            .iter()
            .map(|ts| {
                json!({
                    "step": ts.step,
                    "player_id": py_pid(ts.player_id),
                    "tensor": ts.tensor,
                    "action_mask": ts.action_mask.iter().map(|m| *m as i64).collect::<Vec<_>>(),
                })
            })
            .collect();

        json!({
            "initial_state": initial_state,
            "actions": actions,
            "total_actions": self.actions.len(),
            "tensor_snapshots_count": self.tensor_snapshots.len(),
            "tensor_snapshots": tensors,
        })
    }
}

fn player_initial_json(
    p: &PlayerInitialState,
    py_pid: &dyn Fn(PlayerId) -> i64,
) -> Value {
    json!({
        "player_id": py_pid(p.player_id),
        "deck_list": p.deck_list,
        "library_order": p.library_order,
        "digitama_library_order": p.digitama_library_order,
        "security_order": p.security_order,
        "initial_hand": p.initial_hand,
    })
}

/// Seconds since UNIX epoch as a string. Used as a lightweight timestamp
/// that matches Python's `datetime.now(timezone.utc).isoformat()` at
/// second resolution for replay identification purposes.
fn unix_secs_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}", secs)
}
