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

/// Does the parked flat source-card prompt (`SelectionKind::SourceMulti`) offer
/// a legal card held by the Digimon a carrier-only row names?
///
/// DCGO's `SelectTrashDigivolutionCards` asks for the Digimon that holds the
/// cards first (`SelectPermanentEffect`, a targets-only row) and the cards second
/// (`SelectCardEffect`). The engine parks ONE prompt over every legal source
/// card, so the carrier row only narrows. The fold is proved by the engine's own
/// legality: it applies only when the named Digimon really holds a legal
/// candidate, otherwise the row falls through and surfaces as a divergence.
fn source_multi_carrier_has_candidate(
    game: &Game,
    target: &crate::dcgo_recording::FrameTarget,
) -> bool {
    let Some(pending) = game.pending_selection.as_ref() else {
        return false;
    };
    if !matches!(pending.kind, crate::selection::SelectionKind::SourceMulti { .. }) {
        return false;
    }
    let Some(owner) = pending.card_choice.as_ref() else {
        return false;
    };
    let Ok(slot) = u8::try_from(target.frame) else {
        return false;
    };
    owner
        .view(game, pending.selecting_player)
        .rows
        .iter()
        .any(|row| {
            row.owner == Some(target.player)
                && row.carrier_slot == Some(slot)
                && pending.valid_action_ids.contains(&row.action_id)
        })
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

/// Native recordings without a marker predate owned page aliases. Reject malformed
/// or future markers instead of guessing based on action-space size.
fn native_action_schema_version(recording: &Value) -> Result<u32, ReplayError> {
    let version = match recording.get("action_schema_version") {
        None => 1,
        Some(value) => value.as_u64().and_then(|v| u32::try_from(v).ok())
            .ok_or_else(|| ReplayError::MalformedRecording("action_schema_version must be an integer".into()))?,
    };
    validate_action_schema_version(version)?;
    Ok(version)
}
pub(crate) fn validate_action_schema_version(version: u32) -> Result<(), ReplayError> {
    if version != 1 && version != crate::action::space::SCHEMA_VERSION {
        return Err(ReplayError::MalformedRecording(format!("unsupported action_schema_version {version}")));
    }
    Ok(())
}
fn legacy_owned_card_alias(game: &Game, version: u32) -> bool {
    version < crate::action::space::SCHEMA_VERSION
        && game.pending_selection.as_ref().is_some_and(|s| s.card_choice.is_some())
}

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
    paused: bool,
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
        native_action_schema_version(&recording)?;
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
            paused: false,
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

    pub fn is_paused(&self) -> bool { self.paused }

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
        if self.is_complete() || self.paused {
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

        if legacy_owned_card_alias(&self.game, native_action_schema_version(&self.recording).expect("validated native metadata")) {
            self.paused = true;
            return ReplayStepResult {
                step_number: self.current_step, player_id, action_id,
                phase_before: phase_before.clone(), phase_after: phase_before,
                memory_before, memory_after: memory_before, turn_number,
                is_game_over: self.game.game_over, winner_id: self.game.winner,
                divergences: vec![DivergenceReport { step: self.current_step,
                    field: "action_schema_version", recorded: "1".into(),
                    replayed: format!("owned card page requires action_schema_version {}", crate::action::space::SCHEMA_VERSION) }],
            };
        }
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
        while self.current_step < target && !self.is_game_over() && !self.paused {
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
        while !self.is_complete() && !self.is_game_over() && !self.paused {
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
        self.paused = false;

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
    /// The printed card id the recorded action references (DCGO's diagnostic
    /// `card_id`): lets the driver re-resolve an operand the recorder encoded
    /// by a different index space than the engine's hand slot.
    pub card_id: Option<String>,
    /// The two partner Digimon of a DNA digivolution, from the correlated
    /// `action_detail` (`alt_path == "dna_digivolve"`). DCGO's recorder encodes the
    /// action as a plain digivolve id and carries the pair nowhere else.
    pub dna_materials: Option<Vec<String>>,
    /// Row synthesised by a converter (see `ActionRow::inferred`): skipped, and counted, when the engine is
    /// already waiting on a different seat. Never skipped when the engine accepts the actor.
    pub inferred: bool,
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
    /// Runtime-authored sources use current actions; recording adapters override explicitly.
    fn action_schema_version(&self) -> u32 { crate::action::space::SCHEMA_VERSION }

    /// Select digest semantics from explicit recording metadata. All current
    /// non-verification adapters use the current protocol.
    fn verification_digest_version(&self) -> crate::recorder::VerificationDigestVersion {
        crate::recorder::VerificationDigestVersion::CURRENT
    }

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

    /// True when this source's own steps answer ENGINE-ONLY prompts —
    /// decisions our engine parks that DCGO never makes.
    ///
    /// `false` for every recording-backed source: a DCGO JSONL can only carry
    /// answers to decisions DCGO asked, so an engine-only prompt has to be
    /// auto-answered on the oracle's behalf. No driver auto-answer exists
    /// today: the only one, `ReplayDriver::auto_answer_option_trash_order`,
    /// answered the 9-1-5 trash-order pick, which §9-1-5 (Ver. 4.3) removed
    /// (F-ENGINE-EFFECT-USED-OPTION-MAIN-IS-ORDERABLE). A future one must
    /// return early when this is `true`.
    ///
    /// The exam's `ScenarioAdapter` is the exception and overrides this to
    /// `true`: a scenario author writes such a decision explicitly as a
    /// `sim_only:` row. Auto-answering it there consumes the prompt the row
    /// was written for, the row then lands on whatever the engine parks NEXT,
    /// and every later row is shifted one decision forward — silently eating a
    /// real choice and measuring the wrong state.
    ///
    /// Historical witness (before the auto-answer and its prompt were
    /// removed): `qa/dcgo-exams/BT25/BT25-091-effect2.yaml`. The
    /// auto-answer took the 9-1-5 trash-order pick, so the scenario's own
    /// `choice: "Trash the used Option"` row landed on Monica Simmons'
    /// §15-7-4 [4.3 REVERSED — see changelog-3.6-to-4.3.md] "by suspending this Tamer" decline gate and spent it. The
    /// `select: { yes: true }` row that was written for that gate then
    /// answered the target pick, and the oracle diff reported
    /// `p0.field[0].suspended: ours=true dcgo=false` — a divergence
    /// manufactured entirely by the double answer. The rule stands for any
    /// future engine-only prompt.
    fn answers_engine_only_prompts(&self) -> bool {
        false
    }

    /// True only for a DCGO recording: a source whose rows omit the prompts
    /// DCGO resolves silently (a lone ordering card, a lone trigger, a lone
    /// target, an unrecorded stack order, an ordinary cost route), so the
    /// replay must answer those itself before the next recorded row.
    ///
    /// Native / verification recordings are the engine's own decisions and
    /// record every one of those prompts; auto-answering them there consumes
    /// a recorded row's prompt and shifts every later row (the replay-golden
    /// corpus diverged at steps 6-13 on all ten goldens this way).
    fn omits_dcgo_silent_prompts(&self) -> bool {
        false
    }

    /// DCGO's own identity (`CardSource.CardIndex`) of the card the ACTION
    /// step at `row` plays, when the recording carries it
    /// (`ActionRow::card_index`). A selection step carries its picks' indexes
    /// on its payload (`SelectionRow::card_indexes`). The replay pairs each
    /// index with the engine card the step moved
    /// (`ReplayDriver::record_dcgo_card_indexes`). `None` for every source
    /// without DCGO card identity.
    fn dcgo_action_card_index(&self, _row: u32) -> Option<i32> {
        None
    }

    /// DCGO's own identity of the permanent the ACTION step at `row` names as
    /// its own-board operand (attacker, digivolve target, activated
    /// permanent), with where the index came from
    /// (`ActionRow::operand_card_index` / `operand_card_index_source`).
    /// `None` for every source without it.
    fn dcgo_action_operand_index(&self, _row: u32) -> Option<(i32, Option<crate::dcgo_recording::CardIndexSource>)> {
        None
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
        native_action_schema_version(&recording)?;
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
                    card_id: None,
                    dna_materials: None,
                    inferred: false,
                });
            }
        }

        Ok(Self { recording, steps })
    }
}

impl RecordingSource for NativeAdapter {
    fn action_schema_version(&self) -> u32 {
        native_action_schema_version(&self.recording).expect("validated native metadata")
    }

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
    /// `ActionRow::card_index` of each step (parallel to `steps`; `None` for a
    /// selection step or an action without one). Read through
    /// `RecordingSource::dcgo_action_card_index`.
    action_card_indexes: Vec<Option<i32>>,
    /// `ActionRow::operand_card_index` (+ source) of each step, parallel to
    /// `steps`. Read through `RecordingSource::dcgo_action_operand_index`.
    action_operand_indexes: Vec<Option<(i32, Option<crate::dcgo_recording::CardIndexSource>)>>,
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

        let dna_by_step: std::collections::HashMap<(u32, u8), Vec<String>> = recording
            .rows
            .iter()
            .filter_map(|row| match row {
                Row::ActionDetail(d)
                    if d.alt_path.as_deref() == Some("dna_digivolve")
                        && d.materials.as_ref().is_some_and(|m| m.len() == 2) =>
                {
                    Some(((d.step, d.actor), d.materials.clone().unwrap()))
                }
                _ => None,
            })
            .collect();

        let mut steps = Vec::new();
        let mut action_card_indexes: Vec<Option<i32>> = Vec::new();
        let mut action_operand_indexes: Vec<Option<(i32, Option<crate::dcgo_recording::CardIndexSource>)>> = Vec::new();
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
                        card_id: a.card_id.clone(),
                        dna_materials: dna_by_step.get(&(a.step, a.actor)).cloned(),
                        inferred: a.inferred,
                    });
                    action_card_indexes.push(a.card_index);
                    action_operand_indexes.push(a.operand_card_index.map(|k| (k, a.operand_card_index_source)));
                }
                // Semantic selection payload (task 3.5) — resolved against
                // the live PendingSelection at step time.
                Row::Selection(s) => {
                    action_card_indexes.push(None);
                    action_operand_indexes.push(None);
                    steps.push(StepSpec {
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
                    card_id: None,
                    dna_materials: None,
                    inferred: false,
                    })
                }
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
            action_card_indexes,
            action_operand_indexes,
        })
    }
}

impl RecordingSource for DcgoAdapter {
    // Raw DCGO action IDs never declare the engine-owned paging protocol.
    // Semantic selection payloads have their own live mapping path.
    fn action_schema_version(&self) -> u32 { 1 }

    fn dcgo_action_card_index(&self, row: u32) -> Option<i32> {
        self.action_card_indexes.get(row as usize).copied().flatten()
    }

    fn dcgo_action_operand_index(&self, row: u32) -> Option<(i32, Option<crate::dcgo_recording::CardIndexSource>)> {
        self.action_operand_indexes.get(row as usize).copied().flatten()
    }

    fn omits_dcgo_silent_prompts(&self) -> bool {
        true
    }

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
        validate_action_schema_version(recording.action_schema_version)?;
        if recording.format_version != crate::recorder::VERIFICATION_REPLAY_FORMAT_VERSION {
            return Err(ReplayError::MalformedRecording(format!("unsupported verification replay format {}", recording.format_version)));
        }
        if crate::recorder::VerificationDigestVersion::from_engine_schema(recording.engine_schema_version).is_none() {
            return Err(ReplayError::MalformedRecording(format!("unsupported verification engine schema {}", recording.engine_schema_version)));
        }
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
                card_id: None,
                dna_materials: None,
                inferred: false,
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
    fn action_schema_version(&self) -> u32 { self.recording.action_schema_version }

    fn verification_digest_version(&self) -> crate::recorder::VerificationDigestVersion {
        crate::recorder::VerificationDigestVersion::from_engine_schema(self.recording.engine_schema_version)
            .expect("recording schema validated at construction")
    }

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
    /// Card ids of DIGIVOLVE actions whose board-remapped operand was ILLEGAL
    /// while the recorded one was legal, so the recorded operand was used (see
    /// the operand fallback in `step`). Diagnostic only: it lets a report tell a
    /// digivolve onto a target the engine rejected (a DCGO phantom digivolve)
    /// from an ordinary memory divergence. Cleared on a backward reset.
    digivolve_operand_fallbacks: Vec<String>,
    /// Steps of converter-`inferred` rows the engine had already moved past (skipped, never silently passed).
    inferred_skipped: Vec<u32>,
    /// Non-halting notes about recorded rows the replay consumed without an
    /// engine prompt to apply them to (see [`ReplayDriver::notes`]). Cleared on
    /// a backward reset.
    notes: Vec<String>,
    /// Opponent battle slot a recorded attack named while the engine only offered
    /// the security attack: the attacker has <Raid>, so the recorded target is the
    /// Raid switch (see `raid_declared_target`). Consumed right after the attack.
    raid_retarget: Option<u16>,
    /// Set when a pausing `CheckThenApply` divergence (`MaskMiss` / `Actor`)
    /// halts the cursor. `step()` is a no-op while paused; cleared by a
    /// backward `seek` (reset) or `set_policy`.
    paused: bool,
    /// How deep this driver runs inside a `remote_trigger_order_choice`
    /// lookahead (0 = the real replay). Bounds the nested lookaheads.
    lookahead_depth: u8,
    /// `(row, zone)`: the zone DCGO's "From which area?" menu (int or bool)
    /// named for the pick at recorded row `row`, when that row records no
    /// zone of its own (`selection_resolve::menu_pick_zone`). Applied to that
    /// row only (as its `zone`), so its card resolves in the zone DCGO picked
    /// from (`selection_resolve::row_pick_zone`), never in the other one.
    pick_zone_hint: Option<(u32, &'static str)>,
    /// The recorded row that is a card's zone menu ("From hand" / "From
    /// trash") because the row before it was that card's "do it?" bool
    /// (`selection_resolve::dcgo_bool_gate_before_zone_menu`), read by
    /// `hand_zone_bool`. Without it the gate and the menu are both bare bools.
    zone_menu_row: Option<u32>,
    /// `game.events.len()` when the last recorded row that is NOT an
    /// unstamped "use this effect?" answer began: the start of the engine's
    /// event window an unstamped answer is judged against
    /// (`classify_unstamped_answer`). A run of unstamped answers shares one
    /// window, the effects of the decision in front of them.
    unstamped_window_start: usize,
    /// The recorded `MultipleSkills` row the replay last answered by choosing
    /// an entry of the engine's order of two or more effects, with that
    /// entry's source card (`step`; a lookahead branch starts with its own).
    /// Read by the "not a trigger order" fold: DCGO runs the effect it chose
    /// before it orders again, so the next index is never that effect's.
    order_answer: Option<(u32, crate::card_source::CardHandle)>,
    /// DCGO's own card identity (`CardSource.CardIndex`) -> the engine card the
    /// replay's own applied rows paired it with (`record_dcgo_card_indexes`).
    /// Read by `duplicate_copy_target`: a target naming an index resolves to
    /// the permanent whose top card is that engine card, not to a position.
    /// A lookahead branch starts with a copy; cleared on a backward reset.
    dcgo_cards: HashMap<i32, crate::card_source::CardHandle>,
    /// Taken when the NEXT recorded row is a forced (single-candidate)
    /// `SelectCountEffect`: the row the count follows, the event-log length
    /// and every card's zone as that row began. The count is consumed only
    /// when the engine resolved its effect since then
    /// (`forced_count_evidence`).
    count_evidence: Option<CountEvidenceBase>,
}

/// See `ReplayDriver::count_evidence`.
#[derive(Clone)]
struct CountEvidenceBase {
    row: u32,
    events: usize,
    zones: HashMap<crate::card_source::CardHandle, (PlayerId, CardZone)>,
    /// Every battle-area permanent as the row began: whose, its cards from the
    /// bottom up, and its top card's id. Read by `forced_count_evidence` to tell
    /// a permanent that lost its top cards from one that left the field.
    stacks: Vec<BaseStack>,
}

/// One battle-area permanent as a decision began (`CountEvidenceBase`).
#[derive(Clone)]
struct BaseStack {
    player: PlayerId,
    cards: Vec<crate::card_source::CardHandle>,
    top_card_id: String,
}

impl CountEvidenceBase {
    fn capture(row: u32, game: &Game) -> Self {
        let stacks = game
            .players
            .iter()
            .flat_map(|p| {
                p.battle_area.iter().map(move |perm| BaseStack {
                    player: p.id,
                    cards: perm.card_sources.iter().map(|c| c.handle()).collect(),
                    top_card_id: perm.top_card().card_id(&game.card_data).to_string(),
                })
            })
            .collect();
        Self { row, events: game.events.len(), zones: card_zones(game), stacks }
    }
}

/// Whether `row` is a DCGO count with ONE candidate value - the count DCGO
/// asks even when the number is forced (`IDegeneration.Degeneration`,
/// CardController.cs:5281-5297: `MaxCount = min(digivolution cards, N)`,
/// `CanNoSelect: false`) and the engine never asks (CR 4.3 16-12-1).
fn is_forced_count_row(row: &crate::dcgo_recording::SelectionRow) -> bool {
    row.prompt == "SelectCountEffect"
        && row.cancel != Some(true)
        && row
            .candidates
            .as_deref()
            .is_some_and(|c| c.len() == 1 && row.count.is_some_and(|n| i64::from(n) == c[0]))
}

/// `(entry action, source card)` of every entry of the parked order prompt
/// (a trigger order, or the replacement order DCGO asks in the same panel).
fn order_entry_sources(game: &Game) -> Vec<(u16, crate::card_source::CardHandle)> {
    game.pending_selection
        .as_ref()
        .and_then(|p| p.effect_choices.as_ref())
        .map(|es| es.iter().filter_map(|e| e.source_card.map(|c| (e.action_id, c))).collect())
        .unwrap_or_default()
}

/// Marker in the divergence a card-less `MultipleSkills` row produces when no
/// evidence tells its candidate entries apart (see `remote_trigger_order_choice`).
const AMBIGUOUS_ORDER: &str = "is ambiguous";

/// Marker in the divergence an order produces when the rows after it refute
/// EVERY entry (`LookaheadVerdict::AllRefuted`). Inside a lookahead it refutes
/// the branch that led to the order only where another branch replayed past
/// that row (`settle_nested_refutations`).
const REFUTED_ORDER: &str = "refute each of them";

/// The prompt of the engine's replacement ORDER (an `EffectChoice` over each
/// pending immediate replacement, `replacement_window.rs`), which DCGO asks in
/// the same `MultipleSkills` panel as a trigger order. Any other
/// `EffectChoice` is a choice inside ONE effect.
const REPLACEMENT_ORDER_PROMPT: &str = "Choose the next immediate replacement effect";

/// How many recorded rows a trigger-order lookahead replays under each branch.
const LOOKAHEAD_ROWS: u32 = 8;

/// How many trigger-order lookaheads may nest. Three: in
/// raw-20261113T011922Z#g3 (raw seq 335-347) the three-way order of
/// Gigimon (BT21-001) / Elizamon (EX11-008) / Dimetromon (BT21-017)'s
/// security-removed triggers is told apart only through the two card-less
/// orders that follow it (the remaining two of them, then Medusamon
/// (EX11-012)'s [When Digivolving] vs Owen Dreadnought (EX11-054)), each
/// of which needs a lookahead of its own.
const MAX_LOOKAHEAD_DEPTH: u8 = 3;

/// What a trigger-order lookahead proved (`ReplayDriver::lookahead_trigger_order`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LookaheadVerdict {
    /// The rows prove this entry (`pick_lookahead_branch`).
    Proved(u16),
    /// The rows do not tell the entries apart (or the lookahead may not
    /// nest this deep).
    Undecided,
    /// The rows refute EVERY entry: no order the engine offers leads to
    /// what DCGO recorded. Not an ambiguity - a lookahead one level up
    /// counts it against the branch that led here.
    AllRefuted,
}

/// Whether the permanents of `player` in `slots` are interchangeable: equal
/// in every field but card identity (`card_index`, card handles, the battle
/// identity), and named by NOTHING else in the game, so swapping them
/// changes nothing but which physical copy is which
/// (`ReplayDriver::duplicate_copy_target`).
///
/// "Named" is any reference the game state can hold to one of them, searched
/// in the whole `Game` (`Debug`) outside the permanent itself:
/// * its slot (`PermanentHandle { player, index }`);
/// * ANY card it holds - the top card, the digivolution cards under it and
///   the cards linked to it - as a `CardHandle(n)` or a `card_index: n`;
/// * its battle identity, in every field whose name carries `identity`
///   (`host_identity`, `target_identity`, `source_carrier_identity`,
///   `field_identity`, a count host's `battle_identity`, ...), in
///   `immunity_excluded_source` and in `stable_subject*`, when the value is a
///   bare `u64`, a `Some(n)`, or a tuple whose last number is the identity
///   (the `(player, n)` pair modifiers and grants carry, the `(player, card,
///   n)` triple of `OptionalReplacementState::source_identity`;
///   `identity_references`). A value of any other shape (a struct, a list, a
///   nested enum) is not read. Counters (`next_*`) and install orders
///   (`treated_identity_orders`) are not references.
/// Left out: the prompt being answered (`pending_selection`, which offers every
/// copy alike), the event log (history, not state) and the logger, and each
/// player's per-card bookkeeping of trash / security entries and security
/// effect activations (where a card WAS, not what it is now).
///
/// Not seen (each would make the answer a false "yes"; the corpus only reaches
/// this check after the branch digests and the lookahead failed to tell the
/// copies apart):
/// * a reference that carries the number under a name with none of the
///   markers above and is neither a slot nor a card handle (an unlabelled
///   tuple field); the round-6 review found `stable_subject_card`, now read -
///   others may exist, the search is by field name, not by type;
/// * a reference held by an opaque closure: a modifier's
///   `replacement_condition` / `until_condition`, `DynamicTriggeredGrant::
///   predicate`, a granted effect's body or qualifier capture whatever they
///   like and no text of the game can show it;
/// * a field a hand-written `Debug` omits that holds a reference:
///   `ModifierEntry`'s `finish_non_exhaustive` leaves out only `pending_skips`
///   and the closures, `PlayerModifierEntry`'s only the closure,
///   `PlayCostAdjustments::pending_reduction` (a plain number in a parked
///   declaration; its `Debug` is frozen for the archival verification
///   protocols) and `Permanent::{face_generation, native_play_incarnation,
///   placed_by_effect}` hold no reference (the permanent fields that matter
///   for copy identity are compared as a typed projection,
///   `omitted_debug_fields`).
///
/// This is a text search, not the swap-and-compare-digests test the round-5
/// review proposed, because that test cannot work here: the verification digest
/// hashes every card's `card_index` (`verification.rs` `write_card`), so a swap
/// of two copies changes it whatever refers to them, and it never hashes
/// `battle_identity`, so it is blind to the references that matter most.
fn interchangeable_copies(game: &Game, player: PlayerId, slots: &[u16]) -> bool {
    if !copies_have_one_shape(game, player, slots) {
        return false;
    }
    let mut g = game.clone();
    g.pending_selection = None;
    g.events.clear();
    g.logger = Box::new(crate::logger::SilentLogger);
    for p in &mut g.players {
        p.trash_entry_generations.clear();
        p.security_entry_generations.clear();
        p.security_effect_activations.clear();
    }
    let text = format!("{g:?}");
    let area = &game.player(player).battle_area;
    !slots.iter().any(|j| {
        let Some(perm) = area.get(*j as usize) else {
            return true;
        };
        let own = format!("{perm:?}");
        let slot = format!("PermanentHandle {{ player: {player}, index: {j} }}");
        if text.matches(slot.as_str()).count() > own.matches(slot.as_str()).count() {
            return true;
        }
        let cards_named = perm.card_sources.iter().chain(perm.linked_cards.iter()).any(|c| {
            let h = u64::from(c.handle().0);
            number_references(&text, "CardHandle(", h) > number_references(&own, "CardHandle(", h)
                || number_references(&text, "card_index: ", h) > number_references(&own, "card_index: ", h)
        });
        cards_named
            || perm
                .battle_identity
                .is_some_and(|id| identity_references(&text, id) > identity_references(&own, id))
    })
}

/// How many times `text` holds `prefix` directly followed by exactly the
/// number `value` (not a longer number that starts with it).
fn number_references(text: &str, prefix: &str, value: u64) -> usize {
    let mut n = 0;
    let mut rest = text;
    while let Some(i) = rest.find(prefix) {
        rest = &rest[i + prefix.len()..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if digits.parse::<u64>().ok() == Some(value) {
            n += 1;
        }
    }
    n
}

/// The last element of a tuple body (`tuple` is the text after its opening
/// paren) that is a bare number: the battle identity of `(player, id)` and of
/// `(player, CardHandle(card), id)`. `None` when no element is one.
fn last_tuple_number(tuple: &str) -> Option<u64> {
    let mut depth = 0i32;
    let mut start = 0;
    let mut elements: Vec<&str> = Vec::new();
    for (i, c) in tuple.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                if depth == 0 {
                    elements.push(&tuple[start..i]);
                    break;
                }
                depth -= 1;
            }
            ',' if depth == 0 => {
                elements.push(&tuple[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    elements.iter().rev().find_map(|e| e.trim().parse::<u64>().ok())
}

/// How many times `text` holds a battle-identity reference to `id`: a field
/// named `*identity*`, `immunity_excluded_source` or `stable_subject*`
/// (`stable_subject_card` / `stable_subject` hold `(player, card, identity)`
/// under a name without `identity`) whose value is the bare number,
/// `Some(number)`, or a tuple whose LAST bare-number element is the identity:
/// `(player, number)` and `(player, CardHandle(card), number)`, behind any
/// number of `Some(`. A value of any other shape (a struct, a list, a nested
/// enum) is not read. See `interchangeable_copies`.
fn identity_references(text: &str, id: u64) -> usize {
    fn is_name_char(c: char) -> bool {
        c.is_ascii_lowercase() || c == '_'
    }
    let mut n = 0;
    for needle in ["identity", "immunity_excluded_source", "stable_subject"] {
        let mut from = 0;
        while let Some(found) = text[from..].find(needle) {
            let at = from + found;
            from = at + needle.len();
            let start = text[..at]
                .char_indices()
                .rev()
                .find(|(_, c)| !is_name_char(*c))
                .map_or(0, |(i, c)| i + c.len_utf8());
            let end = at + text[at..].find(|c: char| !is_name_char(c)).unwrap_or(text.len() - at);
            let name = &text[start..end];
            if name.starts_with("next_") || name == "treated_identity_orders" {
                continue;
            }
            let Some(mut value) = text[end..].strip_prefix(": ") else {
                continue;
            };
            while let Some(inner) = value.strip_prefix("Some(") {
                value = inner;
            }
            let number = |s: &str| s.chars().take_while(char::is_ascii_digit).collect::<String>().parse::<u64>().ok();
            let referenced = match value.strip_prefix('(') {
                // `(player, id)` or `(player, CardHandle(card), id)`: the last number
                Some(tuple) => last_tuple_number(tuple),
                None => number(value),
            };
            if referenced == Some(id) {
                n += 1;
            }
        }
    }
    n
}

/// Whether engine state outside the card itself names `handle` (a modifier's
/// source, a pending declaration, a granted effect, a receipt, ...): a
/// `CardHandle(n)` or `card_index: n` anywhere in the game's `Debug` text beyond
/// the card's own entry (`number_references`). A card the game does not hold
/// is reported named: nothing can be vouched for. The prompt being answered,
/// the event log and the logger are left out, as in `interchangeable_copies`.
fn card_named_by_engine_state(game: &Game, handle: crate::card_source::CardHandle) -> bool {
    let value = u64::from(handle.0);
    let all_cards = || {
        game.players
            .iter()
            .flat_map(|p| {
                p.hand
                    .iter()
                    .chain(p.deck.iter())
                    .chain(p.security.iter())
                    .chain(p.trash.iter())
                    .chain(p.digitama_deck.iter())
                    .chain(p.battle_area.iter().flat_map(|perm| perm.card_sources.iter().chain(perm.linked_cards.iter())))
                    .chain(p.breeding_area.iter().flat_map(|perm| perm.card_sources.iter().chain(perm.linked_cards.iter())))
            })
            .chain(game.revealed_cards.iter())
    };
    let Some(card) = all_cards().find(|c| c.handle() == handle) else {
        return true;
    };
    let mut g = game.clone();
    g.pending_selection = None;
    g.events.clear();
    g.logger = Box::new(crate::logger::SilentLogger);
    let (text, own) = (format!("{g:?}"), format!("{card:?}"));
    ["CardHandle(", "card_index: "]
        .iter()
        .any(|prefix| number_references(&text, prefix, value) > number_references(&own, prefix, value))
}

/// The fields of one permanent that `Permanent`'s hand-written `Debug` leaves
/// out and that tell two copies apart or a forced run's trace: `face_generation`
/// (which face incarnation a pending face effect belongs to) and
/// `placed_by_effect` (the breeding-area rule-check exemption). Not
/// `native_play_incarnation`: an opaque harness marker, left out of every replay
/// hash on purpose (permanent.rs).
fn permanent_omitted_fields(perm: &crate::permanent::Permanent) -> String {
    format!("face_generation={} placed_by_effect={}", perm.face_generation, perm.placed_by_effect)
}

/// A typed projection of the state the replay's `{game:?}` comparisons cannot
/// see because a hand-written `Debug` omits it: every permanent's
/// `permanent_omitted_fields` and the modifier entries' omitted fields
/// (`ModifierRegistry::debug_omitted_fields`). Appended to the text digests of
/// `effect_footprint_parts`; not used for reference searches (the omitted
/// fields hold no reference; see `interchangeable_copies`).
fn omitted_debug_fields(game: &Game) -> String {
    let mut out = String::new();
    for (p, player) in game.players.iter().enumerate() {
        for (i, perm) in player.battle_area.iter().enumerate() {
            out.push_str(&format!("{p}.battle[{i}] {}\n", permanent_omitted_fields(perm)));
        }
        for (i, perm) in player.breeding_area.iter().enumerate() {
            out.push_str(&format!("{p}.breeding[{i}] {}\n", permanent_omitted_fields(perm)));
        }
    }
    out.push_str(&game.modifiers.debug_omitted_fields());
    out
}

/// The shape test of `interchangeable_copies`: the permanents in `slots` are
/// equal in every field but card identity.
fn copies_have_one_shape(game: &Game, player: PlayerId, slots: &[u16]) -> bool {
    // Blank the digits after each identity marker.
    fn blank_identity(text: &str) -> String {
        const MARKERS: [&str; 3] = ["card_index: ", "CardHandle(", "battle_identity: Some("];
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while !rest.is_empty() {
            let next = MARKERS.iter().filter_map(|m| rest.find(m).map(|i| (i, m.len()))).min();
            let Some((i, len)) = next else {
                out.push_str(rest);
                break;
            };
            out.push_str(&rest[..i + len]);
            rest = rest[i + len..].trim_start_matches(|c: char| c.is_ascii_digit());
            out.push('_');
        }
        out
    }
    let area = &game.player(player).battle_area;
    let shape = |j: u16| {
        area.get(j as usize).map(|p| format!("{}|{}", blank_identity(&format!("{p:?}")), permanent_omitted_fields(p)))
    };
    let Some(first) = slots.first().and_then(|j| shape(*j)) else {
        return false;
    };
    slots.iter().all(|j| shape(*j).as_ref() == Some(&first))
}

/// Where a card is, for telling which cards a recorded row moved
/// (`ReplayDriver::record_dcgo_card_indexes`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CardZone {
    Hand,
    Deck,
    Security,
    Trash,
    DigitamaDeck,
    /// In a battle-area permanent: whether it is the top card. Not the slot:
    /// a permanent leaving shifts the slots of the others, which moves none
    /// of their cards.
    Battle { top: bool },
    /// In the breeding-area permanent: whether it is the top card.
    Breeding { top: bool },
    Revealed,
}

/// Every card of both players with its zone (`CardZone`).
fn card_zones(game: &Game) -> HashMap<crate::card_source::CardHandle, (PlayerId, CardZone)> {
    let mut out = HashMap::new();
    for p in &game.players {
        let mut put = |cards: &[crate::card_source::CardSource], z: CardZone| {
            for c in cards {
                out.insert(c.handle(), (p.id, z));
            }
        };
        put(&p.hand, CardZone::Hand);
        put(&p.deck, CardZone::Deck);
        put(&p.security, CardZone::Security);
        put(&p.trash, CardZone::Trash);
        put(&p.digitama_deck, CardZone::DigitamaDeck);
        for perm in &p.battle_area {
            let n = perm.card_sources.len();
            for (at, c) in perm.card_sources.iter().enumerate() {
                out.insert(c.handle(), (p.id, CardZone::Battle { top: at + 1 == n }));
            }
        }
        if let Some(perm) = &p.breeding_area {
            let n = perm.card_sources.len();
            for (at, c) in perm.card_sources.iter().enumerate() {
                out.insert(c.handle(), (p.id, CardZone::Breeding { top: at + 1 == n }));
            }
        }
    }
    for c in &game.revealed_cards {
        out.insert(c.handle(), (c.owner, CardZone::Revealed));
    }
    out
}

/// A DCGO per-card "from which zone?" menu loop that the engine asks as ONE
/// up-to-2 pick over the hand and the trash (`ReplayDriver::zone_menu_loop_fold`).
struct ZoneMenuLoop {
    card: &'static str,
    /// The most cards one pass may pick: `None` is every card still left
    /// (`2 - chosen`).
    per_pass: Option<usize>,
    /// DCGO leaves the loop without a menu once no card is choosable.
    ends_without_candidates: bool,
    /// A later pass cannot pick a card an earlier pass chose, so an id
    /// named again in the same zone is a second copy.
    excludes_chosen: bool,
    /// Where DCGO asks the menu (for messages).
    menu_cite: &'static str,
    /// Where DCGO asks each pick (for messages).
    pick_cite: &'static str,
}

/// Every card `zone_menu_loop_fold` answers. Menu values are DCGO's
/// `SelectionElement<int>` constants (from Hand = 1, from Trash = 2,
/// Don't / Do not place = 3), never positions.
const ZONE_MENU_LOOPS: &[ZoneMenuLoop] = &[
    ZoneMenuLoop {
        card: "EX12-018",
        per_pass: None,
        ends_without_candidates: false,
        excludes_chosen: false,
        menu_cite: "EX12_018.cs:87-107",
        pick_cite: "`maxCount = 2 - chosen`, EX12_018.cs:109-162",
    },
    ZoneMenuLoop {
        card: "AD1-023",
        per_pass: Some(1),
        ends_without_candidates: true,
        excludes_chosen: true,
        menu_cite: "AD1_023.cs:80-108",
        pick_cite: "`maxCount: 1`, AD1_023.cs:111-162",
    },
];

/// The verdict of a lookahead's branch runs, once nested all-refuted orders
/// are settled (`settle_nested_refutations`): every branch refuted is
/// `AllRefuted`; otherwise `pick_lookahead_branch` decides.
fn lookahead_verdict(runs: &[LookaheadRun]) -> LookaheadVerdict {
    let runs = settle_nested_refutations(runs);
    if !runs.is_empty() && runs.iter().all(|r| r.refuted_at.is_some()) {
        return LookaheadVerdict::AllRefuted;
    }
    match pick_lookahead_branch(&runs) {
        Some(id) => LookaheadVerdict::Proved(id),
        None => LookaheadVerdict::Undecided,
    }
}

/// One branch of a trigger-order lookahead (`ReplayDriver::lookahead_trigger_order`).
#[derive(Debug, Clone)]
struct LookaheadRun {
    /// The order-prompt entry the branch chose.
    entry: u16,
    /// `(cursor after the row, verification_digest)` for every row applied.
    seen: Vec<(u32, u64)>,
    /// The cursor the branch stopped at: every row before it was applied.
    reached: u32,
    /// The row of the first divergence that refutes the branch (anything but
    /// a nested ambiguity or a nested all-refuted order), if any.
    refuted_at: Option<u32>,
    /// The row of a nested order whose every entry the rows refuted
    /// (`REFUTED_ORDER`), when nothing else refuted the branch. See
    /// `settle_nested_refutations`.
    nested_refuted_at: Option<u32>,
}

/// Settle each branch stopped by a nested order whose every entry the rows
/// refuted. It is refuted at that row when another branch, refuted by
/// nothing, replayed past it: the rows there are passable, so no way on
/// from this branch fits DCGO's game. Otherwise the same rows may stop
/// every branch (an engine fault further on, not this order), so it only
/// ends that branch's evidence, as a nested ambiguity does.
fn settle_nested_refutations(runs: &[LookaheadRun]) -> Vec<LookaheadRun> {
    let clean: Vec<u32> = runs
        .iter()
        .filter(|r| r.refuted_at.is_none() && r.nested_refuted_at.is_none())
        .map(|r| r.reached)
        .collect();
    runs.iter()
        .map(|r| {
            let mut r = r.clone();
            if let (None, Some(at)) = (r.refuted_at, r.nested_refuted_at) {
                if clean.iter().any(|reached| *reached > at) {
                    r.refuted_at = Some(at);
                }
            }
            r
        })
        .collect()
}

/// Which entry a trigger-order lookahead proves, from its branch runs.
///
/// * Several unrefuted branches: the first, when every one of them reaches the
///   same `verification_digest` after the same row (the order is not
///   observable).
/// * Exactly one unrefuted branch: it stands only if it replayed at least as
///   far as EVERY row that refuted the others (it applied that row itself).
///   A branch that stopped earlier - on a nested ambiguity, or a row that did
///   not advance - was never tested by the row that refuted the others, so
///   its survival proves nothing.
/// * Otherwise (every branch refuted, or the survivor fell short): `None`,
///   and the caller reports the honest ambiguity.
fn pick_lookahead_branch(runs: &[LookaheadRun]) -> Option<u16> {
    let standing: Vec<&LookaheadRun> = runs.iter().filter(|r| r.refuted_at.is_none()).collect();
    match standing.as_slice() {
        [] => None,
        [only] => runs
            .iter()
            .filter_map(|r| r.refuted_at)
            .all(|row| only.reached > row)
            .then_some(only.entry),
        [first, rest @ ..] => first
            .seen
            .iter()
            .any(|(cursor, digest)| rest.iter().all(|r| r.seen.iter().any(|(c, d)| c == cursor && d == digest)))
            .then_some(first.entry),
    }
}

/// The fingerprint two candidate branches are compared by (the trigger-order
/// lookahead, `remote_trigger_order_choice`, `probe_branch_for_row`):
/// `Game::verification_digest` plus the optional-ACTIVATION mark of the parked
/// prompt (`Game::pending_optional_activation_effect`).
///
/// The mark is invisible to the digest, but it changes how the replay reads
/// the next unstamped DCGO "use this optional effect?" answer
/// (`dcgo_use_optional_answer`): with it, a `false` is the decline (PASS) of
/// that prompt; without it, the same row halts or becomes a note
/// (`classify_unstamped_answer`). Two branches that differ only in the mark
/// are therefore not the same replay state. Kept out of
/// `verification_digest` itself, whose value native recordings pin as
/// `expected_digest`.
fn branch_digest(game: &Game) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    game.verification_digest().hash(&mut h);
    format!("{:?}", game.pending_optional_activation_effect()).hash(&mut h);
    h.finish()
}

/// The byte ranges of every card-identity number in the `Debug` text of a game,
/// by namespace: `0` the card handle / `card_index` (`CardHandle(n)`,
/// `card_index: n`), `1` the battle identity (any field `identity_references`
/// reads, in the same shapes). Spans may not overlap and are returned in text
/// order.
fn identity_spans(text: &str) -> Vec<(usize, usize, u8)> {
    fn digits_at(text: &str, from: usize) -> Option<(usize, usize)> {
        let len = text[from..].chars().take_while(char::is_ascii_digit).count();
        (len > 0).then_some((from, from + len))
    }
    let mut spans: Vec<(usize, usize, u8)> = Vec::new();
    for prefix in ["CardHandle(", "card_index: "] {
        let mut from = 0;
        while let Some(found) = text[from..].find(prefix) {
            let at = from + found + prefix.len();
            from = at;
            if let Some((s, e)) = digits_at(text, at) {
                spans.push((s, e, 0));
            }
        }
    }
    fn is_name_char(c: char) -> bool {
        c.is_ascii_lowercase() || c == '_'
    }
    for needle in ["identity", "immunity_excluded_source", "stable_subject"] {
        let mut from = 0;
        while let Some(found) = text[from..].find(needle) {
            let at = from + found;
            from = at + needle.len();
            let start = text[..at]
                .char_indices()
                .rev()
                .find(|(_, c)| !is_name_char(*c))
                .map_or(0, |(i, c)| i + c.len_utf8());
            let end = at + text[at..].find(|c: char| !is_name_char(c)).unwrap_or(text.len() - at);
            let name = &text[start..end];
            if name.starts_with("next_") || name == "treated_identity_orders" {
                continue;
            }
            let Some(rest) = text[end..].strip_prefix(": ") else {
                continue;
            };
            let mut value = end + 2;
            let mut rest = rest;
            while let Some(inner) = rest.strip_prefix("Some(") {
                value += "Some(".len();
                rest = inner;
            }
            if let Some(tuple) = rest.strip_prefix('(') {
                // `(player, id)` / `(player, CardHandle(card), id)`: the last
                // bare-number element is the identity.
                let base = value + 1;
                let (mut depth, mut elem_start) = (0i32, 0usize);
                let mut elems: Vec<(usize, usize)> = Vec::new();
                for (i, c) in tuple.char_indices() {
                    match c {
                        '(' | '[' | '{' => depth += 1,
                        ')' | ']' | '}' => {
                            if depth == 0 {
                                elems.push((elem_start, i));
                                break;
                            }
                            depth -= 1;
                        }
                        ',' if depth == 0 => {
                            elems.push((elem_start, i));
                            elem_start = i + 1;
                        }
                        _ => {}
                    }
                }
                let number = elems.iter().rev().find_map(|(s, e)| {
                    let raw = &tuple[*s..*e];
                    let trimmed = raw.trim();
                    (!trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit()))
                        .then(|| (base + s + (raw.len() - raw.trim_start().len()), base + s + (raw.len() - raw.trim_start().len()) + trimmed.len()))
                });
                if let Some((s, e)) = number {
                    spans.push((s, e, 1));
                }
            } else if let Some((s, e)) = digits_at(text, value) {
                spans.push((s, e, 1));
            }
        }
    }
    spans.sort_unstable();
    spans.dedup();
    spans
}

/// `text` with every card-identity number (`identity_spans`) renamed to its
/// order of first appearance in its namespace: two texts are equal afterwards
/// exactly when one maps onto the other by a consistent relabelling of cards
/// and battle identities - the same game up to which physical copy is which.
fn alpha_rename_identities(text: &str) -> String {
    let spans = identity_spans(text);
    let mut seen: [HashMap<&str, usize>; 2] = [HashMap::new(), HashMap::new()];
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for (s, e, ns) in spans {
        if s < at {
            continue;
        }
        out.push_str(&text[at..s]);
        let map = &mut seen[ns as usize];
        let next = map.len();
        let label = *map.entry(&text[s..e]).or_insert(next);
        out.push_str(&format!("#{ns}.{label}"));
        at = e;
    }
    out.push_str(&text[at..]);
    out
}

/// The fingerprint two branches are compared by when the order entries they
/// came from are copies of ONE card (`branch_digest`'s state, with the physical
/// identity of every card - and every battle identity - renamed away): two
/// Tamers of one card id with the same state each queue the same trigger, the
/// recording cannot say which copy DCGO activated, and the branches differ only
/// in which physical copy went where (the verification digest hashes every
/// card's `card_index`, `verification.rs` `write_card`). Positions stay: a
/// slot, a zone order and a stack order are not renamed, so a choice that
/// leaves the copies in different places still differs.
///
/// Built from the game's `Debug` text, like `interchangeable_copies` (the same
/// limits: a reference the text does not show is not seen), plus the fields a
/// hand-written `Debug` omits (`omitted_debug_fields`). The event log and the
/// logger are history, not state, and are left out. The pinned
/// `verification_digest` itself is never touched.
fn identity_free_digest(game: &Game) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut g = game.clone();
    g.events.clear();
    g.logger = Box::new(crate::logger::SilentLogger);
    // Static registries: shared by every branch (clones of one game) and by far
    // the largest part of the text (the DSL alternate-path table alone is
    // megabytes); nothing a choice between copies can change.
    g.token_registry = Default::default();
    g.card_id_index = Default::default();
    #[cfg(feature = "dsl-yaml-loader")]
    {
        g.alt_path_registry = Default::default();
    }
    // A queued trigger's `trigger_context` keeps the slot its carrier had when
    // it was queued; only `source_permanent` is rebased when an earlier slot
    // empties (`shift_effect_queue_after_battle_area_remove`). Two branches
    // that removed DIFFERENT copies of one card then hold the same survivor
    // under different stale slots. Name the permanent by the card the context
    // was taken with, as the carrier of that card now stands.
    for queued in g.effect_queue.iter_mut() {
        let Some(ctx) = queued.trigger_context.as_mut() else {
            continue;
        };
        let card = if ctx.target_card.is_unavailable() { None } else { *ctx.target_card };
        if let (Some(handle), Some(card)) = (ctx.target_permanent, card) {
            let area = g.players.get(handle.player as usize).map(|p| &p.battle_area);
            let holds = |slot: usize| area.and_then(|a| a.get(slot)).is_some_and(|p| p.card_sources.iter().any(|c| c.handle() == card));
            if !holds(handle.index as usize) {
                if let Some(now) = area.and_then(|a| a.iter().position(|p| p.card_sources.iter().any(|c| c.handle() == card))) {
                    if let Some(target) = ctx.target_permanent.as_mut() {
                        target.index = now as u8;
                    }
                }
            }
        }
    }
    // Per-card history the engine only reads for something else, which the
    // two copies' pasts leave different (which copy was revealed from security
    // last, when a card entered the trash or security stack, and the turn a
    // Digimon or Tamer entered play - read only as "is it this turn",
    // `perm.turn_played == turn_count`: the freshness test of the attack mask
    // and of `combat/mod.rs`). Two Tamers played on different earlier turns are
    // the same card in the same state. A Delay Option's `turn_played` is a
    // different matter (`game_phases.rs` keys on it), so only Digimon and
    // Tamers are normalised.
    let turn = g.turn_count;
    {
        let data = &g.card_data;
        for player in g.players.iter_mut() {
            player.last_security_reveal = None;
            player.trash_entry_generations.clear();
            player.security_entry_generations.clear();
            for perm in player.battle_area.iter_mut() {
                let top = perm.top_card();
                if (top.is_digimon(data) || top.is_tamer(data)) && perm.turn_played != turn {
                    perm.turn_played = 0;
                }
            }
        }
    }
    let mut text = blank_declarative_memo(&format!("{g:?}"));
    text.push_str(&omitted_debug_fields(&g));
    // The optional-activation mark, as `branch_digest` reads it: renamed with
    // the rest, so the identity of the copy it names is not compared.
    text.push_str(&format!("
mark={:?}", game.pending_optional_activation_effect()));
    let mut h = std::collections::hash_map::DefaultHasher::new();
    alpha_rename_identities(&text).hash(&mut h);
    h.finish()
}

/// `text` without the declarative-effect memo of the modifier registry
/// (`decl_memo_valid`, `decl_last_fingerprint`): a cache of a hash over handles
/// and play turns, not state.
fn blank_declarative_memo(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while !rest.is_empty() {
        let next = ["decl_last_fingerprint: ", "decl_memo_valid: "]
            .iter()
            .filter_map(|m| rest.find(m).map(|i| (i, m.len())))
            .min();
        let Some((i, len)) = next else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..i + len]);
        rest = rest[i + len..].trim_start_matches(|c: char| c.is_ascii_alphanumeric());
        out.push('_');
    }
    out
}

/// `branch_digest`, or - when `identity_free` - `identity_free_digest`: the
/// comparison of branches that chose between copies of one card.
fn order_branch_digest(game: &Game, identity_free: bool) -> u64 {
    if identity_free {
        identity_free_digest(game)
    } else {
        branch_digest(game)
    }
}

/// Whether two or more entries of the parked order prompt are copies of ONE
/// card: the same effect (same label) of two DIFFERENT physical cards of one
/// card id. The branches they lead to are compared by `identity_free_digest`.
/// Entries that share a source card (the inherited effects of one carrier's
/// stack are all queued under its top card) are not copies of anything.
fn entries_are_copies_of_one_card(game: &Game) -> bool {
    let Some(entries) = game.pending_selection.as_ref().and_then(|p| p.effect_choices.as_ref().map(|e| (p, e))) else {
        return false;
    };
    let (pending, entries) = entries;
    let rows: Vec<(&str, crate::card_source::CardHandle, &str)> = entries
        .iter()
        .filter(|e| pending.valid_action_ids.contains(&e.action_id))
        .filter_map(|e| {
            let card = e.source_card?;
            Some((e.label.as_str(), card, game.card_data_for_handle(card)?.card_id.as_str()))
        })
        .collect();
    rows.iter().enumerate().any(|(i, (label, card, id))| {
        rows[i + 1..].iter().any(|(l, c, other)| l == label && c != card && other == id)
    })
}

/// Cards whose DCGO script picks the card FIRST and THEN asks a
/// `SetBoolSelection` that names the branch the engine asks first, as
/// `(card, engine label index of true, engine label index of false)`.
/// Gazimon (BT25-078): `SimplifiedRevealDeckTopCardsAndSelect` picks the
/// revealed card, then, for a [Three Musketeers] trait card, asks "Add to
/// hand" = true / "Place on digivolution cards" = false (BT25_078.cs:39-84);
/// its YAML lists "Add ... to hand" as label 0 and "Place ... as bottom
/// digivolution card" as label 1 (BT25-078.yaml `select_effect_choice`). The
/// bool is DCGO's very next row for that player. Add a card only after
/// reading its C# and YAML.
fn dcgo_pick_then_branch_bool(card_id: &str) -> Option<(u16, u16)> {
    match card_id {
        "BT25-078" => Some((0, 1)),
        _ => None,
    }
}

/// What a stamped DCGO bool in front of a union pick over the hand and another
/// zone is (`ReplayDriver::hand_zone_bool`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZoneBool {
    /// The card's "From hand" / "From trash" menu: the next row is the pick.
    Menu,
    /// The card's "do it?" bool; `menu_next`: the zone menu is the next row.
    Gate { menu_next: bool },
}

/// DCGO's `MultipleSkills` -1, "Don't activate these effects", applied to the
/// engine's trigger stack: the game after it, and the last engine action.
///
/// DCGO offers -1 only when every stacked effect is skippable
/// (MultipleSkills.cs:393, `_CanNoSelect: skillInfos_active.All(IsSkippable)`)
/// and then drops the whole stack (MultipleSkills.cs:474-480). The engine
/// resolves the stack one trigger at a time: an OPTIONAL order prompt's own
/// PASS is the decline-all; an optional prompt a chosen trigger opens is
/// declined (PASS); a mandatory prompt that is not an order is left for the
/// following rows. A NON-optional order (the engine holds a trigger it treats
/// as mandatory, or one whose cost sits in front of its "may" - BT17-081's "by
/// suspending this Tamer, gain memory") has to be picked, and DCGO recorded no
/// order for it: every order is tried, and it is applied only when all of them
/// lead to the same game. Otherwise the order changes the outcome and the
/// recording does not say which one ran, so this is an error.
///
/// -1 drops ONE stack: the `StackedSkillInfos` of the `ActivateMultipleSkills`
/// call it answers, built from that call's own `skillInfos`
/// (MultipleSkills.cs:166-171, cleared at :474-480). Triggers that were
/// waiting UNDER it (the outer call that was resolving one of its effects when
/// this stack arose, AutoProcessing.cs:584-600) are not in that list and are
/// asked next. So the decline stops at the first trigger order that holds an
/// effect the first order did not: that is the stack under it, and the rows
/// that follow answer it (raw-20261113T025510Z#g6 seq 1006-1007: -1 on two
/// Marcus Damon & Thomas H. Norstein (ST24-13) triggers, then Kudamon
/// (EX13-026) chosen from the attack's stack).
///
/// -1 skips the stack's effects ENTIRELY, a non-optional one included: DCGO
/// offers it when every effect `IsSkippable` (MultipleSkills.cs:393), which a
/// `SetIsSkippable(true)` effect is whatever its `isOptional`
/// (ICardEffect.cs:364-367; Siriusmon (EX12-018), Hiro Amanokawa (EX12-066)).
/// An order entry the engine forces therefore runs an effect DCGO skipped.
/// The decline stands only when that run provably left NOTHING behind, judged
/// against the game DCGO's -1 leaves (`decline_reference`: this game with the
/// order's own entries gone and no prompt parked) - and then with a note (the
/// third value); anything else is an error. "Nothing" is, in `forced_run_residue`:
/// * no prompt still parked, except an order over the entries that were
///   already waiting under the stack (the outer stack: DCGO asks it next);
/// * the trigger queue is exactly the one under the stack (no trigger the
///   run queued, none the engine went on to resolve);
/// * no effect activity of the stack's owner in the events (`effect_activity`);
/// * every part of `effect_footprint_parts` equal to the reference: the
///   players (zones, permanents, per-card bookkeeping), memory, modifiers,
///   floating modifiers, revealed cards, the player's digivolution cost
///   reducers, the granted effects, the attack / security check / option in
///   progress, the replacement state and the phase, turn and result.
///
/// Not seen (a measured limit, not a proof): state outside those fields, and
/// effect activity the event log does not carry (see `effect_activity`).
fn dcgo_decline_all(game: &Game, depth: u8) -> Result<(Game, u16, Option<String>), String> {
    let owner = game.pending_selection.as_ref().map(|p| p.selecting_player).unwrap_or_default();
    let (g, last, forced) = dcgo_decline_stack(game, depth, owner, None, false)?;
    if !forced {
        return Ok((g, last, None));
    }
    let reference = decline_reference(game);
    let mut left = effect_activity(g.events.get(game.events.len()..).unwrap_or_default(), owner);
    left.extend(forced_run_residue(&reference, &g));
    if !left.is_empty() {
        return Err(forced_run_error(&format!("that run did what DCGO skipped ({})", left.join("; "))));
    }
    Ok((
        g,
        last,
        Some(
            "MultipleSkills -1: the engine ran the stack's forced trigger(s) DCGO skipped (a skippable non-optional \
             effect, ICardEffect.cs:364-367), and they left nothing behind - no effect activity, no prompt or \
             queued trigger, no change to the zones, memory, permanents, modifiers, cost reducers, granted effects, \
             attack or replacement state - so the game is the one DCGO's -1 left"
                .to_string(),
        ),
    ))
}

/// The error of a -1 whose forced trigger ran on: `what` says how.
fn forced_run_error(what: &str) -> String {
    format!(
        "MultipleSkills -1 (\"Don't activate these effects\": DCGO skipped every stacked effect, \
         MultipleSkills.cs:474-480, a non-optional one included when it is skippable, ICardEffect.cs:364-367) \
         meets a trigger the engine holds as mandatory and runs, and {what}: the engine's game is not DCGO's"
    )
}

/// The `effect_queue` indices the parked trigger-order prompt of `game` offers.
fn trigger_order_bundle(game: &Game) -> Option<Vec<usize>> {
    match game.pending_selection_resume.as_ref()?.frames.first()? {
        crate::resume::ResumeFrame::TriggerOrderSelection(state) => Some(state.bundle.clone()),
        _ => None,
    }
}

/// The game DCGO's -1 leaves, as far as the engine can state it without
/// running anything: `game` with the parked order's own entries dropped from
/// the trigger queue (DCGO's -1 dropped that stack, MultipleSkills.cs:474-480)
/// and no prompt parked. The entries waiting under the stack stay queued.
fn decline_reference(game: &Game) -> Game {
    let mut reference = game.clone();
    if let Some(mut bundle) = trigger_order_bundle(game) {
        bundle.sort_unstable();
        for i in bundle.into_iter().rev() {
            reference.effect_queue.remove(i);
        }
    }
    if let Some(p) = reference.pending_selection.take() {
        reference.current_phase = p.previous_phase;
    }
    reference.pending_selection_resume = None;
    reference
}

/// The waiting trigger queue of `game`, sorted: each entry's source card,
/// source kind, timing, controller, slot, optionality and granted-body id.
fn queue_keys(game: &Game) -> Vec<String> {
    let mut keys: Vec<String> = game
        .effect_queue
        .iter()
        .map(|q| {
            format!(
                "{:?}",
                (
                    q.source_card,
                    q.source_kind,
                    q.timing,
                    q.controller,
                    q.effect_slot,
                    q.is_optional,
                    q.granted_effect_id,
                    q.dna_origin_context,
                )
            )
        })
        .collect();
    keys.sort();
    keys
}

/// What a forced trigger's run left behind that DCGO's skip of it would not
/// have (`dcgo_decline_all`): `after` is the game after the decline,
/// `reference` the game DCGO's -1 leaves (`decline_reference`). Empty when the
/// run left nothing.
fn forced_run_residue(reference: &Game, after: &Game) -> Vec<String> {
    let mut left = Vec::new();
    // A prompt still parked: only an order over entries that were already
    // waiting (the outer stack, or the other player's) is DCGO's next
    // question; the queue comparison below proves the entries are those.
    match after.pending_selection.as_ref() {
        None => {
            if after.pending_selection_resume.is_some() {
                left.push("a selection continuation is still parked".to_string());
            }
        }
        Some(p) if matches!(p.kind, crate::selection::SelectionKind::TriggerOrder) => {}
        Some(p) => left.push(format!("P{}'s '{}' is still parked (the run is still going)", p.selecting_player, p.prompt)),
    }
    let (want, have) = (queue_keys(reference), queue_keys(after));
    if want != have {
        left.push(format!(
            "the trigger queue is not the one under the stack ({} waiting, {} expected): the run queued a trigger or \
             the engine went on to resolve one",
            have.len(),
            want.len()
        ));
    }
    let (r, a) = (effect_footprint_parts(reference), effect_footprint_parts(after));
    for ((label, before), (_, now)) in r.iter().zip(a.iter()) {
        if before != now {
            left.push(format!("{label} changed"));
        }
    }
    left
}

/// What a forced trigger may change that DCGO's skip of it would not, one
/// fingerprint per kind (`forced_run_residue` names the kind that differs).
/// The event log, the logger and the order prompt itself are left out: the
/// prompt and the stack's queue entries leaving are the run itself, judged
/// against `decline_reference`.
///
/// "No known false accept within the fields it reads", not a proof. Fields
/// read (round-6 review audit of `Game`'s fields): the players (zones,
/// permanents), memory, modifiers, floating mass modifiers, revealed cards,
/// `scheduled_effects`, `scheduled_provenance_deletions(_opp)`,
/// `pending_added_sources`, `active_digixros_wildcards`,
/// `effect_digivolve_scopes`, `effect_dna_would_scopes`, the player digivolve
/// cost reducers, the granted-effect registry (bodies, qualifier ids, next id,
/// pending fires), the attack / security / option in progress, the replacement
/// state, the turn counters (`n_digivolutions`, `n_dna_digivolutions`,
/// `n_digivolve_driven_attacks`, `digimon_attacks_this_turn`, the turn
/// player), `public_security_reveals`, `security_shuffle_generations`, the
/// random state (one draw from a clone of `rng`), the phase / turn / result,
/// plus a typed projection of the fields hand-written `Debug` impls omit
/// (`omitted_debug_fields`).
///
/// Audited and NOT read, with the reason: the receipts and `next_*_receipt`
/// counters (bookkeeping of an effect already visible in the zones),
/// `pending_*` slots that only live inside one effect's synchronous run
/// (`pending_security_moves`, `pending_hand_discard`, ...: empty between
/// prompts, and a parked prompt is judged separately), `parked_overclock_attacks`
/// / `declined_overclock_this_eot` / `end_of_turn_attack_*` (an Overclock offer
/// shows as a parked prompt or a changed attack), `loop_watch*` and the
/// optional-activation marks (loop-guard memory), `native_play_observation`
/// (harness), the card store / registries / token registry / formula
/// extensions (static for the game), and `PlayCostAdjustments::
/// pending_reduction` (inside a parked declaration; its `Debug` is frozen).
/// Opaque closures (`DynamicTriggeredGrant::predicate`, a granted body) show
/// only as presence / count.
fn effect_footprint_parts(game: &Game) -> Vec<(&'static str, u64)> {
    use std::hash::{Hash, Hasher};
    fn digest(text: String) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut h);
        h.finish()
    }
    use rand::RngCore;
    let mut fired: Vec<String> = game.replacement_fired.iter().map(|f| format!("{f:?}")).collect();
    fired.sort();
    // `GrantedEffectBodyRegistry`'s `Debug` prints only the body COUNT: the ids and the qualifier ids are read here.
    let (mut body_ids, mut qualifier_ids): (Vec<u64>, Vec<u64>) =
        (game.granted_effect_bodies.bodies.keys().copied().collect(), game.granted_effect_bodies.qualifiers.keys().copied().collect());
    body_ids.sort_unstable();
    qualifier_ids.sort_unstable();
    let random_state = game.rng.clone().next_u64();
    vec![
        (
            "the zones, memory, permanents or modifiers",
            digest(format!(
                "{:?}|{}|{:?}|{:?}|{:?}|{}",
                game.players,
                game.memory,
                game.modifiers,
                game.floating_mass_modifiers,
                game.revealed_cards,
                omitted_debug_fields(game)
            )),
        ),
        ("the player's digivolution cost reducers", digest(format!("{:?}", game.player_digivolve_cost_reducers))),
        (
            "the granted effects",
            digest(format!(
                "{:?}|{}|{:?}|{body_ids:?}|{qualifier_ids:?}",
                game.granted_effect_bodies, game.next_granted_effect_id, game.pending_granted_fires
            )),
        ),
        (
            "the scheduled (delayed) effects",
            digest(format!(
                "{:?}|{:?}|{:?}",
                game.scheduled_effects, game.scheduled_provenance_deletions, game.scheduled_provenance_deletions_opp
            )),
        ),
        (
            "the added digivolution cards, DigiXros wildcards or digivolution declarations in progress",
            digest(format!(
                "{:?}|{:?}|{:?}|{:?}",
                game.pending_added_sources,
                game.active_digixros_wildcards,
                game.effect_digivolve_scopes,
                game.effect_dna_would_scopes
            )),
        ),
        (
            "the turn counters",
            digest(format!(
                "{:?}|{:?}|{:?}|{:?}|{}",
                game.n_digivolutions,
                game.n_dna_digivolutions,
                game.n_digivolve_driven_attacks,
                game.digimon_attacks_this_turn,
                game.turn_player_idx
            )),
        ),
        (
            "the public security reveals or shuffles",
            digest(format!("{:?}|{:?}", game.public_security_reveals, game.security_shuffle_generations)),
        ),
        ("the random state", digest(format!("{random_state}"))),
        (
            "the attack, security check or option in progress",
            digest(format!(
                "{:?}|{:?}|{:?}|{:?}",
                game.pending_attack, game.pending_security, game.security_resolution, game.pending_option
            )),
        ),
        ("the replacement state", digest(format!("{fired:?}|{:?}", game.after_selection_resume_hooks))),
        (
            "the phase, turn or result",
            digest(format!("{:?}|{}|{}|{:?}", game.current_phase, game.turn_count, game.game_over, game.winner)),
        ),
    ]
}

/// The identity of a trigger-order entry: its source card and timing.
type TriggerKey = (Option<crate::card_source::CardHandle>, Option<crate::enums::EffectTiming>);

fn trigger_keys(p: &crate::selection::PendingSelection) -> Vec<TriggerKey> {
    p.effect_choices
        .iter()
        .flatten()
        .filter(|e| p.valid_action_ids.contains(&e.action_id))
        .map(|e| (e.source_card, e.timing))
        .collect()
}

/// [`dcgo_decline_all`] within the stack `stack` (the entries of the first
/// trigger order met; `None` until one is). The third value: whether an
/// order entry the engine forces was run (`forced`: already so for a branch
/// that just ran one).
///
/// `owner` is the player whose order the -1 answers, fixed for the whole
/// decline (a branch that just ran a forced entry meets prompts of other
/// players, and must not take the first of them for its own panel).
///
/// The decline holds to the panel DCGO answered:
/// * it stops at an order of another player (-1 answers one player's panel;
///   `forced_run_residue` checks the order was already waiting); after a
///   forced entry ran, any OTHER prompt of another player is that run still
///   going: an error;
/// * when the first prompt it declines is not an order (a lone "you may"
///   gate), the next trigger order is a stack DCGO's panel never held: it
///   stops there instead of declining it (round-4 review, finding 7);
/// * a mandatory non-order prompt, met after a forced entry ran, is that run
///   going on where DCGO skipped it - of a stack effect, or of a card outside
///   the stack that the run triggered: an error either way.
fn dcgo_decline_stack(
    game: &Game,
    depth: u8,
    owner: PlayerId,
    stack: Option<Vec<TriggerKey>>,
    forced: bool,
) -> Result<(Game, u16, bool), String> {
    use crate::action::space::PASS;
    use crate::selection::SelectionKind;
    let mut g = game.clone();
    let mut last = 0u16;
    let mut stack = stack;
    let mut forced = forced;
    let mut declined_before_any_order = false;
    for _ in 0..16 {
        let Some(p) = g.pending_selection.as_ref() else {
            break;
        };
        let who = p.selecting_player;
        if who != owner {
            if forced && !matches!(p.kind, SelectionKind::TriggerOrder) {
                return Err(forced_run_error(&format!(
                    "that run now asks P{who}'s '{}' (another player's prompt)",
                    p.prompt
                )));
            }
            break;
        }
        if matches!(p.kind, SelectionKind::TriggerOrder) {
            let keys = trigger_keys(p);
            match &stack {
                // The order after a lone declined gate: not the panel's stack.
                None if declined_before_any_order => break,
                None => stack = Some(keys),
                // An effect the declined stack did not hold: the stack under it.
                Some(s) if keys.iter().any(|k| !s.contains(k)) => break,
                Some(_) => {}
            }
        }
        if matches!(p.kind, SelectionKind::TriggerOrder) && !p.is_optional && p.valid_action_ids.len() > 1 {
            if depth >= 4 {
                return Err("MultipleSkills -1 (\"Don't activate these effects\"): the engine's trigger stack is \
                            nested too deep to compare every order"
                    .into());
            }
            let ids = p.valid_action_ids.clone();
            let mut outcomes: Vec<(u16, Game, u16)> = Vec::new();
            for id in ids {
                let mut branch = g.clone();
                branch.decode_action(id, who);
                let (after, l, _) = dcgo_decline_stack(&branch, depth + 1, owner, stack.clone(), true)?;
                outcomes.push((id, after, if l == 0 { id } else { l }));
            }
            let first = branch_digest(&outcomes[0].1);
            if outcomes.iter().all(|(_, after, _)| branch_digest(after) == first) {
                let (_, after, l) = outcomes.swap_remove(0);
                return Ok((after, l, true));
            }
            return Err(format!(
                "MultipleSkills -1 (\"Don't activate these effects\": DCGO dropped every stacked effect, \
                 MultipleSkills.cs:474-480) meets the engine's mandatory trigger order '{}' {}, whose orders lead \
                 to different games; DCGO offers -1 only when every effect is skippable (MultipleSkills.cs:393) \
                 and recorded no order, so the replay cannot tell which order the engine should run",
                p.prompt,
                describe_choices_of(p),
            ));
        }
        let action = if matches!(p.kind, SelectionKind::TriggerOrder) && !p.is_optional {
            // one trigger left: no order to choose, but the engine runs it
            forced = true;
            p.valid_action_ids[0]
        } else if p.is_optional {
            if stack.is_none() {
                declined_before_any_order = true;
            }
            PASS
        } else {
            // A mandatory prompt that is not an order. Of a stack effect the
            // engine forced: that effect runs on where DCGO skipped it.
            let of_the_stack = stack
                .as_ref()
                .is_some_and(|s| s.iter().any(|(card, _)| *card == Some(p.source_card)));
            if forced {
                return Err(forced_run_error(&format!(
                    "{} now asks P{who}'s '{}'",
                    if of_the_stack { "that effect" } else { "that run (a card outside the stack)" },
                    p.prompt
                )));
            }
            break;
        };
        g.decode_action(action, who);
        last = action;
    }
    Ok((g, last, forced))
}

/// The effect resolutions of `actor` in `window` (engine events), described:
/// a memory change an effect made (`MemoryChange` with a source card), an
/// effect's target (`EffectTarget`), a card an effect revealed (`Reveal`), a
/// card an effect played (`Play` with `effect_initiated`). Draws, trashes and
/// deck moves carry no effect attribution, so they are not counted.
fn effect_activity(window: &[crate::events::GameEvent], actor: PlayerId) -> Vec<String> {
    use crate::events::GameEvent;
    window
        .iter()
        .filter_map(|e| match e {
            GameEvent::MemoryChange { player, delta, source_card_id: Some(c), .. } if *player == actor => {
                Some(format!("memory {delta:+} from {c}"))
            }
            GameEvent::EffectTarget { player, source_card_id, .. } if *player == actor => {
                Some(format!("{source_card_id} chose a target"))
            }
            GameEvent::Reveal { player, card_id, .. } if *player == actor => Some(format!("revealed {card_id}")),
            GameEvent::Play { player, card_id, effect_initiated: true, .. } if *player == actor => {
                Some(format!("an effect played {card_id}"))
            }
            _ => None,
        })
        .collect()
}

/// `[source card slot (optional|mandatory) | ...]` of a trigger-order prompt.
fn describe_choices_of(p: &crate::selection::PendingSelection) -> String {
    format!(
        "{:?}",
        p.effect_choices
            .iter()
            .flatten()
            .map(|e| (e.action_id, e.label.clone(), e.is_optional))
            .collect::<Vec<_>>()
    )
}

/// Stand-in source while the real one is lent to a lookahead driver.
struct NoRecordedSteps;

impl RecordingSource for NoRecordedSteps {
    fn build_initial_game(&self, _: &HashMap<String, CardData>) -> Result<Game, ReplayError> {
        Err(ReplayError::GameConstruction("source lent to a lookahead".into()))
    }
    fn relay_initial_state(&self, _: &mut Game) -> Result<(), ReplayError> {
        Err(ReplayError::GameConstruction("source lent to a lookahead".into()))
    }
    fn steps(&self) -> &[StepSpec] {
        &[]
    }
}

/// A lookahead's loan of the driver's source to a sub-driver
/// (`ReplayDriver::lookahead_trigger_order`). Dropping it - normally, or while
/// a panic unwinds - swaps the real source back into `home`.
struct LentSource<'a> {
    home: &'a mut Box<dyn RecordingSource>,
    sub: ReplayDriver,
}

impl<'a> LentSource<'a> {
    /// Move `home`'s source into `sub` (which must hold a placeholder).
    fn lend(home: &'a mut Box<dyn RecordingSource>, mut sub: ReplayDriver) -> Self {
        std::mem::swap(home, &mut sub.source);
        Self { home, sub }
    }
}

impl Drop for LentSource<'_> {
    fn drop(&mut self) {
        std::mem::swap(self.home, &mut self.sub.source);
    }
}

/// What a recorded selection row decides, for messages; `None` when it carries
/// no decision: a cancel (whatever zone it names), DCGO's `MultipleSkills` -1
/// "Don't activate these effects" (`MultipleSkills.cs:474-480` drops the
/// stacked effects and activates nothing), or a row with no field set. Card
/// ids, targets, a zone, an int / count / bool choice and a playback engine
/// action are decisions.
fn selection_row_content(row: &crate::dcgo_recording::SelectionRow) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(c) = row.card_ids.as_ref().filter(|c| !c.is_empty()) {
        parts.push(format!("cards {c:?}"));
    }
    if let Some(t) = row.targets.as_ref().filter(|t| !t.is_empty()) {
        parts.push(format!("targets {:?}", t.iter().map(|t| (t.player, t.frame)).collect::<Vec<_>>()));
    }
    if parts.is_empty()
        && (row.cancel == Some(true) || (row.prompt == "MultipleSkills" && row.int_value.is_some_and(|v| v < 0)))
    {
        return None;
    }
    if let Some(v) = row.int_value {
        parts.push(format!("int {v}"));
    }
    if let Some(c) = row.count {
        parts.push(format!("count {c}"));
    }
    if let Some(b) = row.bool_value {
        parts.push(format!("bool {b}"));
    }
    if let Some(z) = row.zone.as_deref() {
        parts.push(format!("zone {z}"));
    }
    if let Some(a) = row.engine_action {
        parts.push(format!("engine action {a}"));
    }
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// How a contentless row (see [`selection_row_content`]) declines, for notes.
fn decline_label(row: &crate::dcgo_recording::SelectionRow) -> &'static str {
    if row.prompt == "MultipleSkills" && row.int_value.is_some_and(|v| v < 0) {
        "-1 (\"Don't activate these effects\")"
    } else {
        "cancel"
    }
}

/// Why the engine would refuse `id` from `actor` against the parked prompt,
/// or `None` when it would accept it. The checks are the engine's own
/// (`Game::resolve_selection` -> `resolve_selection_with_lifecycle` in
/// `effect_queue.rs`: the selecting player, an owned card choice's ids, PASS
/// only on an optional prompt, otherwise one of `valid_action_ids`), run before
/// `decode_action`, which discards the refusal.
fn selection_answer_rejection(game: &Game, id: u16, actor: PlayerId) -> Option<String> {
    let p = game.pending_selection.as_ref()?;
    let describe = || format!("{:?} P{} '{}' (valid {:?})", p.kind, p.selecting_player, p.prompt, p.valid_action_ids);
    if p.selecting_player != actor {
        return Some(format!("the engine refuses P{actor}'s answer {id}: the parked prompt is {}", describe()));
    }
    let accepted = match &p.card_choice {
        Some(owner) => owner.valid_action_ids(game).contains(&id),
        None if id == crate::action::space::PASS => p.is_optional,
        None => p.valid_action_ids.contains(&id),
    };
    (!accepted).then(|| format!("the engine refuses answer {id} to {}", describe()))
}

/// Apply `payload`'s picks (and the trailing PASS `resolve_next` asks for) to a
/// clone of `game`, as the main resolution loop of `apply_selection_step`
/// does, for a fold that must answer a prompt from a row it rebuilt. Returns
/// the game and the last engine action, or why the picks cannot be applied
/// (an engine rejection, a resolution failure, or fewer picks than the row).
/// DCGO folds a "you may" gate into the pick itself (`canNoSelect`): the
/// effect is registered non-optional, so there is no `OptionalSkill` row and an
/// affirmative pick row IS the acceptance. The engine models the same "may" as
/// a clause-level outer accept/decline prompt (`ResumeFrame::OuterOptionalTrigger`,
/// single accept id). When `game` parks that prompt for the row's actor and the
/// row picks cards or targets (no cancel, bool or int), accept it so the row
/// meets the body's own prompt. Only the actor's gate: a pick row of one seat
/// says nothing about the other seat's "you may". Returns whether it accepted.
fn accept_outer_gate_for_pick(game: &mut Game, payload: &crate::dcgo_recording::SelectionRow) -> bool {
    let picks = payload.card_ids.as_ref().is_some_and(|c| !c.is_empty())
        || payload.targets.as_ref().is_some_and(|t| !t.is_empty());
    if payload.cancel == Some(true) || payload.bool_value.is_some() || payload.int_value.is_some() || !picks {
        return false;
    }
    let actor = payload.actor;
    let is_outer_gate = game.pending_selection.as_ref().is_some_and(|p| {
        p.is_optional
            && p.selecting_player == actor
            && p.valid_action_ids == [crate::action::space::REPLACEMENT_ACCEPT]
            && game.pending_selection_resume.as_ref().is_some_and(|r| {
                matches!(r.frames.first(), Some(crate::resume::ResumeFrame::OuterOptionalTrigger(_)))
            })
    });
    if is_outer_gate {
        game.decode_action(crate::action::space::REPLACEMENT_ACCEPT, actor);
    }
    is_outer_gate
}

fn resolve_row_on_clone(game: &Game, payload: &crate::dcgo_recording::SelectionRow) -> Result<(Game, u16), String> {
    let actor = payload.actor;
    let mut g = game.clone();
    let n_picks = super::selection_resolve::payload_pick_count(payload);
    let mut done = 0usize;
    let mut last = 0u16;
    loop {
        if g.pending_selection.as_ref().is_some_and(|q| q.selecting_player != actor) && done >= n_picks {
            break;
        }
        match super::selection_resolve::resolve_next(&g, payload, done)? {
            None => break,
            Some(id) => {
                if let Some(reason) = selection_answer_rejection(&g, id, actor) {
                    return Err(reason);
                }
                g.decode_action(id, actor);
                last = id;
                done += 1;
                if done > n_picks + 1 {
                    break;
                }
            }
        }
    }
    if done < n_picks {
        return Err(format!("only {done} of {n_picks} picks applied"));
    }
    Ok((g, last))
}

/// See [`ReplayDriver::classify_unstamped_answer`].
enum UnstampedAnswer {
    /// An optional-activation gate of the actor: apply this engine action
    /// (`None`: the gate needs none, the pick that follows answers it).
    Apply(Option<u16>),
    /// Nothing to apply and nothing visibly lost: consume with this note.
    Note(String),
    /// Consuming the row would lose or hide what DCGO did: halt with this reason.
    Halt(String),
    /// A decline that meets effect activity it cannot be attributed to (the
    /// answer names no card): the replay cannot tell whether the engine ran
    /// the declined effect. A step halts; a trigger-order probe counts the
    /// entry as undetermined - neither confirmed nor refuted.
    CannotTell(String),
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
                digivolve_operand_fallbacks: Vec::new(),
                inferred_skipped: Vec::new(),
                notes: Vec::new(),
                raid_retarget: None,
                paused: false,
                lookahead_depth: 0,
                pick_zone_hint: None,
                zone_menu_row: None,
                unstamped_window_start: 0,
                order_answer: None,
                dcgo_cards: HashMap::new(),
                count_evidence: None,
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

    /// Recorded steps of converter-inferred rows that were skipped because the engine had already advanced.
    pub(crate) fn inferred_skipped(&self) -> &[u32] {
        &self.inferred_skipped
    }

    pub(crate) fn digivolve_operand_fallbacks(&self) -> &[String] {
        &self.digivolve_operand_fallbacks
    }

    /// Non-halting notes: recorded rows that were consumed although the engine
    /// had no prompt they could answer, when consuming them loses no decision:
    /// an unstamped DCGO "use this optional effect?" answer with no
    /// optional-activation gate of that seat parked (see
    /// `classify_unstamped_answer` for the cases that halt instead), and a
    /// contentless cancel with no prompt of its seat parked. Also the detail
    /// of an actor mismatch. Not divergences - the replay continues - but a
    /// later divergence may trace back to one (e.g. a trigger the engine never
    /// offered).
    pub(crate) fn notes(&self) -> &[String] {
        &self.notes
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
        // A target naming one of several copies of a card: the rows that
        // follow decide which copy (`duplicate_copy_target`).
        if !self.source.answers_engine_only_prompts() {
            if let Some(outcome) = self.duplicate_copy_target(game, payload) {
                let (phase_before, memory_before) = (game.current_phase.py_name().to_string(), game.memory);
                match outcome {
                    Ok((next, last)) => {
                        *game = next;
                        self.cursor += 1;
                        return ReplayStepResult {
                            step_number: self.cursor,
                            player_id: payload.actor,
                            action_id: last,
                            phase_before,
                            phase_after: game.current_phase.py_name().to_string(),
                            memory_before,
                            memory_after: game.memory,
                            turn_number: game.turn_count,
                            is_game_over: game.game_over,
                            winner_id: game.winner,
                            divergences: Vec::new(),
                        };
                    }
                    Err(reason) => {
                        self.divergences.push(Divergence {
                            step: self.cursor,
                            action_id: 0,
                            actor: payload.actor,
                            phase: payload.phase.clone(),
                            kind: DivergenceKind::SelectionResolution { reason },
                            board: describe_board(game),
                        });
                        self.paused = true;
                        return self.no_progress_result(game);
                    }
                }
            }
        }

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

        // The zone a "From which area?" menu named for this zone-less pick row.
        let hinted;
        let payload = match self.pick_zone_hint.take() {
            Some((row, zone)) if row == self.cursor && payload.zone.is_none() => {
                let mut p = payload.clone();
                p.zone = Some(zone.to_string());
                hinted = p;
                &hinted
            }
            _ => payload,
        };

        let actor = payload.actor;

        // Every fold below reconciles a DCGO RECORDING's prompt shape with the
        // engine's (an int zone menu, a separate yes/no gate, a MultipleSkills
        // panel, a forced prompt DCGO never asks...). A source that writes every
        // one of OUR prompts itself - the exam's `ScenarioAdapter`, see
        // `RecordingSource::answers_engine_only_prompts` - must meet the engine's
        // prompts exactly as parked: folding there consumes or auto-answers the
        // prompt its own row was written for and shifts every later row.
        let dcgo_folds = !self.source.answers_engine_only_prompts();
        let resolve_next = if dcgo_folds {
            super::selection_resolve::resolve_next_dcgo
        } else {
            super::selection_resolve::resolve_next
        };

        // DCGO's duplicate-candidate effect pick after a refire ("Select 1
        // effect to activate." over several effect objects that are ONE effect
        // in the engine). See `refire_duplicate_effect_pick_fold`.
        if dcgo_folds && payload.prompt == "SelectCardEffect" && payload.engine_action.is_none() {
            if let Some(outcome) = self.refire_duplicate_effect_pick_fold(game, payload) {
                let (phase_before, memory_before) = (game.current_phase.py_name().to_string(), game.memory);
                match outcome {
                    Ok(note) => {
                        self.notes.push(note);
                        game.last_collapsed_refire = None;
                        self.cursor += 1;
                        return ReplayStepResult {
                            step_number: self.cursor,
                            player_id: payload.actor,
                            action_id: 0,
                            phase_before,
                            phase_after: game.current_phase.py_name().to_string(),
                            memory_before,
                            memory_after: game.memory,
                            turn_number: game.turn_count,
                            is_game_over: game.game_over,
                            winner_id: game.winner,
                            divergences: Vec::new(),
                        };
                    }
                    Err(reason) => {
                        self.divergences.push(Divergence {
                            step: self.cursor,
                            action_id: 0,
                            actor: payload.actor,
                            phase: payload.phase.clone(),
                            kind: DivergenceKind::SelectionResolution { reason },
                            board: describe_board(game),
                        });
                        self.paused = true;
                        return self.no_progress_result(game);
                    }
                }
            }
        }

        // An effect-initiated DNA digivolve: DCGO's hand row + two material
        // root rows answer the engine's partner -> hand prompt pair.
        if dcgo_folds {
            if let Some(result) = self.fold_effect_dna_root_rows(game, payload, payload.actor) {
                return result;
            }
        }

        // A recorder playback stamp is already an engine action. Only a
        // currently legal action of the recorded actor is trusted; stale stamps
        // still go through evidence-based DCGO ordering.
        let stamped_order_action = payload.engine_action.is_some_and(|action| {
            game.pending_selection.as_ref().is_some_and(|pending| {
                pending.selecting_player == actor
                    && (matches!(pending.kind, crate::selection::SelectionKind::TriggerOrder)
                        || (matches!(pending.kind, crate::selection::SelectionKind::EffectChoice)
                            && pending.prompt == REPLACEMENT_ORDER_PROMPT))
                    && pending.valid_action_ids.contains(&action)
            })
        });

        // A remote MultipleSkills index is ordered by DCGO, so it cannot be
        // interpreted as an engine trigger-order index. A following named pick
        // can identify the trigger when exactly one branch accepts that pick.
        if dcgo_folds && !stamped_order_action && payload.prompt == "MultipleSkills"
            && payload.int_value.is_some_and(|v| v >= 0)
            && payload.chosen_effect_card.is_none()
            && game.pending_selection.as_ref().is_some_and(|p| p.selecting_player == actor)
        {
            if let Some(next) = self.source.steps().get(self.cursor as usize + 1)
                .and_then(|step| step.selection.as_ref())
                .filter(|next| next.actor == actor)
            {
                // DCGO may ask to activate the chosen trigger before its
                // identifying card/field pick. Probe that subsequent pick,
                // but keep the recorded yes for the actual activation gate.
                if next.prompt == "OptionalSkill" && next.bool_value == Some(true) {
                    if let Some(pick) = self.source.steps().get(self.cursor as usize + 2)
                        .and_then(|step| step.selection.as_ref())
                        .filter(|pick| pick.actor == actor &&
                            (pick.card_ids.as_ref().is_some_and(|ids| !ids.is_empty()) ||
                             pick.targets.as_ref().is_some_and(|targets| !targets.is_empty())))
                    {
                        let remapped = remap_selection_targets(game, pick);
                        if let Some((branch, false)) = self.probe_branch_for_row(game, remapped.as_ref().unwrap_or(pick), actor, self.cursor as usize + 2) {
                            let phase_before = game.current_phase.py_name().to_string();
                            let memory_before = game.memory;
                            let turn_number = game.turn_count;
                            game.decode_action(branch, actor);
                            self.cursor += 1;
                            return ReplayStepResult {
                                step_number: self.cursor, player_id: actor, action_id: branch,
                                phase_before, phase_after: game.current_phase.py_name().to_string(),
                                memory_before, memory_after: game.memory, turn_number,
                                is_game_over: game.game_over, winner_id: game.winner, divergences: Vec::new(),
                            };
                        }
                    }
                }
                // Both ST24-14 copies expose DCGO's 1/2 place-or-decline
                // menu. A following 2 identifies that menu, regardless of
                // its position in DCGO's trigger-order panel.
                if next.prompt == "generic_int" && next.int_value == Some(2)
                    && self.source.steps().get(self.cursor as usize + 2)
                        .and_then(|step| step.selection.as_ref())
                        .is_some_and(|after| after.actor == actor &&
                            (after.prompt == "MultipleSkills" || after.cancel == Some(true)))
                {
                    if let Some(branch) = game.pending_selection.as_ref()
                        .filter(|p| matches!(p.kind, crate::selection::SelectionKind::TriggerOrder))
                        .and_then(|p| p.effect_choices.as_ref())
                        .filter(|entries| entries.iter().all(|e| e.source_card
                            .and_then(|c| game.card_data_for_handle(c))
                            .is_some_and(|d| matches!(d.card_id.as_str(), "ST24-14" | "ST24-15"))))
                        .and_then(|entries| entries.iter().find(|e| {
                            e.timing == Some(crate::enums::EffectTiming::StartOfYourMainPhase)
                                && e.source_card.and_then(|c| game.card_data_for_handle(c))
                                    .is_some_and(|d| d.card_id == "ST24-14")
                        }))
                        .map(|e| e.action_id)
                    {
                        let phase_before = game.current_phase.py_name().to_string();
                        let memory_before = game.memory;
                        let turn_number = game.turn_count;
                        game.decode_action(branch, actor);
                        self.cursor += 1;
                        return ReplayStepResult {
                            step_number: self.cursor, player_id: actor, action_id: branch,
                            phase_before, phase_after: game.current_phase.py_name().to_string(),
                            memory_before, memory_after: game.memory, turn_number,
                            is_game_over: game.game_over, winner_id: game.winner, divergences: Vec::new(),
                        };
                    }
                }
                let remapped_next = remap_selection_targets(game, next);
                let next = remapped_next.as_ref().unwrap_or(next);
                if let Some((branch, false)) = self.probe_branch_for_row(game, next, actor, self.cursor as usize + 1) {
                    game.decode_action(branch, actor);
                    self.cursor += 1;
                    return ReplayStepResult {
                        step_number: self.cursor,
                        player_id: actor,
                        action_id: branch,
                        phase_before: game.current_phase.py_name().to_string(),
                        phase_after: game.current_phase.py_name().to_string(),
                        memory_before: game.memory,
                        memory_after: game.memory,
                        turn_number: game.turn_count,
                        is_game_over: game.game_over,
                        winner_id: game.winner,
                        divergences: Vec::new(),
                    };
                }
            }
        }

        if let Some(pending) = game.pending_selection.as_ref() {
            if pending.selecting_player != actor {
                self.divergences.push(Divergence {
                    step: self.cursor,
                    action_id: 0,
                    actor,
                    phase: payload.phase.clone(),
                    kind: DivergenceKind::Actor {
                        expected: pending.selecting_player,
                        recorded: actor,
                    },
                    board: describe_board(game),
                });
                self.paused = true;
                return self.no_progress_result(game);
            }
        }

        // DCGO's BT25-087 menu uses values 1 = suspend and place,
        // 2 = suspend only, 3 = decline. The engine's optional trigger
        // exposes the first and third choices; do not infer value 2.
        if dcgo_folds && payload.prompt == "generic_int" && payload.int_value == Some(3)
            && game.pending_selection.as_ref().is_some_and(|p| {
                matches!(p.kind, crate::selection::SelectionKind::TriggerOrder)
                    && p.is_optional && p.selecting_player == actor
                    && p.effect_choices.as_ref().is_some_and(|entries| {
                        entries.len() == 1 && entries[0].timing == Some(crate::enums::EffectTiming::OnAddToHand)
                            && entries[0].source_card.and_then(|c| game.card_data_for_handle(c))
                                .is_some_and(|d| d.card_id == "BT25-087")
                    })
            })
        {
            let phase_before = game.current_phase.py_name().to_string();
            let memory_before = game.memory;
            let turn_number = game.turn_count;
            game.decode_action(crate::action::space::PASS, actor);
            self.cursor += 1;
            return ReplayStepResult {
                step_number: self.cursor, player_id: actor, action_id: crate::action::space::PASS,
                phase_before, phase_after: game.current_phase.py_name().to_string(),
                memory_before, memory_after: game.memory, turn_number,
                is_game_over: game.game_over, winner_id: game.winner, divergences: Vec::new(),
            };
        }

        // An UNSTAMPED yes/no (`SelectionRow::unstamped`): a remote seat's
        // "use this optional effect?" answer (DCGO `OptionalSkill.SetUseOptional`),
        // which the converter could not tie to an engine prompt. It answers
        // ONLY the engine's optional-ACTIVATION gate of this actor (see
        // `dcgo_use_optional_answer`), never a choice inside an effect body:
        // ST24-06 / ST24-10's "Trash 2 face-down cards" / "Don't" is a
        // yes/no-shaped `EffectChoice`, but DCGO registers that effect with
        // `optional: false` and never sends `SetUseOptional` for it. With no
        // such gate parked, `classify_unstamped_answer` decides whether the
        // row may be consumed with a non-halting note or must stop the replay
        // (a decline that would be lost, or a yes whose picks the engine never
        // asks for). Nothing is inferred from the rows around it beyond that.
        if payload.unstamped && payload.prompt == "generic_bool" && payload.bool_value.is_some() {
            let value = payload.bool_value.unwrap_or_default();
            let next = self
                .source
                .steps()
                .get(self.cursor as usize + 1)
                .and_then(|s| s.selection.as_ref());
            let (phase_before, memory_before) = (game.current_phase.py_name().to_string(), game.memory);
            // The window starts where the last decision began; a start past the
            // end of the log is a window the replay does not know, never the
            // whole log (`unstamped_window`).
            let classified = match Self::unstamped_window(&game.events, self.unstamped_window_start) {
                Ok(window) => Self::classify_unstamped_answer(game, actor, value, next, self.cursor, window),
                Err(reason) => UnstampedAnswer::Halt(format!("row {}: {reason}", self.cursor)),
            };
            let answer = match classified {
                UnstampedAnswer::Apply(answer) => {
                    if let Some(id) = answer {
                        game.decode_action(id, actor);
                    }
                    answer
                }
                UnstampedAnswer::Note(note) => {
                    self.notes.push(note);
                    None
                }
                UnstampedAnswer::Halt(reason) | UnstampedAnswer::CannotTell(reason) => {
                    self.divergences.push(Divergence {
                        step: self.cursor,
                        action_id: 0,
                        actor,
                        phase: payload.phase.clone(),
                        kind: DivergenceKind::SelectionResolution { reason },
                        board: describe_board(game),
                    });
                    self.paused = true;
                    return self.no_progress_result(game);
                }
            };
            self.cursor += 1;
            return ReplayStepResult {
                step_number: self.cursor,
                player_id: actor,
                action_id: answer.unwrap_or(0),
                phase_before,
                phase_after: game.current_phase.py_name().to_string(),
                memory_before,
                memory_after: game.memory,
                turn_number: game.turn_count,
                is_game_over: game.game_over,
                winner_id: game.winner,
                divergences: Vec::new(),
            };
        }

        // FELLOWSHIP OF HOPE'S KEEPERS (BT20-095)'s <Delay> body: DCGO's
        // "Digivolve?" bool meets the engine's "Move ...?" breeding pick, which
        // DCGO never asks. See `bt20_095_delay_fold`.
        //
        // This fold and the three card folds below read DCGO's own rows. A
        // playback row (`engine_action`: the engine action the mod realised
        // when it answered DCGO) needs no reading - `resolve_next` takes that
        // action when the parked prompt accepts it - so they never touch one.
        // A STALE stamp - one the parked prompt does not list among its
        // answers (sim1b's Siriusmon (EX12-018) menu row stamped 30, which
        // fits an older script that asked DCGO's zone menu itself; a game
        // that diverged would look the same) - names nothing here: the folds
        // read DCGO's own int / card ids instead, with a note that states
        // only what was observed (`stale_stamp`).
        let stale_stamp = Self::stale_stamp(game, payload);
        let folds_read_row = payload.engine_action.is_none() || stale_stamp.is_some();
        if dcgo_folds
            && folds_read_row
            && payload.prompt == "generic_bool"
            && !payload.unstamped
            && payload.bool_value.is_some()
        {
            if let Some(outcome) = self.bt20_095_delay_fold(game, payload) {
                let (phase_before, memory_before) = (game.current_phase.py_name().to_string(), game.memory);
                match outcome {
                    Ok((next, rows)) => {
                        self.note_stale_stamp(stale_stamp, payload, "BT20-095");
                        *game = next;
                        self.cursor += rows;
                        return ReplayStepResult {
                            step_number: self.cursor,
                            player_id: actor,
                            action_id: 0,
                            phase_before,
                            phase_after: game.current_phase.py_name().to_string(),
                            memory_before,
                            memory_after: game.memory,
                            turn_number: game.turn_count,
                            is_game_over: game.game_over,
                            winner_id: game.winner,
                            divergences: Vec::new(),
                        };
                    }
                    Err(reason) => {
                        self.divergences.push(Divergence {
                            step: self.cursor,
                            action_id: 0,
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
        }

        // AMATERASUMON (EX12-047)'s deck-bottom order comes from DCGO's second
        // row. See `ex12_047_bottom_order_fold`.
        if dcgo_folds && folds_read_row && payload.prompt == "SelectCardEffect" {
            if let Some(outcome) = self.ex12_047_bottom_order_fold(game, payload) {
                let (phase_before, memory_before) = (game.current_phase.py_name().to_string(), game.memory);
                match outcome {
                    Ok((next, last)) => {
                        self.note_stale_stamp(stale_stamp, payload, "EX12-047");
                        *game = next;
                        self.cursor += 2;
                        return ReplayStepResult {
                            step_number: self.cursor,
                            player_id: actor,
                            action_id: last,
                            phase_before,
                            phase_after: game.current_phase.py_name().to_string(),
                            memory_before,
                            memory_after: game.memory,
                            turn_number: game.turn_count,
                            is_game_over: game.game_over,
                            winner_id: game.winner,
                            divergences: Vec::new(),
                        };
                    }
                    Err(reason) => {
                        self.divergences.push(Divergence {
                            step: self.cursor,
                            action_id: 0,
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
        }

        // A per-card DCGO zone menu loop (SIRIUSMON (EX12-018), J.P., Koji, & Koichi (AD1-023)), from its
        // first menu row. See `zone_menu_loop_fold`.
        if dcgo_folds
            && folds_read_row
            && (payload.prompt == "SelectHandEffect"
                || payload.prompt == "SelectCardEffect"
                || payload.prompt == "generic_int")
        {
            if let Some(outcome) = self.zone_menu_loop_fold(game, payload) {
                let (phase_before, memory_before) = (game.current_phase.py_name().to_string(), game.memory);
                match outcome {
                    Ok((next, last, rows)) => {
                        let card = game
                            .pending_selection
                            .as_ref()
                            .and_then(|p| game.card_data_for_handle(p.source_card))
                            .map(|d| d.card_id.clone())
                            .unwrap_or_default();
                        self.note_stale_stamp(stale_stamp, payload, &card);
                        *game = next;
                        self.cursor += rows;
                        return ReplayStepResult {
                            step_number: self.cursor,
                            player_id: actor,
                            action_id: last,
                            phase_before,
                            phase_after: game.current_phase.py_name().to_string(),
                            memory_before,
                            memory_after: game.memory,
                            turn_number: game.turn_count,
                            is_game_over: game.game_over,
                            winner_id: game.winner,
                            divergences: Vec::new(),
                        };
                    }
                    Err(reason) => {
                        self.divergences.push(Divergence {
                            step: self.cursor,
                            action_id: 0,
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
        }

        // RYUGUMON (EX12-036)'s no-op unsuspend pick. See `ex12_036_noop_unsuspend_fold`.
        if dcgo_folds && folds_read_row && payload.prompt == "SelectHandEffect" {
            if let Some((next, last, note)) = self.ex12_036_noop_unsuspend_fold(game, payload) {
                let (phase_before, memory_before) = (game.current_phase.py_name().to_string(), game.memory);
                self.note_stale_stamp(stale_stamp, payload, "EX12-036");
                *game = next;
                self.notes.push(note);
                self.cursor += 2;
                return ReplayStepResult {
                    step_number: self.cursor,
                    player_id: actor,
                    action_id: last,
                    phase_before,
                    phase_after: game.current_phase.py_name().to_string(),
                    memory_before,
                    memory_after: game.memory,
                    turn_number: game.turn_count,
                    is_game_over: game.game_over,
                    winner_id: game.winner,
                    divergences: Vec::new(),
                };
            }
        }

        // What a PvP `generic_bool` may answer. See `stamped_bool_meaning_mismatch`.
        if dcgo_folds {
            if let Some(reason) = self.stamped_bool_meaning_mismatch(game, payload) {
                self.divergences.push(Divergence {
                    step: self.cursor,
                    action_id: 0,
                    actor,
                    phase: payload.phase.clone(),
                    kind: DivergenceKind::SelectionResolution { reason },
                    board: describe_board(game),
                });
                self.paused = true;
                return self.no_progress_result(game);
            }
        }

        // DCGO folds a "you may" gate into the pick itself (`canNoSelect`): the
        // effect is registered non-optional, so there is no `OptionalSkill` row -
        // an affirmative pick row IS the acceptance. Our engine models the same
        // "may" as a clause-level outer accept/decline prompt
        // (`ResumeFrame::OuterOptionalTrigger`, single accept id). A recorded
        // pick that names cards/targets against that prompt therefore means
        // "accept, then pick": accept first so the row meets the body's own
        // prompt. A cancel row (decline) is left to resolve as the PASS.
        if dcgo_folds {
            accept_outer_gate_for_pick(game, payload);
        }

        // DCGO's "From which area?" INT menu ahead of a pick over the hand AND
        // the trash. DCGO asks the zone as a numbered menu and then a pick inside
        // that one zone (P-249: From hand = 1 / From trash = 2 / Don't select = 3,
        // then `SelectHandEffect` or a `SelectCardEffect` rooted in the trash,
        // P_249.cs:53-123); the engine asks ONE pick over both zones (a
        // `UnionZone`, or a DSL `select_zone_cards` pick). The int is read with
        // the card's own DCGO menu (`selection_resolve::dcgo_int_zone_menu`; the
        // numbering is card-authored - EX11-022 counts from 0) and must agree
        // with the zone of the actor's next pick row, which then resolves in that
        // zone only (`selection_resolve::row_pick_zone`). See
        // `hand_or_trash_int_menu` for every outcome.
        if dcgo_folds
            && payload.prompt == "generic_int"
            && payload.engine_action.is_none()
            && payload.int_value.is_some()
            && payload.card_ids.is_none()
            && payload.targets.is_none()
            && payload.count.is_none()
            && payload.cancel != Some(true)
            && game.pending_selection.as_ref().is_some_and(|p| {
                (matches!(&p.kind, crate::selection::SelectionKind::UnionZone { zones }
                    if zones.contains(crate::selection::UnionZoneSet::HAND)
                        && zones.contains(crate::selection::UnionZoneSet::TRASH))
                    || super::selection_resolve::fresh_zone_card_pick_stands_for_zone_menu(game))
                    && p.effect_choices.is_none()
                    && p.selecting_player == actor
            })
        {
            let (phase_before, memory_before) = (game.current_phase.py_name().to_string(), game.memory);
            let action = match self.hand_or_trash_int_menu(game, payload) {
                Ok(action) => action,
                Err(reason) => {
                    self.divergences.push(Divergence {
                        step: self.cursor,
                        action_id: 0,
                        actor,
                        phase: payload.phase.clone(),
                        kind: DivergenceKind::SelectionResolution { reason },
                        board: describe_board(game),
                    });
                    self.paused = true;
                    return self.no_progress_result(game);
                }
            };
            if let Some(id) = action {
                game.decode_action(id, actor);
            } else {
                self.set_pick_zone_hint(actor);
            }
            self.cursor += 1;
            return ReplayStepResult {
                step_number: self.cursor,
                player_id: actor,
                action_id: action.unwrap_or(0),
                phase_before,
                phase_after: game.current_phase.py_name().to_string(),
                memory_before,
                memory_after: game.memory,
                turn_number: game.turn_count,
                is_game_over: game.game_over,
                winner_id: game.winner,
                divergences: Vec::new(),
            };
        }

        // A zone menu ahead of any other multi-zone pick (a union that is not
        // over the hand AND the trash). DCGO asks "from where?" as an int menu
        // and then a pick inside that one zone; the int only narrows where the
        // following recorded pick comes from, so it is consumed and the pick
        // that follows answers the engine's prompt.
        if dcgo_folds
            && payload.prompt == "generic_int"
            && payload.int_value.is_some()
            && payload.card_ids.is_none()
            && payload.targets.is_none()
            && payload.cancel != Some(true)
            && game.pending_selection.as_ref().is_some_and(|p| {
                matches!(p.kind, crate::selection::SelectionKind::UnionZone { .. })
                    && p.effect_choices.is_none()
                    && p.selecting_player == actor
            })
            // only when this actor's card pick follows: with no pick after it the int was DCGO's "Don't play"
            // (EX12-074's [Security]: From hand / From trash / Don't play = 3), the engine's decline, which the
            // zone-choice handling below resolves
            && self
                .source
                .steps()
                .get(self.cursor as usize + 1)
                .and_then(|s| s.selection.as_ref())
                .is_some_and(|n| n.actor == actor && n.card_ids.as_ref().is_some_and(|c| !c.is_empty()))
        {
            self.cursor += 1;
            return ReplayStepResult {
                step_number: self.cursor,
                player_id: actor,
                action_id: 0,
                phase_before: game.current_phase.py_name().to_string(),
                phase_after: game.current_phase.py_name().to_string(),
                memory_before: game.memory,
                memory_after: game.memory,
                turn_number: game.turn_count,
                is_game_over: game.game_over,
                winner_id: game.winner,
                divergences: Vec::new(),
            };
        }

        // A mandatory ONE-candidate field pick in front of a `MultipleSkills` row.
        // DCGO skips a pick with a single candidate (EX13-026 Kudamon's "place 1
        // face down under 1 of your [DATA SQUAD] Tamers" with one such Tamer), so
        // the recording has no row for it, while the engine parks the pick. An
        // ordering row can never answer a field pick, and the lone candidate
        // leaves no choice: answer it, then let the row meet what comes next.
        // (`answer_dcgo_silent_lone_target_pick` covers the same skip ahead of an
        // ACTION row.)
        //
        // The same guards as that helper (`dcgo_silent_lone_field_pick`: the
        // DCGO-oracle policy of a source that omits DCGO's silent prompts), plus:
        // the pick belongs to this row's actor (the ordering panel is that
        // seat's, so the skipped pick that precedes it is too), and the row does
        // not itself answer the pick. An ordering row's int only ever indexes
        // DCGO's effect list - its positional reading against a field pick is
        // meaningless - so the one way it can answer the pick is a playback
        // `engine_action` the pick accepts.
        //
        // Answering the pick can park the NEXT prompt DCGO skips - the one-card
        // deck order that ends the same effect (EX13-026's "Return the rest to
        // the bottom of the deck" with one card left; DCGO places a lone card
        // without a prompt) - so `answer_forced_single_ordering_prompt` runs
        // after each answer, exactly as it does in front of every row. Without
        // it the row would meet that order and the fold below would consume it
        // there, never meeting the trigger order it was recorded against
        // (raw-20261113T025510Z#g6 file step 28).
        if dcgo_folds && payload.prompt == "MultipleSkills" {
            for _ in 0..4 {
                let Some((who, id)) = self.dcgo_silent_lone_field_pick(game) else {
                    break;
                };
                if who != actor || payload.engine_action == Some(id) {
                    break;
                }
                game.decode_action(id, who);
                self.answer_forced_single_ordering_prompt(game);
            }
        }

        // A card-less `MultipleSkills` index against an order prompt of 2+
        // effects. A seat without a recorder (the opponent in a PvP recording)
        // logs DCGO's index but no `effect_activation` row, so the row carries
        // no `chosen_effect_card`; and DCGO's index points into ITS ordering of
        // the simultaneous effects, which our order list does not share (EX12-046
        // Shishimamon's [End of Attack] play vs <Execute>'s pending self-delete:
        // DCGO's 0 is the play, our 0 the delete). The actor's next recorded row
        // decides; see `remote_trigger_order_choice`. A row that does carry the
        // card keeps the path through `resolve_next`.
        if dcgo_folds
            && !stamped_order_action
            && payload.prompt == "MultipleSkills"
            && payload.int_value.is_some_and(|v| v >= 0)
            && payload.chosen_effect_card.is_none()
            && payload.card_ids.is_none()
            && payload.targets.is_none()
            && payload.cancel != Some(true)
        {
            match self.remote_trigger_order_choice(game, actor, payload.int_value.unwrap_or(0)) {
                None if game.pending_selection.as_ref().is_some_and(|p| {
                    matches!(p.kind, crate::selection::SelectionKind::TriggerOrder)
                        && p.valid_action_ids.iter().filter(|&&id| id != crate::action::space::PASS).count() > 1
                }) && self.source.steps().get(self.cursor as usize + 1)
                    .and_then(|s| s.selection.as_ref()).is_some_and(|next| {
                        next.actor == actor && matches!(next.prompt.as_str(), "OptionalSkill" | "generic_bool")
                            && next.bool_value == Some(false)
                    }) => {
                    self.divergences.push(Divergence {
                        step: self.cursor, action_id: 0, actor, phase: payload.phase.clone(),
                        kind: DivergenceKind::SelectionResolution { reason:
                            "declined trigger cannot be identified from DCGO's ordering index".into() },
                        board: describe_board(game),
                    });
                    self.paused = true;
                    return self.no_progress_result(game);
                }
                None => {}
                Some(Ok(entry)) => {
                    let (phase_before, memory_before) = (game.current_phase.py_name().to_string(), game.memory);
                    game.decode_action(entry, actor);
                    self.cursor += 1;
                    return ReplayStepResult {
                        step_number: self.cursor,
                        player_id: actor,
                        action_id: entry,
                        phase_before,
                        phase_after: game.current_phase.py_name().to_string(),
                        memory_before,
                        memory_after: game.memory,
                        turn_number: game.turn_count,
                        is_game_over: game.game_over,
                        winner_id: game.winner,
                        divergences: Vec::new(),
                    };
                }
                Some(Err(reason)) => {
                    self.divergences.push(Divergence {
                        step: self.cursor,
                        action_id: 0,
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

        // A `MultipleSkills` row whose chosen card names TWO OR MORE entries of
        // the order prompt (two copies of one card - MetalGarurumon (EX12-035)
        // on two permanents, raw-20261113T025510Z#g4 file step 69 - or two
        // replacements of one card, Venusmon (BT24-040) in the Toho-vs-
        // Jupitermon playback). The card does not say which entry DCGO
        // activated, and DCGO's index points into its own list
        // (MultipleSkills.cs:175-261), so it is not read as a position: the
        // actor's following rows decide (`lookahead_trigger_order`, the same
        // proof a card-less index gets), or the order is unobservable (every
        // entry leads to the same game), or the replay stops. Only when the
        // resolver cannot already single one entry out (by the effect's name).
        if dcgo_folds && !stamped_order_action && payload.prompt == "MultipleSkills" && payload.int_value.is_some_and(|v| v >= 0) {
            if let Some(cid) = payload.chosen_effect_card.as_deref() {
                let matching: Vec<u16> = game
                    .pending_selection
                    .as_ref()
                    .filter(|p| {
                        p.selecting_player == actor
                            && (matches!(p.kind, crate::selection::SelectionKind::TriggerOrder)
                                || (matches!(p.kind, crate::selection::SelectionKind::EffectChoice)
                                    && p.prompt == REPLACEMENT_ORDER_PROMPT))
                    })
                    .map(|p| {
                        p.effect_choices
                            .iter()
                            .flatten()
                            .filter(|e| p.valid_action_ids.contains(&e.action_id))
                            .filter(|e| {
                                e.source_card
                                    .and_then(|c| game.card_data_for_handle(c))
                                    .is_some_and(|d| d.card_id == cid)
                            })
                            .map(|e| e.action_id)
                            .collect()
                    })
                    .unwrap_or_default();
                if matching.len() >= 2 && super::selection_resolve::resolve_next(game, payload, 0).is_err() {
                    let branches: Vec<(u16, Game)> = matching
                        .iter()
                        .map(|id| {
                            let mut g = game.clone();
                            g.decode_action(*id, actor);
                            (*id, g)
                        })
                        .collect();
                    // Entries that are two copies of this card (not two effects of
                    // one card): the branches are compared with the physical
                    // identity of the copies renamed away.
                    let identity_free = entries_are_copies_of_one_card(game);
                    let first = order_branch_digest(&branches[0].1, identity_free);
                    let verdict = if branches.iter().all(|(_, g)| order_branch_digest(g, identity_free) == first) {
                        LookaheadVerdict::Proved(branches[0].0)
                    } else {
                        self.lookahead_trigger_order(branches, self.cursor + 1, &order_entry_sources(game), identity_free)
                    };
                    let LookaheadVerdict::Proved(entry) = verdict else {
                        let prompt = game.pending_selection.as_ref().map(|p| p.prompt.as_str()).unwrap_or_default();
                        let index = payload.int_value.unwrap_or_default();
                        let reason = if verdict == LookaheadVerdict::AllRefuted {
                            format!(
                                "MultipleSkills index {index} (chose {cid}'s effect): every entry {matching:?} of the \
                                 order prompt '{prompt}' belongs to {cid}, and the rows that follow {REFUTED_ORDER}"
                            )
                        } else {
                            format!(
                                "MultipleSkills index {index} (chose {cid}'s effect) {AMBIGUOUS_ORDER}: entries \
                                 {matching:?} of the order prompt '{prompt}' all belong to {cid}, lead to different \
                                 games and are not told apart by the actor's next rows; the recording does not say \
                                 which one DCGO activated"
                            )
                        };
                        self.divergences.push(Divergence {
                            step: self.cursor,
                            action_id: 0,
                            actor,
                            phase: payload.phase.clone(),
                            kind: DivergenceKind::SelectionResolution { reason },
                            board: describe_board(game),
                        });
                        self.paused = true;
                        return self.no_progress_result(game);
                    };
                    let (phase_before, memory_before) = (game.current_phase.py_name().to_string(), game.memory);
                    game.decode_action(entry, actor);
                    self.cursor += 1;
                    return ReplayStepResult {
                        step_number: self.cursor,
                        player_id: actor,
                        action_id: entry,
                        phase_before,
                        phase_after: game.current_phase.py_name().to_string(),
                        memory_before,
                        memory_after: game.memory,
                        turn_number: game.turn_count,
                        is_game_over: game.game_over,
                        winner_id: game.winner,
                        divergences: Vec::new(),
                    };
                }
            }
        }

        // A `MultipleSkills` ordering int against a prompt that is not a trigger
        // order. DCGO lists the simultaneous effects it can activate right now and
        // asks the player to order them; when the engine queued only ONE trigger
        // (the other's cost is unpayable) it parked that trigger's own "you may"
        // gate instead, and an ordering index can never answer an accept/decline.
        // The row is therefore consumed: the engine already ran the only order
        // there was, and the decision that follows (the recorded pick or cancel)
        // meets the gate as usual.
        //
        // Only against THAT trigger's own prompt: the parked prompt's source is
        // the row's `chosen_effect_card` (the card DCGO's `effect_activation`
        // row names), or - for a card-less row of a seat without a recorder -
        // it is an optional-activation gate (`is_outer_optional_gate`,
        // `Game::pending_is_optional_activation`) or the prompt the actor's
        // very next row answers (that row is the chosen effect's first
        // decision: Hiro Amanokawa (EX12-066)'s "Which effect will you
        // activate?" met by its own int menu, raw-20261113T025510Z#g3 file step
        // 38). Any other prompt belongs to a different effect (a pick still
        // open from an earlier effect, an order DCGO never asks): consuming the
        // index there loses it, so the row falls through and halts in
        // `resolve_next`.
        let next_row_answers_it = self
            .source
            .steps()
            .get(self.cursor as usize + 1)
            .and_then(|s| s.selection.as_ref())
            .is_some_and(|n| {
                n.actor == actor
                    && n.prompt != "MultipleSkills"
                    && matches!(super::selection_resolve::resolve_next(game, n, 0), Ok(Some(_)))
            });
        // ... but never the prompt of the effect the actor's previous index
        // chose (with only that effect's unstamped "use?" answers since): DCGO
        // runs the chosen effect - its "use?" prompt first when it is
        // optional (ICardEffect.cs:1241-1246) - before its loop orders again
        // (MultipleSkills.cs:487-545), so an index right after it is a NEW
        // order, and the engine, still inside that effect, ordered
        // differently (raw-20261113T011922Z#g3 raw seq 395-396: Gigimon
        // (BT21-001)'s digivolve pick met the second index).
        let chosen_by_previous_index = |p: &crate::selection::PendingSelection| {
            self.order_answer.is_some_and(|(at, src)| {
                src == p.source_card && at < self.cursor && p.selecting_player == actor
            })
        };
        let own_prompt_of_chosen_effect = |p: &crate::selection::PendingSelection| match payload
            .chosen_effect_card
            .as_deref()
        {
            Some(cid) => game.card_data_for_handle(p.source_card).is_some_and(|d| d.card_id == cid),
            None => {
                !chosen_by_previous_index(p)
                    && (Self::is_outer_optional_gate(game) || game.pending_is_optional_activation() || next_row_answers_it)
            }
        };
        if dcgo_folds
            && payload.prompt == "MultipleSkills"
            && payload.int_value.is_some_and(|v| v >= 0)
            && payload.card_ids.is_none()
            && payload.targets.is_none()
            && payload.cancel != Some(true)
            // ... or the engine queued no ordering decision at all and has nothing parked (it resolved the
            // effects DCGO asks to order - e.g. one whose condition was already false - without a decision)
            && game.pending_selection.as_ref().map_or(true, |p| {
                !matches!(p.kind, crate::selection::SelectionKind::TriggerOrder)
                    && p.selecting_player == actor
                    && own_prompt_of_chosen_effect(p)
                    // a replacement order is DCGO's MultipleSkills panel too: the row's card answers it
                    && !(matches!(p.kind, crate::selection::SelectionKind::EffectChoice)
                        && p.prompt == REPLACEMENT_ORDER_PROMPT
                        && payload.chosen_effect_card.as_deref().is_some_and(|cid| {
                            p.effect_choices.as_ref().is_some_and(|es| {
                                es.iter().any(|e| {
                                    e.source_card
                                        .and_then(|c| game.card_data_for_handle(c))
                                        .is_some_and(|d| d.card_id == cid)
                                })
                            })
                        }))
            })
        {
            // With NOTHING parked the consume is not silent: the effects' own
            // decisions are the evidence (their pick rows halt if the engine never
            // asks them), but a later divergence may trace back to this index.
            if game.pending_selection.is_none() {
                self.notes.push(format!(
                    "row {}: MultipleSkills index {} of P{actor} (DCGO ordered 2+ activatable effects{}) consumed \
                     with no effect: the engine had no trigger order or prompt parked",
                    self.cursor,
                    payload.int_value.unwrap_or_default(),
                    payload
                        .chosen_effect_card
                        .as_deref()
                        .map(|c| format!(", chose {c}'s"))
                        .unwrap_or_default(),
                ));
            } else if let Some(p) = game.pending_selection.as_ref() {
                // DCGO listed 2+ activatable effects; the engine queued only the
                // chosen one. Any other listed effect that the engine dropped
                // shows up as its own unanswerable rows later.
                self.notes.push(format!(
                    "row {}: MultipleSkills index {} of P{actor} consumed by the chosen effect's own {:?} '{}': \
                     the engine queued no trigger order (DCGO listed 2+ activatable effects)",
                    self.cursor,
                    payload.int_value.unwrap_or_default(),
                    p.kind,
                    p.prompt
                ));
            }
            self.cursor += 1;
            return ReplayStepResult {
                step_number: self.cursor,
                player_id: actor,
                action_id: 0,
                phase_before: game.current_phase.py_name().to_string(),
                phase_after: game.current_phase.py_name().to_string(),
                memory_before: game.memory,
                memory_after: game.memory,
                turn_number: game.turn_count,
                is_game_over: game.game_over,
                winner_id: game.winner,
                divergences: Vec::new(),
            };
        }

        // A mandatory ONE-option prompt the recorded row does not answer
        // (`answer_lone_prompts_before_row`).
        if dcgo_folds {
            self.answer_lone_prompts_before_row(game, payload, actor, self.cursor as usize);
        }

        let phase_before = game.current_phase.py_name().to_string();
        let memory_before = game.memory;
        let memory_pair_before = game.memory_pair;
        let turn_number = game.turn_count;

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

        // DCGO asks "from where?" (Hand / Trash / Don't play) as a bare
        // `SetIntSelection`, THEN asks for the card in a second prompt. Our
        // engine models the same question as ONE optional `UnionZone` prompt
        // (every legal card in every zone, plus a decline). The int row is
        // therefore not an answer to the engine's prompt at all:
        //  - followed by this actor's card pick -> it only narrowed the zone,
        //    and the card row carries the whole decision, so consume it
        //    without touching the engine;
        //  - not followed by a card pick -> no card was chosen, i.e. "Don't
        //    play", which is the engine's decline.
        // Driver: AD1-016 ShineGreymon's [When Digivolving], and every
        // `Hand/Trash/Don't-play` card of the same shape (ST24-15 DNA Charge).
        let zone_choice_int = dcgo_folds
            && payload.prompt == "generic_int"
            && payload.int_value.is_some()
            && payload.card_ids.is_none()
            && payload.targets.is_none()
            && payload.count.is_none()
            && game.pending_selection.as_ref().is_some_and(|p| {
                p.is_optional && matches!(p.kind, crate::selection::SelectionKind::UnionZone { .. })
            });
        // The same shape for DCGO's "activate this optional effect?" gate: it
        // asks a Yes/No (`OptionalSkill`, recorded as `bool_value`) and, on Yes,
        // THEN asks for the card/target. Our engine parks only the pick, with
        // decline as its PASS exit. A recorded `yes` against such a prompt
        // (more than one accept id, not an effect-choice branch list) just means
        // "proceed to the pick", so it is consumed when this actor's pick row
        // follows. A `no` still resolves as the decline (the PASS) as before.
        // A ONE-candidate optional pick folds too, but only when the next row
        // actually names a card/target (a bare yes against a single-candidate
        // prompt otherwise still resolves as that one accept id).
        let next_row_names_pick = self
            .source
            .steps()
            .get(self.cursor as usize + 1)
            .and_then(|s| s.selection.as_ref())
            .is_some_and(|n| {
                n.actor == actor
                    && (n.card_ids.as_ref().is_some_and(|c| !c.is_empty())
                        || n.targets.as_ref().is_some_and(|c| !c.is_empty()))
            });
        let optional_yes_gate = dcgo_folds
            && payload.bool_value == Some(true)
            && payload.card_ids.is_none()
            && payload.targets.is_none()
            && game.pending_selection.as_ref().is_some_and(|p| {
                p.is_optional
                    && p.effect_choices.is_none()
                    && (p.valid_action_ids.len() > 1
                        // A lone ACCEPT of a `Replacement` prompt is the real answer
                        // to the recorded yes, not a gate in front of a pick.
                        || (p.valid_action_ids.len() == 1
                            && next_row_names_pick
                            && !matches!(p.kind, crate::selection::SelectionKind::Replacement)))
            })
            && self
                .source
                .steps()
                .get(self.cursor as usize + 1)
                .and_then(|s| s.selection.as_ref())
                // An UNSTAMPED `generic_bool` behind the yes is another optional
                // effect's "use it?" answer (`classify_unstamped_answer` excludes
                // it from "the actor's next selection" for the same reason), not a
                // pick the yes opens: the yes IS the answer to this prompt (e.g.
                // BT25-030's "add top security to hand?", whose added card then
                // queues BT24-084's trigger that the unstamped bool answers).
                .is_some_and(|n| n.actor == actor && !(n.unstamped && n.prompt == "generic_bool"));
        // DCGO picks a digivolution-source card in two steps: the Digimon that
        // holds it (`SelectPermanentEffect`, targets only), THEN the card
        // (`SelectCardEffect`, zone DigivolutionCards). Our engine parks ONE
        // `UnionZone` prompt over every legal source card, so the Digimon row
        // only narrowed the carrier: consume it when this actor's card row
        // follows, and let that row (which names the card) carry the decision.
        let material_carrier_row = dcgo_folds
            && payload.card_ids.is_none()
            && payload.cancel != Some(true)
            && payload.bool_value.is_none()
            && payload.int_value.is_none()
            && payload.targets.as_ref().is_some_and(|t| !t.is_empty())
            && (game.pending_selection.as_ref().is_some_and(|p| {
                matches!(&p.kind, crate::selection::SelectionKind::UnionZone { zones }
                    if zones.contains(crate::selection::UnionZoneSet::MATERIAL))
            }) || payload.targets.as_ref().and_then(|t| t.first()).is_some_and(|t| {
                source_multi_carrier_has_candidate(game, t)
            }))
            && next_row_names_pick
            && self
                .source
                .steps()
                .get(self.cursor as usize + 1)
                .and_then(|s| s.selection.as_ref())
                .is_some_and(|n| n.card_ids.as_ref().is_some_and(|c| !c.is_empty()));
        // DCGO asks a <Link> declaration's "Link (Cost: N)" as an OptionalSkill
        // Yes/No and THEN the host (cost paid after the host pick). Our engine
        // parks only the mandatory host pick, so a recorded yes in front of the host
        // row is just the gate and is consumed without touching the engine.
        let link_host_yes_gate = dcgo_folds
            && payload.bool_value == Some(true)
            && payload.card_ids.is_none()
            && payload.targets.is_none()
            && next_row_names_pick
            && game.pending_selection_resume.as_ref().is_some_and(|r| {
                matches!(
                    r.frames.first(),
                    Some(crate::resume::ResumeFrame::DigimonLinkHostSelection(_))
                        | Some(crate::resume::ResumeFrame::OptionHandLinkHostSelection(_))
                )
            });
        // A generic_int ZONE choice over a hand/trash union is only followed by a
        // CARD pick (card_ids). A bare targets row after it belongs to a later,
        // unrelated permanent prompt, so it must not make the int look like a
        // narrowing step (BT25-085 Option: "Do not place a card" = 3).
        let generic_zone_int = zone_choice_int;
        // DCGO's "From hand" / "From trash" (or "From digivolution cards")
        // question (`SetBoolSelection`) in front of a card pick is ONE union
        // prompt over both zones in the engine. The bool only names the zone of
        // the pick that follows (`hand_zone_bool`), so it is consumed and that
        // pick, resolved inside the named zone, answers the engine's prompt.
        let zone_bool = if dcgo_folds { self.hand_zone_bool(game, payload) } else { None };
        let zone_choice_bool = matches!(zone_bool, Some(Ok(_)));
        match zone_bool {
            // The card's "do it?" bool with the zone menu next: consumed with no
            // engine action, and the next row is read as the menu.
            Some(Ok(ZoneBool::Gate { menu_next: true })) => {
                self.zone_menu_row = Some(self.cursor + 1);
                self.cursor += 1;
                return ReplayStepResult {
                    step_number: self.cursor,
                    player_id: actor,
                    action_id: 0,
                    phase_before,
                    phase_after: game.current_phase.py_name().to_string(),
                    memory_before,
                    memory_after: game.memory,
                    turn_number,
                    is_game_over: game.game_over,
                    winner_id: game.winner,
                    divergences: Vec::new(),
                };
            }
            Some(Ok(_)) => self.set_pick_zone_hint(actor),
            _ => {}
        }
        // DCGO asks "To which area will you place a card?" (Under this Digimon = 1
        // / Under Tamer with inherited effects = 2 / Don't place = 3) as a numbered
        // menu and, for the Tamer branch ONLY, THEN a `canNoSelect`
        // `SelectPermanentEffect` for which Tamer (DCGO `P/Yellow/P_249.cs`).
        // Our engine parks ONE optional `OwnField` pick over this Digimon and every
        // eligible Tamer (P-249 Strabimon's [Start of Your Main Phase]). So a 2
        // followed by this actor's `SelectPermanentEffect` row only chose the area:
        // the int is consumed and that row carries the decision - its target names
        // the Tamer, or, as a cancel with no targets, DCGO's declined Tamer pick,
        // which declines the optional `OwnField` pick (nothing is placed). Only
        // for a card whose area menu is verified to be exactly P-249's
        // (`selection_resolve::dcgo_placement_area_menu`), and only when the pick
        // is this actor's: a 1 or 3 never opens a Tamer pick, so a
        // `SelectPermanentEffect` after one belongs to something else. 1 and 3
        // resolve against the prompt in `selection_resolve`.
        let placement_area_int = dcgo_folds
            && payload.prompt == "generic_int"
            && payload.int_value == Some(2)
            && payload.card_ids.is_none()
            && payload.targets.is_none()
            && payload.count.is_none()
            && payload.cancel != Some(true)
            && game.pending_selection.as_ref().is_some_and(|p| {
                matches!(p.kind, crate::selection::SelectionKind::OwnField)
                    && p.is_optional
                    && p.selecting_player == actor
                    && game
                        .card_data_for_handle(p.source_card)
                        .is_some_and(|d| super::selection_resolve::dcgo_placement_area_menu(&d.card_id))
            })
            && self
                .source
                .steps()
                .get(self.cursor as usize + 1)
                .and_then(|s| s.selection.as_ref())
                .is_some_and(|n| {
                    n.actor == actor
                        && n.prompt == "SelectPermanentEffect"
                        && n.card_ids.as_ref().is_none_or(|c| c.is_empty())
                        && (n.targets.as_ref().is_some_and(|t| !t.is_empty())
                            || (n.cancel == Some(true) && n.targets.as_ref().is_none_or(|t| t.is_empty())))
                });
        if placement_area_int {
            self.cursor += 1;
            return ReplayStepResult {
                step_number: self.cursor,
                player_id: actor,
                action_id: 0,
                phase_before,
                phase_after: game.current_phase.py_name().to_string(),
                memory_before,
                memory_after: game.memory,
                turn_number,
                is_game_over: game.game_over,
                winner_id: game.winner,
                divergences: Vec::new(),
            };
        }
        let zone_choice_int = zone_choice_int
            || optional_yes_gate
            || link_host_yes_gate
            || material_carrier_row
            || zone_choice_bool;
        let declined_zone_choice;
        let payload = if zone_choice_int {
            let next_names_card = self
                .source
                .steps()
                .get(self.cursor as usize + 1)
                .and_then(|s| s.selection.as_ref())
                .is_some_and(|n| {
                    n.actor == actor
                        && (n.card_ids.as_ref().is_some_and(|c| !c.is_empty())
                            || (n.targets.as_ref().is_some_and(|c| !c.is_empty())
                                && !(generic_zone_int
                                    && !optional_yes_gate
                                    && !link_host_yes_gate
                                    && !material_carrier_row)))
                }) || (optional_yes_gate
                    && game.pending_selection.as_ref().is_some_and(|p| matches!(p.kind, crate::selection::SelectionKind::UnionZone { .. }))
                    && self.source.steps().get(self.cursor as usize + 1)
                        .and_then(|s| s.selection.as_ref())
                        .is_some_and(|n| n.actor == actor && n.prompt == "generic_int")
                    && self.source.steps().get(self.cursor as usize + 2)
                        .and_then(|s| s.selection.as_ref())
                        .is_some_and(|n| n.actor == actor && n.card_ids.as_ref().is_some_and(|ids| !ids.is_empty())
                            && matches!(super::selection_resolve::resolve_next(game, n, 0), Ok(Some(id)) if id != crate::action::space::PASS)));
            if next_names_card {
                self.cursor += 1;
                return ReplayStepResult {
                    step_number: self.cursor,
                    player_id: actor,
                    action_id: 0,
                    phase_before,
                    phase_after: game.current_phase.py_name().to_string(),
                    memory_before,
                    memory_after: game.memory,
                    turn_number,
                    is_game_over: game.game_over,
                    winner_id: game.winner,
                    divergences: Vec::new(),
                };
            }
            let mut p = payload.clone();
            p.int_value = None;
            p.cancel = Some(true);
            declined_zone_choice = p;
            &declined_zone_choice
        } else {
            payload
        };

        // `MultipleSkills` -1 against an optional prompt that is NOT a trigger order: DCGO listed the effect the
        // engine parks as a yes/no gate ("Use Cost reduction to reduce Option-use cost?") among several in its
        // "Multiple effects" panel, and declined them all - the engine's decline of that gate.
        if dcgo_folds
            && payload.prompt == "MultipleSkills"
            && payload.int_value == Some(-1)
            && game.pending_selection.as_ref().is_some_and(|p| {
                !matches!(p.kind, crate::selection::SelectionKind::TriggerOrder)
                    && p.is_optional
                    && p.selecting_player == actor
            })
        {
            game.decode_action(crate::action::space::PASS, actor);
            self.cursor += 1;
            return ReplayStepResult {
                step_number: self.cursor,
                player_id: actor,
                action_id: crate::action::space::PASS,
                phase_before,
                phase_after: game.current_phase.py_name().to_string(),
                memory_before,
                memory_after: game.memory,
                turn_number,
                is_game_over: game.game_over,
                winner_id: game.winner,
                divergences: Vec::new(),
            };
        }

        // `MultipleSkills` -1 is DCGO's "Don't activate these effects": every
        // remaining simultaneous effect is declined. Our engine resolves them one
        // at a time - pick the next entry of the trigger-order list, then decline
        // the optional prompt that effect opens - so decline-all is that loop.
        // Stops at the first prompt that is neither (a mandatory effect cannot be
        // declined, and is left for the following rows).
        //
        // Only the order of the seat that answered: P1's -1 against P0's parked
        // order is not an answer to it (the actor check reports the row).
        if dcgo_folds
            && payload.prompt == "MultipleSkills"
            && payload.int_value == Some(-1)
            && game.pending_selection.as_ref().is_some_and(|p| {
                matches!(p.kind, crate::selection::SelectionKind::TriggerOrder) && p.selecting_player == actor
            })
        {
            let last_action = match dcgo_decline_all(game, 0) {
                Ok((after, last, note)) => {
                    *game = after;
                    if let Some(note) = note {
                        self.notes.push(format!("row {}: {note}", self.cursor));
                    }
                    last
                }
                Err(reason) => {
                    self.divergences.push(Divergence {
                        step: self.cursor,
                        action_id: 0,
                        actor,
                        phase: payload.phase.clone(),
                        kind: DivergenceKind::SelectionResolution { reason },
                        board: describe_board(game),
                    });
                    self.paused = true;
                    return self.no_progress_result(game);
                }
            };
            self.cursor += 1;
            return ReplayStepResult {
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
            };
        }

        // The inverse fold: the engine models a "by returning N cards, ..." cost as
        // a leading accept/decline `EffectChoice` that DCGO never asks - it asks
        // only the `canNoSelect` card pick. A recorded card/target pick that names
        // no branch of such a gate is therefore the acceptance AND the pick:
        // accept first, then resolve the row against the pick the branch opens.
        // A gate whose entries DO belong to the picked card (a genuine card
        // choice) keeps the ordinary path.
        if dcgo_folds
            && payload.cancel != Some(true)
            && payload.bool_value.is_none()
            && payload.int_value.is_none()
        {
            let first_card = payload
                .card_ids
                .as_ref()
                .and_then(|c| c.first().cloned());
            let named_pick = first_card.is_some()
                || payload.targets.as_ref().is_some_and(|t| !t.is_empty());
            let gate = game.pending_selection.as_ref().and_then(|p| {
                if !named_pick
                    || !matches!(p.kind, crate::selection::SelectionKind::EffectChoice)
                    || p.selecting_player != actor
                {
                    return None;
                }
                let (yes, _) = super::selection_resolve::yes_no_branches(p)?;
                let belongs_to_pick = first_card.as_deref().is_some_and(|want| {
                    p.effect_choices.iter().flatten().any(|e| {
                        e.source_card
                            .and_then(|c| game.card_data_for_handle(c))
                            .is_some_and(|d| d.card_id == want)
                    })
                });
                (!belongs_to_pick).then_some(yes)
            });
            if let Some(yes) = gate {
                game.decode_action(yes, actor);
            }
        }

        // Branch-then-pick: the engine leads with a branch `EffectChoice` that
        // DCGO derives from the card it asks for (BT25-078 Gazimon: DCGO picks
        // the revealed card first, then asks hand-vs-source only when the card
        // has the trait; the engine asks the branch first). The row therefore
        // answers a prompt it was not recorded against. Each branch is PROBED on
        // a clone of the game: the branch whose follow-up prompt accepts the row
        // (or, for a cancel, can be declined) is the one DCGO took. Applied only
        // when exactly one branch qualifies; two qualifying branches need DCGO's
        // trailing true/false row to name one; none leaves the row to diverge.
        if let Some((branch, row_consumed)) =
            dcgo_folds.then(|| self.probe_branch_for_row(game, payload, actor, self.cursor as usize)).flatten()
        {
            let was_trigger_order = game
                .pending_selection
                .as_ref()
                .is_some_and(|p| matches!(p.kind, crate::selection::SelectionKind::TriggerOrder));
            game.decode_action(branch, actor);
            if was_trigger_order && Self::is_outer_optional_gate(game) {
                game.decode_action(crate::action::space::REPLACEMENT_ACCEPT, actor);
            }
            // A cancel whose chosen branch left nothing to decline (the effect
            // fizzled) has been fully answered by the branch itself.
            if row_consumed {
                self.cursor += 1;
                return ReplayStepResult {
                    step_number: self.cursor,
                    player_id: actor,
                    action_id: branch,
                    phase_before,
                    phase_after: game.current_phase.py_name().to_string(),
                    memory_before,
                    memory_after: game.memory,
                    turn_number,
                    is_game_over: game.game_over,
                    winner_id: game.winner,
                    divergences: Vec::new(),
                };
            }
        }

        // A card's menu int against the actor's engine trigger order: DCGO had
        // one activatable effect there and asked no order (`int_menu_trigger_entry`).
        if dcgo_folds {
            if let Some(entry) = Self::int_menu_trigger_entry(game, payload, actor) {
                game.decode_action(entry, actor);
            }
        }

        // A "may" replacement / triggered effect parks its own accept (REPLACEMENT_ACCEPT, decline = PASS) before
        // its pick; DCGO usually has no separate accept and records only the pick (e.g. a "when this Digimon would
        // leave the battle area, you may play 1 ..." card). When the row cannot answer the accept prompt but
        // answers the prompt that accepting opens (probed on a clone), the row is the acceptance AND the pick.
        if dcgo_folds
            && game.pending_selection.as_ref().is_some_and(|p| {
            p.is_optional
                && p.selecting_player == actor
                && p.valid_action_ids == [crate::action::space::REPLACEMENT_ACCEPT]
        }) && payload.cancel != Some(true)
            && payload.bool_value.is_none()
            && resolve_next(game, payload, 0).is_err()
        {
            let mut probe = game.clone();
            probe.decode_action(crate::action::space::REPLACEMENT_ACCEPT, actor);
            if matches!(resolve_next(&probe, payload, 0), Ok(Some(_))) {
                game.decode_action(crate::action::space::REPLACEMENT_ACCEPT, actor);
            }
        }

        let normalized = dcgo_folds
            .then(|| super::selection_resolve::normalize_lilamon_payment(game, payload))
            .flatten();
        let payload = normalized.as_ref().unwrap_or(payload);
        let n_picks = super::selection_resolve::payload_pick_count(payload);
        let mut picks_done = 0usize;
        let mut last_action: u16 = 0;
        loop {
            // A row of one seat never answers the other seat's prompt: the engine
            // rejects that answer (`WrongPlayer`), so applying it would only count
            // the row as consumed. Once this row's own picks are all applied, a
            // prompt of another seat is simply what comes next (the row is done).
            // A contentless cancel names no decision of the waiting seat: noted.
            if let Some(p) = game.pending_selection.as_ref().filter(|p| p.selecting_player != actor) {
                if picks_done > 0 && picks_done >= n_picks {
                    break;
                }
                // A `MultipleSkills` -1 against another seat's trigger order is
                // not a contentless cancel: it is that seat's answer to ITS OWN
                // stack (DCGO's "Don't activate these effects"), which the
                // engine does not hold. The games differ: an actor divergence.
                let declines_an_order = payload.prompt == "MultipleSkills"
                    && payload.int_value == Some(-1)
                    && matches!(p.kind, crate::selection::SelectionKind::TriggerOrder);
                if picks_done == 0 && selection_row_content(payload).is_none() && !declines_an_order {
                    self.notes.push(format!(
                        "row {}: {} {} of P{actor} consumed with no effect: the engine is waiting on P{}'s \
                         {:?} '{}', not on P{actor}",
                        self.cursor, payload.prompt, decline_label(payload), p.selecting_player, p.kind, p.prompt
                    ));
                    break;
                }
                let expected = p.selecting_player;
                self.notes.push(format!(
                    "row {}: P{actor}'s {} row {} met P{expected}'s {:?} '{}' (actor mismatch)",
                    self.cursor,
                    payload.prompt,
                    selection_row_content(payload).unwrap_or_else(|| decline_label(payload).into()),
                    p.kind,
                    p.prompt
                ));
                self.divergences.push(Divergence {
                    step: self.cursor,
                    action_id: last_action,
                    actor,
                    phase: payload.phase.clone(),
                    kind: DivergenceKind::Actor { expected, recorded: actor },
                    board: describe_board(game),
                });
                self.paused = true;
                return self.no_progress_result(game);
            }
            match resolve_next(game, payload, picks_done) {
                // Nothing parked and nothing applied: the engine never asked this
                // row's question. A row that carries a decision is drift the
                // replay cannot see past (an effect the engine did not run, or
                // ran without this choice): it halts. A contentless row (a
                // cancel) is consumed with a non-halting note.
                Ok(None) if picks_done == 0 && (game.pending_selection.is_none() || is_forced_count_row(payload)) => {
                    // A `SelectCountEffect` whose candidate set is ONE value is
                    // no decision: DCGO opens it even when the number is forced
                    // (`IDegeneration.Degeneration`, CardController.cs:5281-5297:
                    // `MaxCount = min(digivolution cards, N)`, `CanNoSelect:
                    // false`, so one trashable card leaves the set {1}), while
                    // the engine installs no prompt for a forced number (CR 4.3
                    // 16-12-1). It is consumed only when the engine resolved
                    // that effect since the decision before it
                    // (`forced_count_evidence`); otherwise DCGO ran a count the
                    // engine never reached, and the replay halts. With a prompt
                    // parked `resolve_next` consumes it the same way.
                    if is_forced_count_row(payload) {
                        match self.forced_count_evidence(game, payload) {
                            Ok(evidence) => {
                                self.notes.push(format!(
                                    "row {}: SelectCountEffect {} of P{actor} consumed with no effect: its candidate \
                                     set is that one value, so DCGO's count was forced, and the engine asked nothing \
                                     - it resolved the effect without a count ({evidence})",
                                    self.cursor,
                                    payload.count.unwrap_or_default()
                                ));
                                break;
                            }
                            Err(why) => {
                                // This arm also runs with a prompt parked (`is_forced_count_row` above): say which.
                                let prompt = match game.pending_selection.as_ref() {
                                    Some(p) => format!("the engine's own '{}' is parked (not this count)", p.prompt),
                                    None => "met no prompt".to_string(),
                                };
                                self.divergences.push(Divergence {
                                    step: self.cursor,
                                    action_id: 0,
                                    actor,
                                    phase: payload.phase.clone(),
                                    kind: DivergenceKind::SelectionResolution {
                                        reason: format!(
                                            "row {}: P{actor}'s SelectCountEffect {} has one candidate (a forced \
                                             count, CardController.cs:5281-5297) and {prompt}, but {why}: DCGO \
                                             counted an effect the engine did not resolve",
                                            self.cursor,
                                            payload.count.unwrap_or_default()
                                        ),
                                    },
                                    board: describe_board(game),
                                });
                                self.paused = true;
                                return self.no_progress_result(game);
                            }
                        }
                    }
                    match selection_row_content(payload) {
                        Some(content) => {
                            self.divergences.push(Divergence {
                                step: self.cursor,
                                action_id: 0,
                                actor,
                                phase: payload.phase.clone(),
                                kind: DivergenceKind::SelectionResolution {
                                    reason: format!(
                                        "P{actor}'s {} row {content} met no prompt: the engine parked no \
                                         decision for it (DCGO asked one here)",
                                        payload.prompt
                                    ),
                                },
                                board: describe_board(game),
                            });
                            self.paused = true;
                            return self.no_progress_result(game);
                        }
                        None => {
                            self.notes.push(format!(
                                "row {}: {} {} of P{actor} consumed with no effect: no prompt was parked",
                                self.cursor, payload.prompt, decline_label(payload)
                            ));
                            break;
                        }
                    }
                }
                Ok(None) => break,
                Ok(Some(id)) => {
                    // The resolver emits ids from the prompt's own candidates (or
                    // PASS); the engine's checks decide. A rejected answer would be a
                    // silent no-op in `decode_action`, so it is a divergence here.
                    if let Some(reason) = selection_answer_rejection(game, id, actor) {
                        self.divergences.push(Divergence {
                            step: self.cursor,
                            action_id: id,
                            actor,
                            phase: payload.phase.clone(),
                            kind: DivergenceKind::SelectionResolution { reason },
                            board: describe_board(game),
                        });
                        self.paused = true;
                        return self.no_progress_result(game);
                    }
                    game.decode_action(id, actor);
                    last_action = id;
                    picks_done += 1;
                    // Safety valve: a payload can't sanely resolve into more
                    // engine actions than picks + trailing PASS.
                    if picks_done > n_picks + 1 {
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

    /// An EFFECT-initiated DNA digivolve as DCGO records it, folded onto the
    /// engine's partner -> hand prompt pair.
    ///
    /// DCGO's `DNADigivolvePermanentsIntoHandOrTrashCard`
    /// (`CardEffectCommons/DNADigivolveEffects.cs:456-640`, used by every
    /// `may_dna_digivolve_now` card: Nyaromon (EX12-001)'s inherited
    /// [End of Your Turn], BT12-021, BT12-047, BT17-007, BT17-019, BT22-008,
    /// BT22-017...) asks for the DNA card FIRST (`SelectHandEffect`), then for
    /// the two material Digimon (`SelectJogressEffect.SelectDigivolutionRoots`,
    /// recorded as TWO `SelectPermanentEffect` root rows, in the order they were
    /// clicked), and only then moves on (the "may attack" follow-up is its own
    /// `SelectAttackEffect` row). The engine asks the PARTNER first
    /// (`MayDnaPartnerSelection`, an `OwnField` pick that excludes the anchor),
    /// then the hand card (`MayDnaResultSelection`).
    ///
    /// Called when `payload` is such a hand row while the engine is parked on
    /// that partner pick and the next two rows are own-side root rows. The
    /// effect's carrier (`anchor`) is one of the two materials (CR 4.3 8-2: the
    /// effect digivolves "this Digimon and another Digimon"), so the pair must
    /// be exactly `{anchor, partner}`; the non-anchor root answers the partner
    /// pick, the hand row answers the card pick, and all three rows are
    /// consumed. A pair that does not contain the anchor is an honest
    /// divergence (`Some` with a halted result), never a guess. `None` = this
    /// is not that shape; the ordinary path runs.
    fn fold_effect_dna_root_rows(
        &mut self,
        game: &mut Game,
        payload: &crate::dcgo_recording::SelectionRow,
        actor: PlayerId,
    ) -> Option<ReplayStepResult> {
        use crate::action::space::ATTACK_START;

        if payload.prompt != "SelectHandEffect"
            || payload.cancel == Some(true)
            || payload.card_ids.as_ref().map_or(true, |c| c.len() != 1)
        {
            return None;
        }
        let pending = game.pending_selection.as_ref()?;
        if pending.selecting_player != actor
            || pending.kind != crate::selection::SelectionKind::OwnField
        {
            return None;
        }
        let (controller, anchor) = match game.pending_selection_resume.as_ref()?.frames.last()? {
            crate::resume::ResumeFrame::MayDnaPartnerSelection(s) => (s.controller, s.anchor),
            _ => return None,
        };
        if controller != actor {
            return None;
        }
        let valid_partner_actions = pending.valid_action_ids.clone();

        let cur = self.cursor as usize;
        let root_rows: Vec<crate::dcgo_recording::SelectionRow> = (1..=2)
            .map(|k| {
                self.source
                    .steps()
                    .get(cur + k)
                    .and_then(|s| s.selection.clone())
                    .filter(|r| {
                        r.actor == actor
                            && r.prompt == "SelectPermanentEffect"
                            && r.card_ids.is_none()
                            && r.cancel != Some(true)
                            && r.targets.as_ref().is_some_and(|t| {
                                t.len() == 1 && t[0].player == controller && t[0].frame >= 0
                            })
                    })
            })
            .collect::<Option<Vec<_>>>()?;

        let halt = |this: &mut Self, game: &Game, reason: String| {
            this.divergences.push(Divergence {
                step: this.cursor,
                action_id: 0,
                actor,
                phase: payload.phase.clone(),
                kind: DivergenceKind::SelectionResolution { reason },
                board: describe_board(game),
            });
            this.paused = true;
            this.no_progress_result(game)
        };

        // Engine slots of the two roots (DCGO frame order -> play order).
        let slots: Vec<usize> = root_rows
            .iter()
            .map(|row| {
                let mapped = remap_selection_targets(game, row);
                let row = mapped.as_ref().unwrap_or(row);
                row.targets.as_ref().map(|t| t[0].frame as usize).unwrap_or(usize::MAX)
            })
            .collect();
        let ids = engine_board_ids(game, controller);
        let anchor_slot = anchor.index as usize;
        let anchor_id = ids.get(anchor_slot).cloned();
        let describe = |slot: usize| -> String {
            ids.get(slot).cloned().unwrap_or_else(|| format!("<no Digimon at slot {slot}>"))
        };
        if slots[0] == slots[1] {
            return Some(halt(
                self,
                game,
                format!(
                    "DNA material roots name the same Digimon twice (slot {}): an effect-initiated DNA \
                     digivolve needs the anchor (slot {anchor_slot}) and a distinct partner",
                    slots[0]
                ),
            ));
        }
        // The root that is the anchor: its slot, else (DCGO's board is card ids,
        // so a duplicate copy is indistinguishable) the only root showing the
        // anchor's card id.
        let anchor_root = if let Some(i) = slots.iter().position(|s| *s == anchor_slot) {
            Some(i)
        } else {
            let same: Vec<usize> = (0..2)
                .filter(|i| slots[*i] < ids.len() && Some(&ids[slots[*i]]) == anchor_id.as_ref())
                .collect();
            (same.len() == 1).then(|| same[0])
        };
        let Some(anchor_root) = anchor_root else {
            return Some(halt(
                self,
                game,
                format!(
                    "DNA material roots {} (slot {}) and {} (slot {}) do not include the effect's anchor \
                     {} (slot {anchor_slot}): an effect-initiated DNA digivolve is \"this Digimon and another \
                     Digimon\" (CR 4.3 8-2), so the recorded pair is not the engine's {{anchor, partner}}",
                    describe(slots[0]),
                    slots[0],
                    describe(slots[1]),
                    slots[1],
                    anchor_id.as_deref().unwrap_or("?"),
                ),
            ));
        };
        let partner_slot = slots[1 - anchor_root];
        let partner_action = ATTACK_START + partner_slot as u16;
        if !valid_partner_actions.contains(&partner_action) {
            return Some(halt(
                self,
                game,
                format!(
                    "DNA material root {} (slot {partner_slot}) is not an offered partner for the anchor {} \
                     (offered actions {valid_partner_actions:?})",
                    describe(partner_slot),
                    anchor_id.as_deref().unwrap_or("?"),
                ),
            ));
        }

        // DCGO's recorder read its memory gauge at the hand row, before the DNA.
        let (phase_before, memory_before, memory_pair_before, turn_number) =
            (game.current_phase.py_name().to_string(), game.memory, game.memory_pair, game.turn_count);
        check_dcgo_memory(
            &mut self.divergences,
            self.cursor,
            0,
            actor,
            &payload.phase,
            self.source.steps()[cur].dcgo_memory,
            self.source.my_player_id(),
            memory_before,
            memory_pair_before,
            game,
        );

        game.decode_action(partner_action, actor);
        let hand_action = match super::selection_resolve::resolve_next_dcgo(game, payload, 0) {
            Ok(Some(a)) => a,
            Ok(None) => {
                return Some(halt(
                    self,
                    game,
                    format!(
                        "after the DNA partner {} (slot {partner_slot}) the engine offered no DNA card, but \
                         DCGO's hand row picked {:?}",
                        describe(partner_slot),
                        payload.card_ids
                    ),
                ))
            }
            Err(reason) => return Some(halt(self, game, reason)),
        };
        game.decode_action(hand_action, actor);
        self.cursor += 3;
        Some(ReplayStepResult {
            step_number: self.cursor,
            player_id: actor,
            action_id: hand_action,
            phase_before,
            phase_after: game.current_phase.py_name().to_string(),
            memory_before,
            memory_after: game.memory,
            turn_number,
            is_game_over: game.game_over,
            winner_id: game.winner,
            divergences: Vec::new(),
        })
    }

    /// Answer the mandatory ONE-option prompts (and the actor's optional
    /// one-candidate pick) that DCGO resolves without a row, in front of the
    /// recorded selection row `payload` at index `row_at`, when that row cannot
    /// resolve against them. `step()` runs it for every selection row of a
    /// DCGO source; `remote_trigger_order_choice` runs it before testing its
    /// evidence row, so a probe meets that row in the state `step()` would
    /// (Owen Dreadnought (EX11-054)'s lone <Progress> pick, which DCGO takes
    /// without asking, EX11_054.cs:66-69; raw-20261113T011922Z#g3 raw seq
    /// 342-344).
    fn answer_lone_prompts_before_row(
        &self,
        game: &mut Game,
        payload: &crate::dcgo_recording::SelectionRow,
        actor: PlayerId,
        row_at: usize,
    ) {
        // A mandatory ONE-option prompt the recorded row does not answer.
        //
        // Our engine installs a pick even when only one candidate exists (e.g.
        // ST24-01's "by trashing the bottom face-down card from under any of your
        // Tamers" with a single Tamer); DCGO asks only when it is a real choice
        // (`MatchConditionPermanentCount(..) > 1`), so the recording has no row
        // for it. When this row cannot be resolved against that prompt, the
        // forced answer is applied first so the row meets the prompt it was
        // recorded against - and, in `step()`, before its DCGO memory check,
        // which DCGO read AFTER the forced step. With a single option there is
        // no choice a row could have made differently, so no divergence can
        // hide here.
        for _ in 0..4 {
            let Some(pending) = game.pending_selection.as_ref() else {
                break;
            };
            // An OPTIONAL one-candidate pick qualifies too (its decline is the
            // PASS, not a second accept id), but only when the recorded row cannot
            // resolve against it at all - the row then belongs to a LATER prompt,
            // which proves the effect went on, i.e. DCGO took its lone candidate
            // without asking. A `Replacement` accept/decline is a real question and
            // is never auto-answered here.
            // ... and only the ACTOR's: this row's failure to resolve says
            // nothing about whether ANOTHER seat used its "you may" (the actor
            // check below reports that row instead). A MANDATORY lone pick of
            // either seat has no choice to make, so it stays answered.
            let optional_lone_pick = pending.is_optional
                && pending.selecting_player == actor
                && !matches!(pending.kind, crate::selection::SelectionKind::Replacement);
            if (pending.is_optional && !optional_lone_pick) || pending.valid_action_ids.len() != 1 {
                break;
            }
            // An ordering row (`MultipleSkills`) of one seat never stands in front
            // of a prompt DCGO skipped for the OTHER seat: the panel belongs to the
            // seat that orders, and so does the skipped pick before it (the same
            // rule as the lone-pick loop in front of it above). Another seat's
            // prompt is left for the actor check, which reports the row.
            if payload.prompt == "MultipleSkills" && pending.selecting_player != actor {
                break;
            }
            // DCGO's first half of a two-step digivolution-source pick: a
            // carrier-only row (targets, no cards) against our single MATERIAL
            // `UnionZone` prompt, followed by this actor's card row. It is folded by
            // `material_carrier_row` below, so the lone candidate must NOT be
            // auto-answered here (that would pay the following cost and move memory
            // before the DCGO memory check, and orphan the card rows).
            let carrier_only_row = payload.card_ids.is_none()
                && payload.cancel != Some(true)
                && payload.bool_value.is_none()
                && payload.int_value.is_none()
                && payload.targets.as_ref().is_some_and(|t| !t.is_empty());
            if carrier_only_row
                && (matches!(&pending.kind, crate::selection::SelectionKind::UnionZone { zones }
                    if zones.contains(crate::selection::UnionZoneSet::MATERIAL))
                    || payload.targets.as_ref().and_then(|t| t.first()).is_some_and(|t| {
                        source_multi_carrier_has_candidate(game, t)
                    }))
                && self
                    .source
                    .steps()
                    .get(row_at + 1)
                    .and_then(|s| s.selection.as_ref())
                    .is_some_and(|n| {
                        n.actor == actor && n.card_ids.as_ref().is_some_and(|c| !c.is_empty())
                    })
            {
                break;
            }
            if super::selection_resolve::resolve_next(game, payload, 0).is_ok() {
                break;
            }
            let (forced_actor, forced) = (pending.selecting_player, pending.valid_action_ids[0]);
            game.decode_action(forced, forced_actor);
        }
    }

    /// Answer the prompts DCGO resolves without recording a row, in front of
    /// the recorded row at index `at` (see each helper). Only a DCGO recording
    /// omits them; every other source recorded them. `step()` runs this before
    /// every row, and `remote_trigger_order_choice`'s probe runs it before every
    /// row it applies, so a probe meets each row in the state the real replay
    /// would.
    ///
    /// Returns a halting reason when the next prompt is one DCGO decided
    /// without a row and the replay cannot reproduce (`dcgo_unrecorded_stack_order`).
    fn answer_dcgo_silent_prompts(&mut self, game: &mut Game, at: usize) -> Option<String> {
        if !self.source.omits_dcgo_silent_prompts() {
            return None;
        }
        self.answer_forced_single_ordering_prompt(game);
        self.answer_dcgo_silent_partition_pick(game);
        self.answer_dcgo_silent_lone_target_pick(game, at);
        if let Some(reason) = self.dcgo_unrecorded_stack_order(game, at) {
            return Some(reason);
        }
        self.answer_dcgo_ordinary_cost_route(game, at);
        None
    }

    /// Answer a parked, mandatory, ONE-option card-ordering prompt that DCGO
    /// never asked.
    ///
    /// The engine deliberately surfaces even a trivial ordering decision to the
    /// agent (`EffectContext::place_remainder_on_deck`: "1 card -> still
    /// installs a 1-choice `OrderedPermutation`", no-approximations policy), but
    /// DCGO places a lone remaining card without a prompt, so the recording has
    /// no row for it. It is the mirror image of `resolve_next` skipping a
    /// recorded prompt the engine never parked: the forced answer is applied
    /// before the next recorded row so that row meets the prompt it was recorded
    /// against. Only `remaining: 1` qualifies; with two or more cards the order
    /// is a real choice and DCGO records it.
    fn answer_forced_single_ordering_prompt(&mut self, game: &mut Game) {
        use crate::selection::SelectionKind;
        let mut guard = 0;
        while let Some(pending) = game.pending_selection.as_ref() {
            guard += 1;
            // ... and a trigger order with ONE trigger left: DCGO's MultipleSkills resolves a lone effect without
            // asking (`skillInfos_active.Count == 1`), so it records no row; the engine still parks the order. DCGO
            // recordings only - a native recording logs this pick like any other engine action.
            let lone_trigger = matches!(pending.kind, SelectionKind::TriggerOrder);
            // DCGO recordings only. A native recording logs the forced pick itself; answering it here as well made the
            // recorded pick land on the NEXT decision (a main-phase id decodes for the turn player: the sim's
            // Toho-vs-Jupitermon P0 "place the last card" 30 played P1's hand slot - BT24-090 - a step early)
            if self.policy != StepPolicy::CheckThenApply {
                break;
            }
            if guard > 8
                || !(matches!(pending.kind, SelectionKind::OrderedPermutation { remaining: 1 }) || lone_trigger)
                || pending.is_optional
                || pending.valid_action_ids.len() != 1
            {
                break;
            }
            let (actor, action) = (pending.selecting_player, pending.valid_action_ids[0]);
            game.decode_action(action, actor);
        }
    }

    /// Answer a parked `<Partition>` source pick that DCGO never prompted.
    ///
    /// DCGO offers pick k only from slot k's pool and skips a one-card pool
    /// (see `keyword_effects::partition_dcgo_silent_pick`); the engine lets the
    /// controller order the plays (16-29-4 fixes no order), so it parks a pick
    /// for which the recording has no row. Only under the DCGO-oracle policy:
    /// a native recording logs every engine action, this one included.
    fn answer_dcgo_silent_partition_pick(&mut self, game: &mut Game) {
        if self.policy != StepPolicy::CheckThenApply {
            return;
        }
        for _ in 0..4 {
            let Some(actor) = game.pending_selection.as_ref().map(|p| p.selecting_player) else {
                return;
            };
            let Some(action) = crate::cards::keyword_effects::partition_dcgo_silent_pick(game)
            else {
                return;
            };
            game.decode_action(action, actor);
        }
    }

    /// The entry of the actor's parked trigger order that a recorded DCGO
    /// `generic_int` (a card-authored `SelectionElement<int>` menu value)
    /// belongs to, when exactly one entry's own menu takes it.
    ///
    /// DCGO's `MultipleSkills` lists only the stacked effects whose
    /// `CanActivate` holds (MultipleSkills.cs:217-221, :261-262), activates a
    /// lone one without asking (`skillInfos_active.Count == 1`, :273-278) and
    /// records every choice among two or more through `SetTargetSkill`
    /// (:354, :420). So the actor's int meeting the engine's order (no
    /// `MultipleSkills` row in front of it) proves DCGO had ONE activatable
    /// effect there - the one that asked this menu. The engine also queues
    /// effects DCGO could not activate (Virus Busters (EX12-069)'s [Security]
    /// play, whose `CanActivateCondition` needs a same-level [VB] card in hand,
    /// EX12_069.cs:43-50). Each entry is chosen on a clone: it qualifies only
    /// when it parks an `EffectChoice` of the actor, installed by that entry's
    /// own card, whose DCGO script has an int menu
    /// (`dcgo_card_has_int_menu`), and the int resolves there (by the card's
    /// own menu constants, never by position). An entry that parks an
    /// optional-activation gate does not qualify: DCGO would have recorded
    /// that "use?" answer first. `None` unless exactly one entry qualifies; the
    /// row then keeps its ordinary path (and halts if nothing takes it).
    fn int_menu_trigger_entry(
        game: &Game,
        payload: &crate::dcgo_recording::SelectionRow,
        actor: PlayerId,
    ) -> Option<u16> {
        use crate::selection::SelectionKind;
        if payload.prompt != "generic_int"
            || payload.int_value.is_none()
            || payload.unstamped
            || payload.engine_action.is_some()
        {
            return None;
        }
        let pending = game.pending_selection.as_ref()?;
        if !matches!(pending.kind, SelectionKind::TriggerOrder) || pending.selecting_player != actor {
            return None;
        }
        let entries: Vec<(u16, crate::card_source::CardHandle)> = pending
            .effect_choices
            .as_ref()?
            .iter()
            .filter(|e| pending.valid_action_ids.contains(&e.action_id))
            .filter_map(|e| e.source_card.map(|c| (e.action_id, c)))
            .collect();
        if entries.len() < 2 {
            return None;
        }
        let mut qualifying = Vec::new();
        for (id, card) in entries {
            let mut probe = game.clone();
            probe.decode_action(id, actor);
            let Some(p) = probe.pending_selection.as_ref() else {
                continue;
            };
            let card_id = probe.card_data_for_handle(p.source_card).map(|d| d.card_id.clone()).unwrap_or_default();
            if !matches!(p.kind, SelectionKind::EffectChoice)
                || p.selecting_player != actor
                || p.source_card != card
                || !super::selection_resolve::dcgo_card_has_int_menu(&card_id)
            {
                continue;
            }
            if matches!(super::selection_resolve::resolve_next(&probe, payload, 0), Ok(Some(a)) if a != crate::action::space::PASS)
            {
                qualifying.push(id);
            }
        }
        match qualifying.as_slice() {
            [only] => Some(*only),
            _ => None,
        }
    }

    /// Is the parked prompt a clause-level "you may" accept/decline gate
    /// (`ResumeFrame::OuterOptionalTrigger`, single accept id)?
    fn is_outer_optional_gate(game: &Game) -> bool {
        game.pending_selection.as_ref().is_some_and(|p| {
            p.is_optional
                && p.valid_action_ids == [crate::action::space::REPLACEMENT_ACCEPT]
                && game.pending_selection_resume.as_ref().is_some_and(|r| {
                    matches!(
                        r.frames.first(),
                        Some(crate::resume::ResumeFrame::OuterOptionalTrigger(_))
                    )
                })
        })
    }

    /// What an unstamped DCGO `OptionalSkill.SetUseOptional` answer ("use this
    /// optional effect?", `value`) does to the parked prompt when that prompt
    /// is an optional-ACTIVATION gate of `actor`; `None` when it is not one.
    /// `Some(Some(id))` is the engine action to apply, `Some(None)` a matched
    /// gate that needs no engine action.
    ///
    /// The engine asks a "you may" activation in three shapes, each marked
    /// precisely:
    /// * an accept/decline `SelectionKind::Replacement` prompt
    ///   (`REPLACEMENT_ACCEPT` the only id, decline = PASS):
    ///   `install_outer_optional_trigger_selection` (an optional triggered
    ///   effect whose body starts mandatory, `ResumeFrame::OuterOptionalTrigger`)
    ///   or the optional-replacement accept (`ResumeFrame::OptionalReplacement`).
    ///   yes = ACCEPT, no = PASS;
    /// * the pre-cost one-entry `TriggerOrder` with PASS (an optional effect
    ///   with an activation cost). yes = that entry, no = PASS;
    /// * the effect's first pick, when its "may" is folded into it
    ///   (`Game::pending_is_optional_activation`). no = PASS (the decline);
    ///   yes = proceed to that pick, which the following row answers - no
    ///   engine action.
    ///
    /// A yes/no-shaped `EffectChoice` is NOT one: it is a `select_effect_choice`
    /// inside an effect body (ST24-06's "Trash 2 face-down cards" / "Don't"),
    /// which DCGO asks as its own selection, not as `SetUseOptional`.
    fn dcgo_use_optional_answer(game: &Game, actor: PlayerId, value: bool) -> Option<Option<u16>> {
        use crate::action::space::{PASS, REPLACEMENT_ACCEPT};
        use crate::selection::SelectionKind;
        let p = game.pending_selection.as_ref()?;
        if p.selecting_player != actor || !p.is_optional {
            return None;
        }
        let yes = match p.kind {
            SelectionKind::Replacement if p.valid_action_ids == [REPLACEMENT_ACCEPT] => Some(REPLACEMENT_ACCEPT),
            SelectionKind::TriggerOrder if p.valid_action_ids.len() == 1 => Some(p.valid_action_ids[0]),
            _ if game.pending_is_optional_activation() => None,
            _ => return None,
        };
        Some(if value { yes } else { Some(PASS) })
    }

    /// What an unstamped DCGO "use this optional effect?" answer (`value`, row
    /// `row` of `actor`; `next` is the row after it) does here.
    ///
    /// * The parked prompt is an optional-activation gate of `actor`
    ///   (`dcgo_use_optional_answer`): [`UnstampedAnswer::Apply`].
    /// * `false` while `actor` has an OPTIONAL prompt parked that is not such a
    ///   gate: [`UnstampedAnswer::Halt`]. DCGO declined an effect; the replay
    ///   cannot tell whether it was this prompt's, and consuming the row would
    ///   lose the decline.
    /// * `true` with NOTHING parked while `actor`'s next row is a selection (not
    ///   another unstamped answer): [`UnstampedAnswer::Halt`]. DCGO used an
    ///   effect whose picks follow, and the engine never offered it.
    /// * With NOTHING parked, the engine's event log since the last recorded
    ///   row that is not an unstamped answer (`window`) decides, through
    ///   [`effect_activity`]:
    ///   - `true` and NO effect activity: [`UnstampedAnswer::Halt`]. DCGO
    ///     used an optional effect (its "may" was asked, so it could activate,
    ///     OptionalSkill.cs), and since the last decision the engine resolved
    ///     no effect at all: it never offered the effect (e.g. a "you may gain
    ///     1 memory" the engine never triggered).
    ///   - `false` and effect activity: [`UnstampedAnswer::CannotTell`]. DCGO
    ///     declined an optional effect, yet the engine resolved effects with no
    ///     recorded answer (a memory gain, a target, a reveal, an effect play):
    ///     one of them may be the effect DCGO declined, run by the engine as
    ///     mandatory. The answer names no card, so the replay cannot tell. A
    ///     step halts on it; a trigger-order probe counts the entry as
    ///     undetermined, not refuted (`remote_trigger_order_choice`).
    /// * Anything else: [`UnstampedAnswer::Note`]. Nothing the replay can apply
    ///   and nothing visibly lost; the note lets a later divergence be traced.
    ///   MEASURED GAP (round-5 review, finding 7): `effect_activity` reads four
    ///   event kinds only - a `MemoryChange` with a source card, an
    ///   `EffectTarget`, a `Reveal`, an effect-initiated `Play`. The engine's
    ///   event log (`GameEvent`) carries NOTHING else an effect does, so an
    ///   unstamped `false` (or `true`) whose effect only did one of these is a
    ///   note, not a halt: a draw (no event at all), a trash / delete / mill
    ///   (`Trash` / `Mill` carry no source card and rules produce them too, so
    ///   they cannot be attributed to an effect), a hand / deck / security
    ///   move, a DP change, a keyword grant, a suspend / unsuspend, a source
    ///   added or removed, a cost reducer or granted effect armed, a memory
    ///   change without a source card. Widening the detector would need the
    ///   engine to attribute those moves to an effect (a new event with the
    ///   source) or a state snapshot per decision, which cannot separate an
    ///   effect's changes from the decision's own; neither is done by guessing
    ///   from what is there. `unstamped_false_after_a_draw_only_effect_is_still_only_a_note_a_measured_gap`
    ///   pins the draw case.
    fn classify_unstamped_answer(
        game: &Game,
        actor: PlayerId,
        value: bool,
        next: Option<&crate::dcgo_recording::SelectionRow>,
        row: u32,
        window: &[crate::events::GameEvent],
    ) -> UnstampedAnswer {
        if let Some(answer) = Self::dcgo_use_optional_answer(game, actor, value) {
            return UnstampedAnswer::Apply(answer);
        }
        let parked = game
            .pending_selection
            .as_ref()
            .map(|p| format!("{:?} P{} '{}'", p.kind, p.selecting_player, p.prompt))
            .unwrap_or_else(|| "no prompt".into());
        let actors_optional_prompt = game
            .pending_selection
            .as_ref()
            .is_some_and(|p| p.selecting_player == actor && p.is_optional);
        if !value && actors_optional_prompt {
            return UnstampedAnswer::Halt(format!(
                "row {row}: unstamped generic_bool false (P{actor} declined an optional effect in DCGO) meets \
                 P{actor}'s optional {parked}, which is not an optional-activation gate the replay recognises: \
                 the decline cannot be mapped onto it, and consuming the row would lose it"
            ));
        }
        let next_is_actors_selection = next.is_some_and(|n| {
            n.actor == actor && !(n.unstamped && n.prompt == "generic_bool")
        });
        if value && game.pending_selection.is_none() && next_is_actors_selection {
            return UnstampedAnswer::Halt(format!(
                "row {row}: unstamped generic_bool true (P{actor} used an optional effect in DCGO) with no prompt \
                 parked, and P{actor}'s next row is a {} selection inside that effect: the engine never offered \
                 the effect DCGO used",
                next.map(|n| n.prompt.as_str()).unwrap_or_default()
            ));
        }
        if game.pending_selection.is_none() {
            let activity = effect_activity(window, actor);
            if value && activity.is_empty() {
                return UnstampedAnswer::Halt(format!(
                    "row {row}: unstamped generic_bool true (P{actor} used an optional effect in DCGO) with no \
                     prompt parked, and the engine resolved no effect since the last recorded decision: it never \
                     offered the effect DCGO used"
                ));
            }
            if !value && !activity.is_empty() {
                return UnstampedAnswer::CannotTell(format!(
                    "row {row}: unstamped generic_bool false (P{actor} declined an optional effect in DCGO) with no \
                     prompt parked, while the engine resolved effects with no recorded answer since the last \
                     recorded decision ({}): the decline names no card, so the replay cannot tell whether the \
                     engine ran the declined effect as mandatory",
                    activity.join("; ")
                ));
            }
        }
        UnstampedAnswer::Note(format!(
            "row {row}: unstamped generic_bool {value} (P{actor}'s DCGO \"use this optional effect?\" answer) \
             consumed with no effect: no optional-activation gate of P{actor} was parked ({parked})"
        ))
    }

    /// The playback stamp of `payload` when the prompt its actor has parked
    /// does not list it among its answers: it names nothing in the prompt the
    /// engine asks now. The cause is NOT observed - a card whose script changed
    /// since the stamp was realised and an engine game that diverged from the
    /// one it was realised in look the same - so the note says only that.
    /// A stamp the prompt lists but the engine's own checks then refuse is not
    /// stale: it is the engine's answer to the row and is judged as such.
    /// `None` for an unstamped row, a listed stamp, or no prompt of the actor
    /// parked.
    fn stale_stamp(game: &Game, payload: &crate::dcgo_recording::SelectionRow) -> Option<u16> {
        let a = payload.engine_action?;
        let p = game.pending_selection.as_ref().filter(|p| p.selecting_player == payload.actor)?;
        (!p.valid_action_ids.contains(&a)).then_some(a)
    }

    /// The note a card fold leaves when it read a row past a stale stamp.
    fn note_stale_stamp(&mut self, stale: Option<u16>, payload: &crate::dcgo_recording::SelectionRow, card: &str) {
        if let Some(a) = stale {
            self.notes.push(format!(
                "row {}: P{}'s {} carries engine action {a}, which the parked prompt does not accept: stale or \
                 diverged (the replay cannot tell whether {card}'s script changed since the stamp was realised or \
                 the engine's game left the one it was realised in); {card}'s fold read DCGO's own row instead",
                self.cursor, payload.actor, payload.prompt
            ));
        }
    }

    /// The engine events from `start` (where the last decision began) on.
    /// `Err` when `start` lies past the end of the log: that window is not
    /// known, and reading the whole log instead would judge an answer against
    /// every effect of the game.
    fn unstamped_window(events: &[crate::events::GameEvent], start: usize) -> Result<&[crate::events::GameEvent], String> {
        events.get(start..).ok_or_else(|| {
            format!(
                "the event window an unstamped answer is judged against starts at event {start}, past the {} events \
                 the engine logged: the window since the last decision is not known",
                events.len()
            )
        })
    }

    /// DCGO's extra "Select 1 effect to activate." row after a REFIRE
    /// (BT24-102 Homeros, "activate 1 [On Play] or [When Digivolving] effect of
    /// 1 of your [Olympos XII] trait Digimon"), consumed ONLY when the engine's
    /// own state proves it is DCGO's duplicate-candidate prompt.
    ///
    /// DCGO (BT24_102.cs:173-215) lists the chosen Digimon's candidate effects
    /// (`EffectList(OnEnterFieldAnyone).Filter(CanBeEffectCandidate)`) and, from
    /// two candidates up, opens a mandatory `SelectCardEffect` over their source
    /// cards. A printed `[On Play] [When Digivolving] X` effect is built as one
    /// effect object per timing tag (`ActivateClassesForSharedEffects`,
    /// CardEffectFactory.cs:812-880; BT24_041.cs:142-166), so a Digimon with that
    /// single effect already makes two candidates, both naming the same card, and
    /// every choice runs the same shared coroutine. The engine has ONE clause for
    /// it (CR 4.3 15-15-7-2: one printed effect, no choice between its timings;
    /// `collapse_combined_timing_clauses`) and parks nothing. The recorder
    /// logs only the picked card (plus its position in DCGO's list), so the row
    /// then meets whatever prompt the refired effect parks (BT24-041: "play 1 cost
    /// 5 or lower Iliad card") and shifts every later row.
    ///
    /// `None` when the row is not a candidate for this fold at all (no collapsed
    /// refire on record, or a different card named): the normal resolver reads it.
    /// Otherwise `Ok(note)` when ALL of these hold, `Err(why)` when any fails:
    /// * the engine recorded a refire (`Game::last_collapsed_refire`) that ran
    ///   ONE collapsed combined-timing clause standing for `dcgo_entries >= 2`
    ///   DCGO entries, and that refire happened while the row BEFORE this one
    ///   was applied (`first_seen_row`),
    /// * the row's actor controls that refire and the row names the carrier's
    ///   own card (the engine's candidate), not another card,
    /// * the carrier is still on the field and the row's DCGO CardIndex, when
    ///   the replay knows it, is the carrier's own card (not another copy),
    /// * the row's position in DCGO's list, when the converter recorded it
    ///   (`indexes`), is one of the `dcgo_entries` entries, and
    /// * the engine has no effect-pick prompt of its own over that card parked
    ///   (a real two-effect choice is answered by `resolve_next`, never here).
    ///
    /// The choice cannot change the game: every entry is the same effect.
    fn refire_duplicate_effect_pick_fold(
        &self,
        game: &Game,
        payload: &crate::dcgo_recording::SelectionRow,
    ) -> Option<Result<String, String>> {
        let rec = game.last_collapsed_refire.as_ref()?;
        let [card] = payload.card_ids.as_deref()? else {
            return None;
        };
        if *card != rec.carrier_card_id || payload.cancel == Some(true) {
            return None;
        }
        let row = self.cursor;
        let grantor = game
            .card_data_for_handle(rec.grantor)
            .map(|d| d.card_id.clone())
            .unwrap_or_else(|| "?".to_string());
        let refuse = |why: String| -> Option<Result<String, String>> {
            Some(Err(format!(
                "row {row}: P{}'s SelectCardEffect {card:?} looks like DCGO's \"Select 1 effect to activate.\" after \
                 {grantor}'s refire of {} (the engine ran one effect for it), but {why}",
                payload.actor, rec.carrier_card_id
            )))
        };
        if rec.first_seen_row != Some(row) {
            return refuse(format!(
                "that refire did not happen while row {} was applied (first seen at row {:?}): the row cannot be \
                 the prompt that followed it",
                row.wrapping_sub(1),
                rec.first_seen_row
            ));
        }
        if payload.actor != rec.controller {
            return refuse(format!("the refire belongs to P{}", rec.controller));
        }
        let carrier_top = game
            .players
            .get(rec.carrier.player as usize)
            .and_then(|p| p.battle_area.get(rec.carrier.index as usize))
            .map(|perm| perm.top_card().handle());
        if carrier_top != Some(rec.carrier_card) {
            return refuse("the refired Digimon is no longer on the field".into());
        }
        if let Some(sel) = game.pending_selection.as_ref() {
            let own_choice = matches!(sel.kind, crate::selection::SelectionKind::EffectChoice)
                && sel.effect_choices.iter().flatten().any(|e| e.source_card == Some(rec.carrier_card));
            if own_choice {
                return refuse(format!(
                    "the engine has its own effect choice '{}' parked for that card",
                    sel.prompt
                ));
            }
        }
        if let Some([k]) = payload.card_indexes.as_deref() {
            if let Some(h) = self.dcgo_cards.get(k) {
                if *h != rec.carrier_card {
                    return refuse(format!(
                        "DCGO card {k} is another copy of {card} than the one that was refired"
                    ));
                }
            }
        }
        let position = match payload.indexes.as_deref() {
            None => None,
            Some([p]) if *p >= 0 && (*p as usize) < rec.dcgo_entries => Some(*p),
            Some(other) => {
                return refuse(format!(
                    "its position in DCGO's candidate list {other:?} is not one of the {} entries DCGO builds for \
                     that one effect",
                    rec.dcgo_entries
                ));
            }
        };
        Some(Ok(format!(
            "row {row}: SelectCardEffect {card:?} of P{} consumed with no effect: it is DCGO's \"Select 1 effect to \
             activate.\" over the {} effect objects DCGO builds for {}'s ONE printed [On Play]/[When Digivolving] \
             effect (one per timing tag, CardEffectFactory.ActivateClassesForSharedEffects), refired by {grantor}; \
             the engine ran that one effect and asks nothing, and every entry runs the same coroutine, so the \
             choice{} cannot change the game",
            payload.actor,
            rec.dcgo_entries,
            rec.carrier_card_id,
            position.map(|p| format!(" (position {p})")).unwrap_or_default()
        )))
    }

    /// Whether the engine resolved the effect a forced DCGO count
    /// (`is_forced_count_row`) counts, since the decision in front of that
    /// count began (`count_evidence`) - the same evidence whether the engine
    /// parked a prompt afterwards or not. `Ok` (what it resolved) when, in
    /// that window:
    /// * an effect of the count's actor chose a target
    ///   (`GameEvent::EffectTarget`; DCGO's count belongs to `EffectSourceCard.
    ///   Owner`, CardController.cs:5287), and
    /// * exactly `count` cards left the battle area for their owner's trash, all
    ///   of them the TOP `count` cards of ONE permanent (a <De-Digivolve N>
    ///   trashes from the top of the target, CardController.cs:5307-5369; CR 4.3
    ///   16-12), and
    /// * that permanent is still on the field holding exactly the cards below
    ///   them (a deletion of the whole Digimon trashes its cards too, but it
    ///   leaves nothing there), and
    /// * its top card (as the decision began) is named by that actor's
    ///   `EffectTarget` - the event carries the card id, not the permanent, so
    ///   two Digimon of one id cannot be told apart by it (a measured limit).
    /// `Err` (why not) otherwise: no decision precedes the count, no target was
    /// chosen, or the cards that moved are not that.
    fn forced_count_evidence(
        &self,
        game: &Game,
        payload: &crate::dcgo_recording::SelectionRow,
    ) -> Result<String, String> {
        use crate::events::GameEvent;
        let actor = payload.actor;
        let n = payload.count.unwrap_or_default().max(0) as usize;
        let Some(base) = self.count_evidence.as_ref().filter(|b| b.row + 1 == self.cursor) else {
            return Err("no recorded decision in front of it shows the engine resolving anything".into());
        };
        let window = Self::unstamped_window(&game.events, base.events)?;
        let mut chosen: Vec<String> = Vec::new();
        let mut targets: Vec<String> = Vec::new();
        for e in window {
            if let GameEvent::EffectTarget { player, source_card_id, targets: picked, .. } = e {
                if *player == actor {
                    chosen.extend(picked.iter().map(|t| t.card_id.clone()));
                    targets.push(format!(
                        "{source_card_id} chose {}",
                        picked.iter().map(|t| t.card_id.as_str()).collect::<Vec<_>>().join(", ")
                    ));
                }
            }
        }
        if targets.is_empty() {
            return Err(format!("no effect of P{actor} chose a target since row {} began", base.row));
        }
        let now = card_zones(game);
        let mut trashed: Vec<(PlayerId, crate::card_source::CardHandle)> = base
            .zones
            .iter()
            .filter(|(_, (_, z))| matches!(z, CardZone::Battle { .. }))
            .filter(|(h, (p, _))| now.get(*h) == Some(&(*p, CardZone::Trash)))
            .map(|(h, (p, _))| (*p, *h))
            .collect();
        trashed.sort_by_key(|(p, h)| (*p, h.0));
        let owners: HashSet<PlayerId> = trashed.iter().map(|(p, _)| *p).collect();
        let name = |h: &crate::card_source::CardHandle| game.card_data_for_handle(*h).map(|d| d.card_id.clone()).unwrap_or_default();
        let names = trashed.iter().map(|(p, h)| format!("P{p}'s {}", name(h))).collect::<Vec<_>>().join(", ");
        if trashed.len() != n || owners.len() > 1 {
            return Err(format!(
                "since row {} began the engine moved {} card(s) from the battle area to the trash ([{names}]), not \
                 the {n} of one Digimon the count names",
                base.row,
                trashed.len()
            ));
        }
        let owner = trashed[0].0;
        let handles: HashSet<crate::card_source::CardHandle> = trashed.iter().map(|(_, h)| *h).collect();
        let Some(stack) = base.stacks.iter().find(|s| s.player == owner && handles.iter().all(|h| s.cards.contains(h))) else {
            return Err(format!(
                "the {n} card(s) that went to the trash ([{names}]) did not all come from ONE permanent: a \
                 <De-Digivolve {n}> trashes the top {n} cards of its target"
            ));
        };
        let keep = stack.cards.len().saturating_sub(n);
        if keep == 0 || stack.cards[keep..].iter().copied().collect::<HashSet<_>>() != handles {
            return Err(format!(
                "the {n} card(s) that went to the trash ([{names}]) are not the top {n} of a permanent that kept \
                 a card below them: a <De-Digivolve {n}> trashes from the top and leaves the Digimon on the field"
            ));
        }
        let remains = |perm: &crate::permanent::Permanent| {
            perm.card_sources.iter().map(|c| c.handle()).eq(stack.cards[..keep].iter().copied())
        };
        if !game.player(owner).battle_area.iter().any(remains) {
            return Err(format!(
                "no permanent of P{owner} holds the cards that were below the {n} trashed ([{names}]): the \
                 Digimon left the field (deleted or returned), which is not a <De-Digivolve {n}>"
            ));
        }
        if !chosen.contains(&stack.top_card_id) {
            return Err(format!(
                "the permanent that lost its top {n} card(s) was topped by {}, which no target of P{actor} named \
                 ({})",
                stack.top_card_id,
                targets.join("; ")
            ));
        }
        Ok(format!(
            "since row {}: {}; {n} card(s) went from that Digimon to the trash [{names}] and it stayed on the field",
            base.row,
            targets.join("; ")
        ))
    }

    /// AMATERASUMON (EX12-047)'s "return 2 cards from your opponent's trash to
    /// the bottom of their deck", answered from DCGO's two rows. `None` when the
    /// row is not that pick (a 2-card `SelectCardEffect` of the actor whose
    /// parked prompt EX12-047 installed); otherwise the game after the picks
    /// and the last engine action, consuming this row AND the next, or why the
    /// rows cannot be the answer (nothing applied).
    ///
    /// DCGO: the pick (EX12_047.cs:98-130, `maxCount: 2`, `canEndNotMax:
    /// false`) is THIS row; with 2 cards picked it calls
    /// `ReturnRevealedCardsToLibraryBottom` (EX12_047.cs:132-138), which asks
    /// the order of 2+ cards as a second `SelectCardEffect` over exactly those
    /// cards (RevealLibrary.cs:486-509, `canNoSelect: false`) and appends them
    /// to the deck in that order (RevealLibrary.cs:511-516 ->
    /// CardObjectController.cs:954-1020, `LibraryCards.Add`; index 0 is the
    /// top), so the order row's LAST card is the very bottom. CR 4.3 3-1-3-4:
    /// the owner of the card that moves them chooses that order.
    ///
    /// The engine asks no separate order: `return_trash_list_to_deck_bottom`
    /// places the picks in pick order, the FIRST pick at the very bottom
    /// (`EffectContext::return_trash_cards_to_deck_bottom`). So the picks are
    /// fed in the REVERSE of the order row, which leaves the engine's deck
    /// bottom exactly as DCGO's. The pick row's own order only says which
    /// cards were picked. Card-specific: other cards reach the same DCGO
    /// helper, but their engine steps were not checked.
    fn ex12_047_bottom_order_fold(
        &self,
        game: &Game,
        payload: &crate::dcgo_recording::SelectionRow,
    ) -> Option<Result<(Game, u16), String>> {
        const CARD: &str = "EX12-047";
        let actor = payload.actor;
        let picks = payload.card_ids.as_ref().filter(|c| c.len() == 2)?;
        let p = game.pending_selection.as_ref()?;
        if p.selecting_player != actor || game.card_data_for_handle(p.source_card).map(|d| d.card_id.as_str()) != Some(CARD) {
            return None;
        }
        let row = self.cursor;
        let next = self
            .source
            .steps()
            .get(row as usize + 1)
            .and_then(|s| s.selection.as_ref());
        let sorted = |v: &[String]| {
            let mut v = v.to_vec();
            v.sort();
            v
        };
        let order = match next {
            Some(n)
                if n.actor == actor
                    && n.prompt == "SelectCardEffect"
                    && n.card_ids.as_deref().is_some_and(|o| sorted(o) == sorted(picks)) =>
            {
                n.card_ids.clone().unwrap_or_default()
            }
            other => {
                return Some(Err(format!(
                    "row {row}: {CARD} returns 2 cards ({picks:?}) to the bottom of the deck; DCGO asks their order \
                     next, as a SelectCardEffect over those cards (RevealLibrary.cs:486-509), but row {} is {}: \
                     the order cannot be taken from the recording",
                    row + 1,
                    match other {
                        Some(n) => format!(
                            "P{}'s {} ({})",
                            n.actor,
                            n.prompt,
                            selection_row_content(n).unwrap_or_else(|| decline_label(n).into())
                        ),
                        None => "absent".into(),
                    }
                )))
            }
        };
        let mut fed = payload.clone();
        fed.card_ids = Some(order.into_iter().rev().collect());
        Some(resolve_row_on_clone(game, &fed).map_err(|reason| format!("row {row}: {CARD} bottom-order pick: {reason}")))
    }

    /// `(DCGO card index, card id)` of every card the recorded step at `row`
    /// names by DCGO identity: a selection's picks (`card_indexes` with
    /// `card_ids`), an action's played card (`dcgo_action_card_index` with
    /// `card_id`).
    fn dcgo_named_cards(&self, row: u32) -> Vec<(i32, String)> {
        let Some(spec) = self.source.steps().get(row as usize) else {
            return Vec::new();
        };
        match &spec.selection {
            Some(sel) => match (&sel.card_indexes, &sel.card_ids) {
                (Some(ks), Some(ids)) if ks.len() == ids.len() => ks.iter().copied().zip(ids.iter().cloned()).collect(),
                _ => Vec::new(),
            },
            None => match (self.source.dcgo_action_card_index(row), &spec.card_id) {
                (Some(k), Some(id)) => vec![(k, id.clone())],
                _ => Vec::new(),
            },
        }
    }

    fn row_names_dcgo_cards(&self, row: u32) -> bool {
        !self.dcgo_named_cards(row).is_empty()
    }

    /// Pair the DCGO card indexes the rows in `rows` name with the engine cards
    /// those rows moved (`before` = `card_zones` ahead of them). A DCGO
    /// `CardIndex` and an engine `CardHandle` are each one physical card for
    /// the whole game, so a pairing holds until the engines are seen to move
    /// different copies.
    ///
    /// Per card id: when the rows name ONE index and the engine moved exactly
    /// ONE card of that id, the two are the same card. Otherwise nothing is
    /// paired (the moves do not say which is which), and an index paired
    /// earlier with a card that did not move is dropped (DCGO moved it, the
    /// engine did not move that card).
    ///
    /// A copy in the hand is interchangeable with another copy in the hand
    /// when nothing but its place in the hand tells them apart (CR 4.3 3-5-2: a
    /// player may reorder the hand at any time, so its ORDER carries nothing;
    /// the engine and DCGO keep different hand orders). So when the engine took
    /// a different hand copy than the one an index was paired with, the
    /// pairings of the two copies are swapped, never guessed - unless engine
    /// state names the copy that stayed (`card_named_by_engine_state`: a
    /// modifier, a pending declaration, a granted effect), in which case the
    /// copies are not the same and the other index's pairing is dropped:
    /// raw-20261113T025510Z#g6 raw seq 980 / 991 (Marcus Damon & Thomas H.
    /// Norstein (ST24-13) 87 searched into a hand holding another copy, then
    /// played from it). Only the copy that STAYED is checked: the one the
    /// engine moved is named by the move's own effects afterwards.
    fn record_dcgo_card_indexes(
        &mut self,
        before: &HashMap<crate::card_source::CardHandle, (PlayerId, CardZone)>,
        game: &Game,
        rows: std::ops::Range<u32>,
    ) {
        let mut named: std::collections::BTreeMap<String, Vec<i32>> = std::collections::BTreeMap::new();
        for row in rows {
            for (k, id) in self.dcgo_named_cards(row) {
                named.entry(id).or_default().push(k);
            }
        }
        if named.is_empty() {
            return;
        }
        let after = card_zones(game);
        let id_of = |h: &crate::card_source::CardHandle| game.card_data_for_handle(*h).map(|d| d.card_id.clone());
        for (id, ks) in named {
            let moved: Vec<crate::card_source::CardHandle> = before
                .iter()
                .filter(|(h, z)| after.get(h) != Some(z) && id_of(h).as_deref() == Some(id.as_str()))
                .map(|(h, _)| *h)
                .collect();
            if let ([k], [h]) = (ks.as_slice(), moved.as_slice()) {
                self.pair_dcgo_card(*k, *h, before, game);
                continue;
            }
            for k in ks {
                if self.dcgo_cards.get(&k).is_some_and(|h| !moved.contains(h)) {
                    self.dcgo_cards.remove(&k);
                }
            }
        }
    }

    /// Pair DCGO index `k` with engine card `h` (see `record_dcgo_card_indexes`).
    fn pair_dcgo_card(
        &mut self,
        k: i32,
        h: crate::card_source::CardHandle,
        before: &HashMap<crate::card_source::CardHandle, (PlayerId, CardZone)>,
        game: &Game,
    ) {
        let previous = self.dcgo_cards.insert(k, h);
        if previous == Some(h) {
            return;
        }
        let Some(other) = self.dcgo_cards.iter().find(|(o, c)| **o != k && **c == h).map(|(o, _)| *o) else {
            return;
        };
        let in_hand = |c: &crate::card_source::CardHandle| before.get(c).is_some_and(|(_, z)| *z == CardZone::Hand);
        match previous {
            // Both copies were in the hand: the engine took the other one.
            Some(p) if in_hand(&p) && in_hand(&h) && !card_named_by_engine_state(game, p) => {
                self.dcgo_cards.insert(other, p);
            }
            _ => {
                self.dcgo_cards.remove(&other);
            }
        }
    }

    /// The engine battle slot of `player` whose permanent's top card is the
    /// engine card paired with DCGO index `k`, among `copies`; `Err` names why
    /// none (or several) is.
    fn dcgo_identity_slot(&self, game: &Game, player: PlayerId, k: i32, copies: &[usize]) -> Option<Result<usize, String>> {
        let h = *self.dcgo_cards.get(&k)?;
        let area = &game.player(player).battle_area;
        let named: Vec<usize> = copies.iter().copied().filter(|j| area[*j].top_card().handle() == h).collect();
        Some(match named.as_slice() {
            [j] => Ok(*j),
            _ => Err(format!(
                "DCGO card index {k} is the engine's card {} (paired by an earlier row), which tops none of P{player}'s \
                 copies in slots {copies:?}",
                h.0
            )),
        })
    }

    /// The own-board operand of the ACTION row at `cur` (the attacker, the
    /// digivolve target, the host of an activated permanent effect) when the
    /// recorded board holds several copies of its card: `remapped` (the action
    /// after `remap_board_slots`' occurrence-order pairing) with that operand
    /// replaced by the engine slot of the copy DCGO's own card identity names
    /// (`ActionRow::operand_card_index`, paired with an engine card by the rows
    /// that moved it, `record_dcgo_card_indexes`), and whether it was.
    ///
    /// raw-20261115T153822Z#g10 raw seq 821: two ST24-05, DCGO's attacker was
    /// the newer copy (the one holding BT26-036), both copies could attack, and
    /// the occurrence pairing sent the older copy.
    ///
    /// With no index, or one that names no engine card (or one that tops none
    /// of the copies), the occurrence-order pairing stands and a note says it
    /// was a guess when the engine's copies are not interchangeable and the same
    /// action is legal for more than one of them. It never halts: a recording
    /// predating the field cannot do better, and the engine's legality still
    /// decides whether the paired copy's action is accepted.
    fn named_operand_action(
        &mut self,
        game: &Game,
        actor: PlayerId,
        cur: u32,
        raw_action_id: u16,
        remapped: u16,
    ) -> (u16, bool) {
        let (Some(recorded_slot), Some(_)) = (own_board_operand(raw_action_id), own_board_operand(remapped)) else {
            return (remapped, false);
        };
        let Some(step) = self.source.steps().get(cur as usize) else {
            return (remapped, false);
        };
        let recorded = if actor == 0 { step.board_p0.clone() } else { step.board_p1.clone() };
        let Some(recorded) = recorded else {
            return (remapped, false);
        };
        let Some(want) = recorded.get(recorded_slot as usize).cloned() else {
            return (remapped, false);
        };
        let recorded_copies = recorded.iter().filter(|id| **id == want).count();
        if recorded_copies < 2 {
            return (remapped, false);
        }
        let copies: Vec<usize> = engine_board_ids(game, actor)
            .iter()
            .enumerate()
            .filter(|(_, id)| **id == want)
            .map(|(j, _)| j)
            .collect();
        if copies.len() < 2 {
            return (remapped, false);
        }
        if let Some((k, source)) = self.source.dcgo_action_operand_index(cur) {
            match self.dcgo_identity_slot(game, actor, k, &copies) {
                Some(Ok(j)) => {
                    use crate::dcgo_recording::CardIndexSource;
                    let provenance = match source {
                        Some(CardIndexSource::Recorder) => None,
                        Some(CardIndexSource::FrameModel) => Some("inferred by frame model, not recorded"),
                        Some(CardIndexSource::HandListing) => Some("inferred by hand listing, not recorded"),
                        None => Some("taken from a card index of unknown source (provenance not recorded)"),
                    };
                    if let Some(provenance) = provenance {
                        self.notes.push(format!(
                            "row {cur}: P{actor}'s action names {want} by DCGO card index {k}; copy identity \
                             {provenance} (took slot {j})"
                        ));
                    }
                    return (with_own_board_operand(remapped, j as u16), true);
                }
                Some(Err(why)) => self.notes.push(format!(
                    "row {cur}: P{actor}'s action names {want} by DCGO card index {k}, but {why}; the \
                     occurrence-order pairing stands"
                )),
                None => self.notes.push(format!(
                    "row {cur}: P{actor}'s action names {want} by DCGO card index {k}, which no earlier row \
                     paired with an engine card; the occurrence-order pairing stands"
                )),
            }
        }
        let mask = crate::action::build_action_mask(game, actor);
        let legal = |id: u16| mask.get(id as usize).copied().unwrap_or(0.0) > 0.5;
        let slots: Vec<u16> = copies
            .iter()
            .map(|j| *j as u16)
            .filter(|j| legal(with_own_board_operand(remapped, *j)))
            .collect();
        if slots.len() >= 2 && !interchangeable_copies(game, actor, &slots) {
            self.notes.push(format!(
                "row {cur}: P{actor}'s action names one of {recorded_copies} copies of {want} by position only \
                 (DCGO's frame order, not the engine's), and the engine's copies in slots {slots:?} differ and can \
                 all take it: paired in occurrence order (slot {}), which is a guess - the recording names no card \
                 index for the operand",
                own_board_operand(remapped).unwrap_or(0)
            ));
        }
        (remapped, false)
    }

    /// A one-target pick row whose target is one of several copies of a card
    /// on the recorded board: which copy, decided by the rows that follow.
    /// `None` when the row is not that shape (the ordinary remap then runs).
    ///
    /// The recorded boards name permanents by card id only, in DCGO's frame
    /// order (`GetFieldPermanents()`, Player.cs:41-54 / :669-684), and the
    /// engine's battle area is in play order, so two copies cannot be paired by
    /// id; `build_slot_map`'s occurrence order is a guess. raw-20261113T011922Z#g3
    /// raw seq 409-411: Owen Dreadnought (BT21-081)'s [End of Your Turn] pick
    /// named DCGO's first of two Medusamon (EX11-012) - the newer, unsuspended
    /// one, which then attacked (BT21_081.cs:98-145, `CanAttack`) - and the
    /// occurrence order paired it with the engine's older, suspended copy.
    ///
    /// Each engine copy is tried; a copy the row cannot answer is dropped; the
    /// survivors are told apart the way trigger-order entries are
    /// (`lookahead_trigger_order`: the rows that follow refute a copy, or the
    /// copies lead to the same game). Anything else is `Err`: the replay
    /// stops instead of pairing copies by position.
    fn duplicate_copy_target(
        &mut self,
        game: &Game,
        payload: &crate::dcgo_recording::SelectionRow,
    ) -> Option<Result<(Game, u16), String>> {
        let [t] = payload.targets.as_deref()? else {
            return None;
        };
        if t.frame < 0 {
            return None;
        }
        let recorded = if t.player == 0 { payload.board_p0.as_ref() } else { payload.board_p1.as_ref() }?;
        let want = recorded.get(t.frame as usize)?;
        if recorded.iter().filter(|id| *id == want).count() < 2 {
            return None;
        }
        let copies: Vec<usize> = engine_board_ids(game, t.player)
            .iter()
            .enumerate()
            .filter(|(_, id)| *id == want)
            .map(|(j, _)| j)
            .collect();
        if copies.len() < 2 {
            return None;
        }
        // The row meets the engine's prompt as the ordinary path would: a
        // clause-level "you may" accept DCGO folds into this pick is accepted
        // first (`accept_outer_gate_for_pick`; raw-20261113T025510Z#g6 raw seq
        // 1003, Falcomon (ST24-12)'s [On Play], ST24_12.cs `canNoSelect`), so
        // each copy - or the copy a DCGO card index names - is tried on the
        // body's own pick, not refused at the gate.
        let mut gated = game.clone();
        accept_outer_gate_for_pick(&mut gated, payload);
        let game = &gated;
        // The target names its copy by DCGO card identity, paired with an
        // engine card by the rows that moved it: that copy, and only it.
        if let Some(k) = t.card_index {
            if let Some(slot) = self.dcgo_identity_slot(game, t.player, k, &copies) {
                if let Ok(j) = &slot {
                    // Only a recorder index is DCGO's own value; the converter's frame model and hand-listing
                    // indexes are inferences (docs/DCGO_RECORDING_SCHEMA.md, `card_index_source`): say so.
                    use crate::dcgo_recording::CardIndexSource;
                    let provenance = match t.card_index_source {
                        Some(CardIndexSource::Recorder) => None,
                        Some(CardIndexSource::FrameModel) => Some("inferred by frame model, not recorded"),
                        Some(CardIndexSource::HandListing) => Some("inferred by hand listing, not recorded"),
                        None => Some("taken from a card index of unknown source (provenance not recorded: a recording predating card_index_source)"),
                    };
                    if let Some(provenance) = provenance {
                        self.notes.push(format!(
                            "row {}: P{}'s {} names {want} by DCGO card index {k}; copy identity {provenance} \
                             (took slot {j})",
                            self.cursor, payload.actor, payload.prompt
                        ));
                    }
                }
                let named = slot.and_then(|j| {
                    let mut row = payload.clone();
                    row.board_p0 = None;
                    row.board_p1 = None;
                    let mut target = t.clone();
                    target.frame = j as i32;
                    row.targets = Some(vec![target]);
                    resolve_row_on_clone(game, &row).map_err(|reason| format!("its copy in slot {j}: {reason}"))
                });
                return Some(named.map_err(|why| {
                    format!(
                        "row {}: P{}'s {} names {want} (DCGO card index {k}) at DCGO board position {} of {:?}: {why}",
                        self.cursor, payload.actor, payload.prompt, t.frame, recorded
                    )
                }));
            }
        }
        let mut answered: Vec<(u16, Game, u16)> = Vec::new();
        for &j in &copies {
            let mut row = payload.clone();
            row.board_p0 = None;
            row.board_p1 = None;
            let mut target = t.clone();
            target.frame = j as i32;
            row.targets = Some(vec![target]);
            if let Ok((g, last)) = resolve_row_on_clone(game, &row) {
                answered.push((j as u16, g, last));
            }
        }
        match answered.len() {
            0 => return None,
            1 => {
                let (_, g, last) = answered.pop()?;
                return Some(Ok((g, last)));
            }
            _ => {}
        }
        let first = branch_digest(&answered[0].1);
        if answered.iter().all(|(_, g, _)| branch_digest(g) == first) {
            let (_, g, last) = answered.swap_remove(0);
            return Some(Ok((g, last)));
        }
        let slots: Vec<u16> = answered.iter().map(|(j, _, _)| *j).collect();
        let branches: Vec<(u16, Game)> = answered.iter().map(|(j, g, _)| (*j, g.clone())).collect();
        let verdict = self.lookahead_trigger_order(branches, self.cursor + 1, &[], false);
        if let LookaheadVerdict::Proved(j) = verdict {
            let (_, g, last) = answered.into_iter().find(|(k, _, _)| *k == j)?;
            return Some(Ok((g, last)));
        }
        // Copies the engine cannot tell apart either: the same stack, state
        // and turn played, and nothing (a modifier, a queued effect) naming
        // either slot. Whichever DCGO picked, the game is the same up to
        // which physical copy is which, so the first is taken.
        if verdict == LookaheadVerdict::Undecided && interchangeable_copies(game, t.player, &slots) {
            self.notes.push(format!(
                "row {}: P{}'s {} names {want}, of which the engine holds interchangeable copies in slots \
                 {slots:?} (same stack and state, no effect on either); took slot {}",
                self.cursor, payload.actor, payload.prompt, slots[0]
            ));
            let (_, g, last) = answered.swap_remove(0);
            return Some(Ok((g, last)));
        }
        // Worded with the lookahead markers, so a lookahead one level up reads
        // an undecided copy as an ambiguity (it ends that branch's evidence)
        // and an all-refuted one as a refutation.
        let why = if verdict == LookaheadVerdict::AllRefuted {
            format!("the next {LOOKAHEAD_ROWS} rows {REFUTED_ORDER}")
        } else {
            format!(
                "they lead to different games and the next {LOOKAHEAD_ROWS} rows do not tell them apart, so which \
                 copy {AMBIGUOUS_ORDER}"
            )
        };
        Some(Err(format!(
            "row {}: P{}'s {} names {want} at DCGO board position {} of {:?}; that board holds {} copies of {want} \
             and the engine's copies in slots {slots:?} all answer the row, but {why}. The recorded boards name \
             cards by id only, so which copy DCGO picked cannot be read from the recording (a recorder change - \
             each board entry's card_index - would make it exact)",
            self.cursor,
            payload.actor,
            payload.prompt,
            t.frame,
            recorded,
            recorded.iter().filter(|id| *id == want).count(),
        )))
    }

    /// A DCGO per-card "from which zone?" loop (`ZONE_MENU_LOOPS`) that the
    /// engine asks as ONE up-to-2 pick over the hand and the trash, answered
    /// from DCGO's rows. Written for SIRIUSMON (EX12-018)'s "place up to 2
    /// ... from your hand or trash"; J.P., Koji, & Koichi (AD1-023)'s "place
    /// up to 2 [Hybrid] trait cards with different colors from your hand or
    /// trash" has the same loop with one card per pass (AD1_023.cs:80-167,
    /// `maxCount: 1` at :120 / :148) and a silent end: with no card choosable
    /// in either zone it leaves the loop without asking (:84-88, `goto
    /// END_LOOP`), which the fold accepts only when the engine pick, with the
    /// picks so far applied, offers no card either
    /// (raw-20261113T011922Z#g3 raw seq 364-368). It also excludes a card
    /// already chosen (:72), so an id named again in the same zone is a
    /// second copy. The rest of this doc is EX12-018's.
    ///
    /// SIRIUSMON (EX12-018)'s "place up to 2 ... from your hand or trash",
    /// answered from DCGO's loop. `None` when the row is neither the actor's
    /// zone menu (`generic_int`) nor a 1- or 2-card hand / trash pick of the
    /// actor against that card's fresh up-to-2 engine pick; otherwise the game
    /// after the picks, the last engine action and the rows consumed, or why
    /// the rows cannot be the answer (nothing applied).
    ///
    /// DCGO (`CardEffect/EX12/Red/EX12_018.cs:84-164`): `while (cardSources.
    /// Count < 2)` it asks the int menu "Where will you select cards from ...?"
    /// (From Hand = 1, From Trash = 2, Don't place any = 3; :89-107) and, unless
    /// 3, a pick of up to `2 - chosen` cards (:109) in that zone
    /// (`SelectHandEffect` :118-137 / `SelectCardEffect` :139-162,
    /// `canNoSelect`, `canEndNotMax`). So a 1-card pick is ALWAYS followed by
    /// the menu again. The engine asks ONE up-to-2 pick over the hand AND the
    /// trash (EX12-018.yaml `select_zone_cards`, as printed: "from your hand or
    /// trash"), so the whole loop answers it:
    /// * each recorded card is picked in the zone its row names (a trash pick
    ///   never takes a hand copy of the same card, nor the reverse), in DCGO's
    ///   pick order - which is also the stacking order on both sides (DCGO
    ///   :183-196 with `Permanent.cs:1145` / `:1247`; the engine's
    ///   `place_cards_as_top_sources` / `place_cards_as_bottom_sources`);
    /// * menu 3 ends the loop; with fewer than 2 cards the engine pick is
    ///   closed with PASS (with none, that PASS is the decline);
    /// * a cancelled (empty) pick leaves the count as it was, and the loop
    ///   asks again;
    /// * the loop is read from its first menu row; a pick row met first (its
    ///   menu already consumed) names its own zone.
    fn zone_menu_loop_fold(
        &self,
        game: &Game,
        payload: &crate::dcgo_recording::SelectionRow,
    ) -> Option<Result<(Game, u16, u32), String>> {
        use crate::action::space::{PASS, PLAY_HAND_END, PLAY_HAND_START, TRASH_EFFECT_END, TRASH_EFFECT_START};
        use crate::selection::SelectionKind;
        let actor = payload.actor;
        let p = game.pending_selection.as_ref()?;
        if p.selecting_player != actor || !matches!(p.kind, SelectionKind::CountCappedMultiSelect { picked: 0, max: 2, .. }) {
            return None;
        }
        let source_id = game.card_data_for_handle(p.source_card).map(|d| d.card_id.as_str())?;
        let spec = ZONE_MENU_LOOPS.iter().find(|l| l.card == source_id)?;
        #[allow(non_snake_case)]
        let CARD = spec.card;
        let source_card = p.source_card;
        let is_menu = |n: &crate::dcgo_recording::SelectionRow| {
            n.actor == actor
                && n.prompt == "generic_int"
                && n.int_value.is_some()
                && n.card_ids.is_none()
                && n.targets.is_none()
                && n.cancel != Some(true)
        };
        let entered_at_pick = if is_menu(payload) {
            false
        } else if (payload.prompt == "SelectHandEffect" || payload.prompt == "SelectCardEffect")
            && payload.card_ids.as_ref().is_some_and(|c| (1..=2).contains(&c.len()))
        {
            true
        } else {
            return None;
        };
        let zone_name = |hand: bool| if hand { "hand" } else { "trash" };
        let row = self.cursor;
        let steps = self.source.steps();
        let at = |k: u32| steps.get((row + k) as usize).and_then(|s| s.selection.as_ref());
        let describe = |n: Option<&crate::dcgo_recording::SelectionRow>| match n {
            Some(n) => format!("P{}'s {} ({})", n.actor, n.prompt, selection_row_content(n).unwrap_or_else(|| decline_label(n).into())),
            None => "absent".to_string(),
        };
        // (from the hand?, card id), in DCGO's pick order.
        let mut picks: Vec<(bool, String)> = Vec::new();
        let mut k = 0u32;
        if entered_at_pick {
            let hand = payload.prompt == "SelectHandEffect";
            picks.extend(payload.card_ids.clone().unwrap_or_default().into_iter().map(|c| (hand, c)));
            k = 1;
        }
        // The game after `picks`, applied to the engine's one pick in DCGO's
        // order, with the last action; the pick is left open.
        let apply = |picks: &[(bool, String)]| -> Result<(Game, u16), String> {
            let mut g = game.clone();
            let mut last = 0u16;
            for (hand, id) in picks {
                let open = g
                    .pending_selection
                    .as_ref()
                    .filter(|q| {
                        q.selecting_player == actor
                            && q.source_card == source_card
                            && matches!(q.kind, SelectionKind::CountCappedMultiSelect { .. })
                    })
                    .map(|q| q.valid_action_ids.clone());
                let Some(valid) = open else {
                    return Err(format!(
                        "row {row}: {CARD} zone-loop pick: the engine's pick closed before '{id}' (picks {picks:?})"
                    ));
                };
                let player = &g.players[actor as usize];
                let (zone, start, end) = if *hand {
                    (&player.hand, PLAY_HAND_START, PLAY_HAND_END)
                } else {
                    (&player.trash, TRASH_EFFECT_START, TRASH_EFFECT_END)
                };
                let action = zone.iter().enumerate().find_map(|(i, c)| {
                    let a = start + i as u16;
                    (a < end && c.card_id(&g.card_data) == id.as_str() && valid.contains(&a)).then_some(a)
                });
                let Some(action) = action else {
                    return Err(format!(
                        "row {row}: {CARD} zone-loop pick: '{id}' is not a card the engine's pick offers in P{actor}'s {}",
                        zone_name(*hand)
                    ));
                };
                if let Some(reason) = selection_answer_rejection(&g, action, actor) {
                    return Err(format!("row {row}: {CARD} zone-loop pick: {reason}"));
                }
                g.decode_action(action, actor);
                last = action;
            }
            Ok((g, last))
        };
        while picks.len() < 2 {
            // DCGO leaves the loop WITHOUT a menu when no card is choosable in
            // either zone (AD1-023). The engine must agree: with the picks so
            // far applied, its pick offers no card.
            if spec.ends_without_candidates && !picks.is_empty() && !at(k).is_some_and(|n| is_menu(n)) {
                let (g, _) = match apply(&picks) {
                    Ok(x) => x,
                    Err(e) => return Some(Err(e)),
                };
                let offers_a_card = g.pending_selection.as_ref().is_some_and(|q| {
                    q.selecting_player == actor
                        && q.source_card == source_card
                        && matches!(q.kind, SelectionKind::CountCappedMultiSelect { .. })
                        && q.valid_action_ids.iter().any(|a| *a != PASS)
                });
                if !offers_a_card {
                    break;
                }
                return Some(Err(format!(
                    "row {row}: {CARD} chose {} card(s); the engine's pick still offers a card, and DCGO asks the \
                     zone menu again while one is choosable ({}), but row {} is {}",
                    picks.len(),
                    spec.menu_cite,
                    row + k,
                    describe(at(k))
                )));
            }
            if k > 32 {
                return Some(Err(format!(
                    "row {row}: {CARD}'s zone loop runs past row {} without choosing 2 cards or \"Don't place any\"",
                    row + k
                )));
            }
            let Some(menu) = at(k).filter(|n| is_menu(n)) else {
                return Some(Err(format!(
                    "row {row}: {CARD} chose {} card(s) so far; DCGO asks its zone menu again while fewer than 2 \
                     are chosen ({}), but row {} is {}",
                    picks.len(),
                    spec.menu_cite,
                    row + k,
                    describe(at(k))
                )));
            };
            match menu.int_value.unwrap_or_default() {
                3 => {
                    k += 1;
                    break;
                }
                v @ (1 | 2) => {
                    let hand = v == 1;
                    let want = if hand { "SelectHandEffect" } else { "SelectCardEffect" };
                    let Some(pick) = at(k + 1).filter(|n| n.actor == actor && n.prompt == want) else {
                        return Some(Err(format!(
                            "row {row}: {CARD}'s menu {v} at row {} is followed by {}, not P{actor}'s {want} ({})",
                            row + k,
                            describe(at(k + 1)),
                            spec.pick_cite
                        )));
                    };
                    let picked = pick.card_ids.clone().unwrap_or_default();
                    let cap = spec.per_pass.unwrap_or(2).min(2 - picks.len());
                    if picked.len() > cap {
                        return Some(Err(format!(
                            "row {row}: {CARD}'s {} pick at row {} names {picked:?}, more than the {cap} card(s) one \
                             pass may pick ({})",
                            zone_name(hand),
                            row + k + 1,
                            spec.pick_cite
                        )));
                    }
                    // DCGO's later pass re-offers the cards the earlier ones
                    // chose (EX12_018.cs:115-131: `canTargetCondition` only, no
                    // `canTargetCondition_ByPreSelecetedList`, and
                    // `SelectCardCoroutine` appends whatever was picked), so an
                    // id named again in the SAME zone may be the same physical
                    // card picked twice. Ids alone cannot tell that from a
                    // second copy, so the engine is not fed a second copy.
                    if let Some(again) = picked
                        .iter()
                        .find(|id| !spec.excludes_chosen && picks.iter().any(|(h, c)| *h == hand && c == *id))
                    {
                        return Some(Err(format!(
                            "row {row}: {CARD}'s menu loop names {again} in the {} again at row {} after an earlier \
                             pass chose it there: DCGO's loop does not exclude a chosen card (EX12_018.cs:115-131), \
                             so it may have picked the same card twice - which copy (or whether a second one) was \
                             placed cannot be read from the recording",
                            zone_name(hand),
                            row + k + 1
                        )));
                    }
                    // `canNoSelect`: an empty pick chooses nothing and the loop asks again.
                    picks.extend(picked.into_iter().map(|c| (hand, c)));
                    k += 2;
                }
                v => {
                    return Some(Err(format!(
                        "row {row}: {CARD}'s menu row {} carries {v}, which is not one of its values 1 / 2 / 3 ({})",
                        row + k,
                        spec.menu_cite
                    )))
                }
            }
        }
        let (mut g, mut last) = match apply(&picks) {
            Ok(x) => x,
            Err(e) => return Some(Err(e)),
        };
        // Fewer than 2 cards (or none: the decline) leave the engine pick open.
        if g.pending_selection.as_ref().is_some_and(|q| {
            q.selecting_player == actor
                && q.source_card == source_card
                && matches!(q.kind, SelectionKind::CountCappedMultiSelect { .. })
                && q.valid_action_ids.contains(&PASS)
        }) {
            g.decode_action(PASS, actor);
            last = PASS;
        }
        Some(Ok((g, last, k)))
    }

    /// RYUGUMON (EX12-036)'s "[On Play][When Digivolving][When Attacking] By
    /// placing 1 ... card from your hand as this Digimon's bottom digivolution
    /// card, 1 of your Digimon may unsuspend", when DCGO's unsuspend pick named
    /// a Digimon that is not suspended. `Some` (the game after the hand pick,
    /// its last action and a note) only when ALL hold; the caller then
    /// consumes this row and the next:
    /// * the row is the actor's 1-card hand pick against that clause's parked
    ///   hand pick (source EX12-036);
    /// * after it the engine asks the actor nothing more (its unsuspend pick
    ///   offers only suspended Digimon, EX12-036.yaml, and there is none);
    /// * the actor's next row is a `SelectPermanentEffect` naming ONE of the
    ///   actor's own Digimon, and that Digimon is not suspended.
    ///
    /// DCGO (`CardEffect/EX12/Blue/EX12_036.cs:136-160`) asks the unsuspend
    /// pick over every one of your Digimon (`IsYourDigimonCondition`,
    /// `Mode.UnTap`, `canNoSelect`), and unsuspending a Digimon that is not
    /// suspended changes nothing (`IUnsuspendPermanents.Unsuspend` keeps
    /// only `IsSuspended` permanents, CardController.cs:6146-6152). So the
    /// recorded pick is the same game as the engine's skipped pick. Anything
    /// else returns `None` and the rows meet the engine as usual (a pick the
    /// engine does not ask halts there).
    fn ex12_036_noop_unsuspend_fold(
        &self,
        game: &Game,
        payload: &crate::dcgo_recording::SelectionRow,
    ) -> Option<(Game, u16, String)> {
        const CARD: &str = "EX12-036";
        let actor = payload.actor;
        payload.card_ids.as_ref().filter(|c| c.len() == 1)?;
        let p = game.pending_selection.as_ref()?;
        if p.selecting_player != actor
            || !matches!(p.kind, crate::selection::SelectionKind::Hand)
            || game.card_data_for_handle(p.source_card).map(|d| d.card_id.as_str()) != Some(CARD)
        {
            return None;
        }
        let row = self.cursor;
        let next = self
            .source
            .steps()
            .get(row as usize + 1)
            .and_then(|s| s.selection.as_ref())
            .filter(|n| n.actor == actor && n.prompt == "SelectPermanentEffect" && n.card_ids.is_none())?;
        let (g, last) = resolve_row_on_clone(game, payload).ok()?;
        if g.pending_selection.as_ref().is_some_and(|q| q.selecting_player == actor) {
            return None;
        }
        let remapped = remap_selection_targets(&g, next).unwrap_or_else(|| next.clone());
        let [t] = remapped.targets.as_deref()? else {
            return None;
        };
        if t.player != actor || t.frame < 0 {
            return None;
        }
        let perm = g.player(actor).battle_area.get(t.frame as usize)?;
        if perm.is_suspended || perm.top_card().card_kind(&g.card_data) != crate::enums::CardKind::Digimon {
            return None;
        }
        let name = perm.top_card().card_id(&g.card_data).to_string();
        Some((
            g,
            last,
            format!(
                "row {}: P{actor}'s SelectPermanentEffect unsuspended {name} (slot {}), which is not suspended: \
                 {CARD}'s DCGO unsuspend pick offers every own Digimon and unsuspending an unsuspended one does \
                 nothing (EX12_036.cs:143-160, CardController.cs:6146-6152); the engine offers only suspended \
                 Digimon and asked nothing, the same game",
                row + 1,
                t.frame
            ),
        ))
    }

    /// DCGO's "From hand" / "From trash" (or "From digivolution cards") BOOL
    /// in front of the actor's pick, against the engine's ONE union pick over
    /// those zones. `None` when the row is not that shape: not a stamped
    /// `generic_bool` with nothing else (an unstamped or playback row never is),
    /// or the parked prompt is not the actor's `UnionZone` over the hand and
    /// the trash / digivolution cards, or the card that installed it asks no
    /// such bool in DCGO (`selection_resolve::dcgo_bool_zone_menu`). Otherwise:
    ///
    /// * `Ok(())` - the value is the card's "From hand" (or, against a union
    ///   with the trash, its "From trash") and the actor's next row is a pick
    ///   (cards, or a cancel) in that zone (`selection_resolve::row_pick_zone`).
    ///   The caller consumes the bool; the pick resolves inside that zone.
    /// * `Err` - anything else: the value names a zone the union has no
    ///   reading for (Asuna Shiroki (BT25-092)'s "From digivolution cards" =
    ///   false, BT25_092.cs:127-131, is followed by DCGO's two-step carrier and
    ///   card pick, `SelectTrashDigivolutionCards`, TrashDigivolutionCards.cs:
    ///   11-140, whose card row is rooted `Custom`, so its zone cannot be held
    ///   to the source the engine would resolve - not folded), or the next row
    ///   is no pick in the named zone.
    ///
    /// Grounding: Asuna's "From hand" = true / "From trash" = false
    /// (BT25_092.cs:220-227) and "From hand" = true / "From digivolution cards"
    /// = false (BT25_092.cs:127-131); the hand pick is a `SelectHandEffect`
    /// and the trash pick a `SelectCardEffect` rooted in the trash
    /// (CardEffectCommons.cs:1049-1083). The meaning of true / false is the
    /// card's: Lucemon (BT17-025) offers "From trash" = true.
    /// After a "From which area?" menu row (the cursor's) of `actor` was read
    /// as naming the zone of the next row: when that next row is a zone-less
    /// `SelectCardEffect`, record that it is the trash pick
    /// (`selection_resolve::menu_pick_zone`), so it resolves in the trash.
    fn set_pick_zone_hint(&mut self, actor: PlayerId) {
        let at = self.cursor + 1;
        let zoneless_card_pick = self
            .source
            .steps()
            .get(at as usize)
            .and_then(|s| s.selection.as_ref())
            .is_some_and(|n| n.actor == actor && n.prompt == "SelectCardEffect" && n.zone.is_none());
        if zoneless_card_pick {
            self.pick_zone_hint = Some((at, "Trash"));
        }
    }

    ///
    /// A card whose DCGO script asks a "do it?" bool in front of that menu
    /// (`selection_resolve::dcgo_bool_gate_before_zone_menu`, ShineGreymon
    /// (ST24-07)) has two bools: unless this row is the recorded zone menu
    /// (`zone_menu_row`), it is the gate. Its "don't" value is `None` (the
    /// ordinary bool path declines the optional pick). Its "do it" value is
    /// `Ok(ZoneBool::Gate)`, consumed with no engine action, when the actor's
    /// next row is the zone menu (DCGO asks it only when both zones hold a
    /// candidate, so the engine must offer both) or a pick (DCGO set the zone
    /// itself, so the engine must offer one zone only); anything else is `Err`.
    fn hand_zone_bool(&self, game: &Game, payload: &crate::dcgo_recording::SelectionRow) -> Option<Result<ZoneBool, String>> {
        use super::selection_resolve::{
            dcgo_bool_gate_before_zone_menu, dcgo_bool_zone_menu, menu_pick_zone, union_offers_hand_and_trash, PickZone,
        };
        use crate::selection::{SelectionKind, UnionZoneSet};
        if payload.prompt != "generic_bool"
            || payload.unstamped
            || payload.engine_action.is_some()
            || payload.card_ids.is_some()
            || payload.targets.is_some()
            || payload.count.is_some()
            || payload.int_value.is_some()
            || payload.cancel == Some(true)
        {
            return None;
        }
        let value = payload.bool_value?;
        let actor = payload.actor;
        let p = game
            .pending_selection
            .as_ref()
            .filter(|p| p.selecting_player == actor && p.effect_choices.is_none())?;
        let SelectionKind::UnionZone { zones } = &p.kind else {
            return None;
        };
        let with_trash = zones.contains(UnionZoneSet::TRASH);
        if !zones.contains(UnionZoneSet::HAND) || !(with_trash || zones.contains(UnionZoneSet::MATERIAL)) {
            return None;
        }
        let card = game.card_data_for_handle(p.source_card).map(|d| d.card_id.clone()).unwrap_or_default();
        let (hand_value, trash_value) = dcgo_bool_zone_menu(&card)?;
        let row = self.cursor;
        if let Some(do_it) = dcgo_bool_gate_before_zone_menu(&card).filter(|_| self.zone_menu_row != Some(row)) {
            if value != do_it {
                return None;
            }
            let next = self
                .source
                .steps()
                .get(row as usize + 1)
                .and_then(|s| s.selection.as_ref())
                .filter(|n| n.actor == actor);
            let menu_next = next.is_some_and(|n| {
                n.prompt == "generic_bool"
                    && !n.unstamped
                    && n.engine_action.is_none()
                    && n.bool_value.is_some()
                    && n.card_ids.is_none()
                    && n.targets.is_none()
                    && n.int_value.is_none()
                    && n.count.is_none()
            });
            let pick_next = next.is_some_and(|n| {
                n.int_value.is_none()
                    && n.bool_value.is_none()
                    && n.count.is_none()
                    && n.card_ids.as_ref().is_some_and(|c| !c.is_empty())
                    && menu_pick_zone(n).is_some()
            });
            let (in_hand, in_trash) = union_offers_hand_and_trash(game, p);
            let both = in_hand && in_trash;
            return Some(match (menu_next, pick_next) {
                (true, _) if with_trash && both => Ok(ZoneBool::Gate { menu_next: true }),
                (false, true) if with_trash && !both => Ok(ZoneBool::Gate { menu_next: false }),
                _ => Err(format!(
                    "row {row}: P{actor}'s generic_bool {value} is {card}'s \"do it\" bool, which DCGO follows with its \
                     \"From hand\" / \"From trash\" bool when both zones hold a candidate and with the pick otherwise; \
                     the engine offers {} and the actor's next row is {}",
                    match (in_hand, in_trash) {
                        (true, true) => "both zones",
                        (true, false) => "the hand only",
                        (false, true) => "the trash only",
                        (false, false) => "neither zone",
                    },
                    next.map(|n| format!(
                        "{} ({})",
                        n.prompt,
                        selection_row_content(n).unwrap_or_else(|| decline_label(n).into())
                    ))
                    .unwrap_or_else(|| "absent".into())
                )),
            });
        }
        let other = if with_trash { "trash" } else { "digivolution cards" };
        let zone = if hand_value == Some(value) {
            PickZone::Hand
        } else if with_trash && trash_value == Some(value) {
            PickZone::Trash
        } else {
            return Some(Err(format!(
                "row {row}: P{actor}'s generic_bool {value} meets {card}'s pick over the hand and the {other}; in \
                 {card}'s DCGO menu {value} is not \"From hand\" ({hand_value:?}){}, so it names a zone the replay \
                 does not fold into that pick",
                if with_trash { format!(" or \"From trash\" ({trash_value:?})") } else { String::new() }
            )));
        };
        let next = self
            .source
            .steps()
            .get(row as usize + 1)
            .and_then(|s| s.selection.as_ref())
            .filter(|n| n.actor == actor);
        let pick = next.filter(|n| {
            n.int_value.is_none()
                && n.bool_value.is_none()
                && n.count.is_none()
                && (n.card_ids.as_ref().is_some_and(|c| !c.is_empty())
                    || (n.cancel == Some(true) && n.targets.as_ref().is_none_or(|t| t.is_empty())))
        });
        if pick.and_then(menu_pick_zone) == Some(zone) {
            return Some(Ok(ZoneBool::Menu));
        }
        Some(Err(format!(
            "row {row}: P{actor}'s generic_bool {value} is {card}'s \"From {}\" (its DCGO menu), so DCGO's next \
             decision is P{actor}'s pick there, but row {} is {}",
            if zone == PickZone::Hand { "hand" } else { "trash" },
            row + 1,
            self.source
                .steps()
                .get(row as usize + 1)
                .and_then(|s| s.selection.as_ref())
                .map(|n| format!(
                    "P{}'s {} ({}{})",
                    n.actor,
                    n.prompt,
                    selection_row_content(n).unwrap_or_else(|| decline_label(n).into()),
                    n.zone.as_deref().map(|z| format!(", zone {z}")).unwrap_or_default()
                ))
                .unwrap_or_else(|| "absent".into())
        )))
    }

    /// DCGO's "From which area?" INT menu (`payload`) against the actor's ONE
    /// pick over the hand and the trash (see the call site). `Ok(None)`: the
    /// int named the zone of the actor's next pick row and is consumed with no
    /// engine action; `Ok(Some(PASS))`: it was the menu's "Don't" with no pick
    /// after it, the optional pick's decline; `Err`: no grounded reading.
    ///
    /// The values are the card's own (`selection_resolve::dcgo_int_zone_menu`,
    /// read from its DCGO script); a card without a verified menu, a value
    /// outside it, a pick in the other zone, a pick after "Don't" (DCGO opens
    /// none: P_249.cs:76 `if (selectedArea == 3) yield break;`, EX12_074.cs:
    /// 160-170) and a zone with no pick row after it all halt.
    fn hand_or_trash_int_menu(
        &self,
        game: &Game,
        payload: &crate::dcgo_recording::SelectionRow,
    ) -> Result<Option<u16>, String> {
        use super::selection_resolve::{dcgo_int_zone_menu, menu_pick_zone, PickZone};
        let actor = payload.actor;
        let v = payload.int_value.unwrap_or_default();
        let row = self.cursor;
        let p = game
            .pending_selection
            .as_ref()
            .ok_or_else(|| format!("row {row}: no prompt is parked"))?;
        let card = game.card_data_for_handle(p.source_card).map(|d| d.card_id.clone()).unwrap_or_default();
        let next = self
            .source
            .steps()
            .get(row as usize + 1)
            .and_then(|s| s.selection.as_ref())
            .filter(|n| n.actor == actor);
        let names_cards = |n: &crate::dcgo_recording::SelectionRow| n.card_ids.as_ref().is_some_and(|c| !c.is_empty());
        let pick = next.filter(|n| {
            n.int_value.is_none()
                && n.bool_value.is_none()
                && n.count.is_none()
                && (names_cards(n) || (n.cancel == Some(true) && n.targets.as_ref().is_none_or(|t| t.is_empty())))
        });
        let describe = || {
            self.source
                .steps()
                .get(row as usize + 1)
                .and_then(|s| s.selection.as_ref())
                .map(|n| {
                    format!(
                        "P{}'s {} ({}{})",
                        n.actor,
                        n.prompt,
                        selection_row_content(n).unwrap_or_else(|| decline_label(n).into()),
                        n.zone.as_deref().map(|z| format!(", zone {z}")).unwrap_or_default()
                    )
                })
                .unwrap_or_else(|| "absent".into())
        };
        let Some((hand, trash, dont)) = dcgo_int_zone_menu(&card) else {
            return Err(format!(
                "row {row}: P{actor}'s generic_int {v} meets {card}'s pick over the hand and the trash, but {card}'s \
                 DCGO script has no verified \"From hand\" / \"From trash\" int menu \
                 (selection_resolve::dcgo_int_zone_menu): what the int chose cannot be read"
            ));
        };
        if dont == Some(v) {
            if pick.is_some_and(names_cards) {
                return Err(format!(
                    "row {row}: P{actor}'s generic_int {v} is {card}'s \"Don't\", which opens no pick in DCGO, but \
                     row {} is {}: the pick belongs to something else, and the replay does not guess what",
                    row + 1,
                    describe()
                ));
            }
            if p.is_optional {
                return Ok(Some(crate::action::space::PASS));
            }
            return Err(format!(
                "row {row}: P{actor}'s generic_int {v} is {card}'s \"Don't\", but the engine's pick over the hand \
                 and the trash cannot be declined"
            ));
        }
        let zone = if hand == Some(v) {
            PickZone::Hand
        } else if trash == Some(v) {
            PickZone::Trash
        } else {
            return Err(format!(
                "row {row}: P{actor}'s generic_int {v} is not a value of {card}'s DCGO zone menu (From hand \
                 {hand:?} / From trash {trash:?} / Don't {dont:?})"
            ));
        };
        if pick.and_then(menu_pick_zone) == Some(zone) {
            return Ok(None);
        }
        Err(format!(
            "row {row}: P{actor}'s generic_int {v} is {card}'s \"From {}\", so DCGO's next decision is \
             P{actor}'s pick there, but row {} is {}",
            if zone == PickZone::Hand { "hand" } else { "trash" },
            row + 1,
            describe()
        ))
    }

    /// Why a recorded DCGO `generic_bool` cannot answer the parked prompt, or
    /// `None` when it may go on to the ordinary resolution.
    ///
    /// The rows checked are the ones with nothing tying them to an engine
    /// prompt: not `unstamped` (those are `OptionalSkill.SetUseOptional`
    /// "use this effect?" answers, which `classify_unstamped_answer` maps onto
    /// optional-activation gates only) and without an `engine_action` (a
    /// playback recording's answer, made FROM an engine action). That leaves
    /// `UserSelectionManager.SetBoolForPlayer` (UserSelectionManager.cs:82-120):
    /// the answer to a card-authored `SetBoolSelection` menu
    /// (UserSelectionManager.cs:227-281). The row has no message, so it means
    /// whatever that card's two buttons say - "Digivolve" / "Do not digivolve",
    /// "From hand" / "From trash", "Security Top" / "Security Bottom". Read
    /// positionally ("no" passes any optional prompt, "yes" takes a lone accept
    /// id or the yes branch of any yes/no-shaped choice) it answered prompts
    /// DCGO never asked: FELLOWSHIP OF HOPE'S KEEPERS (BT20-095)'s "Digivolve?
    /// yes" took the engine's "Move ...?" pick. A bool may therefore meet the
    /// parked prompt of its actor only when:
    /// * the prompt is a deck top / bottom placement, which DCGO asks as this
    ///   bool (RevealLibrary.cs:627-645, Deck Top = true);
    /// * the prompt is a pick over the hand and the trash (or the digivolution
    ///   cards) of a card whose DCGO script asks the zone as a bool, and the
    ///   bool names the zone of the actor's next pick (`hand_zone_bool`; Asuna
    ///   Shiroki (BT25-092));
    /// * or the card that installed the prompt asks a YES/NO bool in its own
    ///   DCGO script (`selection_resolve::dcgo_card_asks_yes_no_bool`). Within
    ///   such a card the mapping is still by shape (its yes/no bool onto its
    ///   yes/no-shaped prompt); a card asking more than one is resolved only as
    ///   far as that shape allows, or by a card fold like `bt20_095_delay_fold`.
    ///
    /// Anything else - a body choice such as RizeGreymon (ST24-06)'s "Trash 2
    /// face-down cards" / "Don't", which DCGO asks as a `canNoSelect` Tamer
    /// `SelectPermanentEffect`, not a bool - is a divergence. A prompt of the
    /// other seat, or none, is left to the actor check and the "met no prompt"
    /// handling.
    fn stamped_bool_meaning_mismatch(
        &self,
        game: &Game,
        payload: &crate::dcgo_recording::SelectionRow,
    ) -> Option<String> {
        if payload.prompt != "generic_bool" || payload.unstamped || payload.engine_action.is_some() {
            return None;
        }
        let value = payload.bool_value?;
        let actor = payload.actor;
        let p = game.pending_selection.as_ref().filter(|p| p.selecting_player == actor)?;
        if super::selection_resolve::is_top_bottom_choice(p) {
            return None;
        }
        // A pick over the hand and another zone of a card whose DCGO script
        // asks the zone as a bool: the bool must name the zone of the pick that
        // follows (`hand_zone_bool`), whatever its value.
        if let Some(verdict) = self.hand_zone_bool(game, payload) {
            return verdict.err();
        }
        // A card's two-mode body choice that DCGO asks as a bool with the
        // card's own button meanings (Omnimon (EX12-037)'s "DP - 13K" = true /
        // "Trash security" = false, EX12_037.cs:157-170).
        if super::selection_resolve::mode_bool_branches(game, p).is_some() {
            return None;
        }
        let card = game.card_data_for_handle(p.source_card).map(|d| d.card_id.clone()).unwrap_or_default();
        if super::selection_resolve::dcgo_card_asks_yes_no_bool(&card) {
            return None;
        }
        Some(format!(
            "row {}: P{actor}'s generic_bool {value} (a DCGO SetBoolSelection answer, which records no question) meets \
             P{actor}'s {:?} '{}' of {card}, whose DCGO script asks no yes/no bool \
             (runners/dcgo_bool_yes_no_cards.json) and which is no top/bottom or hand/trash zone choice: \
             what DCGO asked cannot be mapped onto this prompt",
            self.cursor, p.kind, p.prompt
        ))
    }

    /// FELLOWSHIP OF HOPE'S KEEPERS (BT20-095)'s <Delay> body, answered from
    /// the rows DCGO writes for it. `None` when the parked prompt is not that
    /// card's breeding "Move ...?" pick of `payload`'s actor; otherwise the game
    /// after the whole body's decisions and the number of rows they consumed,
    /// or the reason the rows cannot be the answer (nothing is applied then).
    ///
    /// DCGO (`CardEffect/BT20/Black/BT20_095.cs`):
    /// * :107-113 - when a breeding Digimon meets the condition (Lv.3+, can
    ///   move) and a frame is empty, it moves `GetBreedingAreaPermanents()[0]`
    ///   with NO prompt. The engine asks the move as an optional breeding pick
    ///   (CR 4.3 15-7-1: in "by X, Y" the X is optional), so DCGO's run is the
    ///   "yes" and the replay answers it without a row. Its candidate is the
    ///   one breeding Digimon, so there is nothing to choose.
    /// * :124-140 - only when a [Chronicle] Digimon card in hand or trash can
    ///   digivolve onto it: the bool "Digivolve?" (Digivolve = true, Do not
    ///   digivolve = false). That is THIS row.
    /// * :142-162 - on true, with candidates in both zones: the bool "From which
    ///   area do you play a card?" (From hand = true, From trash = false); with
    ///   one zone, `SetBool(canSelectHand)`, which records nothing
    ///   (UserSelectionManager.cs:122-126 skips the `SetBoolForPlayer` hook).
    /// * :167-177 - `DigivolveIntoHandOrTrashCard(isHand: <that bool>)`:
    ///   `SelectHandEffect` for the hand, `SelectCardEffect` for the trash,
    ///   both `canNoSelect` (CardEffectCommons.cs:1049-1083).
    ///
    /// The engine asks the digivolution as ONE optional pick over hand and
    /// trash (BT20-095.yaml), which the Digivolve bool, the zone bool (or the
    /// zone the engine's candidates leave) and the card row answer together.
    /// Card-specific: other cards ask "Digivolve?" + "From hand / From trash"
    /// too, but none of them was checked to lead with an unasked move.
    fn bt20_095_delay_fold(
        &self,
        game: &Game,
        payload: &crate::dcgo_recording::SelectionRow,
    ) -> Option<Result<(Game, u32), String>> {
        use crate::action::space::{encode_breeding_select, PASS, PLAY_HAND_END, PLAY_HAND_START, TRASH_EFFECT_END, TRASH_EFFECT_START};
        use crate::owned_card_choice::{CardChoiceOperation, CardChoiceZone};
        const CARD: &str = "BT20-095";
        let actor = payload.actor;
        let digivolve = payload.bool_value?;
        let move_id = encode_breeding_select(actor)?;
        let p = game.pending_selection.as_ref()?;
        if p.selecting_player != actor
            || !p.is_optional
            || p.valid_action_ids != [move_id]
            || game.card_data_for_handle(p.source_card).map(|d| d.card_id.as_str()) != Some(CARD)
        {
            return None;
        }
        let row = self.cursor;
        let legal = |g: &Game, id: u16| crate::action::build_action_mask(g, actor).get(id as usize).is_some_and(|v| *v > 0.5);
        let mut g = game.clone();
        if !legal(&g, move_id) {
            return Some(Err(format!("row {row}: {CARD}'s <Delay>: the engine refuses the move DCGO made without asking")));
        }
        g.decode_action(move_id, actor);
        // The engine's hand-or-trash pick of this card, for this actor.
        let union = g.pending_selection.as_ref().filter(|q| {
            q.selecting_player == actor
                && q.is_optional
                && game.card_data_for_handle(q.source_card).map(|d| d.card_id.as_str()) == Some(CARD)
        });
        let Some(union) = union else {
            return Some(Err(format!(
                "row {row}: {CARD}'s <Delay> moved P{actor}'s breeding Digimon; DCGO then asked \"Digivolve?\" \
                 (answered {digivolve}), which it asks only when a [Chronicle] Digimon card in hand or trash can \
                 digivolve onto it (BT20_095.cs:124-127), but the engine offers no such digivolution"
            )));
        };
        // (card, zone is hand, action) for every card the pick accepts.
        let candidates: Vec<(String, bool, u16)> = match &union.card_choice {
            Some(owner) => {
                let view = owner.view(&g, actor);
                if view.page_count != "1" {
                    return Some(Err(format!(
                        "row {row}: {CARD}'s <Delay> digivolution pick is paginated ({} pages): its candidates by zone are \
                         not all visible, so DCGO's zone and card rows cannot be checked against it",
                        view.page_count
                    )));
                }
                view.rows
                    .iter()
                    .filter(|r| matches!(r.operation, CardChoiceOperation::CommitCard) && legal(&g, r.action_id))
                    .filter_map(|r| {
                        let hand = match r.zone {
                            Some(CardChoiceZone::Hand) => true,
                            Some(CardChoiceZone::Trash) => false,
                            _ => return None,
                        };
                        Some((r.card_id.clone()?, hand, r.action_id))
                    })
                    .collect()
            }
            None => union
                .valid_action_ids
                .iter()
                .filter_map(|&id| {
                    let me = g.player(actor);
                    if (PLAY_HAND_START..PLAY_HAND_END).contains(&id) {
                        me.hand.get((id - PLAY_HAND_START) as usize).map(|c| (c.card_id(&g.card_data).to_string(), true, id))
                    } else if (TRASH_EFFECT_START..TRASH_EFFECT_END).contains(&id) {
                        me.trash.get((id - TRASH_EFFECT_START) as usize).map(|c| (c.card_id(&g.card_data).to_string(), false, id))
                    } else {
                        None
                    }
                })
                .collect(),
        };
        let in_hand = candidates.iter().any(|c| c.1);
        let in_trash = candidates.iter().any(|c| !c.1);
        if !in_hand && !in_trash {
            return Some(Err(format!(
                "row {row}: {CARD}'s <Delay> digivolution pick has no hand or trash candidate the replay can read, \
                 but DCGO asked \"Digivolve?\" (BT20_095.cs:124-127)"
            )));
        }
        if !digivolve {
            if !legal(&g, PASS) {
                return Some(Err(format!("row {row}: {CARD}'s <Delay>: DCGO declined the digivolution, but the engine's pick cannot be declined")));
            }
            g.decode_action(PASS, actor);
            return Some(Ok((g, 1)));
        }
        let steps = self.source.steps();
        let next = |k: u32| steps.get((row + k) as usize).and_then(|s| s.selection.as_ref());
        let describe = |n: Option<&crate::dcgo_recording::SelectionRow>| match n {
            Some(n) => format!("P{}'s {} ({})", n.actor, n.prompt, selection_row_content(n).unwrap_or_else(|| decline_label(n).into())),
            None => "absent".to_string(),
        };
        let (from_hand, pick_at) = if in_hand && in_trash {
            match next(1) {
                Some(n) if n.actor == actor && n.prompt == "generic_bool" && !n.unstamped && n.bool_value.is_some() => {
                    (n.bool_value.unwrap_or_default(), 2)
                }
                other => {
                    return Some(Err(format!(
                        "row {row}: {CARD}'s <Delay>: DCGO said \"Digivolve\", and with candidates in both hand and trash \
                         it asks \"From which area do you play a card?\" next (BT20_095.cs:138-151), but row {} is {}",
                        row + 1,
                        describe(other)
                    )))
                }
            }
        } else {
            (in_hand, 1)
        };
        // CardEffectCommons.cs:1049-1083: the hand pick is a SelectHandEffect,
        // the trash pick a SelectCardEffect.
        let want_prompt = if from_hand { "SelectHandEffect" } else { "SelectCardEffect" };
        let zone = if from_hand { "hand" } else { "trash" };
        let pick = next(pick_at).filter(|n| n.actor == actor && n.prompt == want_prompt);
        let Some(pick) = pick else {
            return Some(Err(format!(
                "row {row}: {CARD}'s <Delay>: DCGO digivolves from the {zone}, so its next decision is P{actor}'s \
                 {want_prompt} (CardEffectCommons.cs:1049-1083), but row {} is {}",
                row + pick_at,
                describe(next(pick_at))
            )));
        };
        let cards = pick.card_ids.as_deref().unwrap_or_default();
        if cards.is_empty() && pick.cancel == Some(true) && pick.targets.is_none() {
            // The pick's own `canNoSelect` decline.
            if !legal(&g, PASS) {
                return Some(Err(format!("row {row}: {CARD}'s <Delay>: DCGO cancelled the digivolution pick, but the engine's pick cannot be declined")));
            }
            g.decode_action(PASS, actor);
            return Some(Ok((g, pick_at + 1)));
        }
        let [want] = cards else {
            return Some(Err(format!(
                "row {row}: {CARD}'s <Delay>: row {} must pick exactly one card, it is {}",
                row + pick_at,
                describe(Some(pick))
            )));
        };
        let Some(&(_, _, id)) = candidates.iter().find(|(c, hand, _)| c == want && *hand == from_hand) else {
            return Some(Err(format!(
                "row {row}: {CARD}'s <Delay>: DCGO digivolved into {want} from the {zone}, which the engine's pick \
                 does not offer there (candidates {:?})",
                candidates.iter().map(|(c, h, _)| format!("{c} ({})", if *h { "hand" } else { "trash" })).collect::<Vec<_>>()
            )));
        };
        g.decode_action(id, actor);
        Some(Ok((g, pick_at + 1)))
    }

    /// Which entry of a parked order prompt a card-less `MultipleSkills` index
    /// chose, proved by the actor's next recorded row.
    ///
    /// `None` when the rule does not apply (no order prompt of `actor` with two
    /// or more effects): the row keeps its ordinary path. Otherwise each entry
    /// is chosen on a clone of the game (with the actor's unstamped yes/no
    /// answers that follow the index applied to the chosen effect's own gate,
    /// and the replay's forced lone-trigger answers), and:
    ///
    /// * when the next row names cards or targets, or is a cancel, an entry
    ///   qualifies only if that row is answered by the prompt the entry leads
    ///   to (a pick row by a pick prompt of the actor; a cancel by an optional
    ///   prompt, another player's prompt, or a fizzled effect);
    /// * an int / count / bool row (or none) proves nothing - every positional
    ///   reading accepts it - so every entry qualifies.
    ///
    /// One qualifying entry, or several that reach the same
    /// `verification_digest` once the row is applied, is the answer. Anything
    /// else is `Err`: DCGO's index cannot be mapped onto our order list, so the
    /// replay stops instead of guessing.
    fn remote_trigger_order_choice(
        &mut self,
        game: &Game,
        actor: PlayerId,
        dcgo_index: i64,
    ) -> Option<Result<u16, String>> {
        use super::selection_resolve::{payload_pick_count, resolve_next};
        use crate::action::space::{PASS, REPLACEMENT_ACCEPT};
        use crate::selection::SelectionKind;
        const REPLACEMENT_ORDER: &str = REPLACEMENT_ORDER_PROMPT;
        let is_order_prompt = |p: &crate::selection::PendingSelection| {
            matches!(p.kind, SelectionKind::TriggerOrder)
                || (matches!(p.kind, SelectionKind::EffectChoice) && p.prompt == REPLACEMENT_ORDER)
        };
        let pending = game.pending_selection.as_ref()?;
        if !is_order_prompt(pending) || pending.selecting_player != actor {
            return None;
        }
        let entries: Vec<(u16, String)> = pending
            .effect_choices
            .as_ref()?
            .iter()
            .filter(|e| e.source_card.is_some() && pending.valid_action_ids.contains(&e.action_id))
            .map(|e| (e.action_id, e.label.clone()))
            .collect();
        if entries.len() < 2 {
            return None;
        }
        let prompt_text = pending.prompt.clone();
        // Entries that are copies of one card lead to branches that differ only
        // in which physical copy is which: compared with that renamed away.
        let identity_free = entries_are_copies_of_one_card(game);

        // The actor's unstamped "use this effect?" answers straight after the
        // index belong to the effect it chose; the row after them is the evidence.
        let (gate_answers, evidence, branch_cursor) = {
            let steps = self.source.steps();
            let mut at = self.cursor as usize + 1;
            let mut gate_answers = Vec::new();
            while let Some(sel) = steps.get(at).and_then(|s| s.selection.as_ref()) {
                if sel.actor == actor && sel.unstamped && sel.prompt == "generic_bool" && sel.bool_value.is_some() {
                    gate_answers.push((at, sel.clone()));
                    at += 1;
                } else {
                    break;
                }
            }
            let evidence = steps
                .get(at)
                .and_then(|s| s.selection.as_ref())
                .filter(|n| {
                    n.actor == actor
                        && n.int_value.is_none()
                        && n.bool_value.is_none()
                        && n.count.is_none()
                        && (n.card_ids.as_ref().is_some_and(|c| !c.is_empty())
                            || n.targets.as_ref().is_some_and(|t| !t.is_empty())
                            || n.cancel == Some(true))
                })
                .cloned();
            (gate_answers, evidence, at as u32)
        };

        // (entry, label, digest once the evidence row is applied under it, the
        // game the entry leads to before that row)
        let mut qualifying: Vec<(u16, String, u64, Game)> = Vec::new();
        let mut rejected: Vec<String> = Vec::new();
        // Entries under which `step()` would stop at a decision DCGO made
        // without a row (`answer_dcgo_silent_prompts`): neither confirmed nor
        // refuted by the evidence, which they never reach. Like a nested
        // ambiguity in `lookahead_trigger_order`, such a branch ends its own
        // evidence: it still stands, and never converges with another.
        let mut undetermined: Vec<String> = Vec::new();
        for (id, label) in &entries {
            let mut probe = game.clone();
            probe.decode_action(*id, actor);
            // Each row is met the way `step()` meets it: DCGO's silent prompts
            // first (`answer_dcgo_silent_prompts`, the same function), then the
            // unstamped answer, which only an optional-activation gate takes.
            let mut halted = None;
            let mut stuck = None;
            for (row_at, answer) in &gate_answers {
                if let Some(reason) = self.answer_dcgo_silent_prompts(&mut probe, *row_at) {
                    stuck = Some(reason);
                    break;
                }
                let value = answer.bool_value.unwrap_or_default();
                let next = self.source.steps().get(*row_at + 1).and_then(|s| s.selection.as_ref());
                let window = match Self::unstamped_window(&probe.events, self.unstamped_window_start) {
                    Ok(window) => window,
                    Err(reason) => return Some(Err(format!("row {row_at}: {reason}"))),
                };
                match Self::classify_unstamped_answer(&probe, actor, value, next, *row_at as u32, window) {
                    UnstampedAnswer::Apply(Some(a)) => probe.decode_action(a, actor),
                    UnstampedAnswer::Apply(None) | UnstampedAnswer::Note(_) => {}
                    // `step()` would stop at this answer under this entry.
                    UnstampedAnswer::Halt(reason) => {
                        halted = Some(reason);
                        break;
                    }
                    // The answer can neither confirm nor refute this entry
                    // (round-4 finding 13): the entry ends its evidence here.
                    UnstampedAnswer::CannotTell(reason) => {
                        stuck = Some(reason);
                        break;
                    }
                }
            }
            if let Some(reason) = stuck {
                undetermined.push(format!("{label}: {reason}"));
                continue;
            }
            if let Some(reason) = halted {
                rejected.push(format!("{label}: {reason}"));
                continue;
            }
            if let Some(reason) = self.answer_dcgo_silent_prompts(&mut probe, branch_cursor as usize) {
                undetermined.push(format!("{label}: {reason}"));
                continue;
            }
            let branch_start = probe.clone();
            let Some(row) = evidence.as_ref() else {
                qualifying.push((*id, label.clone(), order_branch_digest(&probe, identity_free), branch_start));
                continue;
            };
            let row = remap_selection_targets(&probe, row).unwrap_or_else(|| row.clone());
            let is_cancel = row.cancel == Some(true)
                && row.card_ids.as_ref().is_none_or(|c| c.is_empty())
                && row.targets.as_ref().is_none_or(|t| t.is_empty());
            if is_cancel {
                let declinable = match probe.pending_selection.as_ref() {
                    None => true,
                    Some(p) => {
                        p.is_optional
                            || p.selecting_player != actor
                            || matches!(p.kind, SelectionKind::OrderedPermutation { .. })
                    }
                };
                if !declinable {
                    rejected.push(format!("{label}: a mandatory prompt follows, which a cancel cannot answer"));
                    continue;
                }
                if probe
                    .pending_selection
                    .as_ref()
                    .is_some_and(|p| p.is_optional && p.selecting_player == actor)
                {
                    probe.decode_action(PASS, actor);
                }
            } else {
                // The entry's effect leads with a branch `EffectChoice` that DCGO
                // folds into the pick itself (BT20-087's "which area?" ahead of
                // its field pick: DCGO asks one `SelectPermanentEffect` over both
                // areas, BT20_087.cs:114-127). `step()` meets the row through
                // `probe_branch_for_row` (the branch whose follow-up accepts the
                // row, only when exactly one does); the evidence is met the same
                // way, by the same function.
                if !self.source.answers_engine_only_prompts()
                    && probe
                        .pending_selection
                        .as_ref()
                        .is_some_and(|p| {
                            matches!(p.kind, SelectionKind::EffectChoice)
                                && !is_order_prompt(p)
                                && p.selecting_player == actor
                        })
                {
                    if let Some((branch, _)) = self.probe_branch_for_row(&probe, &row, actor, branch_cursor as usize) {
                        probe.decode_action(branch, actor);
                    }
                }
                // A one-option prompt DCGO resolves without a row stands in
                // front of the evidence (Owen Dreadnought (EX11-054)'s lone
                // <Progress> pick, EX11_054.cs:66-69): `step()` answers it
                // before the row, and so does the probe.
                self.answer_lone_prompts_before_row(&mut probe, &row, actor, branch_cursor as usize);
                // A chosen "you may" effect parks its own accept; a pick row is
                // the acceptance AND the pick (the replay's implied-accept fold).
                let accept_first = probe.pending_selection.as_ref().is_some_and(|p| {
                    p.is_optional && p.selecting_player == actor && p.valid_action_ids == [REPLACEMENT_ACCEPT]
                }) && resolve_next(&probe, &row, 0).is_err();
                if accept_first {
                    let mut accepted = probe.clone();
                    accepted.decode_action(REPLACEMENT_ACCEPT, actor);
                    if matches!(resolve_next(&accepted, &row, 0), Ok(Some(_))) {
                        probe = accepted;
                    }
                }
                // A pick row is answered by a PICK prompt of the actor: an order
                // prompt accepts a card id positionally, which proves nothing.
                let answers = probe.pending_selection.as_ref().is_some_and(|p| {
                    p.selecting_player == actor
                        && !is_order_prompt(p)
                        && !matches!(p.kind, SelectionKind::OrderedPermutation { .. })
                }) && matches!(resolve_next(&probe, &row, 0), Ok(Some(a)) if a != PASS);
                if !answers {
                    let showing = probe
                        .pending_selection
                        .as_ref()
                        .map(|p| format!("{:?} P{} '{}'", p.kind, p.selecting_player, p.prompt))
                        .unwrap_or_else(|| "no prompt".into());
                    rejected.push(format!("{label}: leads to {showing}"));
                    continue;
                }
                let mut picks = 0usize;
                while let Ok(Some(a)) = resolve_next(&probe, &row, picks) {
                    probe.decode_action(a, actor);
                    picks += 1;
                    if picks > payload_pick_count(&row) + 1 {
                        break;
                    }
                }
            }
            // ... and the row after the evidence meets the same silent answers.
            if let Some(reason) = self.answer_dcgo_silent_prompts(&mut probe, branch_cursor as usize + 1) {
                undetermined.push(format!("{label}: {reason}"));
                continue;
            }
            qualifying.push((*id, label.clone(), order_branch_digest(&probe, identity_free), branch_start));
        }
        let evidence_text = match evidence.as_ref() {
            Some(r) => format!(
                "next row {} cards {:?} targets {:?}{}",
                r.prompt,
                r.card_ids.clone().unwrap_or_default(),
                r.targets.iter().flatten().map(|t| (t.player, t.frame)).collect::<Vec<_>>(),
                if r.cancel == Some(true) { " (cancel)" } else { "" }
            ),
            None => "next row (none of this actor names a card or target, or cancels, so nothing proves the choice)".into(),
        };
        if !undetermined.is_empty() {
            let fits = match qualifying.len() {
                1 => "1 entry fits".to_string(),
                n => format!("{n} entries fit"),
            };
            return Some(Err(format!(
                "card-less MultipleSkills index {dcgo_index} (DCGO's own ordering, no chosen_effect_card) {AMBIGUOUS_ORDER}: \
                 {fits} the actor's {evidence_text} ([{}]), {} rejected ([{}]), and under [{}] the replay stops at a \
                 decision DCGO made without a row, or at an answer it cannot tell is this entry's, so those entries \
                 can be neither tested nor excluded",
                qualifying.iter().map(|q| q.1.as_str()).collect::<Vec<_>>().join(" | "),
                rejected.len(),
                rejected.join("; "),
                undetermined.join("; ")
            )));
        }
        if qualifying.is_empty() {
            return Some(Err(format!(
                "card-less MultipleSkills index {dcgo_index} (DCGO's own ordering, no chosen_effect_card): no entry of the \
                 {prompt_text:?} prompt leads to the actor's {evidence_text} ({})",
                rejected.join("; ")
            )));
        }
        if qualifying.len() == 1 || qualifying.iter().all(|q| q.2 == qualifying[0].2) {
            return Some(Ok(qualifying[0].0));
        }
        // Still several, leading to different games right after the row: the
        // order may only LOOK observable (two copies of one effect - EX12-061 and
        // EX12-062's identical [When Attacking][OPT] "draw 1, then trash 1" -
        // differ only in which copy is still pending). Replay the rows that
        // follow under each entry: entries that meet the same game are the same
        // choice, and an entry the later rows refute is not DCGO's.
        let labels = qualifying.iter().map(|q| q.1.as_str()).collect::<Vec<_>>().join(" | ");
        let branches: Vec<(u16, Game)> = qualifying.into_iter().map(|(id, _, _, g)| (id, g)).collect();
        match self.lookahead_trigger_order(branches, branch_cursor, &order_entry_sources(game), identity_free) {
            LookaheadVerdict::Proved(id) => return Some(Ok(id)),
            // Not an ambiguity: no entry leads to what DCGO recorded, so the
            // order (or the branch that led to it, inside a lookahead) is
            // wrong.
            LookaheadVerdict::AllRefuted => {
                return Some(Err(format!(
                    "card-less MultipleSkills index {dcgo_index} (DCGO's own ordering, no chosen_effect_card): \
                     every entry [{labels}] of the {prompt_text:?} prompt fits the actor's {evidence_text}, but the \
                     next {LOOKAHEAD_ROWS} rows {REFUTED_ORDER}"
                )))
            }
            LookaheadVerdict::Undecided => {}
        }
        Some(Err(format!(
            "card-less MultipleSkills index {dcgo_index} (DCGO's own ordering, no chosen_effect_card) {AMBIGUOUS_ORDER}: \
             entries [{labels}] of the {prompt_text:?} prompt all fit the actor's {evidence_text}, lead to different \
             games and are not told apart by the next {LOOKAHEAD_ROWS} rows; the recording does not say which effect \
             DCGO activated"
        )))
    }

    /// Replay up to [`LOOKAHEAD_ROWS`] recorded rows from `from` under each
    /// branch (the game a candidate order entry leads to) and return the entry
    /// the rows prove: the first entry when every branch the rows do not refute
    /// reaches the same `verification_digest` at the same row (the order is not
    /// observable), or the only branch they do not refute. `Undecided`
    /// otherwise, and `AllRefuted` when every branch is refuted.
    ///
    /// A branch is refuted by any divergence other than a nested ambiguity; a
    /// nested ambiguity only ends that branch's evidence. The rows run on a
    /// driver of their own (the source is lent to it, with this driver's
    /// per-row state such as a pending `raid_retarget`), at most
    /// [`MAX_LOOKAHEAD_DEPTH`] lookaheads deep. See [`pick_lookahead_branch`]
    /// for when a branch the rows did not refute may stand, and
    /// [`lookahead_verdict`] for when the rows refute every branch (which a
    /// lookahead one level up reads as a refutation of ITS branch, not as an
    /// ambiguity).
    fn lookahead_trigger_order(
        &mut self,
        branches: Vec<(u16, Game)>,
        from: u32,
        sources: &[(u16, crate::card_source::CardHandle)],
        identity_free: bool,
    ) -> LookaheadVerdict {
        // Each branch chose its entry at the row under the cursor (`order_answer`).
        let order_row = self.cursor;
        if self.lookahead_depth >= MAX_LOOKAHEAD_DEPTH {
            return LookaheadVerdict::Undecided;
        }
        let mut runs: Vec<LookaheadRun> = Vec::new();
        let (policy, raid_retarget, depth) = (self.policy, self.raid_retarget, self.lookahead_depth);
        let unstamped_window_start = self.unstamped_window_start;
        for (id, mut g) in branches {
            // The sub-driver borrows the real source for its rows; the guard
            // gives it back when dropped, including while unwinding a panic, so
            // this driver never keeps the `NoRecordedSteps` placeholder.
            let mut lent = LentSource::lend(
                &mut self.source,
                ReplayDriver {
                    source: Box::new(NoRecordedSteps),
                    cursor: from,
                    verify: false,
                    policy,
                    divergences: Vec::new(),
                    digivolve_operand_fallbacks: Vec::new(),
                    inferred_skipped: Vec::new(),
                    notes: Vec::new(),
                    raid_retarget,
                    paused: false,
                    lookahead_depth: depth + 1,
                    pick_zone_hint: None,
                    zone_menu_row: self.zone_menu_row,
                    unstamped_window_start,
                    order_answer: sources.iter().find(|(e, _)| *e == id).map(|(_, src)| (order_row, *src)),
                    dcgo_cards: self.dcgo_cards.clone(),
                    count_evidence: self.count_evidence.clone(),
                },
            );
            let sub = &mut lent.sub;
            let mut seen = Vec::new();
            for _ in 0..LOOKAHEAD_ROWS {
                if sub.is_complete() {
                    break;
                }
                let before = sub.cursor;
                sub.step(&mut g);
                if sub.paused || sub.cursor == before {
                    break;
                }
                seen.push((sub.cursor, order_branch_digest(&g, identity_free)));
            }
            let nested = |d: &Divergence, marker: &str| {
                matches!(&d.kind, DivergenceKind::SelectionResolution { reason } if reason.contains(marker))
            };
            let refuted_at = sub
                .divergences
                .iter()
                .filter(|d| !nested(d, AMBIGUOUS_ORDER) && !nested(d, REFUTED_ORDER))
                .map(|d| d.step)
                .min();
            let nested_refuted_at = if refuted_at.is_none() {
                sub.divergences.iter().filter(|d| nested(d, REFUTED_ORDER)).map(|d| d.step).min()
            } else {
                None
            };
            runs.push(LookaheadRun { entry: id, seen, reached: sub.cursor, refuted_at, nested_refuted_at });
        }
        lookahead_verdict(&runs)
    }

    /// The branch of a parked (non-yes/no) `EffectChoice` that the recorded row
    /// proves, or `None` when the row belongs to the prompt itself or no branch
    /// (or no single branch) is proved. See the call site for the policy.
    fn probe_branch_for_row(
        &self,
        game: &Game,
        payload: &crate::dcgo_recording::SelectionRow,
        actor: PlayerId,
        row_at: usize,
    ) -> Option<(u16, bool)> {
        use crate::selection::SelectionKind;
        let pending = game.pending_selection.as_ref()?;
        // A trigger-ordering prompt too: DCGO's `MultipleSkills` lists only the
        // effects whose `CanActivate` holds RIGHT NOW and skips the question
        // (recording no row) when one is left, while the engine orders every
        // queued trigger. A recorded `MultipleSkills` row is handled by
        // `resolve_next`; any other pick row names the trigger that ran.
        let is_trigger_order = matches!(pending.kind, SelectionKind::TriggerOrder)
            && payload.prompt != "MultipleSkills";
        if !(matches!(pending.kind, SelectionKind::EffectChoice) || is_trigger_order)
            || pending.selecting_player != actor
            || super::selection_resolve::yes_no_branches(pending).is_some()
            || payload.bool_value.is_some()
            || payload.int_value.is_some()
            || payload.count.is_some()
        {
            return None;
        }
        let entries = pending.effect_choices.as_ref()?;
        if entries.len() < 2 {
            return None;
        }
        let named_card = payload.card_ids.as_ref().and_then(|c| c.first().cloned());
        let is_cancel = payload.cancel == Some(true)
            && named_card.is_none()
            && payload.targets.as_ref().is_none_or(|t| t.is_empty());
        let is_named_pick = named_card.is_some()
            || payload.targets.as_ref().is_some_and(|t| !t.is_empty());
        if !is_cancel && !is_named_pick {
            return None;
        }
        // The row already resolves against the branch list itself: not ours.
        if !is_cancel && super::selection_resolve::resolve_next(game, payload, 0).is_ok() {
            return None;
        }
        if let Some(want) = named_card.as_deref() {
            let belongs = entries.iter().any(|e| {
                e.source_card
                    .and_then(|c| game.card_data_for_handle(c))
                    .is_some_and(|d| d.card_id == want)
            });
            if belongs {
                return None;
            }
        }
        // (branch action, the recorded row is fully answered by the branch,
        // digest of the game after the row is applied under that branch)
        let mut passing: Vec<(u16, bool, u64)> = Vec::new();
        // The prompt each qualifying branch leaves parked BEFORE the row is
        // applied: (kind, prompt text, offered ids).
        let mut shown: Vec<(String, String, Vec<u16>)> = Vec::new();
        for entry in entries {
            let mut probe = game.clone();
            probe.decode_action(entry.action_id, actor);
            // A chosen "you may" trigger parks its own accept/decline first; the
            // row is the acceptance AND the pick (see the implied-accept fold).
            if is_trigger_order && Self::is_outer_optional_gate(&probe) {
                probe.decode_action(crate::action::space::REPLACEMENT_ACCEPT, actor);
            }
            let qualifies = if is_cancel {
                // The decline is legal when the follow-up prompt is optional, or
                // the effect fizzled (nothing mandatory left to pick).
                match probe.pending_selection.as_ref() {
                    None => true,
                    Some(p) => {
                        p.is_optional
                            || p.selecting_player != actor
                            || matches!(p.kind, SelectionKind::OrderedPermutation { .. })
                    }
                }
            } else {
                // A card pick is answered by a PICK prompt. An ordering prompt
                // (what an unfillable branch leaves behind) also "accepts" a card
                // id positionally, which proves nothing.
                !matches!(
                    probe.pending_selection.as_ref().map(|p| &p.kind),
                    Some(SelectionKind::OrderedPermutation { .. })
                ) && matches!(super::selection_resolve::resolve_next(&probe, payload, 0), Ok(Some(_)))
            };
            if qualifies {
                let fizzled = is_cancel
                    && probe.pending_selection.as_ref().is_none_or(|p| !p.is_optional);
                // Apply the row under this branch and fingerprint the result, so
                // branches that lead to the SAME game can be told apart from
                // branches that do not (see the selection below).
                let mut applied = probe.clone();
                if !is_cancel {
                    if let Ok(Some(id)) = super::selection_resolve::resolve_next(&applied, payload, 0) {
                        applied.decode_action(id, actor);
                    }
                }
                passing.push((entry.action_id, fizzled, branch_digest(&applied)));
                shown.push(match probe.pending_selection.as_ref() {
                    Some(p) => (format!("{:?}", p.kind), p.prompt.clone(), p.valid_action_ids.clone()),
                    None => (String::new(), String::new(), Vec::new()),
                });
            }
        }
        // Several branches that all lead to the SAME game once the row is applied
        // (e.g. a second clause whose cost cannot be paid fizzles and leaves the
        // first to run) make the order unobservable: any of them is faithful.
        // DCGO only lists effects it can activate right now, so it never asked.
        if passing.len() > 1 && passing.iter().all(|(_, _, d)| *d == passing[0].2) {
            return Some((passing[0].0, passing[0].1));
        }
        // Branches that all park the SAME prompt for the row (an effect with no
        // legal target fizzles silently and leaves the other trigger's prompt)
        // are likewise unobservable as an order: had the skipped trigger done
        // anything, its own prompt would be the one showing. DCGO lists only the
        // effects it can activate, so it never asked.
        if !is_cancel && passing.len() > 1 && !shown.is_empty() && shown.iter().all(|x| *x == shown[0]) {
            return Some((passing[0].0, passing[0].1));
        }
        let passing: Vec<(u16, bool)> = passing.iter().map(|(a, f, _)| (*a, *f)).collect();
        match passing.as_slice() {
            [only] => Some(*only),
            [first, second] if !is_cancel => {
                // DCGO's own bool AFTER the pick names the branch - only for a
                // card whose DCGO script asks exactly that bool right after the
                // pick (`dcgo_pick_then_branch_bool`), read from the actor's
                // very next row, and mapped onto the engine branch by its
                // label index (never by its position among the passing ones).
                let card = game.card_data_for_handle(pending.source_card)?.card_id.clone();
                let (true_label, false_label) = dcgo_pick_then_branch_bool(&card)?;
                let n = self.source.steps().get(row_at + 1)?.selection.as_ref()?;
                if n.actor != actor || n.prompt != "generic_bool" || n.unstamped {
                    return None;
                }
                let want = crate::action::space::SEL_REVEAL_START
                    + if n.bool_value? { true_label } else { false_label };
                [first, second].into_iter().find(|(id, _)| *id == want).copied()
            }
            _ => None,
        }
    }

    /// Answer a mandatory ONE-candidate field pick that DCGO never asked, ahead
    /// of a recorded ACTION row.
    ///
    /// DCGO skips a pick with a single candidate (EX12-026's "1 of their Digimon
    /// with 1 or fewer digivolution cards can't attack or block" against one
    /// eligible Digimon), so the recording has no row for it, while the engine
    /// parks the pick. `apply_selection_step` already answers such a prompt when
    /// a SELECTION row arrives; this covers the case where the next row is an
    /// action. Applied only when that action is not currently legal: the field-
    /// pick ids (100..) overlap the attack ids, so a legal action means the
    /// prompt is not what the row is about. The single candidate leaves no
    /// choice, so legality proves the answer.
    fn answer_dcgo_silent_lone_target_pick(&mut self, game: &mut Game, at: usize) {
        for _ in 0..4 {
            let Some(step) = self.source.steps().get(at) else {
                return;
            };
            if step.selection.is_some() || step.source == "play_card_extra" {
                return;
            }
            let Some((actor, action)) = self.dcgo_silent_lone_field_pick(game) else {
                return;
            };
            // Would the recorded action be legal as it stands? Then the row is
            // an answer to something else, not a prompt DCGO skipped.
            let mask = crate::action::build_action_mask(game, step.actor);
            let remapped = remap_board_slots(
                game,
                step.action_id,
                step.actor,
                step.board_p0.as_ref(),
                step.board_p1.as_ref(),
            );
            let legal = |id: u16| mask.get(id as usize).copied().unwrap_or(0.0) > 0.5;
            if legal(step.action_id) || legal(remapped) {
                return;
            }
            game.decode_action(action, actor);
            // Answering the pick can park the NEXT prompt DCGO skips - the one-card deck order that ends the same
            // effect (EX13-026's "Return the rest to the bottom of the deck" with one card left; DCGO places a lone
            // card without a prompt). `answer_dcgo_silent_prompts` ran the ordering fold BEFORE this pick, so
            // without answering it here the action row would meet the effect controller's order prompt (an actor
            // divergence when the row is the other seat's - raw-20261116T164448Z#g7 file step 45). Exactly what
            // the MultipleSkills fold does after each answer.
            self.answer_forced_single_ordering_prompt(game);
        }
    }

    /// The parked prompt, when it is a field pick DCGO skips because it has a
    /// single candidate (`MatchConditionPermanentCount(..) > 1` before asking):
    /// mandatory, ONE valid id, a battle-area kind, under the DCGO-oracle
    /// policy of a source that omits DCGO's silent prompts. Returns
    /// `(selecting player, the lone id)`. The callers add what the next
    /// recorded row must prove before the answer is applied.
    fn dcgo_silent_lone_field_pick(&self, game: &Game) -> Option<(PlayerId, u16)> {
        use crate::selection::SelectionKind;
        if self.policy != StepPolicy::CheckThenApply || !self.source.omits_dcgo_silent_prompts() {
            return None;
        }
        let pending = game.pending_selection.as_ref()?;
        (!pending.is_optional
            && pending.valid_action_ids.len() == 1
            && matches!(
                pending.kind,
                SelectionKind::OppField | SelectionKind::OwnField | SelectionKind::AnyField
            ))
        .then(|| (pending.selecting_player, pending.valid_action_ids[0]))
    }

    /// Stop at the engine's "which card goes to the deck bottom/top next"
    /// ordering prompt when the recording carries no row for it.
    ///
    /// Cards returned to a deck TOGETHER (BT20-045's "return all of your
    /// opponent's highest-DP Digimon to the bottom of the deck") have an order
    /// the owner of the effect chooses (CR 4.3 3-1-3-4); the engine exposes it
    /// (an `AnyField` pick over the remaining members, from
    /// `replacement_departure_batch`). DCGO asks nothing:
    /// `DeckBottomBounceClass.DeckBounce` places the top cards in the order of
    /// the target list the card script built (CardController.cs:2650-2668), and
    /// `AddLibraryBottomCards` appends them in that order (CardObjectController.cs
    /// :954-1020, `LibraryCards.Add`; index 0 is the top), so the list's LAST
    /// card is the very bottom. That order is deterministic but card-specific:
    /// BT20-045 builds it from `GetBattleAreaDigimons()` (ascending DCGO
    /// FieldPermanents index, Player.cs:621-640), other cards from their own
    /// selections. This helper used to take the lowest engine board slot as the
    /// bottommost - the REVERSE of BT20-045's order, and unverified for every
    /// other card - so it now returns a halting reason instead (worded as an
    /// ambiguity, so a trigger-order lookahead reaching it ends that branch's
    /// evidence rather than refuting it). Only a playback row stamped with an
    /// `engine_action` the prompt accepts answers it. Never fired in the
    /// committed corpus or the 2026-11-13 PvP games when it was replaced.
    fn dcgo_unrecorded_stack_order(&self, game: &Game, at: usize) -> Option<String> {
        use crate::selection::SelectionKind;
        if self.policy != StepPolicy::CheckThenApply {
            return None;
        }
        let step = self.source.steps().get(at)?;
        let pending = game.pending_selection.as_ref()?;
        if pending.is_optional
            || pending.kind != SelectionKind::AnyField
            || pending.valid_action_ids.len() < 2
            || !(pending.prompt == "Choose the bottommost remaining card"
                || pending.prompt == "Choose the topmost remaining card")
        {
            return None;
        }
        // DCGO never asks this order, so no DCGO row can answer it: a row
        // that merely RESOLVES here (an int read by position, a target pick of
        // another effect) is not its answer. Only a playback row stamped with an
        // engine action this prompt accepts, of the seat that owns it, is.
        if let Some(payload) = step.selection.as_ref() {
            if payload.actor == pending.selecting_player
                && payload.engine_action.is_some_and(|a| pending.valid_action_ids.contains(&a))
            {
                return None;
            }
        }
        let src = game
            .card_data_for_handle(pending.source_card)
            .map(|d| d.card_id.clone())
            .unwrap_or_default();
        Some(format!(
            "row {at}: P{}'s '{}' ({src}, {} cards) is an order DCGO decided without asking and without a row \
             (DeckBottomBounceClass places them in its card script's target-list order, CardController.cs:2650-2668): \
             which card DCGO put deepest is ambiguous from the recording, and the replay does not guess it",
            pending.selecting_player,
            pending.prompt,
            pending.valid_action_ids.len()
        ))
    }

    /// After the declared security attack, take the <Raid> trigger and switch the
    /// attack to the opponent slot DCGO recorded. Every step is taken only from
    /// the prompts the engine itself presents; if the switch is not offered the
    /// state is left as it is and the following recorded rows diverge honestly.
    fn apply_raid_switch(&mut self, game: &mut Game, target: u16) {
        use crate::selection::SelectionKind;
        for _ in 0..8 {
            let Some(p) = game.pending_selection.as_ref() else {
                return;
            };
            let who = p.selecting_player;
            if p.prompt == "Raid - switch attack target" {
                let want = crate::action::space::encode_attack(0, target);
                if p.valid_action_ids.contains(&want) {
                    game.decode_action(want, who);
                }
                return;
            }
            if matches!(p.kind, SelectionKind::TriggerOrder) {
                let raid = p
                    .effect_choices
                    .iter()
                    .flatten()
                    .find(|e| matches!(e.keyword, Some(crate::enums::Keyword::Raid)))
                    .map(|e| e.action_id);
                match raid {
                    Some(id) => {
                        game.decode_action(id, who);
                        continue;
                    }
                    None => return,
                }
            }
            if Self::is_outer_optional_gate(game) && p.prompt.contains("Raid") {
                game.decode_action(crate::action::space::REPLACEMENT_ACCEPT, who);
                continue;
            }
            return;
        }
    }

    /// Answer a parked digivolution COST choice that DCGO never asked, taking the
    /// ordinary (non-burst) route.
    ///
    /// DCGO records a Burst Digivolve as the digivolve action followed by a
    /// `play_card_extra` row naming the returned Tamer. When a Tamer is on the
    /// field the engine offers the burst route as a second cost option; if the
    /// row after the digivolve is NOT that `play_card_extra`, DCGO paid the
    /// printed cost (its `action_detail.cost_paid` agrees), so the engine's
    /// non-burst route is the faithful answer. Folded only when the engine's own
    /// route list has exactly one non-burst route and at least one burst route,
    /// and only against an action row (a recorded selection row would carry the
    /// choice itself). The exact inverse of `apply_burst_tamer_extra`.
    fn answer_dcgo_ordinary_cost_route(&mut self, game: &mut Game, at: usize) {
        if self.policy != StepPolicy::CheckThenApply {
            return;
        }
        let Some(step) = self.source.steps().get(at) else {
            return;
        };
        if step.source == "play_card_extra" {
            return;
        }
        let Some(pending) = game.pending_selection.as_ref() else {
            return;
        };
        // A recorded selection row that resolves against the cost prompt is
        // that prompt's own answer; only a row that cannot belongs to a LATER
        // prompt, which proves the cost was settled without being asked.
        if let Some(payload) = step.selection.as_ref() {
            if super::selection_resolve::resolve_next(game, payload, 0).is_ok() {
                return;
            }
            // A recorded Burst Digivolve Tamer pick (`mechanic: "burst"`) says the
            // route was the burst: never the ordinary one, even when the pick
            // names no route (that is a divergence at the row).
            if payload.mechanic.as_deref() == Some("burst") {
                return;
            }
        }
        // An EFFECT digivolve (DCGO `DigivolveIntoHandOrTrashCard`) never asks which route: DCGO pays the
        // cheapest (`PayingCost` takes the minimum over the matching conditions). When our engine parks the
        // choice and the next row cannot answer it, DCGO took the cheapest printed route.
        let effect_routes = game.pending_selection_resume.as_ref().and_then(|stack| {
            stack.frames.iter().rev().find_map(|frame| match frame {
                crate::resume::ResumeFrame::EffectDigivolveCostChoice(state) => Some(state.printed_costs.clone()),
                _ => None,
            })
        });
        if let Some(routes) = effect_routes {
            // ... unless the row IS DCGO's answer to that question: a count row
            // of the choosing player ("Which digivolution cost do you pay?" is a
            // `SelectCountEffect`, CardController.cs:720-736). A count that
            // resolves nothing is a divergence, not a skipped prompt.
            if step.selection.as_ref().is_some_and(|row| {
                row.prompt == "SelectCountEffect" && row.count.is_some() && row.actor == pending.selecting_player
            }) {
                return;
            }
            if routes.len() == pending.valid_action_ids.len() && routes.len() > 1 {
                if let Some((i, _)) = routes.iter().enumerate().min_by_key(|(_, r)| r.memory_cost) {
                    let (actor, action) = (pending.selecting_player, pending.valid_action_ids[i]);
                    game.decode_action(action, actor);
                }
            }
            return;
        }
        let Some(routes) = game.pending_selection_resume.as_ref().and_then(|stack| {
            stack.frames.iter().rev().find_map(|frame| match frame {
                crate::resume::ResumeFrame::DigivolveCostChoice(state) => Some(state.routes.clone()),
                _ => None,
            })
        }) else {
            return;
        };
        if routes.len() != pending.valid_action_ids.len()
            || !routes.iter().any(|route| route.burst.is_some())
        {
            return;
        }
        let ordinary: Vec<usize> = routes
            .iter()
            .enumerate()
            .filter(|(_, route)| route.burst.is_none())
            .map(|(i, _)| i)
            .collect();
        let [only] = ordinary.as_slice() else {
            return;
        };
        let (actor, action) = (pending.selecting_player, pending.valid_action_ids[*only]);
        game.decode_action(action, actor);
    }

    /// Apply a DCGO `play_card_extra` row that names a Burst Digivolve's
    /// returned Tamer (`ActionEncoder.DecomposePlayCardExtras`, an
    /// `EncodeSourceSelect(burstFrame, 0)` id).
    ///
    /// Our engine presents the Tamer as one of the "Choose which digivolution
    /// cost to pay" options (one `Burst Digivolve` option per eligible Tamer),
    /// or - when the burst is the only route - resolves straight through with
    /// no prompt at all. So: with a cost prompt parked, answer it with the burst
    /// option; with none parked, the row is already satisfied.
    ///
    /// DCGO emits the Tamer's RAW frame id here (`ActionEncoder.cs:356-359`,
    /// no compact-slot conversion), which cannot be mapped to an engine slot.
    /// It is therefore only used when exactly one burst option is on offer; two
    /// or more eligible Tamers is reported as an honest divergence rather than
    /// guessed.
    fn apply_burst_tamer_extra(
        &mut self,
        game: &mut Game,
        actor: PlayerId,
        action_id: u16,
        phase: String,
    ) -> ReplayStepResult {
        let phase_before = game.current_phase.py_name().to_string();
        let memory_before = game.memory;
        let turn_number = game.turn_count;

        let mut chosen: Option<u16> = None;
        let mut failure: Option<String> = None;
        if let Some(pending) = game.pending_selection.as_ref() {
            let routes = game.pending_selection_resume.as_ref().and_then(|stack| {
                stack.frames.iter().rev().find_map(|frame| match frame {
                    crate::resume::ResumeFrame::DigivolveCostChoice(state) => {
                        Some(state.routes.clone())
                    }
                    _ => None,
                })
            });
            match routes {
                Some(routes) => {
                    let burst: Vec<usize> = routes
                        .iter()
                        .enumerate()
                        .filter(|(_, route)| route.burst.is_some())
                        .map(|(i, _)| i)
                        .collect();
                    match burst.as_slice() {
                        [one] => chosen = pending.valid_action_ids.get(*one).copied(),
                        [] => failure = Some("recorded Burst Digivolve Tamer, but the engine offers no burst route".to_string()),
                        many => {
                            let (field, _) = crate::action::space::decode_source_select(action_id);
                            failure = Some(format!(
                                "recorded Burst Digivolve Tamer at raw DCGO frame {field}, but the engine offers {} eligible Tamers and the raw frame cannot name one",
                                many.len()
                            ));
                        }
                    }
                }
                None => {
                    failure = Some(
                        "recorded Burst Digivolve Tamer, but the parked prompt is not the digivolution cost choice"
                            .to_string(),
                    );
                }
            }
        }
        if let Some(reason) = failure {
            self.divergences.push(Divergence {
                step: self.cursor,
                action_id,
                actor,
                phase,
                kind: DivergenceKind::SelectionResolution { reason },
                board: describe_board(game),
            });
            self.paused = true;
            return self.no_progress_result(game);
        }
        if let Some(id) = chosen {
            game.decode_action(id, actor);
        }
        self.cursor += 1;
        ReplayStepResult {
            step_number: self.cursor,
            player_id: actor,
            action_id: chosen.unwrap_or(0),
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
    /// Once the last row is applied, the prompts DCGO never offered and whose
    /// answer changes nothing are answered (`answer_dcgo_trailing_gates`).
    pub(crate) fn step(&mut self, game: &mut Game) -> ReplayStepResult {
        // An order of two or more effects of the row's actor, answered by a
        // `MultipleSkills` row: remember which effect the row chose.
        let row_at = self.cursor;
        let order_before = game
            .pending_selection
            .as_ref()
            .filter(|p| matches!(p.kind, crate::selection::SelectionKind::TriggerOrder) && p.valid_action_ids.len() >= 2)
            .map(|p| (p.selecting_player, order_entry_sources(game)));
        let is_order_row = self
            .source
            .steps()
            .get(row_at as usize)
            .and_then(|s| s.selection.as_ref())
            .is_some_and(|r| r.prompt == "MultipleSkills");
        let zones_before = self.row_names_dcgo_cards(row_at).then(|| card_zones(game));
        let mut result = self.step_row(game);
        if let Some(before) = zones_before.filter(|_| self.cursor > row_at) {
            self.record_dcgo_card_indexes(&before, game, row_at..self.cursor);
        }
        if self.cursor != row_at {
            self.order_answer = match order_before {
                Some((who, entries)) if is_order_row && self.cursor == row_at + 1 && who == result.player_id => entries
                    .iter()
                    .find(|(id, _)| *id == result.action_id)
                    .map(|(_, src)| (row_at, *src)),
                // A run of the actor's unstamped "use?" answers keeps it: they
                // belong to the effect it chose.
                _ if self.order_answer.is_some()
                    && self
                        .source
                        .steps()
                        .get(row_at as usize)
                        .and_then(|s| s.selection.as_ref())
                        .is_some_and(|r| r.unstamped && r.prompt == "generic_bool") =>
                {
                    self.order_answer
                }
                _ => None,
            };
        }
        if self.is_complete() && !self.paused && !game.game_over && self.answer_dcgo_trailing_gates(game) {
            result.phase_after = game.current_phase.py_name().to_string();
            result.memory_after = game.memory;
            result.is_game_over = game.game_over;
            result.winner_id = game.winner;
        }
        result
    }

    /// After the LAST recorded row: decline each parked optional-activation
    /// gate ("you may" accept, `is_outer_optional_gate`) whose answer is
    /// unobservable - accepting and declining lead to the same game
    /// (`branch_digest`) with nothing parked. DCGO activates an optional effect
    /// only through its "use?" prompt, which the recorder logs as an unstamped
    /// `OptionalSkill.SetUseOptional` answer for either seat; the recording
    /// has none for it, and DCGO drops an effect whose `CanActivate` fails
    /// without asking (MultipleSkills.cs:217-221). The decline is therefore
    /// DCGO's "not activated", and since the accept would change nothing the
    /// game is the same either way. Virus Busters (EX12-069)'s [Security]
    /// play with no same-level [VB] card in hand (EX12_069.cs:43-50):
    /// raw-20261113T025510Z#g3, whose recording ends on the attack that wins.
    /// A gate whose accept opens a further prompt, or changes the game, is
    /// left parked (the winner check then fails honestly). DCGO recordings
    /// only. Returns whether anything was answered.
    fn answer_dcgo_trailing_gates(&mut self, game: &mut Game) -> bool {
        use crate::action::space::{PASS, REPLACEMENT_ACCEPT};
        if !self.source.omits_dcgo_silent_prompts() || self.policy != StepPolicy::CheckThenApply {
            return false;
        }
        let mut answered = false;
        for _ in 0..8 {
            self.answer_forced_single_ordering_prompt(game);
            if game.game_over || !Self::is_outer_optional_gate(game) {
                break;
            }
            let Some(p) = game.pending_selection.as_ref() else {
                break;
            };
            let (who, prompt) = (p.selecting_player, p.prompt.clone());
            let card = game.card_data_for_handle(p.source_card).map(|d| d.card_id.clone()).unwrap_or_default();
            let mut accepted = game.clone();
            accepted.decode_action(REPLACEMENT_ACCEPT, who);
            let mut declined = game.clone();
            declined.decode_action(PASS, who);
            // Only the gate's own effect is compared: a prompt parked after
            // either answer (the accept opening a pick, or the next trigger)
            // stops the fold, so the two games are compared settled.
            if accepted.pending_selection.is_some()
                || declined.pending_selection.is_some()
                || branch_digest(&accepted) != branch_digest(&declined)
            {
                break;
            }
            *game = declined;
            answered = true;
            self.notes.push(format!(
                "after the last row: P{who}'s '{prompt}' ({card}) declined - DCGO recorded no \"use?\" answer for it, \
                 and accepting would change nothing"
            ));
        }
        answered
    }

    fn step_row(&mut self, game: &mut Game) -> ReplayStepResult {
        if self.is_complete() || self.paused {
            return self.no_progress_result(game);
        }
        // A refire record the engine left for the previous row is stamped with
        // THIS row (`refire_duplicate_effect_pick_fold` reads the stamp).
        if let Some(rec) = game.last_collapsed_refire.as_mut() {
            rec.first_seen_row.get_or_insert(self.cursor);
        }
        if !self.source.steps()[self.cursor as usize]
            .selection
            .as_ref()
            .is_some_and(|r| r.unstamped && r.prompt == "generic_bool")
        {
            self.unstamped_window_start = game.events.len();
        }
        // The decision in front of a forced count: what the engine resolves
        // from here on is the evidence the count is judged by.
        if self
            .source
            .steps()
            .get(self.cursor as usize + 1)
            .and_then(|s| s.selection.as_ref())
            .is_some_and(is_forced_count_row)
        {
            self.count_evidence = Some(CountEvidenceBase::capture(self.cursor, game));
        }

        if let Some(reason) = self.answer_dcgo_silent_prompts(game, self.cursor as usize) {
            let (actor, phase) = self
                .source
                .steps()
                .get(self.cursor as usize)
                .map(|s| (s.actor, s.phase.clone()))
                .unwrap_or_default();
            self.divergences.push(Divergence {
                step: self.cursor,
                action_id: 0,
                actor,
                phase,
                kind: DivergenceKind::SelectionResolution { reason },
                board: describe_board(game),
            });
            self.paused = true;
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

        // A legacy raw ID must never acquire a different meaning simply because
        // an owned physical-card page now occupies that ID range. Stop before
        // remapping, mask checks, callbacks or mutation, even under Trust.
        if legacy_owned_card_alias(game, self.source.action_schema_version()) {
            self.divergences.push(Divergence {
                step: self.cursor, action_id, actor, phase: phase_rec.clone(),
                kind: DivergenceKind::SelectionResolution {
                    reason: format!("action_schema_version {} cannot answer an owned card page; version {} is required",
                        self.source.action_schema_version(), crate::action::space::SCHEMA_VERSION),
                }, board: describe_board(game),
            });
            self.paused = true;
            return self.no_progress_result(game);
        }

        // DCGO records a Burst Digivolve (rule 8-3) as the digivolve action
        // followed by a `play_card_extra` row naming the Tamer that was
        // returned to hand. The engine models that choice as one of its cost
        // options, so this row is not a standalone engine action.
        if self.source.steps()[cur].source == "play_card_extra"
            && (crate::action::space::SOURCE_SELECT_START..crate::action::space::SOURCE_SELECT_END)
                .contains(&action_id)
        {
            return self.apply_burst_tamer_extra(game, actor, action_id, phase_rec);
        }

        // DCGO indexes the battle area by on-screen frame order; the engine
        // indexes it by play order. Rewrite the recorded action's board
        // operands into the engine's index space before anything inspects it.
        // DCGO's recorder encodes a DNA digivolve as the plain digivolve id and
        // keeps the partner pair only in the correlated `action_detail`. The
        // engine's own DNA action is `DNA_DIGIVOLVE_START + hand slot`, followed by
        // two material picks answered below from that pair.
        let dna_materials = self.source.steps()[cur].dna_materials.clone();
        let action_id = if dna_materials.is_some()
            && (crate::action::space::DIGIVOLVE_START..crate::action::space::DIGIVOLVE_END)
                .contains(&action_id)
        {
            let (hand, _) = crate::action::space::decode_digivolve(action_id);
            crate::action::space::DNA_DIGIVOLVE_START + hand
        } else {
            action_id
        };

        let raw_action_id = action_id;
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

        // An own-board operand among copies of one card: DCGO's own card
        // identity (`operand_card_index`) names the permanent, else the
        // occurrence-order pairing above stands (and says so when it is a guess).
        let (action_id, operand_named) = self.named_operand_action(game, actor, cur as u32, raw_action_id, action_id);
        let action_id = if operand_named { action_id } else { resolve_duplicate_attacker(game, actor, action_id) };

        // The frame-order remap trusts the row's board snapshot, which lists
        // permanents in DCGO's own order (Digimon before Tamers/Options) while a
        // recorded operand can already be in play order. When the remapped id is
        // not legal but the recorded one is, the recorded operand was right -
        // the engine's own legality decides, so a genuine mismatch (neither
        // legal) still surfaces unchanged.
        // The recorder (before the fix in `EncodeActivateCard`) wrote an
        // activated HAND card as `HAND_EFFECT_START + <deck-order card index>`
        // rather than its current hand slot, so the id lands on the wrong slot or
        // outside the hand. The row's own `card_id` names the card: if the id is
        // not legal, re-resolve it to the legal hand-effect bit whose hand card
        // has that id (lowest slot first), else leave it to surface.
        let action_id = {
            use crate::action::space::{HAND_EFFECT_END, HAND_EFFECT_START};
            let named = self.source.steps()[cur].card_id.clone();
            match named {
                Some(want) if (HAND_EFFECT_START..HAND_EFFECT_END).contains(&raw_action_id) => {
                    let mask = crate::action::build_action_mask(game, actor);
                    let legal = |id: u16| mask.get(id as usize).copied().unwrap_or(0.0) > 0.5;
                    let hand_ids: Vec<String> = game
                        .player(actor)
                        .hand
                        .iter()
                        .map(|c| c.card_id(&game.card_data).to_string())
                        .collect();
                    let at_slot = |id: u16| hand_ids.get((id - HAND_EFFECT_START) as usize);
                    if legal(raw_action_id) && at_slot(raw_action_id) == Some(&want) {
                        raw_action_id
                    } else {
                        (HAND_EFFECT_START..HAND_EFFECT_END)
                            .find(|id| legal(*id) && at_slot(*id) == Some(&want))
                            .unwrap_or(action_id)
                    }
                }
                _ => action_id,
            }
        };

        // The recorder encodes every activated permanent skill as the [Main]
        // sub-slot, but a <Link> declaration has its own sub-slot in the engine's
        // action space (`FIELD_EFFECT_SLOT_FOR_LINK`). When the [Main] id is not
        // legal and the same permanent's link id is, the recorded activation was
        // the link declaration.
        let action_id = {
            use crate::action::space::{
                decode_field_effect, FIELD_EFFECT_END, FIELD_EFFECT_SLOT_FOR_LINK,
                FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
            };
            if (FIELD_EFFECT_START..FIELD_EFFECT_END).contains(&action_id) {
                let (perm, effect) = decode_field_effect(action_id);
                if effect == FIELD_EFFECT_SLOT_FOR_MAIN {
                    let mask = crate::action::build_action_mask(game, actor);
                    let legal = |id: u16| mask.get(id as usize).copied().unwrap_or(0.0) > 0.5;
                    let link_id = FIELD_EFFECT_START
                        + perm * crate::action::space::EFFECTS_PER_PERMANENT
                        + FIELD_EFFECT_SLOT_FOR_LINK;
                    if !legal(action_id) && legal(link_id) {
                        link_id
                    } else {
                        action_id
                    }
                } else {
                    action_id
                }
            } else {
                action_id
            }
        };

        // A recorded play / digivolve names a HAND SLOT, which is a position in
        // DCGO's hand ordering. A hand is an unordered set (CR 4.3), and when a
        // hand-pick selection names a card with several identical copies, DCGO
        // removes one at random (`SelectHandEffect`'s AI path `GetRandom`) while
        // the engine removes the first, so the two orders drift apart and every
        // later slot-indexed hand action is misaddressed. The row's own `card_id`
        // names the card: when the recorded slot does not hold it, use the legal
        // slot that does (lowest first). Only when the recorded slot is wrong AND
        // a legal slot holding the named card exists; otherwise the id is left to
        // surface as it is.
        let action_id = {
            use crate::action::space::{
                decode_digivolve, encode_digivolve, DIGIVOLVE_END, DIGIVOLVE_START,
                DNA_DIGIVOLVE_END, DNA_DIGIVOLVE_START, PLAY_HAND_END, PLAY_HAND_START,
            };
            match self.source.steps()[cur].card_id.clone() {
                Some(want)
                    if (PLAY_HAND_START..PLAY_HAND_END).contains(&action_id)
                        || (DIGIVOLVE_START..DIGIVOLVE_END).contains(&action_id)
                        || (DNA_DIGIVOLVE_START..DNA_DIGIVOLVE_END).contains(&action_id) =>
                {
                    let mask = crate::action::build_action_mask(game, actor);
                    let legal = |id: u16| mask.get(id as usize).copied().unwrap_or(0.0) > 0.5;
                    let hand_ids: Vec<String> = game
                        .player(actor)
                        .hand
                        .iter()
                        .map(|c| c.card_id(&game.card_data).to_string())
                        .collect();
                    // (recorded hand slot, rebuild the id for another slot)
                    let (slot, rebuild): (u16, Box<dyn Fn(u16) -> u16>) =
                        if (PLAY_HAND_START..PLAY_HAND_END).contains(&action_id) {
                            (action_id - PLAY_HAND_START, Box::new(|h| PLAY_HAND_START + h))
                        } else if (DIGIVOLVE_START..DIGIVOLVE_END).contains(&action_id) {
                            let (hand, field) = decode_digivolve(action_id);
                            (hand, Box::new(move |h| encode_digivolve(h, field)))
                        } else {
                            (
                                action_id - DNA_DIGIVOLVE_START,
                                Box::new(|h| DNA_DIGIVOLVE_START + h),
                            )
                        };
                    let holds_named = hand_ids.get(slot as usize) == Some(&want);
                    if holds_named {
                        action_id
                    } else {
                        (0..hand_ids.len() as u16)
                            .filter(|h| hand_ids[*h as usize] == want)
                            .map(|h| rebuild(h))
                            .find(|id| legal(*id))
                            .unwrap_or(action_id)
                    }
                }
                _ => action_id,
            }
        };

        // A copy named by DCGO's own card identity is never swapped for the
        // recorded position's other copy when the named one's action is illegal:
        // that is a divergence to report, not a guess to make.
        let action_id = if action_id != raw_action_id && !operand_named {
            let mask = crate::action::build_action_mask(game, actor);
            let legal = |id: u16| mask.get(id as usize).copied().unwrap_or(0.0) > 0.5;
            if !legal(action_id) && legal(raw_action_id) {
                if (crate::action::space::DIGIVOLVE_START..crate::action::space::DIGIVOLVE_END)
                    .contains(&raw_action_id)
                {
                    if let Some(card) = self.source.steps()[cur].card_id.clone() {
                        self.digivolve_operand_fallbacks.push(card);
                    }
                }
                raw_action_id
            } else {
                action_id
            }
        } else {
            action_id
        };

        // A recorded attack on an UNSUSPENDED opponent Digimon by a <Raid>
        // attacker. DCGO's action space names the switched target directly; the
        // engine declares the attack on security and then offers the Raid switch
        // (16-23-2, a trigger). Rewritten only when the engine's own mask has the
        // security attack and not the recorded one, and the attacker really has
        // <Raid>.
        self.raid_retarget = None;
        let action_id = match raid_declared_target(game, actor, action_id) {
            Some((security_attack, target)) => {
                self.raid_retarget = Some(target);
                security_attack
            }
            None => action_id,
        };

        if self.policy == StepPolicy::CheckThenApply {
            // DCGO ends a turn WITHOUT a Pass row when the turn player has nothing
            // to do (`TurnStateMachine.MainPhase`: `if (!CanSelect()) yield return
            // EndTurnProcess()`), so the recording jumps straight to the other
            // seat. When the engine still waits on the other seat's Main-phase
            // decision, that decision has exactly one legal action (PASS), and the
            // next recorded row belongs to the opponent, the silent end-turn is
            // what happened: apply it so the row meets the decision it was
            // recorded against. With any other legal action the engine's choice
            // is real, so nothing is skipped and the mismatch still surfaces.
            if game.pending_selection.is_none()
                && game.current_phase == GamePhase::Main
                && current_decision_player(game) != actor
            {
                let who = current_decision_player(game);
                let mask = crate::action::build_action_mask(game, who);
                let legal: Vec<usize> = mask
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| **v > 0.5)
                    .map(|(i, _)| i)
                    .collect();
                if legal == [crate::action::space::PASS as usize] {
                    game.decode_action(crate::action::space::PASS, who);
                }
            }
            // Differential pre-check: does the engine agree the recorded actor
            // is on decision, and is the recorded action legal here?
            // A surrender is available to either seat at any moment (DCGO's Surrender button is not tied to whose
            // decision it is), so CONCEDE_GAME is exempt from the on-decision-player check.
            let expected = if action_id == crate::action::space::CONCEDE_GAME {
                actor
            } else {
                current_decision_player(game)
            };
            if expected != actor && self.source.steps()[cur].inferred {
                // A row the converter had to guess: the engine is already waiting on another seat (e.g. the turn
                // ended by the memory gauge crossing), so the guess was unnecessary. Skip it, and count it.
                self.inferred_skipped.push(self.cursor);
                self.cursor += 1;
                return ReplayStepResult {
                    step_number: self.cursor,
                    player_id: actor,
                    action_id,
                    phase_before: game.current_phase.py_name().to_string(),
                    phase_after: game.current_phase.py_name().to_string(),
                    memory_before: game.memory,
                    memory_after: game.memory,
                    turn_number: game.turn_count,
                    is_game_over: game.game_over,
                    winner_id: game.winner,
                    divergences: Vec::new(),
                };
            }
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
        if let Some(target) = self.raid_retarget.take() {
            self.apply_raid_switch(game, target);
        }
        if let Some(materials) = dna_materials.as_ref().filter(|_| {
            (crate::action::space::DNA_DIGIVOLVE_START..crate::action::space::DNA_DIGIVOLVE_END)
                .contains(&action_id)
        }) {
            for want in materials {
                let Some(p) = game.pending_selection.as_ref() else {
                    break;
                };
                if p.kind != crate::selection::SelectionKind::Material
                    || !crate::selection::is_dna_material_prompt(&p.prompt)
                {
                    break;
                }
                let board = engine_board_ids(game, p.selecting_player);
                let pick = p
                    .valid_action_ids
                    .iter()
                    .copied()
                    .find(|id| board.get(*id as usize) == Some(want));
                let who = p.selecting_player;
                match pick {
                    Some(id) => game.decode_action(id, who),
                    None => break,
                };
            }
        }
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
                let replayed = game.verification_digest_with_version(self.source.verification_digest_version());
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
            self.digivolve_operand_fallbacks.clear();
            self.notes.clear();
            self.paused = false;
            self.pick_zone_hint = None;
            self.zone_menu_row = None;
            self.unstamped_window_start = 0;
            self.dcgo_cards.clear();
            self.count_evidence = None;
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
        // Converter-inferred rows the engine had already moved past count as skipped too: a PASS that quietly
        // dropped a guessed decision must still show it.
        self.driver.skipped_incomplete_plays() + self.driver.inferred_skipped().len() as u32
    }

    /// Card ids of digivolve actions that only replayed because the recorded
    /// operand was legal where the board-remapped one was not.
    pub fn digivolve_operand_fallbacks(&self) -> &[String] {
        self.driver.digivolve_operand_fallbacks()
    }

    /// Non-halting replay notes (see `ReplayDriver::notes`): recorded rows
    /// consumed with no engine prompt to apply them to.
    pub fn replay_notes(&self) -> &[String] {
        self.driver.notes()
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
/// If `action_id` is an attack on an UNSUSPENDED opponent Digimon that the
/// engine does not offer, while the same attacker's security attack IS offered
/// and the attacker has <Raid>, return `(security_attack_id, opponent_slot)`.
///
/// DCGO's recorder names the Raid-switched target as the attack's target; the
/// engine declares the attack on security first and switches in the trigger.
fn raid_declared_target(game: &Game, actor: PlayerId, action_id: u16) -> Option<(u16, u16)> {
    use crate::action::space::{decode_attack, encode_attack, ATTACK_END, ATTACK_START, SECURITY_TARGET};
    if !(ATTACK_START..ATTACK_END).contains(&action_id) {
        return None;
    }
    let (attacker, target) = decode_attack(action_id);
    if target == SECURITY_TARGET {
        return None;
    }
    let mask = crate::action::build_action_mask(game, actor);
    let legal = |id: u16| mask.get(id as usize).copied().unwrap_or(0.0) > 0.5;
    let security_attack = encode_attack(attacker, SECURITY_TARGET);
    if legal(action_id) || !legal(security_attack) {
        return None;
    }
    let handle = crate::permanent::PermanentHandle { player: actor, index: attacker as u8 };
    if !game.has_keyword(handle, crate::enums::Keyword::Raid) {
        return None;
    }
    let victim = game.players.get(1 - actor as usize)?.battle_area.get(target as usize)?;
    (!victim.is_suspended).then_some((security_attack, target))
}

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

/// The own-board slot an action addresses: the attacker of an attack, the
/// battle-area target of a digivolve, the host of an activated permanent
/// effect. `None` for any other action (and for the breeding-area digivolve).
fn own_board_operand(action_id: u16) -> Option<u16> {
    use crate::action::space::*;
    if (ATTACK_START..ATTACK_END).contains(&action_id) {
        Some(decode_attack(action_id).0)
    } else if (DIGIVOLVE_START..DIGIVOLVE_END).contains(&action_id) {
        Some(decode_digivolve(action_id).1).filter(|f| *f != BREEDING_TARGET)
    } else if (FIELD_EFFECT_START..FIELD_EFFECT_END).contains(&action_id) {
        Some(decode_field_effect(action_id).0)
    } else {
        None
    }
}

/// `action_id` with its own-board operand (`own_board_operand`) replaced by
/// `slot`; any other action is returned unchanged.
fn with_own_board_operand(action_id: u16, slot: u16) -> u16 {
    use crate::action::space::*;
    if (ATTACK_START..ATTACK_END).contains(&action_id) {
        encode_attack(slot, decode_attack(action_id).1)
    } else if (DIGIVOLVE_START..DIGIVOLVE_END).contains(&action_id) {
        encode_digivolve(decode_digivolve(action_id).0, slot)
    } else if (FIELD_EFFECT_START..FIELD_EFFECT_END).contains(&action_id) {
        FIELD_EFFECT_START + slot * EFFECTS_PER_PERMANENT + decode_field_effect(action_id).1
    } else {
        action_id
    }
}

/// Resolve a recorded attack whose attacker is ambiguous between duplicate
/// permanents.
///
/// A recording's board is card ids in DCGO frame order, so two copies of one
/// card (two [ST24-05]) cannot be told apart, and `build_slot_map` pairs them in
/// occurrence order. When that pairing makes the attack illegal but exactly one
/// OTHER copy of the same card can legally make the same attack on the same
/// target, that copy is the permanent DCGO meant: the engine's own legality
/// (summoning sickness, suspension) is what separates them. Anything else -
/// already legal, not an attack, no legal sibling, or more than one - is returned
/// unchanged so a genuine mismatch still surfaces as a divergence.
fn resolve_duplicate_attacker(game: &Game, actor: PlayerId, action_id: u16) -> u16 {
    use crate::action::space::{ATTACK_END, ATTACK_START, TARGETS_PER_ATTACKER};
    if !(ATTACK_START..ATTACK_END).contains(&action_id) {
        return action_id;
    }
    let mask = crate::action::build_action_mask(game, actor);
    let legal = |id: u16| mask.get(id as usize).copied().unwrap_or(0.0) > 0.5;
    if legal(action_id) {
        return action_id;
    }
    let offset = action_id - ATTACK_START;
    let (attacker, target) = (offset / TARGETS_PER_ATTACKER, offset % TARGETS_PER_ATTACKER);
    let ids = engine_board_ids(game, actor);
    let Some(card) = ids.get(attacker as usize) else {
        return action_id;
    };
    let siblings: Vec<u16> = ids
        .iter()
        .enumerate()
        .filter(|(i, c)| *i != attacker as usize && *c == card)
        .map(|(i, _)| ATTACK_START + i as u16 * TARGETS_PER_ATTACKER + target)
        .filter(|id| legal(*id))
        .collect();
    match siblings.as_slice() {
        [only] => *only,
        _ => action_id,
    }
}

/// Engine battle-area card IDs for `player`, in `battle_area` index order.
fn engine_board_ids(game: &Game, player: PlayerId) -> Vec<String> {
    let area = &game.player(player).battle_area;
    let top = |perm: &crate::permanent::Permanent| perm.card_sources.last().cloned();
    // DCGO builds every token from a bare `CEntity_Base` that never sets a
    // `CardID` (ContinuousController.cs:440-476), so its recorded boards
    // name a token "" (raw-20261113T011922Z#g3 raw seq 402: Medusamon
    // (EX11-012) deleting a Petrification Token). A token is named "" here
    // too - but only while every token on this board is the same kind: a ""
    // cannot tell kinds apart, so mixed kinds keep their ids and a recorded
    // "" maps to none of them (the operand is then left for a divergence).
    let token_ids: std::collections::BTreeSet<String> = area
        .iter()
        .filter_map(|perm| top(perm).filter(|c| c.is_token))
        .map(|c| c.card_id(&game.card_data).to_string())
        .collect();
    let blank_tokens = token_ids.len() == 1;
    area.iter()
        .map(|perm| match top(perm) {
            Some(c) if c.is_token && blank_tokens => String::new(),
            Some(c) => c.card_id(&game.card_data).to_string(),
            None => String::new(),
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
                        .map(|base| {
                            base + game.modifiers.sum(handle, crate::ModifierType::ChangeDp)
                        })
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
        out.push_str(
            "
",
        );
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
        out.push_str(&format!(
            "P{} hand:  {}
",
            pid,
            list(&p.hand)
        ));
        out.push_str(&format!(
            "P{} trash: {}
",
            pid,
            list(&p.trash)
        ));
        // Security top is the END of the vec. A battle outcome hinges entirely
        // on which card the check flips, so a divergence in the security stack
        // masquerades as a combat bug.
        out.push_str(&format!(
            "P{} sec:   {} (top = last)
",
            pid,
            list(&p.security)
        ));
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

    /// Shishimamon (EX12-046)'s inherited [End of Attack] "you may play 1 [TB]
    /// Digimon ... from your hand" parks its pick as the folded "you may"
    /// (the optional-ACTIVATION mark). A branch identical but for the mark has
    /// the same `verification_digest`, yet the replay reads the next unstamped
    /// DCGO answer differently there (`dcgo_use_optional_answer`), so the
    /// branch fingerprint must tell them apart.
    #[test]
    fn branch_digest_sees_the_optional_activation_mark() {
        use crate::debug_runner::{make_test_card_with_level, DebugRunner};
        use crate::enums::{CardColor, CardKind, EffectTiming};
        let digimon = |id: &str, level: u8, dp: i32, traits: &[&str]| {
            let mut c = make_test_card_with_level(id, id, level);
            c.card_kind = CardKind::Digimon;
            c.colors = vec![CardColor::Yellow];
            c.play_cost = 5;
            c.dp = Some(dp);
            c.traits = traits.iter().map(|t| t.to_string()).collect();
            c
        };
        let mut r = DebugRunner::builder()
            .dsl_card("EX12-046")
            .expect("EX12-046 YAML")
            .add_card(digimon("CARRIER", 6, 9000, &[]))
            .add_card(digimon("TB-LOW", 4, 5000, &["Shambala", "TB"]))
            .hand(0, &["TB-LOW"])
            .memory(3)
            .start();
        r.game.turn_count = 1;
        let carrier = r.place_stack(0, &["EX12-046", "CARRIER"]);
        r.game
            .enqueue_triggered(EffectTiming::EndOfAttack, crate::selection::TriggerSource::Permanent(carrier));
        r.game.drain_effect_queue();
        assert!(r.game.pending_is_optional_activation(), "the folded \"you may\" pick is marked");
        let mut unmarked = r.game.clone();
        unmarked.optional_activation_mark = None;
        assert!(!unmarked.pending_is_optional_activation());
        assert_eq!(
            r.game.verification_digest(),
            unmarked.verification_digest(),
            "the verification digest alone cannot tell the two apart"
        );
        assert_ne!(branch_digest(&r.game), branch_digest(&unmarked));
    }

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

    fn run(entry: u16, from: u32, reached: u32, refuted_at: Option<u32>) -> LookaheadRun {
        LookaheadRun {
            nested_refuted_at: None,
            entry,
            seen: (from + 1..=reached).map(|c| (c, entry as u64 * 1000 + c as u64)).collect(),
            reached,
            refuted_at,
        }
    }

    /// A branch that stopped (nested ambiguity, or a row that did not advance)
    /// BEFORE the row that refuted the other was never tested by that row: it
    /// must not win by default.
    #[test]
    fn lookahead_survivor_that_stopped_before_the_refuting_row_does_not_stand() {
        // from = 10: A stopped at 12 (rows 10, 11 applied); B refuted at row 15.
        let runs = [run(30, 10, 12, None), run(31, 10, 15, Some(15))];
        assert_eq!(pick_lookahead_branch(&runs), None);
        // Stopped exactly AT the refuting row (did not apply it): still no.
        let runs = [run(30, 10, 15, None), run(31, 10, 15, Some(15))];
        assert_eq!(pick_lookahead_branch(&runs), None);
    }

    /// A survivor that applied the row which refuted the other stands.
    #[test]
    fn lookahead_survivor_past_the_refuting_row_stands() {
        let runs = [run(30, 10, 18, None), run(31, 10, 13, Some(13))];
        assert_eq!(pick_lookahead_branch(&runs), Some(30));
        // ... against every refuted branch, not just one.
        let runs = [run(30, 10, 14, None), run(31, 10, 13, Some(13)), run(32, 10, 16, Some(16))];
        assert_eq!(pick_lookahead_branch(&runs), None);
    }

    /// Every branch refuted proves nothing; several survivors stand only when
    /// they meet the same game after the same row.
    #[test]
    fn lookahead_all_refuted_or_divergent_survivors_is_none() {
        assert_eq!(pick_lookahead_branch(&[run(30, 10, 11, Some(11)), run(31, 10, 12, Some(12))]), None);
        assert_eq!(pick_lookahead_branch(&[run(30, 10, 18, None), run(31, 10, 18, None)]), None);
        let mut a = run(30, 10, 18, None);
        let mut b = run(31, 10, 18, None);
        a.seen.push((19, 7));
        b.seen.push((19, 7));
        assert_eq!(pick_lookahead_branch(&[a, b]), Some(30));
    }

    /// Every branch refuted is its own verdict (a lookahead one level up reads
    /// it as a refutation); one standing branch is not.
    #[test]
    fn lookahead_verdict_tells_all_refuted_from_undecided() {
        assert_eq!(
            lookahead_verdict(&[run(30, 10, 11, Some(11)), run(31, 10, 12, Some(12))]),
            LookaheadVerdict::AllRefuted
        );
        assert_eq!(
            lookahead_verdict(&[run(30, 10, 18, None), run(31, 10, 18, None)]),
            LookaheadVerdict::Undecided
        );
        assert_eq!(
            lookahead_verdict(&[run(30, 10, 13, None), run(31, 10, 12, Some(12))]),
            LookaheadVerdict::Proved(30)
        );
        assert_eq!(lookahead_verdict(&[]), LookaheadVerdict::Undecided);
    }

    /// A branch stopped by a nested all-refuted order is refuted there only
    /// when a clean branch replayed past that row (raw-20261113T011922Z#g3
    /// raw seq 335: Gigimon first is refuted two orders deep, Elizamon /
    /// Dimetromon first pass the same rows). With no such branch it only
    /// stops (raw seq 382: BT24-008's [On Play] first meets an order the
    /// rows refute further on, Owen first is refuted outright).
    #[test]
    fn nested_all_refuted_refutes_only_where_another_branch_got_past() {
        let mut deep = run(30, 10, 14, None);
        deep.nested_refuted_at = Some(14);
        let mut a = run(31, 10, 18, None);
        let mut b = run(32, 10, 18, None);
        a.seen.push((18, 7));
        b.seen.push((18, 7));
        assert_eq!(lookahead_verdict(&[deep.clone(), a, b]), LookaheadVerdict::Proved(31));

        let mut later = run(30, 10, 19, None);
        later.nested_refuted_at = Some(19);
        let early = run(31, 10, 12, Some(12));
        assert_eq!(lookahead_verdict(&[later.clone(), early.clone()]), LookaheadVerdict::Proved(30));

        let mut also = run(31, 10, 19, None);
        also.nested_refuted_at = Some(19);
        assert_eq!(lookahead_verdict(&[later, also]), LookaheadVerdict::Undecided);
        assert_eq!(lookahead_verdict(&[early, run(32, 10, 13, Some(13))]), LookaheadVerdict::AllRefuted);
    }

    /// A recording source whose `steps()` panics once `armed` is set: the
    /// first thing a lookahead's sub-driver does with the lent source.
    struct PanicOnSteps {
        armed: std::sync::Arc<std::sync::atomic::AtomicBool>,
        steps: Vec<StepSpec>,
    }

    impl RecordingSource for PanicOnSteps {
        fn build_initial_game(&self, _: &HashMap<String, CardData>) -> Result<Game, ReplayError> {
            Err(ReplayError::GameConstruction("unused".into()))
        }
        fn relay_initial_state(&self, _: &mut Game) -> Result<(), ReplayError> {
            Ok(())
        }
        fn steps(&self) -> &[StepSpec] {
            if self.armed.load(std::sync::atomic::Ordering::SeqCst) {
                panic!("steps() panicked inside a lookahead (test)");
            }
            &self.steps
        }
    }

    /// The lookahead lends the driver's source to a sub-driver. A panic while
    /// the source is lent must not leave the driver holding the
    /// `NoRecordedSteps` placeholder: a drop guard returns the real source.
    #[test]
    fn lookahead_returns_the_source_even_when_a_branch_panics() {
        let armed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let step = StepSpec {
            actor: 0,
            action_id: crate::action::space::PASS,
            phase: "Main".into(),
            source: "main_phase".into(),
            memory_after: None,
            dcgo_memory: None,
            turn: None,
            is_game_over: None,
            expected_digest: None,
            selection: None,
            board_p0: None,
            board_p1: None,
            card_id: None,
            dna_materials: None,
            inferred: false,
        };
        let mut driver = ReplayDriver {
            source: Box::new(PanicOnSteps { armed: armed.clone(), steps: vec![step.clone(), step] }),
            cursor: 0,
            verify: false,
            policy: StepPolicy::CheckThenApply,
            divergences: Vec::new(),
            digivolve_operand_fallbacks: Vec::new(),
            inferred_skipped: Vec::new(),
            notes: Vec::new(),
            raid_retarget: None,
            paused: false,
            lookahead_depth: 0,
            pick_zone_hint: None,
            zone_menu_row: None,
            unstamped_window_start: 0,
            order_answer: None,
            dcgo_cards: HashMap::new(),
            count_evidence: None,
        };
        let game = build_empty_game(&filler_card_data()).expect("empty game");
        armed.store(true, std::sync::atomic::Ordering::SeqCst);
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            driver.lookahead_trigger_order(vec![(30, game.clone()), (31, game)], 0, &[], false)
        }));
        assert!(outcome.is_err(), "the branch panicked");
        armed.store(false, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(driver.source.steps().len(), 2, "the real source is back, not the lookahead placeholder");
    }

    /// P0's hand holds two copies of `FILL` (the first two handles) and its
    /// deck more (the third handle is the top of the deck).
    fn hand_pair_game() -> (Game, [crate::card_source::CardHandle; 3]) {
        use crate::debug_runner::{make_test_card, DebugRunner};
        use crate::enums::CardKind;
        let mut fill = make_test_card("FILL", "FILL");
        fill.card_kind = CardKind::Digimon;
        let mut r = DebugRunner::builder()
            .add_card(fill)
            .hand(0, &["FILL", "FILL"])
            .deck(0, &["FILL"; 8])
            .deck(1, &["FILL"; 8])
            .start();
        r.set_first_player(0);
        r.skip_mulligan();
        let hand = &r.game.player(0).hand;
        let handles = [hand[0].handle(), hand[1].handle(), r.game.player(0).deck[0].handle()];
        (r.game, handles)
    }

    /// `pair_dcgo_card`: two hand copies nothing names are interchangeable (CR
    /// 4.3 3-5-2), so when the engine took the other copy than the one an
    /// index was paired with, the two pairings swap; a copy elsewhere is not
    /// interchangeable, and its stale pairing is dropped instead.
    #[test]
    fn dcgo_card_pairing_swaps_hand_copies_and_drops_others() {
        use crate::card_source::CardHandle;
        let mut driver = ReplayDriver {
            source: Box::new(PanicOnSteps {
                armed: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                steps: Vec::new(),
            }),
            cursor: 0,
            verify: false,
            policy: StepPolicy::CheckThenApply,
            divergences: Vec::new(),
            digivolve_operand_fallbacks: Vec::new(),
            inferred_skipped: Vec::new(),
            notes: Vec::new(),
            raid_retarget: None,
            paused: false,
            lookahead_depth: 0,
            pick_zone_hint: None,
            zone_menu_row: None,
            unstamped_window_start: 0,
            order_answer: None,
            dcgo_cards: HashMap::new(),
            count_evidence: None,
        };
        let (game, [a, b, c]) = hand_pair_game();
        let before: HashMap<CardHandle, (PlayerId, CardZone)> =
            [(a, (0, CardZone::Hand)), (b, (0, CardZone::Hand)), (c, (0, CardZone::Deck))].into_iter().collect();
        // 87 searched into the hand (paired with a), 89 already paired with b.
        driver.dcgo_cards.insert(87, a);
        driver.dcgo_cards.insert(89, b);
        // DCGO plays 87; the engine took b: both were in the hand, so they swap.
        driver.pair_dcgo_card(87, b, &before, &game);
        assert_eq!(driver.dcgo_cards.get(&87), Some(&b));
        assert_eq!(driver.dcgo_cards.get(&89), Some(&a));
        // 90 paired with c (in the deck); the engine moves c for 91: 90's pairing is stale.
        driver.dcgo_cards.insert(90, c);
        driver.pair_dcgo_card(91, c, &before, &game);
        assert_eq!(driver.dcgo_cards.get(&91), Some(&c));
        assert_eq!(driver.dcgo_cards.get(&90), None);
    }

    #[test]
    fn slot_map_matches_by_identity_not_position() {
        // DCGO ordered by frame; the engine by play order. Same two
        // permanents, opposite indexes.
        let map = build_slot_map(
            &ids(&["BT16-082", "EX10-010"]),
            &ids(&["EX10-010", "BT16-082"]),
        );
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

    fn dcgo_action_row(
        step: u32,
        actor: u8,
        action_id: u16,
        phase: &str,
    ) -> crate::dcgo_recording::Row {
        crate::dcgo_recording::Row::Action(crate::dcgo_recording::ActionRow {
            inferred: false,
            step,
            actor,
            action_id,
            phase: phase.to_string(),
            source: "main_phase".to_string(),
            board_p0: None,
            board_p1: None,
            memory: None,
            card_id: None,
            card_index: None,
            operand_card_index: None,
            operand_card_index_source: None,
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

    #[test]
    fn remote_trigger_index_cannot_identify_a_declined_optional_effect() {
        use crate::debug_runner::{make_test_card, DebugRunner};
        use crate::enums::{EffectSourceKind, GamePhase};
        use crate::selection::{PendingSelection, SelectionKind};

        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("FILLER", "Filler"))
            .deck(0, &["FILLER"; 6]).deck(1, &["FILLER"; 6]).start();
        let source = runner.place_on_field(0, "FILLER", None);
        let card = runner.game.players[0].battle_area[source.index as usize].top_card().handle();
        runner.game.pending_selection = Some(PendingSelection {
            kind: SelectionKind::TriggerOrder,
            selecting_player: 0,
            previous_phase: GamePhase::Main,
            valid_action_ids: vec![30, 31],
            is_optional: false,
            prompt: "Choose triggered effect".into(),
            effect_choices: None,
            source_card: card,
            source_permanent: Some(source),
            source_kind: EffectSourceKind::Digimon,
            callback: Box::new(|_, _| panic!("must not guess the declined trigger")),
            on_decline: None,
            card_choice: None,
            zone_owner: None,
        });
        let order: crate::dcgo_recording::SelectionRow = serde_json::from_value(json!({
            "step": 0, "actor": 0, "phase": "Main", "prompt": "MultipleSkills", "int_value": 0
        })).unwrap();
        let decline: crate::dcgo_recording::SelectionRow = serde_json::from_value(json!({
            "step": 1, "actor": 0, "phase": "Main", "prompt": "OptionalSkill", "bool_value": false
        })).unwrap();
        let adapter = DcgoAdapter::from_recording(crate::dcgo_recording::RecordingV1 {
            start: dcgo_game_start(vec![], Some(vec![])),
            rows: vec![crate::dcgo_recording::Row::Selection(order.clone()),
                       crate::dcgo_recording::Row::Selection(decline)],
            end: dcgo_game_end(),
        }, &no_card_data()).unwrap();
        let mut driver = ReplayDriver {
            source: Box::new(adapter), cursor: 0, verify: false,
            policy: StepPolicy::CheckThenApply, divergences: Vec::new(),
            digivolve_operand_fallbacks: Vec::new(), inferred_skipped: Vec::new(),
            notes: Vec::new(), raid_retarget: None, paused: false, lookahead_depth: 0,
            pick_zone_hint: None, zone_menu_row: None, unstamped_window_start: 0,
            order_answer: None, dcgo_cards: HashMap::new(), count_evidence: None,
        };
        let result = driver.apply_selection_step(&mut runner.game, &order);
        assert_eq!(result.step_number, 0);
        assert!(driver.paused);
        assert!(matches!(&driver.divergences()[0].kind,
            DivergenceKind::SelectionResolution { reason } if reason.contains("cannot be identified")));
        assert_eq!(runner.game.pending_selection.as_ref().unwrap().valid_action_ids, vec![30, 31]);
    }

    #[test]
    fn dcgo_lilamon_payment_can_use_one_tamer_two_tamers_or_decline() {
        use crate::debug_runner::{make_test_card, DebugRunner};
        use crate::enums::{CardKind, EffectTiming};
        use crate::selection::TriggerSource;

        for (actor, targets, left, right) in [
            (0, json!({ "targets": [{"player":0,"frame":0}] }), 0, 1),
            (0, json!({ "targets": [{"player":0,"frame":0},{"player":0,"frame":1}] }), 1, 0),
            (0, json!({ "cancel": true }), 2, 1),
            (1, json!({ "targets": [{"player":0,"frame":0}] }), 2, 1),
        ] {
            let filler = make_test_card("FILLER", "Filler");
            let mut tamer = make_test_card("TAMER", "Tamer");
            tamer.card_kind = CardKind::Tamer;
            let mut runner = DebugRunner::builder()
                .dsl_card("ST24-10")
                .unwrap()
                .add_card(filler)
                .add_card(tamer)
                .deck(0, &["FILLER"; 6])
                .deck(1, &["FILLER"; 6])
                .start();
            runner.set_first_player(0);
            let left_tamer = runner.place_stack(0, &["FILLER", "FILLER", "TAMER"]);
            let right_tamer = runner.place_stack(0, &["FILLER", "TAMER"]);
            let lilamon = runner.place_on_field(0, "ST24-10", None);
            for (tamer, count) in [(left_tamer, 2), (right_tamer, 1)] {
                for source in &mut runner.game.players[0].battle_area[tamer.index as usize]
                    .card_sources[..count]
                {
                    source.face_down = true;
                }
            }
            runner.game.enqueue_triggered(
                EffectTiming::WhenDigivolving,
                TriggerSource::Permanent(lilamon),
            );
            runner.game.drain_effect_queue();

            let mut row = json!({
                "step": 0,
                "actor": actor,
                "phase": "Main",
                "prompt": "SelectPermanentEffect",
            });
            row.as_object_mut()
                .unwrap()
                .extend(targets.as_object().unwrap().clone());
            let payload: crate::dcgo_recording::SelectionRow = serde_json::from_value(row).unwrap();
            let adapter = DcgoAdapter::from_recording(
                crate::dcgo_recording::RecordingV1 {
                    start: dcgo_game_start(vec![], Some(vec![])),
                    rows: vec![crate::dcgo_recording::Row::Selection(payload.clone())],
                    end: dcgo_game_end(),
                },
                &no_card_data(),
            )
            .unwrap();
            let mut driver = ReplayDriver {
                source: Box::new(adapter),
                cursor: 0,
                verify: false,
                policy: StepPolicy::CheckThenApply,
                divergences: Vec::new(),
                digivolve_operand_fallbacks: Vec::new(),
                inferred_skipped: Vec::new(), notes: Vec::new(),
                raid_retarget: None, paused: false, lookahead_depth: 0,
                pick_zone_hint: None, zone_menu_row: None, unstamped_window_start: 0,
                order_answer: None, dcgo_cards: HashMap::new(), count_evidence: None,
            };
            let step = driver.apply_selection_step(&mut runner.game, &payload);
            if actor == 0 {
                assert_eq!(step.step_number, 1, "{:?}", driver.divergences());
                assert!(runner.game.pending_selection.is_none());
            } else {
                assert_eq!(step.step_number, 0);
                assert!(matches!(driver.divergences()[0].kind,
                    DivergenceKind::Actor { expected: 0, recorded: 1 }));
                assert!(runner.game.pending_selection.is_some());
            }
            for (tamer, expected) in [(left_tamer, left), (right_tamer, right)] {
                assert_eq!(
                    runner.game.players[0].battle_area[tamer.index as usize]
                        .face_down_source_indices()
                        .count(),
                    expected
                );
            }
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
            rows: vec![
                dcgo_action_row(0, 0, 5, "Main"),
                dcgo_action_detail_row(0, 0),
            ],
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
                dcgo_action_row(1, 0, 62, "Main"),     // pass
                dcgo_action_row(2, 0, 100, "Main"),    // attack slot 0 -> security
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

    /// A driver over `steps` with every per-row state empty.
    fn review_r5_driver(steps: Vec<StepSpec>) -> ReplayDriver {
        ReplayDriver {
            source: Box::new(PanicOnSteps {
                armed: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                steps,
            }),
            cursor: 0,
            verify: false,
            policy: StepPolicy::CheckThenApply,
            divergences: Vec::new(),
            digivolve_operand_fallbacks: Vec::new(),
            inferred_skipped: Vec::new(),
            notes: Vec::new(),
            raid_retarget: None,
            paused: false,
            lookahead_depth: 0,
            pick_zone_hint: None,
            zone_menu_row: None,
            unstamped_window_start: 0,
            order_answer: None,
            dcgo_cards: HashMap::new(),
            count_evidence: None,
        }
    }

    /// Round-5 review, finding 10: the event window an unstamped answer is
    /// judged against starts where the last decision began. A start past the
    /// end of the event log is a window the replay does not know; it must not
    /// be read as the WHOLE log (which judged the answer against every effect
    /// of the game): the replay halts.
    #[test]
    fn unstamped_answer_with_a_window_past_the_event_log_halts() {
        let row: crate::dcgo_recording::SelectionRow = serde_json::from_value(json!({
            "step": 0, "actor": 0, "prompt": "generic_bool", "phase": "Main",
            "bool_value": false, "unstamped": true
        }))
        .expect("selection row");
        let step = StepSpec {
            actor: 0,
            action_id: 0,
            phase: "Main".into(),
            source: "selection".into(),
            memory_after: None,
            dcgo_memory: None,
            turn: None,
            is_game_over: None,
            expected_digest: None,
            selection: Some(row),
            board_p0: None,
            board_p1: None,
            card_id: None,
            dna_materials: None,
            inferred: false,
        };
        let mut driver = review_r5_driver(vec![step]);
        let mut game = build_empty_game(&filler_card_data()).expect("empty game");
        assert!(game.pending_selection.is_none());
        driver.unstamped_window_start = game.events.len() + 3;
        driver.step(&mut game);
        assert!(driver.paused, "notes {:?}", driver.notes);
        match driver.divergences.as_slice() {
            [d] => match &d.kind {
                DivergenceKind::SelectionResolution { reason } => {
                    assert!(reason.contains("event window"), "{reason}")
                }
                other => panic!("{other:?}"),
            },
            other => panic!("expected one divergence, got {other:?}"),
        }
    }

    /// Round-5 review, finding 7: DCGO's -1 drops the stack of the panel it
    /// answers (MultipleSkills.cs:474-480). When the first prompt the decline
    /// meets is NOT a trigger order (a lone "you may" gate, declined), the
    /// trigger order that parks after it is a stack DCGO's panel never held:
    /// the decline stops there instead of taking it as the stack.
    #[test]
    fn decline_stack_stops_at_the_order_after_a_declined_lone_gate() {
        use crate::debug_runner::{make_test_card, DebugRunner};
        use crate::enums::{CardKind, EffectTiming};
        use crate::selection::{SelectionKind, TriggerSource};
        const MAY_GAIN: &str = r#"
card: TEST-MAY-GAIN
name: Test May Gain
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    optional: true
    summary: "[Start of Your Main Phase] You may gain 1 memory"
    process:
      - gain_memory: 1
"#;
        const DRAW: &str = r#"
card: CARD_ID
name: Test Draw CARD_ID
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] Draw 1"
    process:
      - draw: { of: you, count: 1 }
"#;
        let draw = |id: &str| DRAW.replace("CARD_ID", id);
        let mut fill = make_test_card("FILL", "FILL");
        fill.card_kind = CardKind::Digimon;
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(MAY_GAIN)
            .expect("may-gain fixture")
            .from_dsl_yaml(&draw("TEST-DRAW-A"))
            .expect("draw fixture")
            .from_dsl_yaml(&draw("TEST-DRAW-B"))
            .expect("draw fixture")
            .add_card(fill)
            .deck(0, &["FILL"; 8])
            .deck(1, &["FILL"; 8])
            .start();
        r.set_first_player(0);
        r.skip_mulligan();
        r.game.enter_main_phase();
        let may = r.place_on_field(0, "TEST-MAY-GAIN", Some(0));
        r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(may));
        r.game.drain_effect_queue();
        let gate = r.game.pending_selection.as_ref().expect("the lone gate is parked");
        assert!(!matches!(gate.kind, SelectionKind::TriggerOrder) && gate.is_optional, "{:?}", gate.kind);
        // Two triggers of another stack wait behind the gate.
        let a = r.place_on_field(0, "TEST-DRAW-A", Some(0));
        let b = r.place_on_field(0, "TEST-DRAW-B", Some(0));
        for t in [a, b] {
            r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(t));
        }
        let (hand, memory) = (r.game.players[0].hand.len(), r.game.memory);
        let (g, last, _) = dcgo_decline_stack(&r.game, 0, 0, None, false).expect("the decline applies");
        assert_eq!(last, crate::action::space::PASS, "the gate was declined");
        assert_eq!(g.memory, memory, "the gate's gain did not run");
        assert_eq!(g.players[0].hand.len(), hand, "neither draw ran");
        let p = g.pending_selection.as_ref().expect("the other stack's order waits");
        assert!(matches!(p.kind, SelectionKind::TriggerOrder) && p.valid_action_ids.len() == 2, "{:?} {}", p.kind, p.prompt);
    }

    // ── round-5 review, round 6 ──────────────────────────────────────────

    /// P0's main phase, two queued copies of one Tamer's mandatory [Start of
    /// Your Main Phase] "your opponent trashes 1 card in their hand": the
    /// forced order is parked. Picking either entry runs the trigger, which
    /// parks P1's pick.
    fn twin_opponent_trash_order() -> Game {
        use crate::debug_runner::{make_test_card, DebugRunner};
        use crate::enums::{CardKind, EffectTiming};
        use crate::selection::TriggerSource;
        const OPP_TRASH: &str = r#"
card: TEST-OPP-TRASH
name: Test Opponent Trash
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] Your opponent trashes 1 card in their hand"
    process:
      - as_selecting_player:
          of: opponent
          body:
            - select_hand:
                of: opponent
                bind_as: discarded
                filter: {}
                prompt: "Trash 1 card in your hand"
            - trash_from_hand_by_index: { of: opponent, hand_index: discarded }
"#;
        let mut fill = make_test_card("FILL", "FILL");
        fill.card_kind = CardKind::Digimon;
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(OPP_TRASH)
            .expect("fixture compiles")
            .add_card(fill)
            .hand(1, &["FILL", "FILL"])
            .deck(0, &["FILL"; 8])
            .deck(1, &["FILL"; 8])
            .start();
        r.set_first_player(0);
        r.skip_mulligan();
        r.game.enter_main_phase();
        let t = r.place_on_field(0, "TEST-OPP-TRASH", Some(0));
        for _ in 0..2 {
            r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(t));
        }
        r.game.drain_effect_queue();
        let p = r.game.pending_selection.as_ref().expect("the order is parked");
        assert!(matches!(p.kind, crate::selection::SelectionKind::TriggerOrder) && p.valid_action_ids.len() == 2);
        r.game
    }

    /// Round-5 review, finding 1: after a forced entry of the stack ran, a
    /// mandatory prompt that belongs to a card OUTSIDE the stack is not the
    /// outer stack's order (which the decline legitimately leaves for the
    /// following rows): it is the forced run still going. The decline meets
    /// it as an error, not as a stopping point.
    #[test]
    fn decline_stack_after_a_forced_run_errors_on_a_prompt_of_a_card_outside_the_stack() {
        let game = twin_opponent_trash_order();
        let mut g = game.clone();
        let pick = g.pending_selection.as_ref().expect("order").valid_action_ids[0];
        g.decode_action(pick, 0);
        let p = g.pending_selection.as_ref().expect("the forced trigger parks P1's pick");
        assert_eq!(p.selecting_player, 1, "{}", p.prompt);
        // The stack held no entry of the prompt's card: it is an outside card's prompt.
        let outcome = dcgo_decline_stack(&g, 0, 0, Some(Vec::new()), true);
        assert!(outcome.is_err(), "the forced run left P1's prompt parked, and the decline passed it");
    }

    /// Two identical Digimon of P0 (`COPY`, slots 0 and 1; `with_under` gives
    /// each an `UNDER` card beneath it - two physical cards) with a battle identity each, and the
    /// engine hand holding one `FILL`. Returns the game and the two slots.
    fn two_copies(with_under: bool) -> (Game, [crate::permanent::PermanentHandle; 2]) {
        use crate::debug_runner::{make_test_card, DebugRunner};
        use crate::enums::CardKind;
        let digimon = |id: &str| {
            let mut c = make_test_card(id, id);
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(4000);
            c
        };
        let mut r = DebugRunner::builder()
            .add_card(digimon("COPY"))
            .add_card(digimon("UNDER"))
            .add_card(digimon("FILL"))
            .hand(0, &["FILL"])
            .deck(0, &["FILL"; 8])
            .deck(1, &["FILL"; 8])
            .start();
        r.set_first_player(0);
        r.skip_mulligan();
        r.game.enter_main_phase();
        let (a, b) = if with_under {
            (r.place_stack(0, &["UNDER", "COPY"]), r.place_stack(0, &["UNDER", "COPY"]))
        } else {
            (r.place_on_field(0, "COPY", Some(0)), r.place_on_field(0, "COPY", Some(0)))
        };
        r.game.capture_digivolve_host_identity(a);
        r.game.capture_digivolve_host_identity(b);
        (r.game, [a, b])
    }

    /// A pending burst digivolve that names `card` and the battle identity
    /// `host_identity` - plain `u64` fields, the shape of every identity
    /// reference but the `(player, id)` pairs.
    fn burst_naming(card: crate::card_source::CardHandle, host_identity: u64) -> crate::dna_digivolve::PendingBurstDigivolve {
        crate::dna_digivolve::PendingBurstDigivolve {
            player: 0,
            card,
            host_identity,
            route: crate::dna_digivolve::DigivolveRouteMatch::digivolve(0),
        }
    }

    /// Baseline: two same-shape copies nothing names ARE interchangeable (a
    /// reference search that always answered "no" would pass every test
    /// below), with or without different cards beneath them.
    #[test]
    fn copies_nothing_names_are_interchangeable() {
        for with_under in [false, true] {
            let (game, slots) = two_copies(with_under);
            let slots: Vec<u16> = slots.iter().map(|h| u16::from(h.index)).collect();
            assert!(interchangeable_copies(&game, 0, &slots), "with_under {with_under}");
        }
    }

    /// Round-5 review, finding 3: a reference by PLAIN-NUMBER battle identity
    /// (`PendingBurstDigivolve::host_identity`, a bare `u64`) names one copy:
    /// swapping the two would change which Digimon the digivolve is for. The
    /// round-5 search looked for `battle_identity: Some(n)` and the pair
    /// forms only, so it never saw it.
    #[test]
    fn copies_one_of_which_a_plain_identity_field_names_are_not_interchangeable() {
        let (mut game, slots) = two_copies(false);
        let identity = game.player(0).battle_area[slots[0].index as usize].battle_identity.expect("identity captured");
        let hand_card = game.player(0).hand[0].handle();
        game.pending_burst_digivolve = Some(burst_naming(hand_card, identity));
        let slots: Vec<u16> = slots.iter().map(|h| u16::from(h.index)).collect();
        assert!(!interchangeable_copies(&game, 0, &slots));
    }

    /// Round-5 review, finding 3: a reference to a card UNDER (or linked to)
    /// a copy names that copy as surely as one to its top card.
    #[test]
    fn copies_one_of_which_has_a_card_beneath_that_state_names_are_not_interchangeable() {
        let (mut game, slots) = two_copies(true);
        let under = game.player(0).battle_area[slots[0].index as usize].card_sources[0].handle();
        game.pending_burst_digivolve = Some(burst_naming(under, 9_999));
        let slots: Vec<u16> = slots.iter().map(|h| u16::from(h.index)).collect();
        assert!(!interchangeable_copies(&game, 0, &slots));
    }

    /// Round-5 review, finding 6: `pair_dcgo_card`'s hand-copy swap assumes
    /// the two hand copies are interchangeable. CR 4.3 3-5-2 covers their
    /// ORDER only; a copy that engine state names (here a pending digivolve
    /// naming it) is not the same as the other, so the pairing is dropped
    /// rather than swapped.
    #[test]
    fn dcgo_card_pairing_does_not_swap_hand_copies_one_of_which_engine_state_names() {
        let (mut game, [a, b, _]) = hand_pair_game();
        game.pending_burst_digivolve = Some(burst_naming(a, 9_999));
        let before = card_zones(&game);
        let mut driver = review_r5_driver(Vec::new());
        driver.dcgo_cards.insert(87, a);
        driver.dcgo_cards.insert(89, b);
        // DCGO moved 87 (paired with a); the engine took b. `a` is named by
        // engine state, so 89 may not be re-paired with it.
        driver.pair_dcgo_card(87, b, &before, &game);
        assert_eq!(driver.dcgo_cards.get(&87), Some(&b));
        assert_eq!(driver.dcgo_cards.get(&89), None, "89's pairing is dropped, not swapped onto the named copy");
    }

    /// Round-5 review, finding 7 (measured gap): an unstamped `false` with no
    /// prompt parked is judged only by the effect events the log carries. A
    /// draw leaves none (no event of its own, none with a source card), so a
    /// draw-only effect the engine ran as mandatory is invisible and the row
    /// stays a note. Pinned so a widening of the detector shows up here.
    #[test]
    fn unstamped_false_after_a_draw_only_effect_is_still_only_a_note_a_measured_gap() {
        use crate::debug_runner::{make_test_card, DebugRunner};
        use crate::enums::{CardKind, EffectTiming};
        use crate::selection::TriggerSource;
        const DRAW_ONE: &str = r#"
card: TEST-DRAW-ONE
name: Test Draw One
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] Draw 1"
    process:
      - draw: { of: you, count: 1 }
"#;
        let mut fill = make_test_card("FILL", "FILL");
        fill.card_kind = CardKind::Digimon;
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(DRAW_ONE)
            .expect("fixture compiles")
            .add_card(fill)
            .deck(0, &["FILL"; 8])
            .deck(1, &["FILL"; 8])
            .start();
        r.set_first_player(0);
        r.skip_mulligan();
        r.game.enter_main_phase();
        let t = r.place_on_field(0, "TEST-DRAW-ONE", Some(0));
        let (start, hand) = (r.game.events.len(), r.game.players[0].hand.len());
        r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(t));
        r.game.drain_effect_queue();
        assert_eq!(r.game.players[0].hand.len(), hand + 1, "the engine ran the draw");
        assert!(r.game.pending_selection.is_none());
        let window = &r.game.events[start..];
        assert!(effect_activity(window, 0).is_empty(), "a draw leaves no attributable event: {window:?}");
        let verdict = ReplayDriver::classify_unstamped_answer(&r.game, 0, false, None, 0, window);
        assert!(matches!(verdict, UnstampedAnswer::Note(_)), "blind spot: a draw-only effect is not seen");
    }

    // ── Round-6 review: the forced-run footprint, the typed hidden fields, ──
    // ── 3-tuple identity references ─────────────────────────────────────────

    /// Whether `mutate` changes `effect_footprint_parts` of the two-copy game.
    fn footprint_differs(mutate: impl FnOnce(&mut Game)) -> bool {
        let (game, _) = two_copies(false);
        let before = effect_footprint_parts(&game);
        let mut changed = game.clone();
        mutate(&mut changed);
        effect_footprint_parts(&changed) != before
    }

    /// Guard (green on the old code too): a clone of the game has the same
    /// footprint, so every "differs" test below measures its own mutation.
    #[test]
    fn footprint_of_an_unchanged_clone_is_equal() {
        assert!(!footprint_differs(|_| {}));
    }

    /// Round-6 review, finding 1: a forced trigger whose only effect SCHEDULES
    /// a delayed effect (`EffectContext::schedule_delayed`) left something
    /// behind that DCGO's skip would not have.
    #[test]
    fn footprint_reads_the_scheduled_delayed_effects() {
        assert!(footprint_differs(|g| {
            let source_card = g.players[0].hand[0].handle();
            g.scheduled_effects.push(crate::scheduled_effects::ScheduledEffect {
                when: crate::enums::EffectTiming::EndOfYourTurn,
                body: Vec::new(),
                source_card,
                source_permanent: None,
                source_kind: crate::enums::EffectSourceKind::Digimon,
                controller: 0,
                captured_bindings: crate::dsl_cards::bindings::Bindings::default(),
                scheduled_at_turn: g.turn_count,
                runtime: crate::dsl_cards::step::StepRuntime::default(),
            });
        }));
    }

    /// `EffectContext::schedule_delete_at_end_of_turn` and
    /// `..._of_opponents_turn` keep their entries in two further lists.
    #[test]
    fn footprint_reads_the_scheduled_provenance_deletions() {
        let entry = |g: &Game| crate::scheduled_effects::ScheduledProvenanceDeletion {
            token: crate::trigger_context::ProvenanceToken(1),
            controller: 0,
            source_card: g.players[0].hand[0].handle(),
            source_kind: crate::enums::EffectSourceKind::Digimon,
        };
        assert!(footprint_differs(|g| {
            let e = entry(g);
            g.scheduled_provenance_deletions.push(e);
        }));
        assert!(footprint_differs(|g| {
            let e = entry(g);
            g.scheduled_provenance_deletions_opp.push(e);
        }));
    }

    #[test]
    fn footprint_reads_the_pending_added_sources() {
        assert!(footprint_differs(|g| {
            let card = g.players[0].hand[0].handle();
            g.pending_added_sources.push(crate::game::PendingAddedSources {
                host_card: card,
                cause: crate::trigger_context::EffectAttribution {
                    controller: 0,
                    source_card: Some(card).into(),
                    source_permanent: None,
                },
                cards: vec![card],
            });
        }));
    }

    #[test]
    fn footprint_reads_the_digixros_wildcard_substitutions() {
        assert!(footprint_differs(|g| {
            let card = g.players[0].hand[0].handle();
            g.active_digixros_wildcards.push(crate::digixros::ActiveDigiXrosWildcardSubstitution::for_current_turn(
                0, card, None, 0,
            ));
        }));
    }

    /// Counters a forced effect can move without leaving a zone change
    /// (the digivolution counts and the attack count feed "this turn" conditions).
    #[test]
    fn footprint_reads_the_turn_counters() {
        assert!(footprint_differs(|g| g.n_digivolutions[0] += 1));
        assert!(footprint_differs(|g| g.n_dna_digivolutions[1] += 1));
        assert!(footprint_differs(|g| g.n_digivolve_driven_attacks[0] += 1));
        assert!(footprint_differs(|g| g.digimon_attacks_this_turn[0] += 1));
    }

    /// An effect that only looked at (revealed) a security card changed what
    /// the opponent knows, and the shuffle generation is part of that record.
    #[test]
    fn footprint_reads_the_public_security_reveals() {
        assert!(footprint_differs(|g| {
            let card = g.players[0].hand[0].handle();
            g.public_security_reveals.push((0, card, 0, 0));
        }));
        assert!(footprint_differs(|g| g.security_shuffle_generations[0] += 1));
    }

    /// `GrantedEffectBodyRegistry`'s `Debug` prints only the body COUNT: a
    /// changed qualifier set (or a replaced body) is invisible to it.
    #[test]
    fn footprint_reads_the_granted_effect_qualifiers() {
        assert!(footprint_differs(|g| {
            g.granted_effect_bodies.insert_qualifier(7, std::sync::Arc::new(|_| true));
        }));
    }

    /// Round-6 review, finding 2: `Permanent`'s `Debug` omits `face_generation`
    /// and `placed_by_effect`, so a forced effect that only changed one of
    /// them left the footprint (and `copies_have_one_shape`) unchanged.
    #[test]
    fn footprint_reads_the_permanent_fields_debug_omits() {
        assert!(footprint_differs(|g| g.players[0].battle_area[0].placed_by_effect = true));
        assert!(footprint_differs(|g| g.players[0].battle_area[0].face_generation += 1));
    }

    /// `ModifierEntry`'s `Debug` omits `pending_skips` (and the replacement
    /// condition's presence); `PlayerModifierEntry`'s omits the replacement
    /// condition. The same visible entry with a different hidden field is a
    /// different game.
    #[test]
    fn footprint_reads_the_modifier_fields_debug_omits() {
        use crate::enums::{Expiry, ModifierType};
        use crate::modifiers::{ModifierEntry, PlayerModifierEntry};
        let permanent = crate::permanent::PermanentHandle { player: 0, index: 0 };
        let with_skips = |skips: u8| {
            let (mut g, _) = two_copies(false);
            let mut entry = ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::EndOfTurn, 0);
            entry.pending_skips = skips;
            g.modifiers.add(permanent, entry);
            g
        };
        assert_ne!(effect_footprint_parts(&with_skips(0)), effect_footprint_parts(&with_skips(1)));

        let with_condition = |on: bool| {
            let (mut g, _) = two_copies(false);
            let mut entry = ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::EndOfTurn, 0);
            if on {
                entry.replacement_condition = Some(std::sync::Arc::new(|_, _| true));
            }
            g.modifiers.add(permanent, entry);
            g
        };
        assert_ne!(effect_footprint_parts(&with_condition(false)), effect_footprint_parts(&with_condition(true)));

        let player_with_condition = |on: bool| {
            let (mut g, _) = two_copies(false);
            let mut entry = PlayerModifierEntry::simple(ModifierType::CannotGainMemoryByEffect, 0, Expiry::EndOfTurn, None, 0);
            if on {
                entry.replacement_condition = Some(std::sync::Arc::new(|_, _| true));
            }
            g.modifiers.add_player_modifier(0, entry);
            g
        };
        assert_ne!(
            effect_footprint_parts(&player_with_condition(false)),
            effect_footprint_parts(&player_with_condition(true))
        );
    }

    /// Round-6 review, finding 2: two copies that differ only in a field
    /// `Permanent`'s `Debug` omits are not "equal in every field but card
    /// identity".
    #[test]
    fn copies_differing_in_a_field_debug_omits_are_not_interchangeable() {
        let (mut game, slots) = two_copies(false);
        let slots: Vec<u16> = slots.iter().map(|h| u16::from(h.index)).collect();
        assert!(interchangeable_copies(&game, 0, &slots), "baseline");
        game.players[0].battle_area[1].face_generation += 1;
        assert!(!interchangeable_copies(&game, 0, &slots), "face_generation");
        let (mut game, slots) = two_copies(false);
        let slots: Vec<u16> = slots.iter().map(|h| u16::from(h.index)).collect();
        game.players[0].battle_area[1].placed_by_effect = true;
        assert!(!interchangeable_copies(&game, 0, &slots), "placed_by_effect");
    }

    /// Round-6 review, finding 3: a 3-tuple identity such as
    /// `OptionalReplacementState::source_identity: Option<(PlayerId,
    /// CardHandle, u64)>` prints `Some((0, CardHandle(5), 7))`; the
    /// battle identity is its last number.
    #[test]
    fn identity_references_reads_the_last_number_of_a_three_tuple() {
        assert_eq!(identity_references("source_identity: Some((0, CardHandle(5), 7))", 7), 1);
        assert_eq!(identity_references("source_identity: Some((0, CardHandle(7), 5))", 7), 0, "7 is the card, not the identity");
        assert_eq!(identity_references("source_identity: (1, CardHandle(5), 7)", 7), 1);
    }

    /// `stable_subject_card` / `stable_subject` (replacement.rs, replacement_window.rs) are
    /// `(PlayerId, CardHandle, battle identity)` under a name with no `identity` in it: the
    /// round-6 reference search never read them.
    #[test]
    fn identity_references_reads_the_stable_subject_fields() {
        assert_eq!(identity_references("stable_subject_card: Some((0, CardHandle(5), 7))", 7), 1);
        assert_eq!(identity_references("stable_subject: Some((1, CardHandle(5), 7))", 7), 1);
        assert_eq!(identity_references("stable_subject_card: None", 7), 0);
    }

    /// A forced effect that only consumed randomness (a shuffle with no visible reorder, a random
    /// pick) left the game in another state than DCGO's skip.
    #[test]
    fn footprint_reads_the_random_state() {
        use rand::RngCore;
        assert!(footprint_differs(|g| {
            g.rng.next_u64();
        }));
    }

    /// Guard (green on the old code): the shapes the search already read.
    #[test]
    fn identity_references_still_reads_bare_some_and_pair_shapes() {
        assert_eq!(identity_references("host_identity: 7", 7), 1);
        assert_eq!(identity_references("target_identity: Some(7)", 7), 1);
        assert_eq!(identity_references("attack_attacker_identity: Some((1, 7))", 7), 1);
        assert_eq!(identity_references("next_battle_identity: 7", 7), 0, "a counter is not a reference");
        assert_eq!(identity_references("host_identity: 71", 7), 0, "not a longer number");
    }
}
