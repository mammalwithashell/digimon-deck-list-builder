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
//! One signal decides whether mulligan-phase actions are filtered out of
//! the replayable step stream or replayed like any other decision: does the
//! recording carry a genuine post-mulligan `initial_state` snapshot? See
//! `should_filter_mulligan_actions`, the single shared decision point both
//! adapters below call into — the branch is driven by that presence check,
//! never by a hard-coded "this source never/always has one" assumption, so
//! a recorder that starts emitting `initial_state` later gets the right
//! behavior without another code change here.
//!
//! - **Native `GameRecorder` recordings carry one.** `GameRecorder` captures
//!   `initial_state` AFTER mulligan completes (security has been dealt,
//!   hands settled). The `actions` array still contains the mulligan
//!   decisions, but replaying them against an already-post-mulligan game
//!   would error, so [`NativeAdapter`] (and the older [`ReplayRunner`],
//!   which requires `initial_state` unconditionally) filter mulligan-phase
//!   actions out of the replayable stream.
//! - **DCGO recordings never carry one.** DCGO's own recorder only ever
//!   logs the keep/redraw *decision* for a mulligan
//!   (`Digimon.Recording.GameRecorder.LogMulligan` in
//!   `TurnStateMachine.SetRedraw`) — it never captures the resulting
//!   redrawn hand, so there is no post-mulligan snapshot to bake the
//!   decisions into. [`DcgoAdapter`] therefore leaves the constructed game
//!   in `Mulligan` phase and replays the recorded mulligan actions like any
//!   other decision. A player who redraws lands on a fresh, real 5-card
//!   hand (the engine performs the actual reshuffle-then-draw DCGO also
//!   performs — general_rule.pdf rule 5-2-1-5) — but since DCGO's own
//!   reshuffle is never recorded, that hand's *exact content* cannot be
//!   expected to match DCGO's, which is the same "RNG path isn't
//!   exercised" limitation called out below, not a defect in this
//!   filtering logic.
//!
//! ## Skipping DCGO plays that never completed
//!
//! DCGO's recorder hooks `TurnStateMachine.QueueMainPhaseAction`, which
//! fires when an action is QUEUED — before DCGO itself decides whether it
//! is legal. So the recorded `action` stream includes attempted plays DCGO
//! went on to reject (e.g. it couldn't actually afford the memory cost).
//! Naively replaying one of those against our engine looks like the engine
//! wrongly refusing a legal action (`MaskMiss` / `illegal_action`), when in
//! fact DCGO refused it too — the worst kind of oracle defect, since it
//! blames the engine for something it did correctly.
//!
//! [`ActionDetailRow`](crate::dcgo_recording::ActionDetailRow) is positive
//! evidence a play actually completed: DCGO's recorder emits one, correlated
//! by `(step, actor)`, only from inside `PlayCardClass.PlayCard()` — i.e.
//! only once the cost was actually paid. `DcgoAdapter` uses its ABSENCE as
//! the (only) skip signal: a recorded `action` row is dropped from the
//! replayable stream when, and only when, it is BOTH
//!
//!   1. in the hand-play range (`PLAY_HAND_START..PLAY_HAND_END`) under
//!      `GamePhase::Main` — see `is_hand_play_action`. The action space is
//!      phase-dependent (`Game::decode_mulligan` vs `decode_main`), so the
//!      same raw ids 0/1 are the mulligan keep/redraw decision under
//!      `GamePhase::Mulligan`, NOT a play — those are never skipped.
//!   2. NOT correlated to any `Row::ActionDetail` in this recording.
//!
//! This is deliberately narrow. `ActionDetailRow` is scoped to player-facing
//! top-level plays only (see its doc comment) — a pass, hatch, attack, or
//! phase action legitimately has no correlated detail row, and skipping one
//! of those would silently drop a real decision instead of an unconfirmed
//! one. Absence of a signal is never treated as evidence of non-completion
//! outside the hand-play range.
//!
//! For the same reason, a recording with NO `action_detail` rows at all
//! (the entire pre-existing corpus, recorded before this diagnostic field
//! existed) must not have every hand-play attempt look "unconfirmed" and
//! get skipped wholesale — `DcgoAdapter` gates the whole mechanism on
//! whether the recording carries at least one `Row::ActionDetail` anywhere;
//! recordings without one replay exactly as they did before this feature.
//!
//! Skips are counted, not silent — see `RecordingSource::skipped_incomplete_plays`
//! / `ReplaySession::skipped_incomplete_plays`. A replay that skipped a
//! meaningful fraction of its actions must not read as a clean, fully-
//! verified pass.
//!
//! ## Known v1 limitations
//!
//! - Games whose start-of-turn triggers install a `pending_selection`
//!   may diverge if the recording is sensitive to the timing of that
//!   selection. The `verify: true` mode reports divergences but does
//!   not halt replay.
//! - Replay does not exercise the RNG path. Effects that consume RNG
//!   (e.g. random reveal selection, or a DCGO mulligan redraw) will diverge
//!   from the recorded outcome since the post-construction RNG state is
//!   fresh.

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use serde_json::Value;

use crate::action::space::{PLAY_HAND_END, PLAY_HAND_START};
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
    /// Replayed memory differs from a DCGO row's recorded `memory` field.
    /// Both `recorded` and `replayed` are already converted to `my_player_id`'s
    /// perspective (see `crate::dcgo_recording::memory_from_recording_perspective`)
    /// so they're directly comparable. Distinct from the native
    /// `GameRecorder`'s `memory_after` check (reported as a `DivergenceReport`
    /// with `field: "memory_after"`, not a `DivergenceKind`) — DCGO's `memory`
    /// uses a different (fixed-player, not active-player) perspective, and
    /// keeping the two apart lets triage cluster memory-gauge drift found via
    /// the DCGO oracle separately from anything else.
    Memory {
        recorded: i64,
        replayed: i64,
        my_player_id: PlayerId,
    },
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
    /// Both players' battle areas as the engine saw them at the moment of
    /// divergence, in `battle_area` index order.
    ///
    /// Board-position action IDs are indices into this list, so triaging any
    /// target mismatch ("engine wanted slot 1, recording said slot 0") is
    /// guesswork without it. Captured eagerly because the divergence pauses
    /// the cursor but callers routinely inspect it after further stepping.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub board: String,
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

        // See module docs "Mulligan handling": `ReplayRunner::new` requires
        // `initial_state` (checked above / by the caller), so mulligan
        // decisions are already baked into it — filter them out of the
        // replayable stream. Driven by the same shared decision point
        // `NativeAdapter` and `DcgoAdapter` use, not a bespoke check here.
        let filter_mulligan = should_filter_mulligan_actions(true);
        self.replayable_action_indices.clear();
        if let Some(actions) = self.recording["actions"].as_array() {
            for (idx, a) in actions.iter().enumerate() {
                let phase = a["phase"].as_str().unwrap_or("");
                if !filter_mulligan || phase != "Mulligan" {
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
    /// DCGO row's `memory` field — already in the recording's
    /// `my_player_id` perspective (positive = favor of that player). NOT
    /// comparable to `memory_after` directly (different perspective
    /// convention); the driver converts `Game::memory` via
    /// `RecordingSource::my_player_id` + `Game::memory_pair` before
    /// comparing. `None` for native/verification recordings (which never
    /// carry this field) and for DCGO rows recorded before the field
    /// existed.
    pub dcgo_memory: Option<i32>,
    pub turn: Option<u64>,
    pub is_game_over: Option<bool>,
    pub expected_digest: Option<u64>,
    /// DCGO semantic selection payload (task 3.5). When set, `action_id` is
    /// a placeholder and the step resolves against the engine's live
    /// `PendingSelection` (possibly into MULTIPLE engine actions — one per
    /// pick plus an optional trailing PASS).
    pub selection: Option<crate::dcgo_recording::SelectionRow>,
    /// DCGO battle-area snapshots at decision time, in the compact order this
    /// step's board operands index. Used to remap those operands into the
    /// engine's play-ordered `battle_area` (see `remap_board_slots`). Empty
    /// for native recordings and pre-0.4 DCGO ones.
    pub board_p0: Option<Vec<String>>,
    pub board_p1: Option<Vec<String>>,
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

    /// The fixed player id a `StepSpec::dcgo_memory` reading is relative
    /// to, when this source carries recorded memory at all. `None` for
    /// sources that never do (native/verification recordings are the
    /// engine's own perspective already, so there is no separate "my
    /// player" to convert against). `DcgoAdapter` overrides this to
    /// `Some(my_player_id)`.
    fn my_player_id(&self) -> Option<PlayerId> {
        None
    }

    /// Count of recorded actions dropped from the replayable step stream
    /// because the recording carries positive evidence they never actually
    /// completed (see the module docs' "Skipping DCGO plays that never
    /// completed"). `0` for every source except `DcgoAdapter`, which is the
    /// only one that ever has this evidence available. Surfaced so a skip
    /// is counted, not silent — a replay result must not read as a clean
    /// pass when it quietly dropped recorded decisions.
    fn skipped_incomplete_plays(&self) -> u32 {
        0
    }
}

/// The single shared decision point for whether mulligan-phase actions
/// should be filtered out of a recording's replayable step stream. See the
/// module docs' "Mulligan handling" section for the full contract.
///
/// `has_initial_state` must reflect whether *this specific recording*
/// actually carries a genuine post-mulligan `initial_state` snapshot — not
/// a hard-coded assumption about the recording's source/format. Native
/// `GameRecorder` recordings always do (by construction: both
/// `ReplayRunner::new` and `NativeAdapter::from_recording` reject a missing
/// one before reaching this point), so they pass `true` and mulligan
/// actions get filtered — they're already baked into the snapshot. DCGO
/// recordings never carry one (DCGO's recorder only logs the keep/redraw
/// decision, never the resulting hand), so `DcgoAdapter` passes `false` and
/// mulligan actions replay like any other decision. A future recorder that
/// starts emitting an equivalent snapshot only needs to thread that
/// presence through to this call — not add a new branch here.
fn should_filter_mulligan_actions(has_initial_state: bool) -> bool {
    has_initial_state
}

/// True when a recorded DCGO `action` row is a hand-play attempt — the ONLY
/// action shape `ActionDetailRow` is ever correlated to (see the module docs'
/// "Skipping DCGO plays that never completed"). `PLAY_HAND_START..
/// PLAY_HAND_END` (raw ids 0..30) is reused across phases for unrelated
/// decisions — most importantly `GamePhase::Mulligan`'s keep (0) / redraw
/// (1) — so the id range alone is not enough; only `Game::decode_main`
/// (`GamePhase::Main`) actually dispatches that range as "play the hand
/// card at this index" (see `action/decode.rs`). Requiring `phase == "Main"`
/// keeps ids 0/1 recorded under `GamePhase::Mulligan` (or any other phase)
/// from ever being mistaken for a play.
fn is_hand_play_action(action_id: u16, phase: &str) -> bool {
    phase == "Main" && (PLAY_HAND_START..PLAY_HAND_END).contains(&action_id)
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

        // See module docs "Mulligan handling": `initial_state` is present
        // (validated above), so mulligan decisions are already baked into
        // it — filter them out of the replayable stream. Driven by the same
        // shared decision point `ReplayRunner` and `DcgoAdapter` use.
        let filter_mulligan = should_filter_mulligan_actions(true);
        let mut steps = Vec::new();
        if let Some(actions) = recording["actions"].as_array() {
            for a in actions {
                let phase = a["phase"].as_str().unwrap_or("").to_string();
                if filter_mulligan && phase == "Mulligan" {
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
                    dcgo_memory: None,
                    turn: a.get("turn").and_then(|v| v.as_u64()),
                    is_game_over: a.get("is_game_over").and_then(|v| v.as_bool()),
                    expected_digest: None,
                    selection: None,
                    board_p0: None,
                    board_p1: None,
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
    /// Local player's digitama (egg) deck in recorded post-shuffle order.
    /// Empty for recordings predating egg capture.
    my_egg_deck: Vec<String>,
    /// Ordered opponent deck (bot-vs-bot). `None` for opaque PvP.
    opp_deck: Option<Vec<String>>,
    /// Opponent's digitama (egg) deck in recorded post-shuffle order
    /// (bot-vs-bot only). Empty for PvP and pre-egg-capture recordings.
    opp_egg_deck: Vec<String>,
    /// Opaque opponent decklist multiset (PvP). `None` for bot-vs-bot.
    opp_decklist: Option<Vec<String>>,
    /// Ordered reveal stream consumed by the opaque `RevealQueue`.
    reveal_pairs: Vec<(crate::opaque_deck::RevealKind, String)>,
    steps: Vec<StepSpec>,
    /// Post-mulligan zone snapshot (`Row::InitialState`), when the
    /// recording carries one. `Some` drives BOTH `should_filter_mulligan_actions`
    /// (mulligan actions are already baked in — filter them like a native
    /// recording) AND `build_initial_game` (zone-restore instead of
    /// replaying the mulligan through RNG). `None` preserves today's
    /// behavior for recordings made before the recorder emitted this row.
    initial_state: Option<crate::dcgo_recording::InitialStateRow>,
    /// Card DB retained so backward-seek can reconstruct (DCGO games are not
    /// restored zone-by-zone; see `relay_initial_state`).
    card_data: HashMap<String, CardData>,
    /// Count of recorded hand-play attempts dropped from `steps` because the
    /// recording carries positive evidence (an absent correlated
    /// `Row::ActionDetail`) they never actually completed. See the module
    /// docs' "Skipping DCGO plays that never completed".
    skipped_incomplete_plays: u32,
}

impl DcgoAdapter {
    /// Build an adapter from a parsed DCGO recording. Lowers the `Action`
    /// rows up to the first `encoder_failure` into the step stream. See
    /// module docs "Mulligan handling": whether mulligan rows are filtered
    /// out of the replayable stream is driven by `should_filter_mulligan_actions`,
    /// keyed off whether THIS recording actually carries a `Row::InitialState`
    /// snapshot (recordings made before the recorder emitted that row never
    /// do, and fall back to replaying the mulligan actions like any other
    /// decision — the game lands in `Mulligan` phase). Precomputes the
    /// opaque opponent decklist + reveal stream when the recording is PvP.
    pub fn from_recording(
        recording: crate::dcgo_recording::RecordingV1,
        card_data: &HashMap<String, CardData>,
    ) -> Result<Self, ReplayError> {
        use crate::dcgo_recording::Row;

        let initial_state = recording.rows.iter().find_map(|row| match row {
            Row::InitialState(s) => Some(s.clone()),
            _ => None,
        });
        // See module docs "Mulligan handling" — driven by the same shared
        // decision point `ReplayRunner`/`NativeAdapter` use, keyed off
        // whether THIS recording actually carries the snapshot (not a
        // hard-coded "DCGO never has one" assumption).
        let filter_mulligan = should_filter_mulligan_actions(initial_state.is_some());

        let my_pid = recording.start.my_player_id;
        let my_deck = recording.start.my_deck_post_shuffle.clone();
        let my_egg_deck = recording.start.my_egg_deck.clone().unwrap_or_default();
        let opp_deck = recording.start.opp_deck_post_shuffle.clone();
        let opp_egg_deck = recording.start.opp_egg_deck.clone().unwrap_or_default();

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

        // See module docs "Skipping DCGO plays that never completed":
        // `ActionDetailRow` is positive evidence a hand-play attempt
        // actually completed (DCGO's recorder emits one, correlated by
        // (step, actor), only from inside `PlayCardClass.PlayCard()` — after
        // the cost was paid). Gate the whole mechanism on whether this
        // recording carries the diagnostic field set AT ALL: an empty
        // `completed_plays` set must mean "no signal available" (today's
        // whole pre-existing corpus), never "nothing completed" — the
        // latter reading would skip every real play in an old recording.
        let has_action_detail_rows = recording
            .rows
            .iter()
            .any(|row| matches!(row, Row::ActionDetail(_)));
        let completed_plays: HashSet<(u32, u8)> = recording
            .rows
            .iter()
            .filter_map(|row| match row {
                Row::ActionDetail(d) => Some((d.step, d.actor)),
                _ => None,
            })
            .collect();

        let mut steps = Vec::new();
        let mut skipped_incomplete_plays: u32 = 0;
        for row in &recording.rows {
            match row {
                Row::Action(a) => {
                    if filter_mulligan && a.phase == "Mulligan" {
                        continue;
                    }
                    // Conservative skip: ONLY a hand-play attempt (see
                    // `is_hand_play_action` — phase-disambiguated, so
                    // Mulligan's ids 0/1 are never mistaken for a play)
                    // with NO correlated `action_detail` is dropped. Every
                    // other action kind (pass/hatch/attack/…) legitimately
                    // never gets a detail row, so absence there is not
                    // evidence of anything — never skip on absence alone.
                    if has_action_detail_rows
                        && is_hand_play_action(a.action_id, &a.phase)
                        && !completed_plays.contains(&(a.step, a.actor))
                    {
                        skipped_incomplete_plays += 1;
                        continue;
                    }
                    steps.push(StepSpec {
                        actor: a.actor,
                        action_id: a.action_id,
                        phase: a.phase.clone(),
                        source: a.source.clone(),
                        memory_after: None,
                        dcgo_memory: a.memory,
                        turn: None,
                        is_game_over: None,
                        expected_digest: None,
                        selection: None,
                        board_p0: a.board_p0.clone(),
                        board_p1: a.board_p1.clone(),
                    })
                }
                // Semantic selection payload (task 3.5) — resolved against
                // the live PendingSelection at step time.
                Row::Selection(s) => steps.push(StepSpec {
                    actor: s.actor,
                    action_id: 0,
                    phase: s.phase.clone(),
                    source: "selection".to_string(),
                    memory_after: None,
                    dcgo_memory: s.memory,
                    turn: None,
                    is_game_over: None,
                    expected_digest: None,
                    selection: Some(s.clone()),
                    board_p0: s.board_p0.clone(),
                    board_p1: s.board_p1.clone(),
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
            my_egg_deck,
            opp_deck,
            opp_egg_deck,
            opp_decklist,
            reveal_pairs,
            steps,
            initial_state,
            card_data: card_data.clone(),
            skipped_incomplete_plays,
        })
    }
}

impl RecordingSource for DcgoAdapter {
    fn build_initial_game(
        &self,
        _card_data: &HashMap<String, CardData>,
    ) -> Result<Game, ReplayError> {
        let mut game = self.build_initial_game_inner()?;
        // Post-mulligan snapshot present: overwrite the observable side(s)'
        // zones with the exact recorded post-mulligan state instead of
        // leaving the mulligan to be replayed through the engine's own RNG
        // (see module docs "Mulligan handling" and `InitialStateRow`'s doc
        // comment). This runs AFTER the ordered/opaque construction above
        // rather than replacing it, so the opaque path's opponent-side
        // `RevealQueue` setup is untouched — only `my` (and, bot-vs-bot,
        // `opp`) zones get overwritten.
        if let Some(snapshot) = &self.initial_state {
            apply_dcgo_initial_state(&mut game, snapshot, self.my_pid)?;
        }
        Ok(game)
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

    fn my_player_id(&self) -> Option<PlayerId> {
        Some(self.my_pid)
    }

    fn skipped_incomplete_plays(&self) -> u32 {
        self.skipped_incomplete_plays
    }
}

impl DcgoAdapter {
    /// The pre-snapshot construction: bot-vs-bot (ordered decks) or opaque
    /// PvP (RevealQueue). Unchanged from before the post-mulligan-snapshot
    /// feature — `build_initial_game` applies `self.initial_state` on top
    /// of this when present.
    fn build_initial_game_inner(&self) -> Result<Game, ReplayError> {
        match &self.opp_deck {
            Some(opp_deck) => {
                // Bot-vs-bot — both decks known. Ordered construction: the
                // recorded post-shuffle order is taken verbatim (`Game::new`
                // would re-shuffle it), and the first player comes from the
                // recording rather than seed parity. Seed 0 still drives
                // card-internal random effects.
                //
                // Egg (digitama) decks ride the same list: `Game::new_inner`
                // routes `CardKind::DigiEgg` entries into
                // `player.digitama_deck` (relative order preserved) and the
                // ordered constructor skips the digitama shuffle, so the
                // recorded egg order is placed verbatim — the same
                // orientation convention as the main deck.
                // Orientation: DCGO draws/hatches from index 0 of its lists;
                // the engine pops from the END of its deck Vecs. Reverse each
                // recorded list (main and egg separately — the kind-splitter
                // preserves relative order) so the engine's first draw is the
                // recording's first card.
                let rev = |v: &[String]| v.iter().rev().cloned().collect::<Vec<_>>();
                let mut my_full = rev(&self.my_deck);
                my_full.extend(rev(&self.my_egg_deck));
                let mut opp_full = rev(&opp_deck);
                opp_full.extend(rev(&self.opp_egg_deck));
                let (deck1, deck2) = if self.my_pid == 0 {
                    (my_full, opp_full)
                } else {
                    (opp_full, my_full)
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
}

/// Apply a DCGO post-mulligan zone snapshot on top of an already-constructed
/// `Game` (built by `DcgoAdapter::build_initial_game_inner`, still sitting in
/// `Mulligan` phase with pre-mulligan hands dealt and both players pending).
/// Overwrites the observable side(s)' zones with the exact recorded state,
/// clears the mulligan queue, sets turn order from `first_player_id`, and
/// fast-forwards into turn 1 — mirroring `NativeAdapter::relay_initial_state`
/// / `restore_player_zone`'s proven zone-restore pattern (this is the same
/// structural problem: a recording that captured a post-mulligan snapshot
/// rather than the mulligan's random outcome).
///
/// `snapshot.opp` is `None` for PvP (not observable) — only `my_pid`'s
/// zones get overwritten; the opaque opponent's `RevealQueue`-backed
/// representation from `build_initial_game_inner` is left untouched.
/// Bot-vs-bot recordings normally carry `opp` too (fully observable), and
/// both sides get overwritten.
///
/// Does NOT rebuild `original_deck` — mulligan reshuffles the deck, it
/// doesn't change which cards the player started with, so the aggregate
/// ledger `build_initial_game_inner` already populated (from
/// `my_deck`/`my_egg_deck`) remains accurate post-mulligan.
fn apply_dcgo_initial_state(
    game: &mut Game,
    snapshot: &crate::dcgo_recording::InitialStateRow,
    my_pid: PlayerId,
) -> Result<(), ReplayError> {
    // `next_card_index` must keep counting up from wherever the opening
    // construction left it — reusing an index would alias two `CardSource`s
    // onto one card-object slot. `Game::next_card_index()` allocates-and-
    // returns the current counter value (the field itself is private); the
    // single index it allocates here before we start assigning our own is
    // simply left unused — a harmless gap, not a collision. Read this
    // BEFORE building `data_index_map` below — that map borrows
    // `game.card_data` immutably, which would conflict with the `&mut
    // self` this method needs.
    let mut next_card_index = game.next_card_index();

    let data_index_map: HashMap<&str, usize> = game
        .card_data
        .iter()
        .enumerate()
        .map(|(idx, cd)| (cd.card_id.as_str(), idx))
        .collect();

    restore_side_from_dcgo_snapshot(
        &mut game.players[my_pid as usize],
        &snapshot.my,
        &data_index_map,
        my_pid,
        &mut next_card_index,
    )?;
    if let Some(opp) = &snapshot.opp {
        let opp_pid: PlayerId = 1 - my_pid;
        restore_side_from_dcgo_snapshot(
            &mut game.players[opp_pid as usize],
            opp,
            &data_index_map,
            opp_pid,
            &mut next_card_index,
        )?;
    }
    game.advance_card_index_to(next_card_index);

    game.mulligan_pending.clear();
    for used in game.mulligan_used.iter_mut() {
        *used = false;
    }

    let first_player = snapshot.first_player_id;
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

/// Wipe and restore ONE player's zones from a DCGO post-mulligan snapshot
/// side. Reverses `library_order`/`digitama_library_order`/`security_order`
/// (DCGO's "index 0 = top" convention -> the engine's pop-from-end `Vec`
/// convention — see `InitialStateSide`'s doc comment); leaves `initial_hand`
/// in recorded order (no top/bottom concept, same as `restore_player_zone`'s
/// treatment of the native `initial_hand` field).
fn restore_side_from_dcgo_snapshot(
    player: &mut Player,
    side: &crate::dcgo_recording::InitialStateSide,
    data_index_map: &HashMap<&str, usize>,
    owner: PlayerId,
    next_card_index: &mut u16,
) -> Result<(), ReplayError> {
    player.hand.clear();
    player.deck.clear();
    player.security.clear();
    player.digitama_deck.clear();
    player.battle_area.clear();
    player.breeding_area = None;
    player.trash.clear();

    fn push(
        zone: &mut Vec<CardSource>,
        ids: &[String],
        data_index_map: &HashMap<&str, usize>,
        owner: PlayerId,
        next_card_index: &mut u16,
    ) -> Result<(), ReplayError> {
        for card_id in ids {
            let data_idx = *data_index_map
                .get(card_id.as_str())
                .ok_or_else(|| ReplayError::UnknownCard(vec![card_id.clone()]))?;
            let cs = CardSource::new(data_idx, owner, *next_card_index);
            *next_card_index += 1;
            zone.push(cs);
        }
        Ok(())
    }

    let rev = |v: &[String]| -> Vec<String> { v.iter().rev().cloned().collect() };
    push(
        &mut player.deck,
        &rev(&side.library_order),
        data_index_map,
        owner,
        next_card_index,
    )?;
    push(
        &mut player.digitama_deck,
        &rev(&side.digitama_library_order),
        data_index_map,
        owner,
        next_card_index,
    )?;
    push(
        &mut player.security,
        &rev(&side.security_order),
        data_index_map,
        owner,
        next_card_index,
    )?;
    push(
        &mut player.hand,
        &side.initial_hand,
        data_index_map,
        owner,
        next_card_index,
    )?;

    Ok(())
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
                dcgo_memory: None,
                turn: None,
                is_game_over: None,
                expected_digest: Some(*digest),
                selection: None,
                board_p0: None,
                board_p1: None,
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

    pub(crate) fn skipped_incomplete_plays(&self) -> u32 {
        self.source.skipped_incomplete_plays()
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

        // Selection `targets` carry DCGO board slots, which need the same
        // frame-order -> play-order remap as action operands.
        let remapped;
        let payload = match remap_selection_targets(game, payload) {
            Some(p) => {
                remapped = p;
                &remapped
            }
            None => payload,
        };

        let phase_before = game.current_phase.py_name().to_string();
        let memory_before = game.memory;
        let memory_pair_before = game.memory_pair;
        let turn_number = game.turn_count;
        let actor = payload.actor;

        // DCGO memory-gauge check (see `check_dcgo_memory` doc comment):
        // compare BEFORE applying the picks, matching when DCGO's own
        // recorder read the gauge (at prompt-answer time, before the
        // resolution that follows takes effect).
        check_dcgo_memory(
            &mut self.divergences,
            self.cursor,
            0,
            actor,
            &payload.phase,
            self.source.steps()[self.cursor as usize].dcgo_memory,
            self.source.my_player_id(),
            memory_before,
            memory_pair_before,
            game,
        );

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
                        board: describe_board(game),
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

        // DCGO indexes the battle area by on-screen frame order; the engine
        // indexes it by play order. Rewrite the recorded action's board
        // operands into the engine's index space before anything inspects it.
        let action_id = {
            let s = &self.source.steps()[cur];
            remap_board_slots(
                game,
                action_id,
                actor,
                s.board_p0.as_ref(),
                s.board_p1.as_ref(),
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
                    board: describe_board(game),
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
                    board: describe_board(game),
                });
                self.paused = true;
                return self.no_progress_result(game);
            }
        }

        let phase_before = game.current_phase.py_name().to_string();
        let memory_before = game.memory;
        let memory_pair_before = game.memory_pair;
        let turn_number = game.turn_count;

        // DCGO memory-gauge check (see `check_dcgo_memory` doc comment):
        // compare BEFORE applying the action, matching when DCGO's own
        // recorder read the gauge (immediately before the RPC dispatch
        // that will mutate it). No-op when this step carries no recorded
        // memory (native/verification sources, or a pre-memory-field DCGO
        // row) — absence is never treated as agreement.
        check_dcgo_memory(
            &mut self.divergences,
            self.cursor,
            action_id,
            actor,
            &phase_rec,
            self.source.steps()[cur].dcgo_memory,
            self.source.my_player_id(),
            memory_before,
            memory_pair_before,
            game,
        );

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

    /// Recorded hand-play attempts dropped from the replayable stream
    /// because the recording carries positive evidence they never actually
    /// completed in DCGO — see the module docs' "Skipping DCGO plays that
    /// never completed". Always `0` for non-DCGO sources and for DCGO
    /// recordings that carry no `action_detail` rows at all.
    pub fn skipped_incomplete_plays(&self) -> u32 {
        self.driver.skipped_incomplete_plays()
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

/// Compare a DCGO row's recorded `memory` (already in `my_player_id`
/// perspective) against the engine's `Game::memory` immediately before this
/// step, converted to the same perspective via
/// `crate::dcgo_recording::memory_from_recording_perspective`. Pushes a
/// non-pausing [`DivergenceKind::Memory`] on disagreement — non-pausing so a
/// batch replay keeps going and the drift is visible at the exact step it
/// happened, rather than surfacing later as unrelated-looking illegal-action
/// noise (the motivating failure this check exists to prevent).
///
/// No-op when either `dcgo_recorded` is `None` (old corpus predating the
/// field, or a native/verification `StepSpec` which never populates
/// `dcgo_memory`) or `my_player_id` is `None` (source has no fixed
/// recording-player perspective to convert against) — absence must never be
/// reported as agreement.
///
/// `memory_pair_before` MUST be `game.memory_pair` read at the same instant
/// as `memory_before` (the seesaw only moves at turn rotation, but pairing
/// a stale reading would silently reintroduce the perspective bug this
/// check exists to prevent).
#[allow(clippy::too_many_arguments)]
fn check_dcgo_memory(
    divergences: &mut Vec<Divergence>,
    cursor: u32,
    action_id: u16,
    actor: PlayerId,
    phase: &str,
    dcgo_recorded: Option<i32>,
    my_player_id: Option<PlayerId>,
    memory_before: i16,
    memory_pair_before: (PlayerId, PlayerId),
    game: &Game,
) {
    let (Some(recorded), Some(my_pid)) = (dcgo_recorded, my_player_id) else {
        return;
    };
    let replayed = crate::dcgo_recording::memory_from_recording_perspective(
        memory_before,
        memory_pair_before.0,
        my_pid,
    );
    if replayed != recorded {
        divergences.push(Divergence {
            step: cursor,
            action_id,
            actor,
            phase: phase.to_string(),
            kind: DivergenceKind::Memory {
                recorded: recorded as i64,
                replayed: replayed as i64,
                my_player_id: my_pid,
            },
            board: describe_board(game),
        });
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

/// Rewrite a selection payload's `targets` slots from DCGO's board ordering
/// into the engine's. Returns `None` when nothing needs rewriting (no board
/// snapshots, no targets, or every slot already maps to itself), so the
/// caller can keep borrowing the original payload.
fn remap_selection_targets(
    game: &Game,
    payload: &crate::dcgo_recording::SelectionRow,
) -> Option<crate::dcgo_recording::SelectionRow> {
    let targets = payload.targets.as_ref()?;
    if payload.board_p0.is_none() && payload.board_p1.is_none() {
        return None;
    }

    let mut out = payload.clone();
    let mut changed = false;
    let slots = out.targets.as_mut()?;
    for t in slots.iter_mut() {
        // -1 addresses the player (security), not a permanent.
        if t.frame < 0 {
            continue;
        }
        let recorded = match t.player {
            0 => payload.board_p0.as_ref(),
            _ => payload.board_p1.as_ref(),
        };
        let Some(recorded) = recorded else { continue };
        let mapped = build_slot_map(recorded, &engine_board_ids(game, t.player));
        if let Some(Some(j)) = mapped.get(t.frame as usize).copied() {
            if j as i32 != t.frame {
                t.frame = j as i32;
                changed = true;
            }
        }
    }
    let _ = targets;
    changed.then_some(out)
}

/// Rewrite the board-position operands of `action_id` from DCGO's slot
/// ordering into the engine's, using the recording's board snapshots.
///
/// Returns the action unchanged when the recording predates board snapshots,
/// when the action carries no board operand, or when any operand fails to map
/// — an unmapped operand means the boards genuinely disagree, which is a
/// divergence to report rather than paper over.
fn remap_board_slots(
    game: &Game,
    action_id: u16,
    actor: PlayerId,
    board_p0: Option<&Vec<String>>,
    board_p1: Option<&Vec<String>>,
) -> u16 {
    use crate::action::space::*;

    let recorded_for = |p: PlayerId| -> Option<&Vec<String>> {
        if p == 0 {
            board_p0
        } else {
            board_p1
        }
    };
    // Map one player's DCGO slot to the engine's, or None if unmappable.
    let map_slot = |p: PlayerId, slot: u16| -> Option<u16> {
        let recorded = recorded_for(p)?;
        let mapped = build_slot_map(recorded, &engine_board_ids(game, p));
        mapped.get(slot as usize).copied().flatten()
    };
    let opponent = 1 - actor;

    if (ATTACK_START..ATTACK_END).contains(&action_id) {
        let (attacker, target) = decode_attack(action_id);
        let new_attacker = match map_slot(actor, attacker) {
            Some(v) => v,
            None => return action_id,
        };
        // SECURITY_TARGET (14) addresses the player, not a permanent.
        let new_target = if target == SECURITY_TARGET {
            target
        } else {
            match map_slot(opponent, target) {
                Some(v) => v,
                None => return action_id,
            }
        };
        return encode_attack(new_attacker, new_target);
    }

    if (DIGIVOLVE_START..DIGIVOLVE_END).contains(&action_id) {
        let (hand, field) = decode_digivolve(action_id);
        // BREEDING_TARGET (14) is a named slot on both sides.
        if field == BREEDING_TARGET {
            return action_id;
        }
        return match map_slot(actor, field) {
            Some(v) => encode_digivolve(hand, v),
            None => action_id,
        };
    }

    if (FIELD_EFFECT_START..FIELD_EFFECT_END).contains(&action_id) {
        let (perm, effect) = decode_field_effect(action_id);
        return match map_slot(actor, perm) {
            Some(v) => FIELD_EFFECT_START + v * EFFECTS_PER_PERMANENT + effect,
            None => action_id,
        };
    }

    if (SOURCE_SELECT_START..SOURCE_SELECT_END).contains(&action_id) {
        let (field, source) = decode_source_select(action_id);
        return match map_slot(actor, field).and_then(|v| encode_source_select(v, source)) {
            Some(v) => v,
            None => action_id,
        };
    }

    action_id
}

/// Engine battle-area card IDs for `player`, in `battle_area` index order.
fn engine_board_ids(game: &Game, player: PlayerId) -> Vec<String> {
    game.player(player)
        .battle_area
        .iter()
        .map(|perm| {
            perm.card_sources
                .last()
                .map(|c| c.card_id(&game.card_data).to_string())
                .unwrap_or_default()
        })
        .collect()
}

/// Map each DCGO battle-area slot onto the engine's `battle_area` index for
/// the same permanent, by card identity.
///
/// DCGO orders its compact battle area by on-screen frame and lets permanents
/// migrate between frames at runtime; the engine's `battle_area` is in play
/// order. The two therefore disagree routinely, not exceptionally — a recorded
/// `attack_0` can mean the engine's slot 1.
///
/// Duplicates (two copies of one card on the same board) are matched in
/// occurrence order, which is exact whenever the two boards hold the same
/// multiset. A slot whose card the engine does not have maps to `None`, and
/// the caller leaves the operand alone so the mismatch surfaces as a
/// divergence instead of being silently rewritten onto the wrong permanent.
fn build_slot_map(recorded: &[String], engine: &[String]) -> Vec<Option<u16>> {
    let mut claimed = vec![false; engine.len()];
    recorded
        .iter()
        .map(|want| {
            let hit = engine
                .iter()
                .enumerate()
                .find(|(j, have)| !claimed[*j] && *have == want)
                .map(|(j, _)| j);
            if let Some(j) = hit {
                claimed[j] = true;
                Some(j as u16)
            } else {
                None
            }
        })
        .collect()
}

/// Render both players' battle areas in `battle_area` index order, which is
/// the index space every board-position action ID uses.
///
/// Shows each permanent's slot, card id, DP, and whether it is suspended, plus
/// the digivolution-source depth — enough to answer the three questions a
/// target mismatch raises: does the engine hold the same permanents as the
/// oracle, in the same order, in the same state?
fn describe_board(game: &Game) -> String {
    let mut out = String::new();
    for pid in [0u8, 1u8] {
        let area = &game.player(pid).battle_area;
        out.push_str(&format!("P{} battle_area ({}): ", pid, area.len()));
        if area.is_empty() {
            out.push_str("(empty)");
        } else {
            let entries: Vec<String> = area
                .iter()
                .enumerate()
                .map(|(i, perm)| {
                    let card_id = perm
                        .card_sources
                        .last()
                        .map(|c| c.card_id(&game.card_data).to_string())
                        .unwrap_or_else(|| "?".to_string());
                    let handle = crate::permanent::PermanentHandle {
                        player: pid,
                        index: i as u8,
                    };
                    let dp = perm
                        .base_dp(&game.card_data)
                        .map(|base| base + game.modifiers.sum(handle, crate::ModifierType::ChangeDp))
                        .map(|d| d.to_string())
                        .unwrap_or_else(|| "-".to_string());
                    format!(
                        "[{}] {} dp={}{} srcs={}",
                        i,
                        card_id,
                        dp,
                        if perm.is_suspended { " SUSPENDED" } else { "" },
                        perm.card_sources.len().saturating_sub(1)
                    )
                })
                .collect();
            out.push_str(&entries.join(", "));
        }
        out.push_str("
");
    }
    // Zone counts alongside the board. A permanent that vanished between two
    // steps went SOMEWHERE, and trash-vs-security tells you whether it was
    // deleted, returned, or lost to a security check -- which is the difference
    // between an engine bug and a mis-mapped action.
    for pid in [0u8, 1u8] {
        let p = game.player(pid);
        // Hand and trash contents, indexed. A digivolve/play action names a hand
        // SLOT, so "is the recorded slot even that card?" is unanswerable without
        // the list -- and a card the recording plays from one zone while the
        // engine holds it in another is exactly the kind of divergence worth
        // separating from a plain mistarget.
        let list = |cards: &[crate::card_source::CardSource]| -> String {
            cards
                .iter()
                .enumerate()
                .map(|(i, c)| format!("[{}]{}", i, c.card_id(&game.card_data)))
                .collect::<Vec<_>>()
                .join(" ")
        };
        out.push_str(&format!("P{} hand:  {}
", pid, list(&p.hand)));
        out.push_str(&format!("P{} trash: {}
", pid, list(&p.trash)));
        // Security top is the END of the vec. A battle outcome hinges entirely
        // on which card the check flips, so a divergence in the security stack
        // masquerades as a combat bug.
        out.push_str(&format!("P{} sec:   {} (top = last)
", pid, list(&p.security)));
        out.push_str(&format!(
            "P{} zones: hand={} deck={} trash={} security={} memory={}
",
            pid,
            p.hand.len(),
            p.deck.len(),
            p.trash.len(),
            p.security.len(),
            game.memory
        ));
    }
    out
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

    // ── DCGO board-slot remapping ────────────────────────────────────────

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn slot_map_matches_by_identity_not_position() {
        // DCGO ordered by frame; the engine by play order. Same two
        // permanents, opposite indexes.
        let map = build_slot_map(&ids(&["BT16-082", "EX10-010"]), &ids(&["EX10-010", "BT16-082"]));
        assert_eq!(map, vec![Some(1), Some(0)]);
    }

    #[test]
    fn slot_map_pairs_duplicates_in_occurrence_order() {
        let map = build_slot_map(
            &ids(&["EX12-035", "BT16-082", "EX12-035"]),
            &ids(&["EX12-035", "EX12-035", "BT16-082"]),
        );
        assert_eq!(map, vec![Some(0), Some(2), Some(1)]);
    }

    #[test]
    fn slot_map_reports_unknown_card_as_unmapped() {
        // A card the engine does not have on board must NOT be silently
        // rewritten onto some other permanent.
        let map = build_slot_map(&ids(&["EX10-010", "ZZ99-999"]), &ids(&["EX10-010"]));
        assert_eq!(map, vec![Some(0), None]);
    }

    #[test]
    fn remap_rewrites_attacker_and_preserves_security_target() {
        use crate::action::space::{decode_attack, encode_attack, SECURITY_TARGET};
        // Engine play order: EX10-010 then BT16-082.
        let engine = ids(&["EX10-010", "BT16-082"]);
        let dcgo = ids(&["BT16-082", "EX10-010"]);
        let map = build_slot_map(&dcgo, &engine);
        assert_eq!(map[0], Some(1));

        // Sanity on the encoding itself: DCGO slot 0 attacking security must
        // become engine slot 1 attacking security.
        let recorded = encode_attack(0, SECURITY_TARGET);
        let (a, t) = decode_attack(recorded);
        assert_eq!((a, t), (0, SECURITY_TARGET));
        let expected = encode_attack(1, SECURITY_TARGET);
        let (a2, t2) = decode_attack(expected);
        assert_eq!((a2, t2), (1, SECURITY_TARGET));
    }

    // ── Skipping DCGO plays that never completed ────────────────────────
    //
    // `DcgoAdapter::from_recording` must skip a recorded play-range action
    // only when there is POSITIVE evidence (an absent correlated
    // `action_detail` row, in a recording that otherwise carries the field
    // set) it never actually completed in DCGO. Everything else — other
    // action kinds, mulligan decisions at the same raw ids, and recordings
    // that never carry `action_detail` at all — must replay unchanged. See
    // the module docs' "Skipping DCGO plays that never completed".

    fn dcgo_game_start(
        my_deck: Vec<String>,
        opp_deck: Option<Vec<String>>,
    ) -> crate::dcgo_recording::GameStart {
        crate::dcgo_recording::GameStart {
            v: 1,
            game_id: "skip-test".into(),
            timestamp: "t".into(),
            my_player_id: 0,
            // Fixed explicitly so construction never depends on inferring
            // it from a mulligan row's `source` string (irrelevant to what
            // these tests are checking).
            first_player: Some(0),
            is_ai: true,
            my_deck_post_shuffle: my_deck,
            opp_deck_post_shuffle: opp_deck,
            opp_decklist_composition: None,
            my_egg_deck: None,
            opp_egg_deck: None,
        }
    }

    fn dcgo_action_row(step: u32, actor: u8, action_id: u16, phase: &str) -> crate::dcgo_recording::Row {
        crate::dcgo_recording::Row::Action(crate::dcgo_recording::ActionRow {
            step,
            actor,
            action_id,
            phase: phase.to_string(),
            source: "main_phase".to_string(),
            board_p0: None,
            board_p1: None,
            memory: None,
            card_id: None,
            memory_before: None,
        })
    }

    /// An `action_detail` row correlated to `(step, actor)` — positive
    /// evidence that play actually completed in DCGO.
    fn dcgo_action_detail_row(step: u32, actor: u8) -> crate::dcgo_recording::Row {
        crate::dcgo_recording::Row::ActionDetail(crate::dcgo_recording::ActionDetailRow {
            step,
            actor,
            card_id: Some("BT1-010".to_string()),
            cost_paid: 3,
            memory_after: None,
            alt_path: None,
            materials: None,
        })
    }

    fn dcgo_game_end() -> crate::dcgo_recording::GameEnd {
        crate::dcgo_recording::GameEnd {
            winner: -1,
            reason: "test".into(),
            total_steps: 0,
        }
    }

    /// `DcgoAdapter::from_recording` performs no card-data validation (that
    /// is `NativeAdapter`'s contract, not this one's — see its doc comment),
    /// so construction-only tests can pass an empty pool.
    fn no_card_data() -> HashMap<String, CardData> {
        HashMap::new()
    }

    // Test 2 of the skip spec: a play-range action WITH a correlated
    // `action_detail` is retained — replayed like any other decision.
    #[test]
    fn dcgo_adapter_retains_play_range_action_with_correlated_action_detail() {
        let recording = crate::dcgo_recording::RecordingV1 {
            start: dcgo_game_start(vec![], Some(vec![])),
            rows: vec![dcgo_action_row(0, 0, 5, "Main"), dcgo_action_detail_row(0, 0)],
            end: dcgo_game_end(),
        };
        let adapter =
            DcgoAdapter::from_recording(recording, &no_card_data()).expect("adapter builds");
        assert_eq!(
            adapter.steps().len(),
            1,
            "a confirmed play must be retained in the replayable stream"
        );
        assert_eq!(adapter.skipped_incomplete_plays(), 0);
    }

    // Test 1 of the skip spec (construction half — the end-to-end
    // divergence-free half is `dcgo_session_skips_incomplete_play_...`
    // below): a play-range action with NO correlated `action_detail`, in a
    // recording that otherwise carries the field, is dropped.
    #[test]
    fn dcgo_adapter_skips_play_range_action_with_no_correlated_action_detail() {
        let recording = crate::dcgo_recording::RecordingV1 {
            start: dcgo_game_start(vec![], Some(vec![])),
            rows: vec![
                dcgo_action_row(0, 0, 5, "Main"),
                // A detail row for a DIFFERENT step keeps the "this
                // recording carries action_detail rows" gate open while
                // leaving step 0 itself uncorrelated.
                dcgo_action_detail_row(1, 0),
            ],
            end: dcgo_game_end(),
        };
        let adapter =
            DcgoAdapter::from_recording(recording, &no_card_data()).expect("adapter builds");
        assert!(
            adapter.steps().is_empty(),
            "an unconfirmed play must be dropped from the replayable stream, got {:?}",
            adapter.steps()
        );
        assert_eq!(adapter.skipped_incomplete_plays(), 1);
    }

    // Test 3 of the skip spec — the regression that matters most: a
    // non-play action (pass / attack / hatch) with no correlated
    // `action_detail` must NEVER be skipped. `action_detail` is only ever
    // emitted for `PlayCardAction`s, so absence here carries no signal.
    #[test]
    fn dcgo_adapter_never_skips_non_play_actions_even_without_action_detail() {
        let recording = crate::dcgo_recording::RecordingV1 {
            start: dcgo_game_start(vec![], Some(vec![])),
            rows: vec![
                dcgo_action_row(0, 0, 60, "Breeding"), // hatch
                dcgo_action_row(1, 0, 62, "Main"),      // pass
                dcgo_action_row(2, 0, 100, "Main"),     // attack slot 0 -> security
                // Detail row elsewhere proves the gate is armed and these
                // still are not skipped.
                dcgo_action_detail_row(99, 0),
            ],
            end: dcgo_game_end(),
        };
        let adapter =
            DcgoAdapter::from_recording(recording, &no_card_data()).expect("adapter builds");
        assert_eq!(
            adapter.steps().len(),
            3,
            "pass/hatch/attack must never be skipped, got {:?}",
            adapter.steps()
        );
        assert_eq!(adapter.skipped_incomplete_plays(), 0);
    }

    // Test 4 of the skip spec: ids 0/1 recorded under `GamePhase::Mulligan`
    // are the keep/redraw decision, NOT plays, even though they alias the
    // play-hand range's first two ids. Must never be skipped.
    #[test]
    fn dcgo_adapter_never_skips_mulligan_decisions_even_at_play_range_ids() {
        let recording = crate::dcgo_recording::RecordingV1 {
            start: dcgo_game_start(vec![], Some(vec![])),
            rows: vec![
                dcgo_action_row(0, 0, 0, "Mulligan"), // keep — aliases play_hand[0]
                dcgo_action_row(1, 1, 1, "Mulligan"), // redraw — aliases play_hand[1]
                dcgo_action_detail_row(99, 0),        // gate armed
            ],
            end: dcgo_game_end(),
        };
        let adapter =
            DcgoAdapter::from_recording(recording, &no_card_data()).expect("adapter builds");
        assert_eq!(
            adapter.steps().len(),
            2,
            "mulligan decisions must never be treated as plays, got {:?}",
            adapter.steps()
        );
        assert_eq!(adapter.skipped_incomplete_plays(), 0);
    }

    // Test 5 of the skip spec: a recording with ZERO `action_detail` rows —
    // today's entire pre-existing corpus, recorded before the field
    // existed — must replay exactly as before this feature. Total absence
    // of the diagnostic field set is "no signal available", never "nothing
    // completed".
    #[test]
    fn dcgo_adapter_never_skips_when_recording_has_no_action_detail_rows_at_all() {
        let recording = crate::dcgo_recording::RecordingV1 {
            start: dcgo_game_start(vec![], Some(vec![])),
            rows: vec![dcgo_action_row(0, 0, 5, "Main")],
            end: dcgo_game_end(),
        };
        let adapter =
            DcgoAdapter::from_recording(recording, &no_card_data()).expect("adapter builds");
        assert_eq!(
            adapter.steps().len(),
            1,
            "a recording with no action_detail rows must replay unchanged"
        );
        assert_eq!(adapter.skipped_incomplete_plays(), 0);
    }

    /// Minimal viable card pool for the end-to-end skip test: a DigiEgg +
    /// a Rookie, enough to build a legal 50-card deck and let mulligan +
    /// main-phase actions play out. Mirrors
    /// `dcgo_replay::replay::tests::micro_card_db`.
    fn skip_test_card_db() -> HashMap<String, CardData> {
        let json = r#"{
            "BT1-001": {
                "card_id": "BT1-001", "card_name_eng": "Koromon",
                "card_effect_class_name": "BT1_001", "play_cost": 0, "dp": -1,
                "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
                "type_eng": ["Lesser"], "form_eng": ["In-Training"], "attribute_eng": [],
                "effect_description_eng": "", "inherited_effect_description_eng": "",
                "security_effect_description_eng": "", "evo_costs": []
            },
            "BT1-010": {
                "card_id": "BT1-010", "card_name_eng": "Agumon",
                "card_effect_class_name": "BT1_010", "play_cost": 3, "dp": 2000,
                "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
                "type_eng": ["Reptile"], "form_eng": ["Rookie"], "attribute_eng": ["Vaccine"],
                "effect_description_eng": "", "inherited_effect_description_eng": "",
                "security_effect_description_eng": "", "evo_costs": []
            }
        }"#;
        CardData::load_from_str(json).unwrap()
    }

    fn skip_test_deck() -> Vec<String> {
        let mut d = Vec::new();
        for _ in 0..4 {
            d.push("BT1-001".to_string());
        }
        for _ in 0..46 {
            d.push("BT1-010".to_string());
        }
        d
    }

    // Test 1 (end-to-end half) + Test 6 of the skip spec: a play DCGO
    // attempted but never completed must never surface as an
    // `illegal_action` (`DivergenceKind::MaskMiss`) divergence, and the
    // skip must be counted, not silent.
    #[test]
    fn dcgo_session_skips_incomplete_play_and_reports_no_illegal_action() {
        let card_data = skip_test_card_db();
        let deck = skip_test_deck();
        let recording = crate::dcgo_recording::RecordingV1 {
            start: dcgo_game_start(deck.clone(), Some(deck)),
            rows: vec![
                dcgo_action_row(0, 0, 0, "Mulligan"),
                dcgo_action_row(1, 1, 0, "Mulligan"),
                // Player 0's post-mulligan hand only has 5 cards (indices
                // 0..5) — the LAST play-hand id is out of range and would
                // be a genuine MaskMiss if replayed. DCGO attempted it (the
                // row exists) but never completed it (no correlated
                // `action_detail`, even though the recorder does emit the
                // field elsewhere in this same recording).
                dcgo_action_row(2, 0, PLAY_HAND_END - 1, "Main"),
                dcgo_action_detail_row(50, 0),
                // Ends the game cleanly once the unconfirmed play is
                // skipped — CONCEDE_GAME is legal regardless of mask state.
                dcgo_action_row(3, 0, crate::action::space::CONCEDE_GAME, "Main"),
            ],
            end: crate::dcgo_recording::GameEnd {
                winner: 1,
                reason: "concede".into(),
                total_steps: 4,
            },
        };

        let mut session =
            ReplaySession::from_dcgo(recording, &card_data, false).expect("session builds");
        session.run_to_completion();

        assert!(
            session.divergences().is_empty(),
            "the unconfirmed play must never reach the legality check: {:?}",
            session.divergences()
        );
        assert!(!session.is_paused());
        assert_eq!(
            session.skipped_incomplete_plays(),
            1,
            "the skip must be counted, not silent"
        );
        // Only the 2 mulligan keeps + the concede made it into the
        // replayable stream — the unconfirmed play did not.
        assert_eq!(session.total_steps(), 3);
        assert!(session.is_game_over());
        assert_eq!(session.winner_id(), Some(1));
    }
}
