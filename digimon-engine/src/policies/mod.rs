//! Opponent policies — pluggable action selection for bot players.
//!
//! Used by the Tauri desktop build to drive AI opponents in-process (no
//! Python sidecar), and by `HeadlessRunner::run_until_conclusion` for
//! headless simulation. The signature matches the closure shape that
//! runner already expects: `FnMut(&Game, &[f32]) -> u16`.
//!
//! The ONNX-backed policy (MLP + LSTM) lives alongside this crate in the
//! `inference` module (see `digimon-engine/src/inference/`) — this module
//! carries only game-state-driven policies (greedy heuristic + random).

pub mod greedy;
pub mod random;

use crate::enums::PlayerId;
use crate::game::Game;

pub use greedy::GreedyPolicy;
pub use random::RandomPolicy;

/// Who is expected to submit the next action — mulligan decider > pending
/// selection holder > turn player. Matches
/// `engine_commands.rs::current_decision_player` and
/// `HeadlessRunner::current_decision_player`.
pub fn current_decision_player(game: &Game) -> PlayerId {
    if let Some(p) = game.mulligan_current_player() {
        return p;
    }
    if let Some(sel) = game.pending_selection.as_ref() {
        return sel.selecting_player;
    }
    game.turn_player()
}

/// Pluggable decision-maker for a single seat in a game.
///
/// Implementations should be deterministic given fixed inputs, or seeded
/// to be reproducible. The `mask` slice is `ACTION_SPACE_SIZE` floats; an
/// action is legal when its entry is > 0.
pub trait Policy: Send {
    /// Pick a valid action id. Implementations must not return a masked-out
    /// action — the game will panic or silently misbehave on decode.
    fn select(&mut self, game: &Game, mask: &[f32]) -> u16;

    /// Reset any per-episode state (e.g. LSTM hidden state). Default is a
    /// no-op for stateless policies.
    fn reset(&mut self) {}
}
