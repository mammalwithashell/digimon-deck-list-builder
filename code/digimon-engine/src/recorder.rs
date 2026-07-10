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
use sha2::{Digest, Sha256};

use crate::enums::PlayerId;
use crate::game::Game;

pub const VERIFICATION_REPLAY_FORMAT_VERSION: u32 = 1;
pub const VERIFICATION_ENGINE_SCHEMA_VERSION: u32 = 1;

/// Deck references embedded in a canonical verification replay. Goldens may
/// use stable decklist ids when available, or inline card-id lists for
/// self-contained/generated cases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayDeckRefs {
    pub player1: ReplayDeckRef,
    pub player2: ReplayDeckRef,
}

impl ReplayDeckRefs {
    pub fn inline(player1: Vec<String>, player2: Vec<String>) -> Self {
        Self {
            player1: ReplayDeckRef::Inline { cards: player1 },
            player2: ReplayDeckRef::Inline { cards: player2 },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReplayDeckRef {
    Decklist { id: String },
    Inline { cards: Vec<String> },
}

impl ReplayDeckRef {
    pub fn inline_cards(&self) -> Option<&[String]> {
        match self {
            Self::Inline { cards } => Some(cards),
            Self::Decklist { .. } => None,
        }
    }
}

/// A single engine-native replay action. `seat` is the zero-based Rust player
/// seat; this format intentionally avoids Python's legacy 1/2 player-id
/// convention.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReplayAction {
    pub seat: PlayerId,
    pub action_id: u16,
}

/// Compact golden replay format for `qa/replay-goldens/*.replay.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReplayRecording {
    pub format_version: u32,
    pub engine_schema_version: u32,
    pub cards_hash: String,
    pub deck_refs: ReplayDeckRefs,
    pub seed: Option<u64>,
    pub actions: Vec<VerificationReplayAction>,
    pub digests: Vec<u64>,
    pub final_digest: u64,
}

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
    digests: Vec<u64>,
    tensor_snapshots: Vec<TensorSnapshot>,
    step_counter: u32,
}

impl GameRecorder {
    pub fn new(record_tensors: bool) -> Self {
        Self {
            record_tensors,
            initial: None,
            actions: Vec::new(),
            digests: Vec::new(),
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
    pub fn capture_initial_state(&mut self, game: &Game, deck_lists: (&[String], &[String])) {
        let p1 = &game.players[0];
        let p2 = &game.players[1];

        let to_ids = |v: &[crate::card_source::CardSource]| -> Vec<String> {
            v.iter()
                .map(|c| c.card_id(&game.card_data).to_string())
                .collect()
        };

        self.initial = Some(InitialState {
            first_player_id: game.turn_order[0],
            timestamp: timestamp_iso8601(),
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
    pub fn record_action(&mut self, game: &Game, action_id: u16, player_id: PlayerId) -> usize {
        self.step_counter += 1;
        let entry = RecordedAction {
            step_number: self.step_counter,
            player_id,
            action_id,
            phase: game.current_phase.py_name().to_string(),
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
        self.digests.push(game.verification_digest());
    }

    /// Capture tensor snapshot. No-op when `record_tensors` is false.
    pub fn record_tensor(&mut self, player_id: PlayerId, tensor: Vec<f32>, action_mask: Vec<f32>) {
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

    pub fn digests(&self) -> &[u64] {
        &self.digests
    }

    /// Project the recorder's action stream into the pinned, compact replay
    /// format used by the verification ladder. Unlike [`Self::to_json`], this
    /// surface is engine-native: seats are zero-based and the digest stream is
    /// part of the contract.
    pub fn to_verification_replay<S: Into<String>>(
        &self,
        game: &Game,
        deck_refs: ReplayDeckRefs,
        seed: Option<u64>,
        cards_hash: S,
    ) -> VerificationReplayRecording {
        VerificationReplayRecording {
            format_version: VERIFICATION_REPLAY_FORMAT_VERSION,
            engine_schema_version: VERIFICATION_ENGINE_SCHEMA_VERSION,
            cards_hash: cards_hash.into(),
            deck_refs,
            seed,
            actions: self
                .actions
                .iter()
                .map(|action| VerificationReplayAction {
                    seat: action.player_id,
                    action_id: action.action_id,
                })
                .collect(),
            digests: self.digests.clone(),
            final_digest: self
                .digests
                .last()
                .copied()
                .unwrap_or_else(|| game.verification_digest()),
        }
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

        let mut result = serde_json::Map::new();
        result.insert("initial_state".into(), initial_state);
        result.insert("actions".into(), Value::Array(actions));
        result.insert("total_actions".into(), json!(self.actions.len()));
        result.insert(
            "tensor_snapshots_count".into(),
            json!(self.tensor_snapshots.len()),
        );
        // Omit "tensor_snapshots" when empty — mirrors Python's conditional
        // inclusion in `GameRecorder.to_dict` (recording.py:213-222).
        if !self.tensor_snapshots.is_empty() {
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
            result.insert("tensor_snapshots".into(), Value::Array(tensors));
        }
        Value::Object(result)
    }
}

/// Return the canonical content hash string for a `cards.json` byte stream.
/// The `sha256:` prefix keeps the algorithm explicit in committed goldens.
pub fn cards_json_content_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{}", hex_lower(&digest))
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn player_initial_json(p: &PlayerInitialState, py_pid: &dyn Fn(PlayerId) -> i64) -> Value {
    json!({
        "player_id": py_pid(p.player_id),
        "deck_list": p.deck_list,
        "library_order": p.library_order,
        "digitama_library_order": p.digitama_library_order,
        "security_order": p.security_order,
        "initial_hand": p.initial_hand,
    })
}

/// Produce an ISO-8601 UTC timestamp string matching Python's
/// `datetime.now(timezone.utc).isoformat()` format:
/// `YYYY-MM-DDTHH:MM:SS+00:00`.
///
/// Uses a manual epoch-to-calendar conversion (Howard Hinnant's
/// `civil_from_days` algorithm) to avoid external crate dependencies.
fn timestamp_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let time_of_day = secs % 86_400;
    let hour = (time_of_day / 3600) as u32;
    let minute = ((time_of_day % 3600) / 60) as u32;
    let second = (time_of_day % 60) as u32;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}+00:00",
        year, month, day, hour, minute, second
    )
}

/// Howard Hinnant's `civil_from_days` algorithm. Returns `(year, month, day)`
/// in the proleptic Gregorian calendar for a count of days since 1970-01-01.
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_is_iso8601_format() {
        let ts = timestamp_iso8601();
        // Basic ISO-8601 shape: YYYY-MM-DDTHH:MM:SS+00:00 (25 chars)
        assert_eq!(ts.len(), 25, "unexpected timestamp length: {:?}", ts);
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], "T");
        assert_eq!(&ts[13..14], ":");
        assert_eq!(&ts[16..17], ":");
        assert_eq!(&ts[19..], "+00:00");
    }

    #[test]
    fn civil_from_days_epoch_is_1970_01_01() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }

    #[test]
    fn civil_from_days_known_date() {
        // 2026-04-18: days since epoch = 20561
        assert_eq!(civil_from_days(20561), (2026, 4, 18));
    }
}
