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
            // Backward: rebuild.
            let card_data = self.snapshot_card_data();
            self.build_game(&card_data)?;
        }
        while self.current_step < target && !self.is_game_over() {
            self.step();
        }
        Ok(())
    }

    /// Step through every remaining action.
    pub fn run_to_completion(&mut self) {
        while !self.is_complete() && !self.is_game_over() {
            self.step();
        }
    }

    // ── internals ────────────────────────────────────────────────────────

    fn snapshot_card_data(&self) -> HashMap<String, CardData> {
        self.game
            .card_data
            .iter()
            .map(|cd| (cd.card_id.clone(), cd.clone()))
            .collect()
    }

    fn build_game(&mut self, all_card_data: &HashMap<String, CardData>) -> Result<(), ReplayError> {
        // Fresh empty Game.
        self.game = build_empty_game(all_card_data)?;
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
    push_zone(&mut player.security, &data["security_order"], next_card_index)?;
    push_zone(&mut player.hand, &data["initial_hand"], next_card_index)?;
    Ok(())
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
