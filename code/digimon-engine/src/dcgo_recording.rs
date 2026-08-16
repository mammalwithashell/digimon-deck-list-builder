//! DCGO recording JSONL schema (version 1) and parser.
//!
//! Mirrors the C# emitter at
//! `DCGO/Assets/Scripts/Script/Recording/GameRecorder.cs`. The schema is
//! defined authoritatively in
//! `openspec/changes/add-dcgo-recording-parity-harness/specs/dcgo-parity-harness/spec.md`.
//!
//! Format: one JSON object per line. Each row carries a `"type"` field used
//! as a serde tag. Recognized types are `game_start`, `action`,
//! `encoder_failure`, `reveal`, and `game_end`. Unknown row types are tolerated
//! (logged but not fatal) so old-harness-new-recorder pairings don't break
//! catastrophically.
//!
//! Relocated into `digimon-engine` (was `code/tools/dcgo-replay/src/recording.rs`)
//! so both the `dcgo-replay` batch tool and the engine's `DcgoAdapter`
//! (interactive replay / MCP) build from one parser — keeping the batch parity
//! report and the interactive bug-hunter on a single code path.

use serde::{Deserialize, Serialize};

/// Schema version this build understands. Bumped in lockstep with
/// `crate::action::space::SCHEMA_VERSION` when the action-space layout or wire
/// format changes.
pub const SUPPORTED_SCHEMA_VERSION: u32 = 1;

/// One row in the JSONL recording. The `type` field is the discriminator.
///
/// We use serde's internally-tagged enum to deserialize one JSON object per
/// row by inspecting the `"type"` field. Rows whose `type` is not recognized
/// surface as [`Row::Unknown`] so the parser does not silently drop them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Row {
    /// Header row — exactly one per recording, the first line.
    GameStart(GameStart),
    /// An encoded decision. Either an `action` (success) or `encoder_failure`
    /// (the recorder couldn't map the decision to a 2192-space ID).
    Action(ActionRow),
    /// A selection answer with semantic payload (task 3.5) — resolved to
    /// engine action IDs at replay time against the live `PendingSelection`.
    Selection(SelectionRow),
    EncoderFailure(EncoderFailureRow),
    /// A card revealed from the opaque opponent's pile — produced by the
    /// DCGO recorder's PvP mode for every observed opponent reveal (draws,
    /// security pops, mill effects). The replay harness preloads these
    /// into a `RevealQueue` and the engine consumes them when its opaque
    /// pile is touched.
    ///
    /// Order matters: the queue serves reveals FIFO, so this row's
    /// position in the JSONL stream dictates when the engine sees it.
    Reveal(RevealRow),
    /// Terminal row — exactly one per recording, the last line.
    GameEnd(GameEnd),
    /// Tolerated escape hatch — captures rows whose `type` is unrecognized
    /// so newer recorders don't crash older harnesses (they're skipped by
    /// the replay loop). We serialize these as the JSON object payload they
    /// arrived with, minus the `type` tag.
    #[serde(other)]
    Unknown,
}

/// Header row payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStart {
    /// Schema version stamped by the C# recorder. The harness compares this
    /// to [`SUPPORTED_SCHEMA_VERSION`] and rejects mismatched recordings.
    pub v: u32,
    /// Stable unique ID for this game (used for cross-referencing in the
    /// parity report).
    pub game_id: String,
    /// UTC ISO-8601 timestamp.
    pub timestamp: String,
    /// 0 or 1 — the DCGO client's player ID (which side the recording is
    /// from). In bot mode this is conventionally 0.
    pub my_player_id: u8,
    /// 0 or 1 — who takes turn 1. Absent in recordings made before the
    /// recorder started emitting it; the adapter then infers it from the
    /// first mulligan actor (DCGO's first player mulligans first).
    #[serde(default)]
    pub first_player: Option<u8>,
    /// True for bot-vs-bot games (both decks observable). False for PvP.
    pub is_ai: bool,
    /// Local player's deck in post-shuffle order. Drawn from index 0 first.
    pub my_deck_post_shuffle: Vec<String>,
    /// Opponent's deck in post-shuffle order, or `None` for opaque PvP.
    pub opp_deck_post_shuffle: Option<Vec<String>>,
    /// Opponent's deck composition (multiset), populated for opaque PvP
    /// recordings. When `opp_deck_post_shuffle` is `None`, the recording
    /// MUST supply this so the harness can construct an opaque-mode game
    /// with the correct multiset. Optional in JSON to keep the v1 schema
    /// backward-compatible with bot-vs-bot recordings (which set
    /// `opp_deck_post_shuffle` instead).
    ///
    /// Anticipates task 7.x's DCGO mod update: the recorder will know the
    /// opponent's decklist from the matchmaking handshake (Photon room
    /// custom properties carry both players' decklists) even when it
    /// doesn't know the post-shuffle order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opp_decklist_composition: Option<Vec<String>>,
}

/// An encoded decision row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRow {
    pub step: u32,
    pub actor: u8,
    pub action_id: u16,
    pub phase: String,
    pub source: String,
}

/// A selection answer captured with SEMANTIC payload rather than a
/// pre-encoded action ID (task 3.5). The C# recorder writes the prompt's
/// class name plus absolute identities (frame targets, card ids, counts,
/// bools); the harness resolves them against the engine's live
/// `PendingSelection` at replay time, where candidate ordering and the
/// action-id scheme are authoritative. All payload fields are optional —
/// each prompt type fills only what it knows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionRow {
    pub step: u32,
    pub actor: u8,
    /// DCGO prompt class name (e.g. `SelectPermanentEffect`) or
    /// `generic_int` / `generic_bool` for the UserSelectionManager channel.
    pub prompt: String,
    #[serde(default)]
    pub phase: String,
    /// Field-permanent picks: absolute (owner, DCGO FrameID) pairs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<FrameTarget>>,
    /// Card identities picked (hand/trash/reveal prompts).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_ids: Option<Vec<String>>,
    /// Zone positions picked, when DCGO knows them (hand indexes etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indexes: Option<Vec<i32>>,
    /// SelectCountEffect payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    /// Generic int channel payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub int_value: Option<i64>,
    /// Generic bool channel payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bool_value: Option<bool>,
    /// True when the player cancelled / declined the prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancel: Option<bool>,
}

/// One absolute field-permanent reference: `player` seat + DCGO FrameID
/// (battle frames 0.., breeding frame = last — the harness maps to engine
/// slots, breeding → 14).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FrameTarget {
    pub player: u8,
    pub frame: i32,
}

/// An unencodable decision — the recorder saw it happen but couldn't map
/// it into the 2192-space. The Phase 1 harness halts cleanly when it hits
/// one of these (rather than guessing the recording mid-stream).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncoderFailureRow {
    pub step: u32,
    pub actor: u8,
    pub phase: String,
    pub source: String,
    pub reason: String,
    #[serde(default)]
    pub raw_value: String,
}

/// Opponent-deck reveal observed during PvP recording. The recorder
/// emits one of these every time the opaque opponent's pile is touched
/// (a card moves out of it into a zone the recording client can see).
///
/// `source` enumerates: `"draw"`, `"security"`, `"mill"`, `"effect"`.
/// The harness maps these strings to `crate::opaque_deck::RevealKind`
/// at queue-load time and surfaces a parse failure for unknown values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevealRow {
    pub step: u32,
    /// The player who owned the opaque pile this reveal came from
    /// (i.e., the opaque opponent in one-sided recordings).
    pub actor: u8,
    pub card_id: String,
    pub source: String,
}

/// Terminal row payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEnd {
    /// Winning player's ID (0 or 1), or `-1` for draw/disconnect.
    /// Signed so the JSON sentinel `-1` round-trips cleanly.
    pub winner: i8,
    pub reason: String,
    pub total_steps: u32,
}

/// A complete, validated recording. Produced by [`parse_jsonl`] when the
/// input has a single `game_start`, zero-or-more action / encoder_failure
/// rows, and a single `game_end`, with matching schema version.
#[derive(Debug, Clone)]
pub struct RecordingV1 {
    pub start: GameStart,
    pub rows: Vec<Row>, // includes Action, EncoderFailure, Reveal, Unknown rows in order
    pub end: GameEnd,
}

/// Errors surfaced while parsing or validating a recording.
#[derive(Debug, Clone)]
pub enum SchemaError {
    /// File could not be read.
    Io(String),
    /// A line failed to deserialize as a `Row`.
    InvalidJson { line: usize, message: String },
    /// First row was not `game_start`.
    MissingGameStart,
    /// Last row was not `game_end`.
    MissingGameEnd,
    /// `game_start.v` does not match [`SUPPORTED_SCHEMA_VERSION`].
    UnsupportedSchemaVersion { observed: u32, supported: u32 },
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaError::Io(m) => write!(f, "I/O error reading recording: {}", m),
            SchemaError::InvalidJson { line, message } => {
                write!(f, "invalid JSON at line {}: {}", line, message)
            }
            SchemaError::MissingGameStart => write!(f, "first row must be game_start"),
            SchemaError::MissingGameEnd => write!(f, "last row must be game_end"),
            SchemaError::UnsupportedSchemaVersion {
                observed,
                supported,
            } => write!(
                f,
                "recording schema version {} not supported by this harness (built for v{}). \
                 Either rebuild the harness against this recording's schema, or re-record \
                 from a DCGO build matching v{}.",
                observed, supported, supported
            ),
        }
    }
}

impl std::error::Error for SchemaError {}

/// Parse a JSONL recording into a structured [`RecordingV1`].
///
/// Validates: first row is `game_start`, last row is `game_end`, schema
/// version matches [`SUPPORTED_SCHEMA_VERSION`]. Intermediate rows of
/// unknown type are preserved as [`Row::Unknown`] so the harness can log
/// them without rejecting the recording outright.
pub fn parse_jsonl(text: &str) -> Result<RecordingV1, SchemaError> {
    let mut rows: Vec<Row> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row: Row = serde_json::from_str(trimmed).map_err(|e| SchemaError::InvalidJson {
            line: i + 1,
            message: e.to_string(),
        })?;
        rows.push(row);
    }

    // Pop the leading game_start.
    let start = match rows.first() {
        Some(Row::GameStart(s)) => s.clone(),
        _ => return Err(SchemaError::MissingGameStart),
    };
    // Schema version gate.
    if start.v != SUPPORTED_SCHEMA_VERSION {
        return Err(SchemaError::UnsupportedSchemaVersion {
            observed: start.v,
            supported: SUPPORTED_SCHEMA_VERSION,
        });
    }
    // Pop the trailing game_end.
    let end = match rows.last() {
        Some(Row::GameEnd(e)) => e.clone(),
        _ => return Err(SchemaError::MissingGameEnd),
    };

    // Strip header and footer so `rows` is just the decision stream.
    let middle = if rows.len() > 2 {
        rows[1..rows.len() - 1].to_vec()
    } else {
        Vec::new()
    };

    Ok(RecordingV1 {
        start,
        rows: middle,
        end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn well_formed_jsonl() -> &'static str {
        r#"{"v":1,"type":"game_start","game_id":"abc","timestamp":"2026-05-25T00:00:00Z","my_player_id":0,"is_ai":true,"my_deck_post_shuffle":["BT1-010"],"opp_deck_post_shuffle":["BT1-010"]}
{"type":"action","step":0,"actor":0,"action_id":0,"phase":"Mulligan","source":"mulligan"}
{"type":"action","step":1,"actor":1,"action_id":0,"phase":"Mulligan","source":"mulligan"}
{"type":"game_end","winner":0,"reason":"win","total_steps":2}
"#
    }

    #[test]
    fn parses_well_formed_recording() {
        let r = parse_jsonl(well_formed_jsonl()).expect("parse");
        assert_eq!(r.start.v, 1);
        assert_eq!(r.start.game_id, "abc");
        assert_eq!(r.start.my_player_id, 0);
        assert_eq!(r.rows.len(), 2);
        assert_eq!(r.end.winner, 0);
        assert_eq!(r.end.total_steps, 2);
    }

    #[test]
    fn rejects_missing_game_start() {
        let txt = r#"{"type":"action","step":0,"actor":0,"action_id":0,"phase":"Main","source":"mulligan"}
{"type":"game_end","winner":0,"reason":"win","total_steps":1}
"#;
        let err = parse_jsonl(txt).unwrap_err();
        assert!(matches!(err, SchemaError::MissingGameStart));
    }

    #[test]
    fn rejects_missing_game_end() {
        let txt = r#"{"v":1,"type":"game_start","game_id":"x","timestamp":"t","my_player_id":0,"is_ai":true,"my_deck_post_shuffle":[],"opp_deck_post_shuffle":[]}
{"type":"action","step":0,"actor":0,"action_id":0,"phase":"Main","source":"mulligan"}
"#;
        let err = parse_jsonl(txt).unwrap_err();
        assert!(matches!(err, SchemaError::MissingGameEnd));
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let txt = r#"{"v":999,"type":"game_start","game_id":"x","timestamp":"t","my_player_id":0,"is_ai":true,"my_deck_post_shuffle":[],"opp_deck_post_shuffle":[]}
{"type":"game_end","winner":0,"reason":"win","total_steps":0}
"#;
        let err = parse_jsonl(txt).unwrap_err();
        match err {
            SchemaError::UnsupportedSchemaVersion {
                observed,
                supported,
            } => {
                assert_eq!(observed, 999);
                assert_eq!(supported, 1);
            }
            _ => panic!("wrong error kind: {:?}", err),
        }
    }

    #[test]
    fn rejects_invalid_json_with_line_number() {
        let txt = r#"{"v":1,"type":"game_start","game_id":"x","timestamp":"t","my_player_id":0,"is_ai":true,"my_deck_post_shuffle":[],"opp_deck_post_shuffle":[]}
this is not json
{"type":"game_end","winner":0,"reason":"win","total_steps":0}
"#;
        let err = parse_jsonl(txt).unwrap_err();
        match err {
            SchemaError::InvalidJson { line, .. } => assert_eq!(line, 2),
            _ => panic!("wrong error kind: {:?}", err),
        }
    }

    #[test]
    fn preserves_encoder_failure_rows() {
        let txt = r#"{"v":1,"type":"game_start","game_id":"x","timestamp":"t","my_player_id":0,"is_ai":true,"my_deck_post_shuffle":[],"opp_deck_post_shuffle":[]}
{"type":"encoder_failure","step":0,"actor":0,"phase":"Main","source":"selection_int","reason":"unknown_prompt","raw_value":"int_value=3 phase=Main"}
{"type":"game_end","winner":0,"reason":"win","total_steps":1}
"#;
        let r = parse_jsonl(txt).expect("parse");
        assert_eq!(r.rows.len(), 1);
        match &r.rows[0] {
            Row::EncoderFailure(ef) => {
                assert_eq!(ef.step, 0);
                assert_eq!(ef.reason, "unknown_prompt");
            }
            other => panic!("expected EncoderFailure, got {:?}", other),
        }
    }

    #[test]
    fn parses_reveal_rows() {
        let txt = r#"{"v":1,"type":"game_start","game_id":"x","timestamp":"t","my_player_id":0,"is_ai":false,"my_deck_post_shuffle":[],"opp_deck_post_shuffle":null}
{"type":"reveal","step":0,"actor":1,"card_id":"BT1-010","source":"draw"}
{"type":"reveal","step":1,"actor":1,"card_id":"BT1-025","source":"security"}
{"type":"game_end","winner":0,"reason":"win","total_steps":2}
"#;
        let r = parse_jsonl(txt).expect("parse");
        assert_eq!(r.rows.len(), 2);
        match &r.rows[0] {
            Row::Reveal(rv) => {
                assert_eq!(rv.actor, 1);
                assert_eq!(rv.card_id, "BT1-010");
                assert_eq!(rv.source, "draw");
            }
            other => panic!("expected Reveal, got {:?}", other),
        }
        match &r.rows[1] {
            Row::Reveal(rv) => {
                assert_eq!(rv.source, "security");
            }
            other => panic!("expected Reveal, got {:?}", other),
        }
    }

    #[test]
    fn tolerates_unknown_row_types_for_forward_compat() {
        // Same forward-compat path, exercised with a hypothetical future
        // row type the harness doesn't know about.
        let txt = r#"{"v":1,"type":"game_start","game_id":"x","timestamp":"t","my_player_id":0,"is_ai":true,"my_deck_post_shuffle":[],"opp_deck_post_shuffle":[]}
{"type":"future_event","step":0,"some_field":"some_value"}
{"type":"game_end","winner":0,"reason":"win","total_steps":1}
"#;
        let r = parse_jsonl(txt).expect("parse");
        assert_eq!(r.rows.len(), 1);
        assert!(matches!(r.rows[0], Row::Unknown));
    }
}
