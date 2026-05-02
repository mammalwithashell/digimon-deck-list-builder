//! Opponent policies — pluggable action selection for bot players.
//!
//! Two shapes coexist here by design:
//!
//! 1. **Free-function API** — `greedy_action(game, mask) -> u16` (and the
//!    trait's stateless implementations) for callers that just want a
//!    one-shot pick. Used by the Tauri `PlayerKind::Greedy` seat in the
//!    desktop build; lands on this branch via #332.
//!
//! 2. **`Policy` trait** — stateful action selection (`select` +
//!    `reset`), for callers that want to swap in alternative heuristics
//!    (`RandomPolicy`), thread an LSTM hidden state, or plug in an ONNX
//!    inference policy from `crate::inference`. Used by
//!    `HeadlessRunner::run_until_conclusion` and by gauntlet-style
//!    self-play drivers.
//!
//! Both APIs delegate to the same underlying heuristic in `greedy::`,
//! so Python-parity behavior is maintained regardless of shape.
//!
//! The ONNX-backed policy (MLP + LSTM) lives in `crate::inference` so
//! this module stays free of the `ort` dependency.

pub mod greedy;
pub mod random;

use crate::enums::PlayerId;
use crate::game::Game;

pub use greedy::greedy_action;
pub use random::RandomPolicy;

/// Stateful action selection. Implementations must return a valid action
/// id from the supplied mask — the game will panic on decode if a masked
/// bit is returned.
pub trait Policy {
    /// Pick the next action for the player whose turn it is (or the
    /// player `current_decision_player` identifies during a selection
    /// phase). The returned id must be valid in `mask`.
    fn select(&mut self, game: &Game, mask: &[f32]) -> u16;

    /// Reset any per-episode state (e.g. LSTM hidden state). Default is a
    /// no-op for stateless policies.
    fn reset(&mut self) {}
}

/// Stateless wrapper around the `greedy_action` free function so callers
/// with a `Box<dyn Policy>` can slot in the Python-parity heuristic.
pub struct GreedyPolicy;

impl GreedyPolicy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GreedyPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl Policy for GreedyPolicy {
    fn select(&mut self, game: &Game, mask: &[f32]) -> u16 {
        greedy_action(game, mask)
    }
}

/// Who should answer the current decision point. During Mulligan that's
/// `mulligan_current_player`; during a selection phase it's
/// `pending_selection.selecting_player`; otherwise it's the turn player.
/// Exposed here so alternative policies can make the same routing decision
/// the heuristic does.
pub fn current_decision_player(game: &Game) -> PlayerId {
    if let Some(p) = game.mulligan_current_player() {
        return p;
    }
    if let Some(sel) = game.pending_selection.as_ref() {
        return sel.selecting_player;
    }
    game.turn_player()
}
