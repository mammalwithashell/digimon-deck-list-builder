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
    /// The RESOLVED detail of a preceding `action` row (diagnostic fields,
    /// added post-v1 — see [`ActionDetailRow`]). Correlate to the `action`
    /// row that shares its `step` (and `actor`).
    ActionDetail(ActionDetailRow),
    /// A selection answer with semantic payload (task 3.5) — resolved to
    /// engine action IDs at replay time against the live `PendingSelection`.
    Selection(SelectionRow),
    EncoderFailure(EncoderFailureRow),
    /// Post-mulligan zone snapshot — emitted once per game, after BOTH
    /// players' mulligan decisions have resolved and security has been
    /// dealt. See [`InitialStateRow`] for the full contract. Optional:
    /// absent in recordings made before the recorder started emitting it.
    InitialState(InitialStateRow),
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
    /// Local player's digitama (egg) deck in post-shuffle order — index 0
    /// is hatched first in DCGO. Absent (`None`) in recordings made before
    /// the recorder started emitting egg decks; the adapter then replays
    /// with an empty egg deck (pre-egg-capture behavior).
    #[serde(default)]
    pub my_egg_deck: Option<Vec<String>>,
    /// Opponent's digitama (egg) deck in post-shuffle order. Bot matches
    /// only — `None` for PvP (the opponent's digitama order is not
    /// observable, same as their main deck) and for pre-egg-capture
    /// recordings.
    #[serde(default)]
    pub opp_egg_deck: Option<Vec<String>>,
}

/// An encoded decision row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRow {
    pub step: u32,
    pub actor: u8,
    pub action_id: u16,
    pub phase: String,
    pub source: String,
    /// Battle-area card IDs at decision time, in the DCGO compact order the
    /// row's board operands index. Absent in pre-0.4 recordings.
    ///
    /// DCGO orders its compact battle area by on-screen frame, and permanents
    /// physically migrate between frames at runtime (`PreferredFrame`), so a
    /// DCGO slot index does NOT correspond to the engine's play-ordered
    /// `battle_area`. The replay harness rebuilds the mapping by matching card
    /// identity against the engine's board; without these lists it can only
    /// assume the orders agree, which they routinely do not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub board_p0: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub board_p1: Option<Vec<String>>,
    /// Shared memory gauge at decision time, converted to THIS
    /// recording's `my_player_id` perspective — positive favors the
    /// recording player, negative favors the opponent. Always relative to
    /// the SAME fixed player for the whole recording, never to whoever is
    /// turn-player at this row (that would require re-deriving whose
    /// favor a bare number means at every row — see `GameContext.Memory`
    /// / `Player.MemoryForPlayer` in the C# recorder and the engine's
    /// opposite, active-player-relative `Game::memory` convention;
    /// `runners::replay::memory_from_recording_perspective` reconciles
    /// the two explicitly, never by inference). Absent in recordings made
    /// before the recorder started emitting this field — `None` must be
    /// treated as "unknown", not as agreement or zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<i32>,
    /// [Diagnostic, added post-v1] The physical card this action references
    /// (play/digivolve card, activated hand card, activated permanent's top
    /// card) — the printed card ID (e.g. `"EX12-035"`), resolved by the C#
    /// recorder BEFORE the `photonView.RPC` dispatch (index lookup into the
    /// actor's own already-known hand/field, not the outcome of resolving
    /// the action). `None` when the action references no card (pass, hatch,
    /// phase actions) or in recordings predating this field. Not to be
    /// confused with the AUTHORITATIVE resolved identity carried on this
    /// action's correlated [`ActionDetailRow`] (same value in practice,
    /// captured from the actual `CardSource` instance once resolved,
    /// intended as a same-source cross-check).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_id: Option<String>,
    /// [Diagnostic, added post-v1] Identical read to `memory`, under its
    /// own key — see `memory`'s doc for the perspective convention. Exists
    /// so a reader never has to know that, specifically for an `action`
    /// row, `memory` means "before this action resolves" (true only
    /// because `LogAction` fires pre-dispatch) as opposed to what `memory`
    /// means on a `selection` row (captured mid-resolution). `None` in
    /// recordings predating this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_before: Option<i32>,
}

/// [Diagnostic, added post-v1] The RESOLVED detail of a preceding `action`
/// row — the data that genuinely cannot be known until DCGO's
/// `CardController.PlayCardClass.PlayCard()` actually pays the cost
/// (Assembly/DigiXros material selection, cost-modifying effects, and the
/// memory gauge afterward all resolve strictly AFTER the `action` row's
/// pre-dispatch `LogAction` call already ran).
///
/// Emitted as a SEPARATE row rather than inline fields on the `action` row
/// because the C# recorder streams each row to disk immediately
/// (append-only, no buffering) — by the time this data exists, the
/// `action` row's line is already flushed. Correlate the two via matching
/// `step` (this row's `step` equals its `action` row's `step`) and
/// `actor`.
///
/// Scoped to player-facing top-level plays only: the C# recorder emits
/// this only when a `PlayCardAction` row was JUST logged for the same
/// actor (see `GameRecorder.LogActionResolution`'s doc) — an internal
/// `PlayCardClass.PlayCard()` invocation triggered by a card EFFECT (e.g.
/// "play a card from your security/trash", ~5 call sites in
/// `CardEffectCommons.cs`) does not produce one.
///
/// This round is diagnostic only — no assertions are made on these fields
/// yet; parsing plus availability is the goal (see the `dcgo-recorder-
/// action-detail-fields` investigation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDetailRow {
    /// Matches the correlated `action` row's `step`.
    pub step: u32,
    pub actor: u8,
    /// The resolved card's printed ID, read directly from the actual
    /// `CardSource` instance being played (not re-derived from index
    /// lookups) — a same-source cross-check against the correlated
    /// `action` row's own `card_id`. `None` only defensively (should
    /// always be present when this row is emitted at all).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_id: Option<String>,
    /// The memory actually deducted for this play/digivolve — DCGO's
    /// `CardController.cs` local `Cost` at its `AddMemory(-1 * Cost, ...)`
    /// call site. This is the value AFTER any Assembly/DigiXros/DNA/
    /// Burst/AppFusion alternate-path reduction or other cost-modifying
    /// effect — never the printed cost.
    pub cost_paid: i32,
    /// Shared memory gauge immediately AFTER this play's cost was
    /// deducted, same `my_player_id`-relative perspective convention as
    /// [`ActionRow::memory`] / [`ActionRow::memory_before`]. `None` when
    /// the recording client's own `Player` (`GManager.instance.You`)
    /// wasn't available at write time (defensive; should not happen
    /// mid-game).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_after: Option<i32>,
    /// One of `"assembly"`, `"digixros"`, `"dna_digivolve"`, `"burst"`,
    /// `"app_fusion"`, `"cost_modifier"` (a reduction/change from some
    /// other effect not captured by the named paths), or `None` when the
    /// ordinary path/cost applied. See DCGO's
    /// `CardController.PlayCardClass.DetermineAltPath` for exactly which
    /// state each label reads — each named path mirrors the SAME gate
    /// DCGO's own `CardSource.GetPayingCostWithBaseCost` checks before
    /// applying that path's reduction, not a guess.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt_path: Option<String>,
    /// Card IDs placed/used for the alt path (Assembly/DigiXros trash
    /// materials, the two DNA digivolve partner permanents' top cards, the
    /// Burst tamer, the AppFusion linked card). `None`/absent when not
    /// applicable or not identifiable (e.g. `"cost_modifier"`, which has
    /// no discrete materials — see the doc on the C# side:
    /// Assembly/DigiXros material selection is driven by DCGO's
    /// `SelectCardEffect.SetTargetCardAndIndicies` RPC, a chokepoint
    /// entirely outside the recorder's existing hooks, so prior to this
    /// field there was NO recorded row at all for that selection despite
    /// it being a real player-facing prompt in-game).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub materials: Option<Vec<String>>,
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
    /// SelectCountEffect payload: the VALUE chosen (a digivolution cost, a
    /// number of cards), NOT an index. See `candidates`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    /// The option set `count` was chosen from, ascending, as DCGO presented it.
    ///
    /// A count answer is a domain value; our engine models the same question as
    /// an indexed branch list. Mapping one to the other needs the option set,
    /// which is why the recorder emits it. Absent in pre-0.5 recordings, and a
    /// count without it cannot be resolved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidates: Option<Vec<i64>>,
    /// Generic int channel payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub int_value: Option<i64>,
    /// Generic bool channel payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bool_value: Option<bool>,
    /// True when the player cancelled / declined the prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancel: Option<bool>,
    /// Battle-area card IDs at decision time, in the DCGO compact order the
    /// row's board operands index. Absent in pre-0.4 recordings.
    ///
    /// DCGO orders its compact battle area by on-screen frame, and permanents
    /// physically migrate between frames at runtime (`PreferredFrame`), so a
    /// DCGO slot index does NOT correspond to the engine's play-ordered
    /// `battle_area`. The replay harness rebuilds the mapping by matching card
    /// identity against the engine's board; without these lists it can only
    /// assume the orders agree, which they routinely do not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub board_p0: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub board_p1: Option<Vec<String>>,
    /// Shared memory gauge at decision time, `my_player_id` perspective.
    /// Same convention as [`ActionRow::memory`] — see its doc comment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<i32>,
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
    /// [Diagnostic, added post-v1] The physical card the encoder choked on,
    /// when resolvable — same field/semantics as [`ActionRow::card_id`].
    /// `None` when the action referenced no card, the lookup failed, or in
    /// recordings predating this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_id: Option<String>,
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

/// A post-mulligan zone snapshot for ONE player, as observed by the
/// recording client.
///
/// **Ordering convention**: card-id lists use the SAME "index 0 = first
/// drawn / top" convention as [`GameStart::my_deck_post_shuffle`] — NOT
/// the native `GameRecorder`'s opposite, bottom-first `library_order`
/// convention (see `runners::replay::restore_player_zone`, which expects
/// index 0 = bottom because it pushes straight into the engine's
/// pop-from-end `Vec` zones). Callers restoring engine zones from THIS
/// struct must reverse `library_order`, `digitama_library_order`, and
/// `security_order` before pushing (`runners::replay`'s DCGO snapshot
/// restorer does this — mirroring the existing `rev()` used for
/// `DcgoAdapter`'s bot-vs-bot ordered-deck construction). `initial_hand`
/// has no top/bottom concept and is pushed in recorded order, unreversed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialStateSide {
    pub library_order: Vec<String>,
    #[serde(default)]
    pub digitama_library_order: Vec<String>,
    pub security_order: Vec<String>,
    pub initial_hand: Vec<String>,
}

/// Post-mulligan zone snapshot row (added post-v1, optional field on the
/// wire — see [`Row::InitialState`]). Emitted once per game, from DCGO's
/// `TurnStateMachine.StartGame` immediately after BOTH players' mulligan
/// decisions have resolved and security has been dealt.
///
/// Closes a real gap: rule 5-2-1-5 of the official rules manual (general
/// rule PDF) makes a mulligan a TRUE reshuffle — "the player returns
/// their entire hand to their deck, shuffles it, then draws 5 cards for
/// their new initial hand" — so [`GameStart::my_deck_post_shuffle`]
/// (captured BEFORE mulligan) does not reflect a mulliganed game's actual
/// post-mulligan order. Absent in recordings made before the recorder
/// started emitting it; `DcgoAdapter` then falls back to replaying the
/// recorded mulligan actions against the pre-mulligan order (today's
/// behavior — see `should_filter_mulligan_actions`).
///
/// `my`/`opp` mirrors [`GameStart::my_deck_post_shuffle`] /
/// `opp_deck_post_shuffle`: `opp` is `None` for PvP (the opponent's
/// post-mulligan hand/library isn't observable — same visibility split
/// as the pre-mulligan deck), `Some` for Bot Match (fully observable).
///
/// `first_player_id`: 0 or 1 — DCGO's OWN player-id convention (matches
/// [`GameStart::my_player_id`] / `first_player`), explicitly NOT the
/// native `GameRecorder`'s 1/2 (Python) convention that
/// `ReplayRunner`/`NativeAdapter` translate via `saturating_sub(1)`. Do
/// not reuse that translation for this row — it would silently swap
/// which player's deck is which.
///
/// `memory`: shared gauge from `my_player_id`'s perspective, positive =
/// favor of the recording player — same convention as the per-row
/// `memory` field on [`ActionRow`]/[`SelectionRow`]. Always 0 at this
/// point in a real game (the memory gauge only starts moving once turn 1
/// begins) — carried for schema symmetry / a sanity cross-check, not
/// because reconstruction needs to read it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialStateRow {
    pub first_player_id: u8,
    pub my: InitialStateSide,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opp: Option<InitialStateSide>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<i32>,
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

/// Convert the engine's active-player-relative memory seesaw value
/// (`Game::memory` — positive favors `Game::memory_pair.0`, whichever
/// player currently holds turn-player status; the seesaw flips every
/// turn, see `game_phases.rs`'s `rotate_turn_player`) into the
/// perspective a DCGO recording's `memory` field uses: always relative
/// to the recording's FIXED `my_player_id`, positive = favor of that
/// player, never to whoever is active at a given row.
///
/// These are genuinely different conventions reconciling two independent
/// systems (the engine's turn-relative seesaw vs. DCGO's own
/// `GameContext.Memory` / `Player.MemoryForPlayer`, which is fixed to
/// PlayerID rather than to turn order) — comparing the raw numbers
/// without this conversion silently inverts on every turn where
/// `memory_pair_favored != my_player_id`, which is exactly the kind of
/// unfalsifiable-parity-investigation bug this field exists to prevent.
///
/// `memory_pair_favored` must be `Game::memory_pair.0` read at the SAME
/// instant as `game_memory` — the seesaw only moves at turn rotation, so
/// pairing a `memory` reading with a `memory_pair` reading taken at a
/// later point would silently reintroduce the same bug this function
/// exists to prevent.
pub fn memory_from_recording_perspective(
    game_memory: i16,
    memory_pair_favored: u8,
    my_player_id: u8,
) -> i32 {
    if memory_pair_favored == my_player_id {
        game_memory as i32
    } else {
        -(game_memory as i32)
    }
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

    // ── memory field (task: memory-gauge parity) ────────────────────────

    #[test]
    fn action_row_parses_memory_when_present() {
        let txt = r#"{"v":1,"type":"game_start","game_id":"x","timestamp":"t","my_player_id":0,"is_ai":true,"my_deck_post_shuffle":[],"opp_deck_post_shuffle":[]}
{"type":"action","step":0,"actor":0,"action_id":0,"phase":"Main","source":"main_phase","memory":3}
{"type":"game_end","winner":0,"reason":"win","total_steps":1}
"#;
        let r = parse_jsonl(txt).expect("parse");
        match &r.rows[0] {
            Row::Action(a) => assert_eq!(a.memory, Some(3)),
            other => panic!("expected Action, got {:?}", other),
        }
    }

    #[test]
    fn action_row_memory_absent_parses_as_none() {
        // Shape of the existing 9-game corpus (predates this field) — must
        // still parse, and absence must not be inferred as agreement/zero.
        let txt = r#"{"v":1,"type":"game_start","game_id":"x","timestamp":"t","my_player_id":0,"is_ai":true,"my_deck_post_shuffle":[],"opp_deck_post_shuffle":[]}
{"type":"action","step":0,"actor":0,"action_id":0,"phase":"Main","source":"main_phase"}
{"type":"game_end","winner":0,"reason":"win","total_steps":1}
"#;
        let r = parse_jsonl(txt).expect("parse");
        match &r.rows[0] {
            Row::Action(a) => assert_eq!(a.memory, None),
            other => panic!("expected Action, got {:?}", other),
        }
    }

    #[test]
    fn selection_row_parses_memory_when_present() {
        let txt = r#"{"v":1,"type":"game_start","game_id":"x","timestamp":"t","my_player_id":1,"is_ai":true,"my_deck_post_shuffle":[],"opp_deck_post_shuffle":[]}
{"type":"selection","step":0,"actor":1,"prompt":"generic_int","phase":"Main","int_value":2,"memory":-4}
{"type":"game_end","winner":0,"reason":"win","total_steps":1}
"#;
        let r = parse_jsonl(txt).expect("parse");
        match &r.rows[0] {
            Row::Selection(s) => assert_eq!(s.memory, Some(-4)),
            other => panic!("expected Selection, got {:?}", other),
        }
    }

    // ── initial_state row (task: post-mulligan snapshot) ────────────────

    #[test]
    fn parses_initial_state_row_bot_match() {
        let txt = r#"{"v":1,"type":"game_start","game_id":"x","timestamp":"t","my_player_id":0,"is_ai":true,"my_deck_post_shuffle":[],"opp_deck_post_shuffle":[]}
{"type":"initial_state","first_player_id":0,"my":{"library_order":["BT1-010"],"digitama_library_order":[],"security_order":["BT1-025"],"initial_hand":["BT1-010"]},"opp":{"library_order":["BT1-025"],"digitama_library_order":[],"security_order":["BT1-010"],"initial_hand":["BT1-025"]},"memory":0}
{"type":"game_end","winner":0,"reason":"win","total_steps":1}
"#;
        let r = parse_jsonl(txt).expect("parse");
        assert_eq!(r.rows.len(), 1);
        match &r.rows[0] {
            Row::InitialState(s) => {
                assert_eq!(s.first_player_id, 0);
                assert_eq!(s.my.library_order, vec!["BT1-010".to_string()]);
                let opp = s.opp.as_ref().expect("opp side present for bot match");
                assert_eq!(opp.initial_hand, vec!["BT1-025".to_string()]);
                assert_eq!(s.memory, Some(0));
            }
            other => panic!("expected InitialState, got {:?}", other),
        }
    }

    #[test]
    fn parses_initial_state_row_pvp_without_opp_side() {
        let txt = r#"{"v":1,"type":"game_start","game_id":"x","timestamp":"t","my_player_id":0,"is_ai":false,"my_deck_post_shuffle":[],"opp_deck_post_shuffle":null}
{"type":"initial_state","first_player_id":1,"my":{"library_order":["BT1-010"],"security_order":["BT1-025"],"initial_hand":["BT1-010"]}}
{"type":"game_end","winner":0,"reason":"win","total_steps":1}
"#;
        let r = parse_jsonl(txt).expect("parse");
        match &r.rows[0] {
            Row::InitialState(s) => {
                assert_eq!(s.first_player_id, 1);
                assert!(s.opp.is_none(), "opp side absent for PvP");
                // digitama_library_order defaults to empty when omitted.
                assert!(s.my.digitama_library_order.is_empty());
                assert_eq!(s.memory, None);
            }
            other => panic!("expected InitialState, got {:?}", other),
        }
    }

    #[test]
    fn recording_without_initial_state_row_still_parses() {
        // Today's corpus shape — no initial_state row at all.
        let r = parse_jsonl(well_formed_jsonl()).expect("parse");
        assert!(
            !r.rows.iter().any(|row| matches!(row, Row::InitialState(_))),
            "well_formed_jsonl fixture carries no initial_state row"
        );
    }

    // ── memory perspective conversion (pinned both directions, both seats) ──

    #[test]
    fn perspective_conversion_my_player_id_0_favored() {
        // memory_pair favors player 0, recording is player 0's own —
        // no sign flip.
        assert_eq!(memory_from_recording_perspective(5, 0, 0), 5);
    }

    #[test]
    fn perspective_conversion_my_player_id_0_not_favored() {
        // memory_pair favors player 1, recording is player 0's own —
        // sign flips: what favors the engine's active side (P1) reads as
        // negative (unfavorable) from P0's fixed perspective.
        assert_eq!(memory_from_recording_perspective(5, 1, 0), -5);
    }

    #[test]
    fn perspective_conversion_my_player_id_1_favored() {
        // memory_pair favors player 1, recording is player 1's own —
        // no sign flip.
        assert_eq!(memory_from_recording_perspective(-7, 1, 1), -7);
    }

    #[test]
    fn perspective_conversion_my_player_id_1_not_favored() {
        // memory_pair favors player 0, recording is player 1's own —
        // sign flips.
        assert_eq!(memory_from_recording_perspective(-7, 0, 1), 7);
    }
}
