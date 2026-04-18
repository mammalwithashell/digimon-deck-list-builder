//! RL-shaped game runner — a thin wrapper over `Game` that mirrors
//! `digimon_gym/engine/runners/headless_game.py::HeadlessGame` so PyO3
//! bindings can sit on a stable surface and `DigimonEnv` can swap backends
//! via a flag.

use std::collections::HashMap;

use crate::action::build_action_mask;
use crate::card_data::CardData;
use crate::card_registry::CardRegistry;
use crate::enums::{GamePhase, PlayerId};
use crate::game::Game;
use crate::recorder::GameRecorder;
use crate::rules::Rules;
use crate::tensor::{build_tensor, TENSOR_SIZE};

/// Default PASS action — matches Python `ACTION_PASS_TURN = 62`.
const ACTION_PASS_TURN: u16 = 62;
/// Default mulligan-keep action.
const ACTION_MULLIGAN_KEEP: u16 = 0;

/// A minimal RL-driver wrapper over `Game`.
///
/// Mirrors the Python `HeadlessGame` contract so `DigimonEnv` can duck-type
/// on: `step`, `get_action_mask`, `get_board_tensor`, `is_game_over`,
/// `winner_id`, `run_until_conclusion`.
pub struct HeadlessRunner {
    pub game: Game,
    registry: CardRegistry,
    #[allow(dead_code)]
    verbose: bool,
    #[allow(dead_code)]
    record_actions: bool,
    record_tensors: bool,
    /// Recorder — `Some` when `record_actions` is true, `None` otherwise.
    recorder: Option<GameRecorder>,
    /// Original deck lists retained for lazy `capture_initial_state`.
    deck1_ids: Vec<String>,
    deck2_ids: Vec<String>,
}

impl HeadlessRunner {
    /// Construct a new runner. `deck1_ids` and `deck2_ids` are the two
    /// players' deck lists (card_id strings, including 4 DigiEggs). The
    /// runner builds its own `CardRegistry` from `all_card_data` so the
    /// tensor encodes identical indices to the Python side.
    pub fn new(
        deck1_ids: Vec<String>,
        deck2_ids: Vec<String>,
        all_card_data: &HashMap<String, CardData>,
        verbose: bool,
        record_actions: bool,
        record_tensors: bool,
        seed: Option<u64>,
    ) -> Result<Self, String> {
        let registry = CardRegistry::from_cards(all_card_data);
        let decks = vec![deck1_ids.clone(), deck2_ids.clone()];
        let game = Game::new(&decks, all_card_data, Rules::standard(), seed)?;

        // Do NOT capture initial state here — security is not yet dealt
        // (Game is still in Mulligan phase). Capture lazily on first step()
        // after mulligan completes. See Issue 1 in the spec-compliance fixes.
        let recorder = if record_actions {
            Some(GameRecorder::new(record_tensors))
        } else {
            None
        };

        Ok(Self {
            game,
            registry,
            verbose,
            record_actions,
            record_tensors,
            recorder,
            deck1_ids,
            deck2_ids,
        })
    }

    /// Execute a single action for the current player. No-op after
    /// `game_over`, matching `headless_game.py:41`.
    pub fn step(&mut self, action_id: u16) {
        if self.game.game_over {
            return;
        }
        let pid = self.current_decision_player();

        // Lazy initial-state capture (pre-action): fires if mulligan is already
        // done before this action (e.g. the very first post-mulligan step).
        if let Some(r) = self.recorder.as_mut() {
            if r.initial_state().is_none() && self.game.mulligan_current_player().is_none() {
                r.capture_initial_state(&self.game, (&self.deck1_ids, &self.deck2_ids));
            }
        }

        // Snapshot tensor BEFORE taking &mut self.recorder (avoids borrow
        // conflict). Only snapshot after mulligan is complete, matching Python
        // parity: headless_game.py:46-49 captures tensor before record_action.
        let snapshot = if self.record_tensors
            && self.recorder.is_some()
            && self.game.mulligan_current_player().is_none()
        {
            Some((self.get_board_tensor(None), self.get_action_mask()))
        } else {
            None
        };
        if let (Some((t, m)), Some(r)) = (snapshot, self.recorder.as_mut()) {
            r.record_tensor(pid, t, m);
        }

        let idx = self
            .recorder
            .as_mut()
            .map(|r| r.record_action(&self.game, action_id, pid));
        self.game.decode_action(action_id, pid);
        if let (Some(i), Some(r)) = (idx, self.recorder.as_mut()) {
            r.finalize_action(i, &self.game);
        }

        // Lazy initial-state capture (post-action): fires if this action just
        // completed the mulligan phase (last player's mulligan decision).
        // Security is now dealt; this is the correct moment to capture.
        if let Some(r) = self.recorder.as_mut() {
            if r.initial_state().is_none() && self.game.mulligan_current_player().is_none() {
                r.capture_initial_state(&self.game, (&self.deck1_ids, &self.deck2_ids));
            }
        }
    }

    /// Run to conclusion using `policy` (or a pass-everything default).
    /// Returns the winner's `PlayerId` (0-indexed in Rust — callers should
    /// convert to Python's 1/2 convention at the binding layer). Returns
    /// `u8::MAX` to signal no winner (draw/timeout). Matches the Python
    /// semantics of declaring player 1 (Rust player 0) at timeout.
    pub fn run_until_conclusion<F>(&mut self, max_turns: u32, mut policy: Option<F>) -> u8
    where
        F: FnMut(&Game, &[f32]) -> u16,
    {
        let mut steps = 0u32;
        while !self.game.game_over && steps < max_turns {
            let action = if let Some(p) = policy.as_mut() {
                let mask = self.get_action_mask();
                p(&self.game, &mask)
            } else {
                self.default_action()
            };
            self.step(action);
            steps += 1;
        }

        if !self.game.game_over {
            self.game.declare_winner(0);
        }

        self.game.winner.unwrap_or(u8::MAX)
    }

    /// Action mask for the current deciding player (float32 `ACTION_SPACE_SIZE`).
    pub fn get_action_mask(&self) -> Vec<f32> {
        build_action_mask(&self.game, self.current_decision_player())
    }

    /// Observation tensor from `player_id`'s perspective (defaults to the
    /// current decision player).
    pub fn get_board_tensor(&self, player_id: Option<PlayerId>) -> Vec<f32> {
        let pid = player_id.unwrap_or_else(|| self.current_decision_player());
        build_tensor(&self.game, pid, &self.registry)
    }

    pub fn get_last_log(&self) -> Vec<String> {
        Vec::new()
    }

    /// Return the serialized recording as a JSON value, or `None` if
    /// recording was not enabled (`record_actions=false`).
    ///
    /// The returned `Value` matches Python's `GameRecorder.to_dict` shape:
    /// `{ initial_state, actions, total_actions, tensor_snapshots_count,
    ///   tensor_snapshots }`. Player IDs use the Python 1/2 convention.
    pub fn get_recording(&self) -> Option<serde_json::Value> {
        self.recorder.as_ref().map(|r| r.to_json())
    }

    pub fn is_game_over(&self) -> bool {
        self.game.game_over
    }

    /// Winner's player_id, or `u8::MAX` if no winner yet. The PyO3 layer
    /// maps this to the Python `1/2/0` convention.
    pub fn winner_id(&self) -> u8 {
        self.game.winner.unwrap_or(u8::MAX)
    }

    pub fn accept_mulligan(&mut self, pid: PlayerId, keep: bool) -> Result<(), &'static str> {
        self.game.accept_mulligan(pid, keep)
    }

    pub fn mulligan_current_player(&self) -> Option<PlayerId> {
        self.game.mulligan_current_player()
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    /// Who is expected to submit the next action. In most phases that's
    /// `turn_player`; during a selection/interrupt it's the selecting
    /// player; during mulligan it's whoever is next in `mulligan_pending`.
    fn current_decision_player(&self) -> PlayerId {
        if let Some(p) = self.game.mulligan_current_player() {
            return p;
        }
        if let Some(sel) = self.game.pending_selection.as_ref() {
            return sel.selecting_player;
        }
        self.game.turn_player()
    }

    fn default_action(&self) -> u16 {
        if self.game.current_phase == GamePhase::Mulligan {
            ACTION_MULLIGAN_KEEP
        } else {
            ACTION_PASS_TURN
        }
    }
}

/// Tensor size for callers that want to allocate up-front.
pub const RUNNER_TENSOR_SIZE: usize = TENSOR_SIZE;
