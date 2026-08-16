//! Deterministic game replay from a `GameRecorder` recording.
//!
//! Port of `engine_py_legacy/engine/runners/replay_runner.py`. Closes the
//! Python/Rust parity gap so the source-of-truth engine can investigate
//! recordings produced by training, smoke tests, or PvP games.
//!
//! ## Construction strategy
//!
//! `Game::new` shuffles libraries and deals hands; we need to bypass that
//! and inject the recorded post-shuffle state directly. We follow the
//! `DebugRunner` pattern:
//!
//! 1. Call `Game::new` with empty decks (to materialize the card_data
//!    store, registries, RNG, and other shared engine state).
//! 2. Clear every player zone — `Game::new` placed nothing because the
//!    decks were empty, but we defensively wipe.
//! 3. Manually populate library / hand / security / digitama from the
//!    recording's `initial_state` (in the order it recorded — index 0
//!    is the bottom for libraries, top for hands).
//! 4. Set `turn_count`, `memory`, clear `mulligan_pending`, set
//!    `turn_order` to match `first_player_id`.
//! 5. Call `begin_turn` to fire start-of-turn triggers and advance into
//!    the engine's first action point.
//!
//! ## Mulligan handling
//!
//! `GameRecorder` captures `initial_state` AFTER mulligan completes
//! (security has been dealt, hands settled). The actions array still
//! contains the mulligan decisions, but replaying them against an
//! already-post-mulligan game would error. We filter mulligan-phase
//! actions out during replay.
//!
//! ## Known v1 limitations
//!
//! - Games whose start-of-turn triggers install a `pending_selection`
//!   may diverge if the recording is sensitive to the timing of that
//!   selection. The `verify: true` mode reports divergences but does
//!   not halt replay.
//! - Replay does not exercise the RNG path. Effects that consume RNG
//!   (e.g. random reveal selection) will diverge from the recorded
//!   outcome since the post-construction RNG state is fresh.

use std::collections::HashMap;

use serde::Serialize;
use serde_json::Value;

use crate::card_data::CardData;
use crate::card_source::CardSource;
use crate::enums::{GamePhase, PlayerId};
use crate::game::Game;
use crate::player::Player;
use crate::recorder::{ReplayDeckRef, VerificationReplayRecording};
use crate::rules::Rules;

/// Result of a single replay step.
#[derive(Debug, Clone, Serialize)]
pub struct ReplayStepResult {
    pub step_number: u32,
    pub player_id: PlayerId,
    pub action_id: u16,
    pub phase_before: String,
    pub phase_after: String,
    pub memory_before: i16,
    pub memory_after: i16,
    pub turn_number: u16,
    pub is_game_over: bool,
    pub winner_id: Option<PlayerId>,
    /// Populated only when the runner was constructed with `verify: true`.
    /// Empty Vec when no divergences. Multiple entries are possible per
    /// step (e.g. memory and phase both diverged).
    pub divergences: Vec<DivergenceReport>,
}

/// A discrepancy between replayed state and the value recorded for the
/// same step. Non-fatal — replay continues past divergences.
#[derive(Debug, Clone, Serialize)]
pub struct DivergenceReport {
    pub step: u32,
    pub field: &'static str,
    pub recorded: String,
    pub replayed: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationReplayCheck {
    Actor,
    Legality,
    Digest,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationReplayDivergence {
    pub game: String,
    pub step: u32,
    pub check_type: VerificationReplayCheck,
    pub recorded: String,
    pub replayed: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationReplayReport {
    pub game: String,
    pub checked_steps: u32,
    pub divergence: Option<VerificationReplayDivergence>,
}

/// How a [`ReplaySession`] applies each recorded step.
///
/// - `Trust`: apply the recorded action directly (the recording came from
///   the engine itself — native self-play / eval). Default for
///   `NativeAdapter`.
/// - `CheckThenApply`: verify the recorded actor and mask-membership of the
///   action BEFORE applying. `CONCEDE_GAME` is accepted as the explicit
///   surrender primitive even when hidden from RL masks. On a mismatch record
///   a [`Divergence`] and pause (differential replay against a battle-tested
///   oracle — DCGO). Default for `DcgoAdapter` (Group 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepPolicy {
    Trust,
    CheckThenApply,
}

/// The flavor of a differential [`Divergence`]. Detection is one-directional:
/// it cannot flag an action the engine *over-permits* relative to the oracle
/// (the recording stores only the action taken, not the oracle's full mask).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DivergenceKind {
    /// Recorded action not set in the engine's legal-action mask.
    MaskMiss { sample_legal_ids: Vec<u16> },
    /// Engine expected a different decision player than the recording.
    Actor {
        expected: PlayerId,
        recorded: PlayerId,
    },
    /// Replayed memory differs from the recorded value.
    Memory { recorded: i64, replayed: i64 },
    /// Replayed phase differs from the recorded value.
    Phase { recorded: String, replayed: String },
    /// Terminal winner differs (populated at completion by the DCGO path).
    Winner {
        recorded: Option<PlayerId>,
        replayed: Option<PlayerId>,
    },
    /// Opaque: engine requested a different reveal kind than the queue's next.
    RevealKind { message: String },
    /// Opaque: engine requested a reveal with none remaining.
    RevealExhausted { message: String },
    /// A semantic selection payload (task 3.5) could not be mapped onto the
    /// engine's live `PendingSelection`.
    SelectionResolution { reason: String },
}

/// A differential divergence recorded by `CheckThenApply` — the engine
/// disagreed with the recording about what was legal / who acted / the
/// outcome. Recorded into the session's divergence log; pausing variants
/// (`MaskMiss`, `Actor`) halt the cursor for inspection without aborting.
#[derive(Debug, Clone, Serialize)]
pub struct Divergence {
    pub step: u32,
    pub action_id: u16,
    pub actor: PlayerId,
    pub phase: String,
    pub kind: DivergenceKind,
}

/// Errors surfaced by `ReplayRunner::new`.
#[derive(Debug)]
pub enum ReplayError {
    MissingInitialState,
    MalformedRecording(String),
    UnknownCard(Vec<String>),
    GameConstruction(String),
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingInitialState => {
                write!(f, "recording is missing the `initial_state` field")
            }
            Self::MalformedRecording(s) => write!(f, "malformed recording: {}", s),
            Self::UnknownCard(cards) => {
                write!(f, "recording references unknown cards: {:?}", cards)
            }
            Self::GameConstruction(s) => write!(f, "game construction failed: {}", s),
        }
    }
}

impl std::error::Error for ReplayError {}

/// A scripted game replay over a `GameRecorder` recording.
pub struct ReplayRunner {
    pub game: Game,
    recording: Value,
    /// Indices into `recording["actions"]` that are NON-mulligan and
    /// therefore replayable. The first mulligan actions were already
    /// baked into the recording's `initial_state`.
    replayable_action_indices: Vec<usize>,
    current_step: u32,
    verify: bool,
}

impl ReplayRunner {
    /// Construct a replay runner from a `GameRecorder` JSON value.
    ///
    /// `all_card_data` must contain every `card_id` referenced by the
    /// recording's libraries, hands, security, and digitama; missing
    /// cards produce `ReplayError::UnknownCard`.
    pub fn new(
        recording: Value,
        all_card_data: &HashMap<String, CardData>,
        verify: bool,
    ) -> Result<Self, ReplayError> {
        let initial = recording
            .get("initial_state")
            .ok_or(ReplayError::MissingInitialState)?;
        if initial.is_null() {
            return Err(ReplayError::MissingInitialState);
        }

        // Check that every referenced card_id is in the pool.
        let mut missing: Vec<String> = Vec::new();
        for player_key in &["player1", "player2"] {
            let Some(p) = initial.get(player_key) else {
                continue;
            };
            for zone_key in &[
                "library_order",
                "digitama_library_order",
                "security_order",
                "initial_hand",
            ] {
                if let Some(arr) = p.get(zone_key).and_then(|v| v.as_array()) {
                    for id in arr {
                        if let Some(s) = id.as_str() {
                            if !all_card_data.contains_key(s) && !missing.iter().any(|m| m == s) {
                                missing.push(s.to_string());
                            }
                        }
                    }
                }
            }
        }
        if !missing.is_empty() {
            return Err(ReplayError::UnknownCard(missing));
        }

        let mut runner = Self {
            // Placeholder — replaced by build_game(). Cargo's strict-init
            // means we have to construct a throwaway Game here.
            game: build_empty_game(all_card_data)?,
            recording,
            replayable_action_indices: Vec::new(),
            current_step: 0,
            verify,
        };
        runner.build_game(all_card_data)?;
        Ok(runner)
    }

    /// Number of replayable (non-mulligan) actions in the recording.
    pub fn total_steps(&self) -> u32 {
        self.replayable_action_indices.len() as u32
    }

    /// Current step index — 0 means no actions applied yet.
    pub fn current_step(&self) -> u32 {
        self.current_step
    }

    pub fn is_complete(&self) -> bool {
        self.current_step as usize >= self.replayable_action_indices.len()
    }

    pub fn is_game_over(&self) -> bool {
        self.game.game_over
    }

    pub fn winner_id(&self) -> Option<PlayerId> {
        self.game.winner
    }

    /// Apply exactly one replayable action and return a `ReplayStepResult`.
    /// Returns a no-op result when called past completion (matches the spec).
    pub fn step(&mut self) -> ReplayStepResult {
        if self.is_complete() {
            return ReplayStepResult {
                step_number: self.current_step,
                player_id: 0,
                action_id: 0,
                phase_before: self.game.current_phase.py_name().to_string(),
                phase_after: self.game.current_phase.py_name().to_string(),
                memory_before: self.game.memory,
                memory_after: self.game.memory,
                turn_number: self.game.turn_count,
                is_game_over: self.game.game_over,
                winner_id: self.game.winner,
                divergences: Vec::new(),
            };
        }

        let idx = self.replayable_action_indices[self.current_step as usize];
        let action = &self.recording["actions"][idx];
        let action_id = action["action_id"].as_u64().unwrap_or(0) as u16;
        let recorded_player_py = action["player_id"].as_u64().unwrap_or(1) as u8;
        let player_id: PlayerId = recorded_player_py.saturating_sub(1);

        let phase_before = self.game.current_phase.py_name().to_string();
        let memory_before = self.game.memory;
        let turn_number = self.game.turn_count;

        self.game.decode_action(action_id, player_id);
        self.current_step += 1;

        let phase_after = self.game.current_phase.py_name().to_string();

        let mut result = ReplayStepResult {
            step_number: self.current_step,
            player_id,
            action_id,
            phase_before,
            phase_after,
            memory_before,
            memory_after: self.game.memory,
            turn_number,
            is_game_over: self.game.game_over,
            winner_id: self.game.winner,
            divergences: Vec::new(),
        };

        if self.verify {
            self.verify_step(&mut result, action);
        }

        result
    }

    /// Jump to a specific step number.
    ///
    /// Forward seek replays missing actions. Backward seek rebuilds from
    /// scratch and re-walks to the target. Out-of-range targets clamp.
    pub fn seek(&mut self, target_step: u32) -> Result<(), ReplayError> {
        let total = self.total_steps();
        let target = target_step.min(total);
        if target < self.current_step {
            // Backward: reset-and-replay. Reset the existing game's mutable
            // state in place (reusing card_data + registries — no CardData
            // clone, no registry rebuild), re-lay the initial state, then
            // replay forward below. See `Game::reset_for_replay`.
            self.game.reset_for_replay();
            self.relay_initial_state()?;
        }
        while self.current_step < target && !self.is_game_over() {
            self.step();
        }
        Ok(())
    }

    /// Restore the game to `step_n` via reset-and-replay. Alias for [`seek`]
    /// named to match the MCP `restore_checkpoint` surface; no state snapshot
    /// is taken (the mutable game graph is closure-bearing and uncloneable —
    /// backward restore resets in place and replays).
    ///
    /// [`seek`]: ReplayRunner::seek
    pub fn restore(&mut self, step_n: u32) -> Result<(), ReplayError> {
        self.seek(step_n)
    }

    /// Step through every remaining action.
    pub fn run_to_completion(&mut self) {
        while !self.is_complete() && !self.is_game_over() {
            self.step();
        }
    }

    // ── internals ────────────────────────────────────────────────────────

    fn build_game(&mut self, all_card_data: &HashMap<String, CardData>) -> Result<(), ReplayError> {
        // Fresh empty Game — immutable shared state (card_data + registries)
        // is built once here via `Game::new`.
        self.game = build_empty_game(all_card_data)?;
        self.relay_initial_state()
    }

    /// Reset the game's mutable state to the recording's post-mulligan initial
    /// state and re-lay all zones, WITHOUT reconstructing the immutable shared
    /// state (`card_data` / registries). Reused by initial construction (after
    /// `build_empty_game`) and by backward seek (after
    /// `Game::reset_for_replay`) — this is the reset-and-replay core that
    /// makes backward stepping cheap.
    fn relay_initial_state(&mut self) -> Result<(), ReplayError> {
        self.current_step = 0;

        let initial = &self.recording["initial_state"];
        let first_player_py = initial["first_player_id"].as_u64().ok_or_else(|| {
            ReplayError::MalformedRecording("first_player_id missing or non-numeric".into())
        })? as u8;
        let first_player: PlayerId = first_player_py.saturating_sub(1);

        // Map card_id → data_index for fast restoration.
        let data_index_map: HashMap<&str, usize> = self
            .game
            .card_data
            .iter()
            .enumerate()
            .map(|(idx, cd)| (cd.card_id.as_str(), idx))
            .collect();

        // Wipe whatever the empty-deck Game::new produced (defensive).
        for p in &mut self.game.players {
            p.hand.clear();
            p.deck.clear();
            p.security.clear();
            p.digitama_deck.clear();
            p.battle_area.clear();
            p.breeding_area = None;
            p.trash.clear();
        }

        let mut next_card_index: u16 = 0;

        for (player_idx, key) in ["player1", "player2"].iter().enumerate() {
            let player_id = player_idx as PlayerId;
            let pdata = &initial[key];
            restore_player_zone(
                &mut self.game.players[player_idx],
                pdata,
                &data_index_map,
                player_id,
                &mut next_card_index,
            )?;
        }
        self.game.advance_card_index_to(next_card_index);

        // Mulligan already happened — wipe the pending queue so subsequent
        // logic doesn't try to walk it.
        self.game.mulligan_pending.clear();
        for used in self.game.mulligan_used.iter_mut() {
            *used = false;
        }

        // Set turn order so first_player_id acts first.
        self.game.turn_order = {
            let mut order: Vec<PlayerId> = (0..self.game.rules.player_count).collect();
            // Rotate so first_player is at index 0.
            if let Some(pos) = order.iter().position(|&p| p == first_player) {
                order.rotate_left(pos);
            }
            order
        };
        self.game.turn_player_idx = 0;
        self.game.memory_pair = if self.game.turn_order.len() >= 2 {
            (self.game.turn_order[0], self.game.turn_order[1])
        } else {
            (self.game.turn_order[0], self.game.turn_order[0])
        };

        self.game.turn_count = 1;
        self.game.memory = 0;
        self.game.current_phase = GamePhase::Unsuspend;

        // Fire start-of-turn triggers etc. v1 limitation: if begin_turn
        // installs a pending selection, the replayed action stream may
        // diverge from the recorded one. Verify mode will report it.
        self.game.begin_turn();

        // Filter out mulligan-phase actions — those are already baked into
        // initial_state. Capture indices into the actions array.
        self.replayable_action_indices.clear();
        if let Some(actions) = self.recording["actions"].as_array() {
            for (idx, a) in actions.iter().enumerate() {
                let phase = a["phase"].as_str().unwrap_or("");
                if phase != "Mulligan" {
                    self.replayable_action_indices.push(idx);
                }
            }
        }

        Ok(())
    }

    fn verify_step(&self, result: &mut ReplayStepResult, recorded: &Value) {
        macro_rules! check_field {
            ($field:expr, $recorded_key:expr, $replayed_expr:expr) => {{
                if let Some(rec) = recorded.get($recorded_key) {
                    let recorded_norm = rec.to_string().trim_matches('"').to_string();
                    let replayed_norm = format!("{}", $replayed_expr);
                    if recorded_norm != replayed_norm {
                        result.divergences.push(DivergenceReport {
                            step: result.step_number,
                            field: $field,
                            recorded: recorded_norm,
                            replayed: replayed_norm,
                        });
                    }
                }
            }};
        }

        // memory_after — recorded as i64
        if let Some(rec) = recorded.get("memory_after").and_then(|v| v.as_i64()) {
            if rec != result.memory_after as i64 {
                result.divergences.push(DivergenceReport {
                    step: result.step_number,
                    field: "memory_after",
                    recorded: rec.to_string(),
                    replayed: result.memory_after.to_string(),
                });
            }
        }
        // turn — recorded as u64
        if let Some(rec) = recorded.get("turn").and_then(|v| v.as_u64()) {
            if rec != result.turn_number as u64 {
                result.divergences.push(DivergenceReport {
                    step: result.step_number,
                    field: "turn",
                    recorded: rec.to_string(),
                    replayed: result.turn_number.to_string(),
                });
            }
        }
        // phase — string
        check_field!("phase", "phase", result.phase_before);
        // is_game_over — bool
        if let Some(rec) = recorded.get("is_game_over").and_then(|v| v.as_bool()) {
            if rec != result.is_game_over {
                result.divergences.push(DivergenceReport {
                    step: result.step_number,
                    field: "is_game_over",
                    recorded: rec.to_string(),
                    replayed: result.is_game_over.to_string(),
                });
            }
        }
    }
}

// ── generic replay session (Group 2) ──────────────────────────────────────

/// One normalized replayable decision, decoupled from the source recording
/// format. Native (`GameRecorder`) recordings and — in Group 4 — DCGO
/// recordings both lower to this. The optional recorded fields carry the
/// values a native recording captured for that step; `verify` mode compares
/// them against the replayed state.
#[derive(Debug, Clone)]
pub struct StepSpec {
    pub actor: PlayerId,
    pub action_id: u16,
    /// Phase the action was recorded in (used by `verify`).
    pub phase: String,
    /// Source tag (DCGO: `mulligan`/`main_phase`/…; native: empty).
    pub source: String,
    pub memory_after: Option<i64>,
    pub turn: Option<u64>,
    pub is_game_over: Option<bool>,
    pub expected_digest: Option<u64>,
    /// DCGO semantic selection payload (task 3.5). When set, `action_id` is
    /// a placeholder and the step resolves against the engine's live
    /// `PendingSelection` (possibly into MULTIPLE engine actions — one per
    /// pick plus an optional trailing PASS).
    pub selection: Option<crate::dcgo_recording::SelectionRow>,
}

/// A pluggable replay source. Knows how to build the initial post-mulligan
/// game, re-lay that initial state onto a reset game (for reset-and-replay
/// backward seek), and produce the normalized replayable step list.
///
/// `NativeAdapter` implements this over `GameRecorder` JSON. `DcgoAdapter`
/// (Group 4) will implement it over DCGO `RecordingV1` recordings, so the
/// single [`ReplaySession`] core serves both batch and interactive callers
/// and both recording families.
pub trait RecordingSource: Send {
    /// Build the immutable shared state and lay the initial post-mulligan
    /// zones, returning a ready-to-step `Game`.
    fn build_initial_game(
        &self,
        card_data: &HashMap<String, CardData>,
    ) -> Result<Game, ReplayError>;

    /// Re-lay the initial mutable state onto an already-reset `Game`
    /// (reusing its `card_data` + registries). Used by reset-and-replay
    /// backward seek after `Game::reset_for_replay`.
    fn relay_initial_state(&self, game: &mut Game) -> Result<(), ReplayError>;

    /// The normalized replayable step list (mulligan filtered out).
    fn steps(&self) -> &[StepSpec];

    /// Default step policy for this source. `NativeAdapter` → `Trust`
    /// (engine-generated recording); `DcgoAdapter` (Group 4) →
    /// `CheckThenApply` (differential against the DCGO oracle).
    fn default_policy(&self) -> StepPolicy {
        StepPolicy::Trust
    }
}

/// [`RecordingSource`] over a native `GameRecorder` JSON recording. Holds
/// the canonical native relay logic (previously inlined in `ReplayRunner`),
/// so `ReplaySession` and any future native caller share one code path.
pub struct NativeAdapter {
    recording: Value,
    steps: Vec<StepSpec>,
}

impl NativeAdapter {
    /// Parse + validate a native recording. Errors on a missing
    /// `initial_state` or unknown card IDs (same contract as the historical
    /// `ReplayRunner::new`), and lowers non-mulligan actions to `StepSpec`s.
    pub fn from_recording(
        recording: Value,
        all_card_data: &HashMap<String, CardData>,
    ) -> Result<Self, ReplayError> {
        let initial = recording
            .get("initial_state")
            .ok_or(ReplayError::MissingInitialState)?;
        if initial.is_null() {
            return Err(ReplayError::MissingInitialState);
        }

        let mut missing: Vec<String> = Vec::new();
        for player_key in &["player1", "player2"] {
            let Some(p) = initial.get(player_key) else {
                continue;
            };
            for zone_key in &[
                "library_order",
                "digitama_library_order",
                "security_order",
                "initial_hand",
            ] {
                if let Some(arr) = p.get(zone_key).and_then(|v| v.as_array()) {
                    for id in arr {
                        if let Some(s) = id.as_str() {
                            if !all_card_data.contains_key(s) && !missing.iter().any(|m| m == s) {
                                missing.push(s.to_string());
                            }
                        }
                    }
                }
            }
        }
        if !missing.is_empty() {
            return Err(ReplayError::UnknownCard(missing));
        }

        let mut steps = Vec::new();
        if let Some(actions) = recording["actions"].as_array() {
            for a in actions {
                let phase = a["phase"].as_str().unwrap_or("").to_string();
                if phase == "Mulligan" {
                    continue;
                }
                let action_id = a["action_id"].as_u64().unwrap_or(0) as u16;
                let actor = (a["player_id"].as_u64().unwrap_or(1) as u8).saturating_sub(1);
                steps.push(StepSpec {
                    actor,
                    action_id,
                    phase,
                    source: String::new(),
                    memory_after: a.get("memory_after").and_then(|v| v.as_i64()),
                    turn: a.get("turn").and_then(|v| v.as_u64()),
                    is_game_over: a.get("is_game_over").and_then(|v| v.as_bool()),
                    expected_digest: None,
                    selection: None,
                });
            }
        }

        Ok(Self { recording, steps })
    }
}

impl RecordingSource for NativeAdapter {
    fn build_initial_game(
        &self,
        card_data: &HashMap<String, CardData>,
    ) -> Result<Game, ReplayError> {
        let mut game = build_empty_game(card_data)?;
        self.relay_initial_state(&mut game)?;
        Ok(game)
    }

    fn relay_initial_state(&self, game: &mut Game) -> Result<(), ReplayError> {
        let initial = &self.recording["initial_state"];
        let first_player_py = initial["first_player_id"].as_u64().ok_or_else(|| {
            ReplayError::MalformedRecording("first_player_id missing or non-numeric".into())
        })? as u8;
        let first_player: PlayerId = first_player_py.saturating_sub(1);

        let data_index_map: HashMap<&str, usize> = game
            .card_data
            .iter()
            .enumerate()
            .map(|(idx, cd)| (cd.card_id.as_str(), idx))
            .collect();

        for p in &mut game.players {
            p.hand.clear();
            p.deck.clear();
            p.security.clear();
            p.digitama_deck.clear();
            p.battle_area.clear();
            p.breeding_area = None;
            p.trash.clear();
        }

        let mut next_card_index: u16 = 0;
        for (player_idx, key) in ["player1", "player2"].iter().enumerate() {
            let player_id = player_idx as PlayerId;
            let pdata = &initial[key];
            restore_player_zone(
                &mut game.players[player_idx],
                pdata,
                &data_index_map,
                player_id,
                &mut next_card_index,
            )?;
        }
        game.advance_card_index_to(next_card_index);

        game.mulligan_pending.clear();
        for used in game.mulligan_used.iter_mut() {
            *used = false;
        }

        game.turn_order = {
            let mut order: Vec<PlayerId> = (0..game.rules.player_count).collect();
            if let Some(pos) = order.iter().position(|&p| p == first_player) {
                order.rotate_left(pos);
            }
            order
        };
        game.turn_player_idx = 0;
        game.memory_pair = if game.turn_order.len() >= 2 {
            (game.turn_order[0], game.turn_order[1])
        } else {
            (game.turn_order[0], game.turn_order[0])
        };
        game.turn_count = 1;
        game.memory = 0;
        game.current_phase = GamePhase::Unsuspend;
        game.begin_turn();
        Ok(())
    }

    fn steps(&self) -> &[StepSpec] {
        &self.steps
    }
}

/// [`RecordingSource`] over a DCGO `RecordingV1` recording. Mirrors the
/// `dcgo-replay` batch harness's construction so the batch parity report and
/// the interactive bug-hunter stay on one code path: bot-vs-bot recordings
/// (`opp_deck_post_shuffle` present) build via `Game::new` with both ordered
/// decks; opaque PvP recordings (`opp_deck_post_shuffle == null`) build via
/// `Game::new_with_opaque_opponent` with a `RevealQueue` preloaded from the
/// recording's reveal stream. Default policy is `CheckThenApply` (differential
/// against the DCGO oracle).
pub struct DcgoAdapter {
    my_pid: u8,
    /// Who takes turn 1: `game_start.first_player` when the recording carries
    /// it, else inferred from the first mulligan actor (DCGO's first player
    /// mulligans first), else 0.
    first_player: u8,
    my_deck: Vec<String>,
    /// Ordered opponent deck (bot-vs-bot). `None` for opaque PvP.
    opp_deck: Option<Vec<String>>,
    /// Opaque opponent decklist multiset (PvP). `None` for bot-vs-bot.
    opp_decklist: Option<Vec<String>>,
    /// Ordered reveal stream consumed by the opaque `RevealQueue`.
    reveal_pairs: Vec<(crate::opaque_deck::RevealKind, String)>,
    steps: Vec<StepSpec>,
    /// Card DB retained so backward-seek can reconstruct (DCGO games are not
    /// restored zone-by-zone; see `relay_initial_state`).
    card_data: HashMap<String, CardData>,
}

impl DcgoAdapter {
    /// Build an adapter from a parsed DCGO recording. Lowers the `Action`
    /// rows up to the first `encoder_failure` into the step stream (DCGO
    /// replays mulligan actions too — the game lands in `Mulligan` phase),
    /// and precomputes the opaque opponent decklist + reveal stream when the
    /// recording is PvP.
    pub fn from_recording(
        recording: crate::dcgo_recording::RecordingV1,
        card_data: &HashMap<String, CardData>,
    ) -> Result<Self, ReplayError> {
        use crate::dcgo_recording::Row;

        let my_pid = recording.start.my_player_id;
        let my_deck = recording.start.my_deck_post_shuffle.clone();
        let opp_deck = recording.start.opp_deck_post_shuffle.clone();

        // Turn-1 player: explicit field when present, else inferred from the
        // first mulligan actor (DCGO's first player mulligans first — pre-
        // first_player recordings), else 0.
        let first_player = recording.start.first_player.unwrap_or_else(|| {
            recording
                .rows
                .iter()
                .find_map(|row| match row {
                    Row::Action(a) if a.source == "mulligan" => Some(a.actor),
                    _ => None,
                })
                .unwrap_or(0)
        });

        let mut steps = Vec::new();
        for row in &recording.rows {
            match row {
                Row::Action(a) => steps.push(StepSpec {
                    actor: a.actor,
                    action_id: a.action_id,
                    phase: a.phase.clone(),
                    source: a.source.clone(),
                    memory_after: None,
                    turn: None,
                    is_game_over: None,
                    expected_digest: None,
                    selection: None,
                }),
                // Semantic selection payload (task 3.5) — resolved against
                // the live PendingSelection at step time.
                Row::Selection(s) => steps.push(StepSpec {
                    actor: s.actor,
                    action_id: 0,
                    phase: s.phase.clone(),
                    source: "selection".to_string(),
                    memory_after: None,
                    turn: None,
                    is_game_over: None,
                    expected_digest: None,
                    selection: Some(s.clone()),
                }),
                // Can't fabricate an unencoded selection — the replayable
                // prefix ends here (matches the batch harness's PartialPass).
                Row::EncoderFailure(_) => break,
                _ => {}
            }
        }

        let (opp_decklist, reveal_pairs) = if opp_deck.is_none() {
            (
                Some(derive_opp_decklist(&recording)?),
                collect_reveal_pairs(&recording.rows)?,
            )
        } else {
            (None, Vec::new())
        };

        Ok(Self {
            my_pid,
            first_player,
            my_deck,
            opp_deck,
            opp_decklist,
            reveal_pairs,
            steps,
            card_data: card_data.clone(),
        })
    }
}

impl RecordingSource for DcgoAdapter {
    fn build_initial_game(
        &self,
        _card_data: &HashMap<String, CardData>,
    ) -> Result<Game, ReplayError> {
        match &self.opp_deck {
            Some(opp_deck) => {
                // Bot-vs-bot — both decks known. Ordered construction: the
                // recorded post-shuffle order is taken verbatim (`Game::new`
                // would re-shuffle it), and the first player comes from the
                // recording rather than seed parity. Seed 0 still drives
                // card-internal random effects.
                let (deck1, deck2) = if self.my_pid == 0 {
                    (self.my_deck.clone(), opp_deck.clone())
                } else {
                    (opp_deck.clone(), self.my_deck.clone())
                };
                Game::new_with_ordered_decks(
                    &[deck1, deck2],
                    &self.card_data,
                    Rules::standard(),
                    Some(0),
                    self.first_player,
                )
                .map_err(ReplayError::GameConstruction)
            }
            None => {
                // Opaque PvP — fresh RevealQueue at cursor 0 so a rebuild
                // (incl. backward-seek) consumes the same reveals in order.
                let opp_decklist = self.opp_decklist.clone().ok_or_else(|| {
                    ReplayError::MalformedRecording(
                        "opaque recording missing opponent decklist".into(),
                    )
                })?;
                let queue = crate::opaque_deck::RevealQueue::from_pairs(self.reveal_pairs.clone());
                Game::new_with_opaque_opponent(
                    self.my_pid,
                    self.my_deck.clone(),
                    opp_decklist,
                    Box::new(queue),
                    &self.card_data,
                    Rules::standard(),
                    Some(0),
                )
                .map_err(ReplayError::GameConstruction)
            }
        }
    }

    fn relay_initial_state(&self, game: &mut Game) -> Result<(), ReplayError> {
        // DCGO games are reconstructed rather than restored zone-by-zone:
        // `Game::new` shuffles from the recorded post-shuffle order (seed 0),
        // and the opaque path must re-attach a fresh `RevealQueue` at cursor 0.
        // Backward seek therefore rebuilds — deterministic and correct, but not
        // as cheap as the native zone-restore. (The batch parity replay never
        // seeks backward, so this only costs interactive DCGO back-stepping; a
        // future Game-setup extraction could make it in-place.)
        *game = self.build_initial_game(&self.card_data)?;
        Ok(())
    }

    fn steps(&self) -> &[StepSpec] {
        &self.steps
    }

    fn default_policy(&self) -> StepPolicy {
        StepPolicy::CheckThenApply
    }
}

/// [`RecordingSource`] over the compact verification-ladder replay format.
/// This source replays from `(seed, inline deck refs, actions)` rather than a
/// post-mulligan zone snapshot, so mulligan actions remain part of the stream.
pub struct VerificationReplayAdapter {
    recording: VerificationReplayRecording,
    deck1: Vec<String>,
    deck2: Vec<String>,
    steps: Vec<StepSpec>,
    card_data: HashMap<String, CardData>,
}

impl VerificationReplayAdapter {
    pub fn from_recording(
        recording: VerificationReplayRecording,
        card_data: &HashMap<String, CardData>,
    ) -> Result<Self, ReplayError> {
        if recording.actions.len() != recording.digests.len() {
            return Err(ReplayError::MalformedRecording(format!(
                "verification replay has {} actions but {} digests",
                recording.actions.len(),
                recording.digests.len()
            )));
        }

        let deck1 = resolve_inline_deck_ref("player1", &recording.deck_refs.player1)?;
        let deck2 = resolve_inline_deck_ref("player2", &recording.deck_refs.player2)?;
        validate_deck_cards(&deck1, card_data)?;
        validate_deck_cards(&deck2, card_data)?;

        let steps = recording
            .actions
            .iter()
            .zip(recording.digests.iter())
            .map(|(action, digest)| StepSpec {
                actor: action.seat,
                action_id: action.action_id,
                phase: String::new(),
                source: "verification_replay".to_string(),
                memory_after: None,
                turn: None,
                is_game_over: None,
                expected_digest: Some(*digest),
                selection: None,
            })
            .collect();

        Ok(Self {
            recording,
            deck1,
            deck2,
            steps,
            card_data: card_data.clone(),
        })
    }
}

impl RecordingSource for VerificationReplayAdapter {
    fn build_initial_game(
        &self,
        _card_data: &HashMap<String, CardData>,
    ) -> Result<Game, ReplayError> {
        let mut game = Game::new(
            &[self.deck1.clone(), self.deck2.clone()],
            &self.card_data,
            Rules::standard(),
            self.recording.seed,
        )
        .map_err(ReplayError::GameConstruction)?;
        // HeadlessRunner records from the RL surface, which exposes turn 1
        // during mulligan. Mirror that construction contract so the digest
        // stream is replayed from the same pre-action state.
        game.turn_count = 1;
        Ok(game)
    }

    fn relay_initial_state(&self, game: &mut Game) -> Result<(), ReplayError> {
        *game = self.build_initial_game(&self.card_data)?;
        Ok(())
    }

    fn steps(&self) -> &[StepSpec] {
        &self.steps
    }

    fn default_policy(&self) -> StepPolicy {
        StepPolicy::CheckThenApply
    }
}

fn resolve_inline_deck_ref(
    label: &str,
    deck_ref: &ReplayDeckRef,
) -> Result<Vec<String>, ReplayError> {
    match deck_ref {
        ReplayDeckRef::Inline { cards } => Ok(cards.clone()),
        ReplayDeckRef::Decklist { id } => Err(ReplayError::MalformedRecording(format!(
            "{label} deck ref `{id}` requires an external decklist resolver; inline deck refs are supported by the core runner"
        ))),
    }
}

fn validate_deck_cards(
    deck: &[String],
    card_data: &HashMap<String, CardData>,
) -> Result<(), ReplayError> {
    let mut missing: Vec<String> = Vec::new();
    for card_id in deck {
        if !card_data.contains_key(card_id) && !missing.iter().any(|id| id == card_id) {
            missing.push(card_id.clone());
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(ReplayError::UnknownCard(missing))
    }
}

/// Walk a DCGO row stream collecting reveal rows in order, mapping each
/// `source` tag to a [`RevealKind`](crate::opaque_deck::RevealKind). Errors
/// on an unrecognized tag. Mirrors the dcgo-replay batch helper.
fn collect_reveal_pairs(
    rows: &[crate::dcgo_recording::Row],
) -> Result<Vec<(crate::opaque_deck::RevealKind, String)>, ReplayError> {
    use crate::dcgo_recording::Row;
    use crate::opaque_deck::RevealKind;
    let mut out = Vec::new();
    for row in rows {
        if let Row::Reveal(rv) = row {
            let kind = match rv.source.as_str() {
                "draw" => RevealKind::Draw,
                "security" => RevealKind::Security,
                "mill" => RevealKind::Mill,
                "effect" => RevealKind::Effect,
                other => {
                    return Err(ReplayError::MalformedRecording(format!(
                        "reveal row at step {} has unknown source `{}` \
                         (expected draw|security|mill|effect)",
                        rv.step, other
                    )));
                }
            };
            out.push((kind, rv.card_id.clone()));
        }
    }
    Ok(out)
}

/// Determine the opaque opponent's decklist: prefer the explicit
/// `opp_decklist_composition` header, else fall back to the reveal stream
/// (requires at least `deck_size` reveals). Mirrors the dcgo-replay helper.
fn derive_opp_decklist(
    recording: &crate::dcgo_recording::RecordingV1,
) -> Result<Vec<String>, ReplayError> {
    use crate::dcgo_recording::Row;
    let expected_size = recording.start.my_deck_post_shuffle.len();

    if let Some(comp) = &recording.start.opp_decklist_composition {
        if comp.len() != expected_size {
            return Err(ReplayError::MalformedRecording(format!(
                "opp_decklist_composition has {} cards but my_deck_post_shuffle has {}",
                comp.len(),
                expected_size
            )));
        }
        return Ok(comp.clone());
    }

    let reveal_cards: Vec<String> = recording
        .rows
        .iter()
        .filter_map(|r| match r {
            Row::Reveal(rv) => Some(rv.card_id.clone()),
            _ => None,
        })
        .collect();
    if reveal_cards.len() < expected_size {
        return Err(ReplayError::MalformedRecording(format!(
            "opaque recording is missing an explicit `opp_decklist_composition` and its \
             reveal stream has only {} entries (fewer than the deck size of {})",
            reveal_cards.len(),
            expected_size
        )));
    }
    Ok(reveal_cards.into_iter().take(expected_size).collect())
}

/// A scripted, steppable game replay driven by a pluggable
/// [`RecordingSource`]. Owns the live `Game` and a step cursor. Backward
/// seek uses reset-and-replay (`Game::reset_for_replay` + the source's
/// `relay_initial_state`) — no state snapshot, no `Game::new` rebuild.
///
/// This is the unified core the MCP stepping tools (Group 7) and the DCGO
/// differential path (Group 4) build on. `ReplayRunner` above is the
/// historical native-only type and will collapse onto this once its other
/// callers (CLI, integration tests) migrate.
pub struct ReplaySession {
    pub game: Game,
    driver: ReplayDriver,
}

/// Game-agnostic replay stepping state: the recording source, cursor,
/// policy, divergence log, and pause flag. Every stepping operation takes
/// the `&mut Game` it should drive, so the same driver powers both
/// [`ReplaySession`] (which owns its `Game`) and `LiveGame` (whose `game`
/// field is the canonical one the MCP view tools read). Extracted so the
/// two surfaces share one verified implementation rather than drifting.
pub(crate) struct ReplayDriver {
    source: Box<dyn RecordingSource>,
    cursor: u32,
    verify: bool,
    policy: StepPolicy,
    /// Differential divergences recorded under `CheckThenApply`. Accumulates
    /// across forward stepping; cleared on a backward reset.
    divergences: Vec<Divergence>,
    /// Set when a pausing `CheckThenApply` divergence (`MaskMiss` / `Actor`)
    /// halts the cursor. `step()` is a no-op while paused; cleared by a
    /// backward `seek` (reset) or `set_policy`.
    paused: bool,
}

impl ReplayDriver {
    /// Build the initial `Game` from `source` and pair it with a fresh
    /// driver. The caller owns the returned `Game`; the driver steps it.
    pub(crate) fn from_source(
        source: Box<dyn RecordingSource>,
        all_card_data: &HashMap<String, CardData>,
        verify: bool,
    ) -> Result<(Game, Self), ReplayError> {
        let policy = source.default_policy();
        let game = source.build_initial_game(all_card_data)?;
        Ok((
            game,
            Self {
                source,
                cursor: 0,
                verify,
                policy,
                divergences: Vec::new(),
                paused: false,
            },
        ))
    }

    pub(crate) fn policy(&self) -> StepPolicy {
        self.policy
    }

    pub(crate) fn set_policy(&mut self, policy: StepPolicy) {
        self.policy = policy;
        self.paused = false;
    }

    pub(crate) fn divergences(&self) -> &[Divergence] {
        &self.divergences
    }

    pub(crate) fn is_paused(&self) -> bool {
        self.paused
    }

    pub(crate) fn total_steps(&self) -> u32 {
        self.source.steps().len() as u32
    }

    pub(crate) fn cursor(&self) -> u32 {
        self.cursor
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.cursor as usize >= self.source.steps().len()
    }

    /// The recorded step at the current cursor — the one `step` would apply
    /// next. `None` once the cursor has reached the end.
    pub(crate) fn current_spec(&self) -> Option<&StepSpec> {
        self.source.steps().get(self.cursor as usize)
    }

    /// A no-progress step result reflecting `game`'s current (unchanged)
    /// state — returned when complete or paused.
    fn no_progress_result(&self, game: &Game) -> ReplayStepResult {
        ReplayStepResult {
            step_number: self.cursor,
            player_id: 0,
            action_id: 0,
            phase_before: game.current_phase.py_name().to_string(),
            phase_after: game.current_phase.py_name().to_string(),
            memory_before: game.memory,
            memory_after: game.memory,
            turn_number: game.turn_count,
            is_game_over: game.game_over,
            winner_id: game.winner,
            divergences: Vec::new(),
        }
    }

    /// Apply one recorded SELECTION step (task 3.5): resolve the semantic
    /// payload against the live `PendingSelection` pick-by-pick, applying
    /// each resolved engine action. Advances the cursor once for the whole
    /// payload. Resolution failure records a
    /// [`DivergenceKind::SelectionResolution`] and pauses.
    fn apply_selection_step(
        &mut self,
        game: &mut Game,
        payload: &crate::dcgo_recording::SelectionRow,
    ) -> ReplayStepResult {
        use super::selection_resolve::resolve_next;

        let phase_before = game.current_phase.py_name().to_string();
        let memory_before = game.memory;
        let turn_number = game.turn_count;
        let actor = payload.actor;

        let mut picks_done = 0usize;
        let mut last_action: u16 = 0;
        loop {
            match resolve_next(game, payload, picks_done) {
                Ok(None) => break,
                Ok(Some(id)) => {
                    // Selection answers must come from the recorded actor and
                    // be accepted by the parked prompt (or PASS). The
                    // resolver only emits ids from valid_action_ids/PASS, so
                    // a decode no-op here means engine-side disagreement.
                    game.decode_action(id, actor);
                    last_action = id;
                    picks_done += 1;
                    // Safety valve: a payload can't sanely resolve into more
                    // engine actions than picks + trailing PASS.
                    if picks_done > super::selection_resolve::payload_pick_count(payload) + 1 {
                        break;
                    }
                }
                Err(reason) => {
                    self.divergences.push(Divergence {
                        step: self.cursor,
                        action_id: last_action,
                        actor,
                        phase: payload.phase.clone(),
                        kind: DivergenceKind::SelectionResolution { reason },
                    });
                    self.paused = true;
                    return self.no_progress_result(game);
                }
            }
        }

        self.cursor += 1;
        ReplayStepResult {
            step_number: self.cursor,
            player_id: actor,
            action_id: last_action,
            phase_before,
            phase_after: game.current_phase.py_name().to_string(),
            memory_before,
            memory_after: game.memory,
            turn_number,
            is_game_over: game.game_over,
            winner_id: game.winner,
            divergences: Vec::new(),
        }
    }

    /// Apply exactly one replayable step against `game`. No-op once complete
    /// or paused. Under `CheckThenApply`, verifies actor + mask-membership
    /// first. `CONCEDE_GAME` is accepted as the decoder-supported surrender
    /// primitive even though RL masks intentionally hide it. A mismatch
    /// records a [`Divergence`] and pauses without applying or aborting.
    pub(crate) fn step(&mut self, game: &mut Game) -> ReplayStepResult {
        if self.is_complete() || self.paused {
            return self.no_progress_result(game);
        }

        let cur = self.cursor as usize;

        // Semantic selection step (task 3.5): resolve the DCGO payload
        // against the live PendingSelection, applying one engine action per
        // pick (plus an optional trailing PASS). Handled as its own path —
        // one recorded step may consume several engine actions.
        if let Some(payload) = self.source.steps()[cur].selection.clone() {
            return self.apply_selection_step(game, &payload);
        }

        let (action_id, actor, phase_rec, mem_after_rec, turn_rec, game_over_rec, expected_digest) = {
            let s = &self.source.steps()[cur];
            (
                s.action_id,
                s.actor,
                s.phase.clone(),
                s.memory_after,
                s.turn,
                s.is_game_over,
                s.expected_digest,
            )
        };

        if self.policy == StepPolicy::CheckThenApply {
            // Differential pre-check: does the engine agree the recorded actor
            // is on decision, and is the recorded action legal here?
            let expected = current_decision_player(game);
            if expected != actor {
                self.divergences.push(Divergence {
                    step: self.cursor,
                    action_id,
                    actor,
                    phase: phase_rec.clone(),
                    kind: DivergenceKind::Actor {
                        expected,
                        recorded: actor,
                    },
                });
                self.paused = true;
                return self.no_progress_result(game);
            }
            let mask = crate::action::build_action_mask(game, actor);
            let legal = action_id == crate::action::space::CONCEDE_GAME
                || mask.get(action_id as usize).copied().unwrap_or(0.0) > 0.5;
            if !legal {
                let sample = sample_legal_ids(&mask, 10);
                self.divergences.push(Divergence {
                    step: self.cursor,
                    action_id,
                    actor,
                    phase: phase_rec.clone(),
                    kind: DivergenceKind::MaskMiss {
                        sample_legal_ids: sample,
                    },
                });
                self.paused = true;
                return self.no_progress_result(game);
            }
        }

        let phase_before = game.current_phase.py_name().to_string();
        let memory_before = game.memory;
        let turn_number = game.turn_count;

        game.decode_action(action_id, actor);
        self.cursor += 1;

        let phase_after = game.current_phase.py_name().to_string();

        let mut result = ReplayStepResult {
            step_number: self.cursor,
            player_id: actor,
            action_id,
            phase_before: phase_before.clone(),
            phase_after,
            memory_before,
            memory_after: game.memory,
            turn_number,
            is_game_over: game.game_over,
            winner_id: game.winner,
            divergences: Vec::new(),
        };

        if self.verify {
            if let Some(rec) = mem_after_rec {
                if rec != result.memory_after as i64 {
                    result.divergences.push(DivergenceReport {
                        step: result.step_number,
                        field: "memory_after",
                        recorded: rec.to_string(),
                        replayed: result.memory_after.to_string(),
                    });
                }
            }
            if let Some(rec) = turn_rec {
                if rec != result.turn_number as u64 {
                    result.divergences.push(DivergenceReport {
                        step: result.step_number,
                        field: "turn",
                        recorded: rec.to_string(),
                        replayed: result.turn_number.to_string(),
                    });
                }
            }
            if !phase_rec.is_empty() && phase_rec != phase_before {
                result.divergences.push(DivergenceReport {
                    step: result.step_number,
                    field: "phase",
                    recorded: phase_rec,
                    replayed: phase_before,
                });
            }
            if let Some(rec) = game_over_rec {
                if rec != result.is_game_over {
                    result.divergences.push(DivergenceReport {
                        step: result.step_number,
                        field: "is_game_over",
                        recorded: rec.to_string(),
                        replayed: result.is_game_over.to_string(),
                    });
                }
            }
            if let Some(recorded) = expected_digest {
                let replayed = game.verification_digest();
                if recorded != replayed {
                    result.divergences.push(DivergenceReport {
                        step: result.step_number,
                        field: "verification_digest",
                        recorded: recorded.to_string(),
                        replayed: replayed.to_string(),
                    });
                    self.paused = true;
                }
            }
        }

        result
    }

    /// Seek the cursor to `target_step`, driving `game`. Forward replays
    /// missing steps; backward resets `game` in place (reusing `card_data`
    /// + registries — no clone, no rebuild) and replays forward. Clamps to
    /// `[0, total]`.
    pub(crate) fn seek(&mut self, game: &mut Game, target_step: u32) -> Result<(), ReplayError> {
        let total = self.total_steps();
        let target = target_step.min(total);
        if target < self.cursor {
            game.reset_for_replay();
            self.source.relay_initial_state(game)?;
            self.cursor = 0;
            self.divergences.clear();
            self.paused = false;
        }
        while self.cursor < target && !game.game_over && !self.paused {
            self.step(game);
        }
        Ok(())
    }

    /// Move the cursor back by one step (reset-and-replay).
    pub(crate) fn step_back(&mut self, game: &mut Game) -> Result<(), ReplayError> {
        if self.cursor == 0 {
            return Ok(());
        }
        self.seek(game, self.cursor - 1)
    }

    /// Restore `game` to `step_n` (alias for [`seek`](Self::seek), matching
    /// the MCP `restore_checkpoint` surface).
    pub(crate) fn restore(&mut self, game: &mut Game, step_n: u32) -> Result<(), ReplayError> {
        self.seek(game, step_n)
    }

    pub(crate) fn run_to_completion(&mut self, game: &mut Game) {
        while !self.is_complete() && !game.game_over && !self.paused {
            self.step(game);
        }
    }
}

impl ReplaySession {
    /// Construct from a native `GameRecorder` JSON recording.
    pub fn new(
        recording: Value,
        all_card_data: &HashMap<String, CardData>,
        verify: bool,
    ) -> Result<Self, ReplayError> {
        let adapter = NativeAdapter::from_recording(recording, all_card_data)?;
        Self::with_source(Box::new(adapter), all_card_data, verify)
    }

    /// Construct from a parsed DCGO `RecordingV1` (bot-vs-bot or opaque PvP).
    /// Defaults to `CheckThenApply` (differential against the DCGO oracle).
    pub fn from_dcgo(
        recording: crate::dcgo_recording::RecordingV1,
        all_card_data: &HashMap<String, CardData>,
        verify: bool,
    ) -> Result<Self, ReplayError> {
        let adapter = DcgoAdapter::from_recording(recording, all_card_data)?;
        Self::with_source(Box::new(adapter), all_card_data, verify)
    }

    /// Construct from a compact verification-ladder replay recording.
    pub fn from_verification_recording(
        recording: VerificationReplayRecording,
        all_card_data: &HashMap<String, CardData>,
        verify: bool,
    ) -> Result<Self, ReplayError> {
        let adapter = VerificationReplayAdapter::from_recording(recording, all_card_data)?;
        Self::with_source(Box::new(adapter), all_card_data, verify)
    }

    /// Replay a verification recording with actor/mask pre-checks and digest
    /// comparisons, returning only the first divergence for the game.
    pub fn verify_verification_recording<S: Into<String>>(
        game_id: S,
        recording: VerificationReplayRecording,
        all_card_data: &HashMap<String, CardData>,
    ) -> Result<VerificationReplayReport, ReplayError> {
        let game = game_id.into();
        let mut session = Self::from_verification_recording(recording, all_card_data, true)?;

        while !session.is_complete() && !session.is_game_over() && !session.is_paused() {
            let result = session.step();
            if let Some(divergence) = session.divergences().first() {
                return Ok(report_from_differential_divergence(
                    &game,
                    session.current_step(),
                    divergence,
                ));
            }
            if let Some(divergence) = result
                .divergences
                .iter()
                .find(|d| d.field == "verification_digest")
                .or_else(|| result.divergences.first())
            {
                return Ok(report_from_step_divergence(
                    &game,
                    session.current_step(),
                    divergence,
                ));
            }
        }

        Ok(VerificationReplayReport {
            game,
            checked_steps: session.current_step(),
            divergence: None,
        })
    }

    /// Construct from any [`RecordingSource`] (native, DCGO, …).
    pub fn with_source(
        source: Box<dyn RecordingSource>,
        all_card_data: &HashMap<String, CardData>,
        verify: bool,
    ) -> Result<Self, ReplayError> {
        let (game, driver) = ReplayDriver::from_source(source, all_card_data, verify)?;
        Ok(Self { game, driver })
    }

    /// The active step policy.
    pub fn policy(&self) -> StepPolicy {
        self.driver.policy()
    }

    /// Override the step policy (e.g. force `CheckThenApply` to scan a native
    /// recording, or `Trust` to step past a recorded divergence). Clears the
    /// paused flag so stepping can resume under the new policy.
    pub fn set_policy(&mut self, policy: StepPolicy) {
        self.driver.set_policy(policy);
    }

    /// Differential divergences recorded so far (under `CheckThenApply`).
    pub fn divergences(&self) -> &[Divergence] {
        self.driver.divergences()
    }

    /// True when a pausing divergence has halted the cursor.
    pub fn is_paused(&self) -> bool {
        self.driver.is_paused()
    }

    pub fn total_steps(&self) -> u32 {
        self.driver.total_steps()
    }

    pub fn current_step(&self) -> u32 {
        self.driver.cursor()
    }

    pub fn is_complete(&self) -> bool {
        self.driver.is_complete()
    }

    pub fn is_game_over(&self) -> bool {
        self.game.game_over
    }

    pub fn winner_id(&self) -> Option<PlayerId> {
        self.game.winner
    }

    /// Apply exactly one replayable step. No-op once complete or paused.
    pub fn step(&mut self) -> ReplayStepResult {
        self.driver.step(&mut self.game)
    }

    /// Seek the cursor to `target_step` (reset-and-replay on backward seek).
    pub fn seek(&mut self, target_step: u32) -> Result<(), ReplayError> {
        self.driver.seek(&mut self.game, target_step)
    }

    /// Move the cursor back by one step (reset-and-replay).
    pub fn step_back(&mut self) -> Result<(), ReplayError> {
        self.driver.step_back(&mut self.game)
    }

    /// Restore to `step_n` (alias for [`seek`](Self::seek)).
    pub fn restore(&mut self, step_n: u32) -> Result<(), ReplayError> {
        self.driver.restore(&mut self.game, step_n)
    }

    pub fn run_to_completion(&mut self) {
        self.driver.run_to_completion(&mut self.game)
    }
}

fn report_from_differential_divergence(
    game: &str,
    checked_steps: u32,
    divergence: &Divergence,
) -> VerificationReplayReport {
    let (check_type, recorded, replayed) = match &divergence.kind {
        DivergenceKind::Actor { expected, recorded } => (
            VerificationReplayCheck::Actor,
            format!("actor {}", recorded),
            format!("expected actor {}", expected),
        ),
        DivergenceKind::MaskMiss { sample_legal_ids } => (
            VerificationReplayCheck::Legality,
            format!("action_id {}", divergence.action_id),
            format!("legal action sample {:?}", sample_legal_ids),
        ),
        other => (
            VerificationReplayCheck::Legality,
            format!("{:?}", other),
            "differential replay divergence".to_string(),
        ),
    };

    VerificationReplayReport {
        game: game.to_string(),
        checked_steps,
        divergence: Some(VerificationReplayDivergence {
            game: game.to_string(),
            step: divergence.step,
            check_type,
            recorded,
            replayed,
        }),
    }
}

fn report_from_step_divergence(
    game: &str,
    checked_steps: u32,
    divergence: &DivergenceReport,
) -> VerificationReplayReport {
    let check_type = if divergence.field == "verification_digest" {
        VerificationReplayCheck::Digest
    } else {
        VerificationReplayCheck::Digest
    };
    VerificationReplayReport {
        game: game.to_string(),
        checked_steps,
        divergence: Some(VerificationReplayDivergence {
            game: game.to_string(),
            step: divergence.step,
            check_type,
            recorded: divergence.recorded.clone(),
            replayed: divergence.replayed.clone(),
        }),
    }
}

/// The player the engine expects to make the next decision: a pending
/// mulligan, else a pending selection's owner, else the turn player. Mirrors
/// `LiveGame::current_decision_player` and the dcgo-replay harness helper.
fn current_decision_player(game: &Game) -> PlayerId {
    if let Some(p) = game.mulligan_current_player() {
        return p;
    }
    if let Some(sel) = game.pending_selection.as_ref() {
        return sel.selecting_player;
    }
    game.turn_player()
}

/// Up to `max` action IDs the engine WOULD accept here (`mask` value > 0.5).
/// Surfaced in `MaskMiss` divergences to help triage "wrong target index"
/// vs "engine refuses a legal action".
fn sample_legal_ids(mask: &[f32], max: usize) -> Vec<u16> {
    let mut out = Vec::with_capacity(max);
    for (i, &v) in mask.iter().enumerate() {
        if v > 0.5 {
            out.push(i as u16);
            if out.len() >= max {
                break;
            }
        }
    }
    out
}

// ── helpers ───────────────────────────────────────────────────────────────

fn build_empty_game(all_card_data: &HashMap<String, CardData>) -> Result<Game, ReplayError> {
    let empty_decks: Vec<Vec<String>> = vec![Vec::new(), Vec::new()];
    Game::new(&empty_decks, all_card_data, Rules::standard(), Some(0))
        .map_err(ReplayError::GameConstruction)
}

fn restore_player_zone(
    player: &mut Player,
    data: &Value,
    data_index_map: &HashMap<&str, usize>,
    owner: PlayerId,
    next_card_index: &mut u16,
) -> Result<(), ReplayError> {
    let push_zone = |zone: &mut Vec<CardSource>,
                     arr: &Value,
                     next_card_index: &mut u16|
     -> Result<(), ReplayError> {
        let arr = arr.as_array().cloned().unwrap_or_default();
        for v in arr {
            let card_id = v.as_str().ok_or_else(|| {
                ReplayError::MalformedRecording("non-string entry in card-id array".into())
            })?;
            let data_idx = *data_index_map
                .get(card_id)
                .ok_or_else(|| ReplayError::UnknownCard(vec![card_id.to_string()]))?;
            let cs = CardSource::new(data_idx, owner, *next_card_index);
            *next_card_index += 1;
            zone.push(cs);
        }
        Ok(())
    };

    push_zone(&mut player.deck, &data["library_order"], next_card_index)?;
    push_zone(
        &mut player.digitama_deck,
        &data["digitama_library_order"],
        next_card_index,
    )?;
    push_zone(
        &mut player.security,
        &data["security_order"],
        next_card_index,
    )?;
    push_zone(&mut player.hand, &data["initial_hand"], next_card_index)?;

    // Repopulate `original_deck` from the union of the four zones, mirroring
    // what `Game::new` does. The `standard_lite_deck_v2` observation profile
    // reads this — without it, the own-original-decklist section is empty
    // for any tensor built during replay.
    rebuild_original_deck(player, data_index_map);
    Ok(())
}

/// Aggregate every card in this player's deck/digitama/security/hand zones
/// into the `original_deck` ledger. Mirrors the logic in `Game::new` that
/// runs immediately before the post-shuffle library is laid down.
fn rebuild_original_deck(player: &mut Player, data_index_map: &HashMap<&str, usize>) {
    use crate::player::OriginalDeckCardCount;
    use std::collections::BTreeMap;

    // Build a reverse index once: data_index → card_id. The map currently
    // borrows `&str`, so collect into a `Vec<Option<&str>>` keyed by index.
    let max_idx = data_index_map.values().copied().max().unwrap_or(0);
    let mut reverse: Vec<Option<&str>> = vec![None; max_idx + 1];
    for (card_id, idx) in data_index_map.iter() {
        reverse[*idx] = Some(*card_id);
    }

    let mut counts: BTreeMap<(String, bool), u16> = BTreeMap::new();
    let mut bump = |idx: usize, is_digitama: bool| {
        if let Some(card_id) = reverse.get(idx).and_then(|c| *c) {
            *counts
                .entry((card_id.to_string(), is_digitama))
                .or_insert(0) += 1;
        }
    };

    // Digitama zone is tautologically eggs.
    for cs in player.digitama_deck.iter() {
        bump(cs.data_index, true);
    }
    // Every other zone holds main-deck cards. (Eggs are legal only in
    // `digitama_deck` by deck-construction rules.)
    for cs in player
        .deck
        .iter()
        .chain(player.security.iter())
        .chain(player.hand.iter())
    {
        bump(cs.data_index, false);
    }

    player.original_deck = counts
        .into_iter()
        .map(|((card_id, is_digitama), count)| OriginalDeckCardCount {
            card_id,
            count,
            is_digitama,
        })
        .collect();
}

// ── tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card_data::CardData;
    use serde_json::json;

    fn filler_card_data() -> HashMap<String, CardData> {
        // Build via JSON to avoid coupling test code to every CardData field
        // (the struct evolves frequently as new mechanics land).
        let mut m = HashMap::new();
        for (id, kind) in &[
            ("FILLER-001", "Digimon"),
            ("FILLER-002", "Digimon"),
            ("FILLER-EGG", "DigiEgg"),
        ] {
            let cd: CardData = serde_json::from_value(json!({
                "card_id": id,
                "card_name": id,
                "card_kind": kind,
                "level": 3,
                "dp": 1000,
                "play_cost": 1,
                "colors": ["Red"],
                "traits": [],
                "evo_costs": [],
                "effect_text": "",
                "inherited_text": "",
                "security_text": "",
                "effect_class_name": "",
            }))
            .expect("filler CardData deserializes");
            m.insert(id.to_string(), cd);
        }
        m
    }

    fn minimal_recording() -> Value {
        // Hand-crafted recording: post-mulligan state with empty action log.
        // Used to verify construction + initial-state restoration without
        // depending on a full game's recording machinery.
        let library_p1: Vec<&str> = vec!["FILLER-001"; 40];
        let library_p2: Vec<&str> = vec!["FILLER-002"; 40];
        let egg: Vec<&str> = vec!["FILLER-EGG"; 4];
        let security_p1: Vec<&str> = vec!["FILLER-001"; 5];
        let security_p2: Vec<&str> = vec!["FILLER-002"; 5];
        let hand_p1: Vec<&str> = vec!["FILLER-001"; 5];
        let hand_p2: Vec<&str> = vec!["FILLER-002"; 5];
        json!({
            "initial_state": {
                "first_player_id": 1,
                "timestamp": "2026-01-01T00:00:00+00:00",
                "player1": {
                    "player_id": 1,
                    "deck_list": [],
                    "library_order": library_p1,
                    "digitama_library_order": egg.clone(),
                    "security_order": security_p1,
                    "initial_hand": hand_p1,
                },
                "player2": {
                    "player_id": 2,
                    "deck_list": [],
                    "library_order": library_p2,
                    "digitama_library_order": egg,
                    "security_order": security_p2,
                    "initial_hand": hand_p2,
                },
            },
            "actions": [],
            "total_actions": 0,
        })
    }

    #[test]
    fn construct_from_minimal_recording() {
        let cd = filler_card_data();
        let rec = minimal_recording();
        let r = ReplayRunner::new(rec, &cd, false).expect("constructs");
        assert_eq!(r.current_step(), 0);
        assert_eq!(r.total_steps(), 0);
        assert!(r.is_complete());
        // First player is recorded as player_id=1 (Python convention), so Rust 0.
        assert_eq!(r.game.turn_player(), 0);
        // Libraries restored to the recorded ordering.
        assert_eq!(r.game.players[0].deck.len(), 40);
        assert_eq!(r.game.players[1].deck.len(), 40);
        assert_eq!(r.game.players[0].security.len(), 5);
        assert_eq!(r.game.players[0].hand.len(), 5);
        assert_eq!(r.game.players[0].digitama_deck.len(), 4);
    }

    #[test]
    fn missing_initial_state_errors() {
        let cd = filler_card_data();
        let rec = json!({ "actions": [] });
        match ReplayRunner::new(rec, &cd, false) {
            Err(ReplayError::MissingInitialState) => {}
            other => panic!("expected MissingInitialState, got {:?}", other.err()),
        }
    }

    #[test]
    fn null_initial_state_errors() {
        let cd = filler_card_data();
        let rec = json!({ "initial_state": null, "actions": [] });
        assert!(matches!(
            ReplayRunner::new(rec, &cd, false),
            Err(ReplayError::MissingInitialState)
        ));
    }

    #[test]
    fn unknown_card_errors_with_offender_listed() {
        let cd = filler_card_data();
        let mut rec = minimal_recording();
        rec["initial_state"]["player1"]["library_order"][0] = json!("UNKNOWN-999");
        match ReplayRunner::new(rec, &cd, false) {
            Err(ReplayError::UnknownCard(ids)) => assert!(ids.contains(&"UNKNOWN-999".to_string())),
            other => panic!("expected UnknownCard, got {:?}", other.err()),
        }
    }

    #[test]
    fn step_past_completion_is_no_op() {
        let cd = filler_card_data();
        let r = ReplayRunner::new(minimal_recording(), &cd, false).expect("constructs");
        let mut r = r;
        // 0 replayable actions; calling step() once should not panic and
        // should not advance.
        let before = r.current_step();
        let _ = r.step();
        assert_eq!(r.current_step(), before);
    }

    #[test]
    fn seek_to_zero_rebuilds_from_initial() {
        let cd = filler_card_data();
        let mut r = ReplayRunner::new(minimal_recording(), &cd, false).expect("constructs");
        r.seek(0).expect("seek 0");
        assert_eq!(r.current_step(), 0);
        // After rebuild, zones still match the recording.
        assert_eq!(r.game.players[0].deck.len(), 40);
    }

    #[test]
    fn backward_seek_resets_in_place_via_reset_for_replay() {
        let cd = filler_card_data();
        let mut r = ReplayRunner::new(minimal_recording(), &cd, false).expect("constructs");
        // Force a backward seek to exercise the reset-and-replay path
        // (`Game::reset_for_replay` + `relay_initial_state`) rather than a
        // `Game::new` rebuild. With a 0-action recording the forward re-walk
        // is a no-op; the point is that reset + relay run cleanly and restore
        // the initial zones.
        r.current_step = 5;
        r.seek(0).expect("backward seek resets and relays");
        assert_eq!(r.current_step(), 0);
        assert_eq!(r.game.players[0].deck.len(), 40);
        assert_eq!(r.game.players[0].security.len(), 5);
        assert_eq!(r.game.players[0].hand.len(), 5);
        assert_eq!(r.game.players[0].digitama_deck.len(), 4);
    }

    #[test]
    fn run_to_completion_no_actions_does_nothing() {
        let cd = filler_card_data();
        let mut r = ReplayRunner::new(minimal_recording(), &cd, false).expect("constructs");
        r.run_to_completion();
        assert!(r.is_complete());
        assert_eq!(r.current_step(), 0);
    }

    #[test]
    fn divergence_report_serializes() {
        let d = DivergenceReport {
            step: 7,
            field: "memory_after",
            recorded: "3".to_string(),
            replayed: "4".to_string(),
        };
        let json = serde_json::to_value(&d).unwrap();
        assert_eq!(json["step"], 7);
        assert_eq!(json["field"], "memory_after");
    }

    #[test]
    fn replay_step_result_serializes() {
        let cd = filler_card_data();
        let r = ReplayRunner::new(minimal_recording(), &cd, false).expect("constructs");
        let result = ReplayStepResult {
            step_number: 1,
            player_id: 0,
            action_id: 0,
            phase_before: "Main".to_string(),
            phase_after: "Main".to_string(),
            memory_before: 0,
            memory_after: 0,
            turn_number: 1,
            is_game_over: false,
            winner_id: None,
            divergences: Vec::new(),
        };
        let _ = serde_json::to_string(&result).unwrap();
        // r is in scope for borrow-checker fragility tests.
        drop(r);
    }
}
