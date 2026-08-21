//! Verification replay corpus helpers.
//!
//! This module builds the canonical `*.replay.json` payloads used by the
//! verification ladder. It keeps generated games self-contained with inline
//! deck refs so the replay adapter can verify them without a deck-library
//! resolver.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::card_data::CardData;
use crate::enums::PlayerId;
use crate::policies::{GreedyPolicy, Policy};
use crate::recorder::{
    cards_json_content_hash, VerificationReplayAction, VerificationReplayRecording,
};
use crate::runners::replay::VerificationReplayReport;
use crate::runners::replay::{Divergence, DivergenceKind, DivergenceReport, ReplaySession};
use crate::runners::HeadlessRunner;

/// Default number of generated games for the tier-2 golden replay corpus.
pub const DEFAULT_TIER2_REPLAY_CORPUS_GAMES: usize = 50;
/// Default action budget per generated game. Partial games are still useful
/// replay assets and keep tier-2 wall time bounded.
pub const DEFAULT_REPLAY_CORPUS_MAX_STEPS_PER_GAME: u32 = 256;
pub const DEFAULT_REPLAY_CORPUS_BASE_SEED: u64 = 20_260_708;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayCorpusConfig {
    pub game_count: usize,
    pub max_steps_per_game: u32,
    pub base_seed: u64,
}

impl Default for ReplayCorpusConfig {
    fn default() -> Self {
        Self {
            game_count: DEFAULT_TIER2_REPLAY_CORPUS_GAMES,
            max_steps_per_game: DEFAULT_REPLAY_CORPUS_MAX_STEPS_PER_GAME,
            base_seed: DEFAULT_REPLAY_CORPUS_BASE_SEED,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayCorpus {
    pub entries: Vec<ReplayCorpusEntry>,
    pub skips: Vec<ReplayCorpusSkip>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayCorpusEntry {
    pub name: String,
    pub recording: VerificationReplayRecording,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayCorpusSkip {
    pub id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionOutcome {
    Converted(VerificationReplayRecording),
    Skipped(ReplayCorpusSkip),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlessOutcome {
    Blessed(VerificationReplayRecording),
    Retired(ReplayCorpusRetirement),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayCorpusRetirement {
    pub source_file: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlessReplayCorpusSummary {
    pub updated: usize,
    pub unchanged: usize,
    pub retired: Vec<ReplayCorpusRetirement>,
}

#[derive(Debug, Clone)]
pub struct VerifyReplayCorpusSummary {
    pub checked: usize,
    pub divergent: Vec<VerificationReplayReport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayCorpusError {
    Json(String),
    Engine(String),
    Io(String),
    NoImplementedDecks,
}

impl fmt::Display for ReplayCorpusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(msg) => write!(f, "json error: {msg}"),
            Self::Engine(msg) => write!(f, "engine error: {msg}"),
            Self::Io(msg) => write!(f, "io error: {msg}"),
            Self::NoImplementedDecks => {
                write!(f, "deck_library contains no fully implemented decks")
            }
        }
    }
}

impl std::error::Error for ReplayCorpusError {}

/// Generate deterministic greedy-vs-greedy canonical replay recordings from
/// deck-library decks whose card IDs are all present in both `card_data` and
/// the supplied implemented-card set.
pub fn generate_greedy_replay_corpus(
    deck_library_json: &str,
    cards_json_bytes: &[u8],
    card_data: &HashMap<String, CardData>,
    implemented_card_ids: &HashSet<String>,
    config: ReplayCorpusConfig,
) -> Result<ReplayCorpus, ReplayCorpusError> {
    let value: Value = serde_json::from_str(deck_library_json)
        .map_err(|err| ReplayCorpusError::Json(err.to_string()))?;
    let (decks, mut skips) = implemented_deck_candidates(&value, card_data, implemented_card_ids)?;
    if decks.is_empty() {
        return Err(ReplayCorpusError::NoImplementedDecks);
    }

    let cards_hash = cards_json_content_hash(cards_json_bytes);
    let mut entries = Vec::new();
    for idx in 0..config.game_count {
        let player1 = &decks[idx % decks.len()];
        let player2 = &decks[(idx + 1) % decks.len()];
        let seed = seat_balanced_seed(config.base_seed, idx);
        let recording = record_greedy_game(
            player1.deck.clone(),
            player2.deck.clone(),
            card_data,
            seed,
            config.max_steps_per_game,
            cards_hash.clone(),
        )?;
        entries.push(ReplayCorpusEntry {
            name: format!(
                "generated_greedy_{idx:03}_{}_vs_{}_seed_{seed}.replay.json",
                slug(&player1.deck_id),
                slug(&player2.deck_id)
            ),
            recording,
        });
    }
    skips.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.reason.cmp(&b.reason)));
    Ok(ReplayCorpus { entries, skips })
}

/// Convert a legacy training-recording artifact into the canonical
/// verification replay format by reconstructing the game from its runner seed,
/// original deck lists, and action IDs. Artifacts missing any reconstructible
/// field are reported as skips instead of guessed.
pub fn convert_training_recording_artifact(
    artifact: &Value,
    cards_hash: &str,
    card_data: &HashMap<String, CardData>,
) -> Result<ConversionOutcome, ReplayCorpusError> {
    let Some(seed) = find_seed(artifact) else {
        return Ok(ConversionOutcome::Skipped(skip(
            "training-recording",
            "missing runner seed",
        )));
    };
    let recording = artifact.get("recording").unwrap_or(artifact);
    let Some(initial) = recording.get("initial_state").and_then(Value::as_object) else {
        return Ok(ConversionOutcome::Skipped(skip(
            "training-recording",
            "missing initial_state",
        )));
    };
    let Some(player1) = initial.get("player1") else {
        return Ok(ConversionOutcome::Skipped(skip(
            "training-recording",
            "missing player1 initial state",
        )));
    };
    let Some(player2) = initial.get("player2") else {
        return Ok(ConversionOutcome::Skipped(skip(
            "training-recording",
            "missing player2 initial state",
        )));
    };
    let Some(deck1) = string_array(player1.get("deck_list")) else {
        return Ok(ConversionOutcome::Skipped(skip(
            "training-recording",
            "missing player1 deck_list",
        )));
    };
    let Some(deck2) = string_array(player2.get("deck_list")) else {
        return Ok(ConversionOutcome::Skipped(skip(
            "training-recording",
            "missing player2 deck_list",
        )));
    };
    let Some(actions) = recording.get("actions").and_then(Value::as_array) else {
        return Ok(ConversionOutcome::Skipped(skip(
            "training-recording",
            "missing actions",
        )));
    };
    let parsed_actions = match parse_training_actions(actions) {
        Ok(actions) => actions,
        Err(reason) => {
            return Ok(ConversionOutcome::Skipped(skip(
                "training-recording",
                reason,
            )))
        }
    };

    let mut runner = HeadlessRunner::new(
        deck1.clone(),
        deck2.clone(),
        card_data,
        false,
        true,
        false,
        Some(seed),
    )
    .map_err(ReplayCorpusError::Engine)?;
    for (idx, action) in parsed_actions.iter().enumerate() {
        let expected = runner.current_decision_player();
        if expected != action.seat {
            return Ok(ConversionOutcome::Skipped(skip(
                "training-recording",
                format!(
                    "actor mismatch at step {}: recording seat {}, replay seat {}",
                    idx, action.seat, expected
                ),
            )));
        }
        let mask = runner.get_action_mask();
        if action.action_id as usize >= mask.len() || mask[action.action_id as usize] <= 0.0 {
            return Ok(ConversionOutcome::Skipped(skip(
                "training-recording",
                format!("illegal action {} at step {}", action.action_id, idx),
            )));
        }
        runner.step(action.action_id);
    }

    let Some(converted) = runner.get_verification_replay_recording(cards_hash.to_string()) else {
        return Ok(ConversionOutcome::Skipped(skip(
            "training-recording",
            "recording was not enabled during conversion",
        )));
    };
    if converted.actions.len() != parsed_actions.len() {
        return Ok(ConversionOutcome::Skipped(skip(
            "training-recording",
            format!(
                "replayed action count {} did not match source action count {}",
                converted.actions.len(),
                parsed_actions.len()
            ),
        )));
    }
    Ok(ConversionOutcome::Converted(converted))
}

/// Rebuild the verification digest stream for a recording whose action path is
/// still reconstructible under the current engine. Actor/mask mismatches are
/// treated as retirement candidates, never force-blessed.
pub fn bless_verification_recording(
    recording: VerificationReplayRecording,
    card_data: &HashMap<String, CardData>,
) -> Result<BlessOutcome, ReplayCorpusError> {
    let mut replayable = recording.clone();
    if replayable.actions.len() != replayable.digests.len() {
        replayable.digests = vec![0; replayable.actions.len()];
    }

    let mut session = match ReplaySession::from_verification_recording(replayable, card_data, false)
    {
        Ok(session) => session,
        Err(err) => {
            return Ok(BlessOutcome::Retired(retire(format!(
                "unreconstructible replay: {err}"
            ))))
        }
    };

    let mut digests = Vec::with_capacity(recording.actions.len());
    while !session.is_complete() && !session.is_game_over() && !session.is_paused() {
        let result = session.step();
        if let Some(divergence) = session.divergences().first() {
            return Ok(BlessOutcome::Retired(retire(describe_divergence(
                divergence,
            ))));
        }
        if let Some(divergence) = result.divergences.first() {
            return Ok(BlessOutcome::Retired(retire(describe_step_divergence(
                divergence,
            ))));
        }
        digests.push(session.game.verification_digest());
    }

    if let Some(divergence) = session.divergences().first() {
        return Ok(BlessOutcome::Retired(retire(describe_divergence(
            divergence,
        ))));
    }

    if !session.is_complete() {
        let remaining = session.total_steps().saturating_sub(session.current_step());
        let reason = if session.is_game_over() {
            format!(
                "recording has {remaining} unapplied action(s) after terminal game state at step {}",
                session.current_step()
            )
        } else if session.is_paused() {
            format!(
                "replay paused before completion at step {}",
                session.current_step()
            )
        } else {
            format!(
                "replay stopped before completion at step {} of {}",
                session.current_step(),
                session.total_steps()
            )
        };
        return Ok(BlessOutcome::Retired(retire(reason)));
    }

    let mut blessed = recording;
    blessed.digests = digests;
    blessed.final_digest = blessed
        .digests
        .last()
        .copied()
        .unwrap_or_else(|| session.game.verification_digest());
    Ok(BlessOutcome::Blessed(blessed))
}

/// Bless every `*.replay.json` in a flat corpus directory. Reconstructible
/// recordings are rewritten with canonical pretty JSON when their digest stream
/// changes. Unreconstructible recordings are removed from the active corpus
/// only after a `.retired.json` review record has been written.
pub fn bless_replay_corpus_dir(
    goldens_dir: &Path,
    retired_dir: &Path,
    card_data: &HashMap<String, CardData>,
) -> Result<BlessReplayCorpusSummary, ReplayCorpusError> {
    let files = replay_json_files(goldens_dir)?;

    let mut summary = BlessReplayCorpusSummary {
        updated: 0,
        unchanged: 0,
        retired: Vec::new(),
    };

    for path in files {
        let text = fs::read_to_string(&path)
            .map_err(|err| ReplayCorpusError::Io(format!("read {}: {err}", path.display())))?;
        let recording: VerificationReplayRecording = serde_json::from_str(&text)
            .map_err(|err| ReplayCorpusError::Json(format!("parse {}: {err}", path.display())))?;
        match bless_verification_recording(recording.clone(), card_data)? {
            BlessOutcome::Blessed(blessed) => {
                let new_text = pretty_recording(&blessed)?;
                if text != new_text {
                    fs::write(&path, &new_text).map_err(|err| {
                        ReplayCorpusError::Io(format!("write {}: {err}", path.display()))
                    })?;
                    summary.updated += 1;
                } else {
                    summary.unchanged += 1;
                }
            }
            BlessOutcome::Retired(retired) => {
                let source_file = file_name_string(&path);
                fs::create_dir_all(retired_dir).map_err(|err| {
                    ReplayCorpusError::Io(format!("create {}: {err}", retired_dir.display()))
                })?;
                let retired_path = retired_dir.join(retired_file_name(&path));
                let reason = retired.reason;
                let retired_text = serde_json::to_string_pretty(&RetiredReplayFile {
                    source_file: &source_file,
                    reason: &reason,
                    recording: &recording,
                })
                .map_err(|err| ReplayCorpusError::Json(err.to_string()))?;
                fs::write(&retired_path, format!("{retired_text}\n")).map_err(|err| {
                    ReplayCorpusError::Io(format!("write {}: {err}", retired_path.display()))
                })?;
                fs::remove_file(&path).map_err(|err| {
                    ReplayCorpusError::Io(format!("remove {}: {err}", path.display()))
                })?;
                summary.retired.push(ReplayCorpusRetirement {
                    source_file: Some(source_file),
                    reason,
                });
            }
        }
    }

    Ok(summary)
}

/// Verify every active `*.replay.json` in a flat corpus directory without
/// mutating it. A divergence is reported with the first failed check for that
/// game; callers decide whether to fail immediately or aggregate.
pub fn verify_replay_corpus_dir(
    goldens_dir: &Path,
    card_data: &HashMap<String, CardData>,
) -> Result<VerifyReplayCorpusSummary, ReplayCorpusError> {
    let files = replay_json_files(goldens_dir)?;
    if files.is_empty() {
        return Err(ReplayCorpusError::Json(format!(
            "{} contains no *.replay.json files",
            goldens_dir.display()
        )));
    }

    let mut summary = VerifyReplayCorpusSummary {
        checked: 0,
        divergent: Vec::new(),
    };
    for path in files {
        let text = fs::read_to_string(&path)
            .map_err(|err| ReplayCorpusError::Io(format!("read {}: {err}", path.display())))?;
        let recording: VerificationReplayRecording = serde_json::from_str(&text)
            .map_err(|err| ReplayCorpusError::Json(format!("parse {}: {err}", path.display())))?;
        let game_id = file_name_string(&path);
        let report = ReplaySession::verify_verification_recording(game_id, recording, card_data)
            .map_err(|err| ReplayCorpusError::Engine(err.to_string()))?;
        summary.checked += 1;
        if report.divergence.is_some() {
            summary.divergent.push(report);
        }
    }
    Ok(summary)
}

pub fn write_replay_corpus_entries(
    output_dir: &Path,
    entries: &[ReplayCorpusEntry],
) -> Result<Vec<PathBuf>, ReplayCorpusError> {
    fs::create_dir_all(output_dir)
        .map_err(|err| ReplayCorpusError::Io(format!("create {}: {err}", output_dir.display())))?;
    let mut paths = Vec::with_capacity(entries.len());
    for entry in entries {
        let path = output_dir.join(&entry.name);
        let text = serde_json::to_string_pretty(&entry.recording)
            .map_err(|err| ReplayCorpusError::Json(err.to_string()))?;
        fs::write(&path, format!("{text}\n"))
            .map_err(|err| ReplayCorpusError::Io(format!("write {}: {err}", path.display())))?;
        paths.push(path);
    }
    Ok(paths)
}

fn replay_json_files(goldens_dir: &Path) -> Result<Vec<PathBuf>, ReplayCorpusError> {
    let mut files = Vec::new();
    for entry in fs::read_dir(goldens_dir)
        .map_err(|err| ReplayCorpusError::Io(format!("read {}: {err}", goldens_dir.display())))?
    {
        let path = entry
            .map_err(|err| {
                ReplayCorpusError::Io(format!("read {} entry: {err}", goldens_dir.display()))
            })?
            .path();
        if path.is_file() && is_replay_json_path(&path) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

#[derive(Serialize)]
struct RetiredReplayFile<'a> {
    source_file: &'a str,
    reason: &'a str,
    recording: &'a VerificationReplayRecording,
}

fn record_greedy_game(
    deck1: Vec<String>,
    deck2: Vec<String>,
    card_data: &HashMap<String, CardData>,
    seed: u64,
    max_steps: u32,
    cards_hash: String,
) -> Result<VerificationReplayRecording, ReplayCorpusError> {
    let mut runner = HeadlessRunner::new(deck1, deck2, card_data, false, true, false, Some(seed))
        .map_err(ReplayCorpusError::Engine)?;
    let mut player1 = GreedyPolicy::new();
    let mut player2 = GreedyPolicy::new();

    for _ in 0..max_steps {
        if runner.is_game_over() {
            break;
        }
        let mask = runner.get_action_mask();
        let action = if runner.current_decision_player() == 0 {
            player1.select(runner.game(), &mask)
        } else {
            player2.select(runner.game(), &mask)
        };
        runner.step(action);
    }

    runner
        .get_verification_replay_recording(cards_hash)
        .ok_or_else(|| ReplayCorpusError::Engine("recording was not enabled".to_string()))
}

fn implemented_deck_candidates(
    deck_library: &Value,
    card_data: &HashMap<String, CardData>,
    implemented_card_ids: &HashSet<String>,
) -> Result<(Vec<DeckCandidate>, Vec<ReplayCorpusSkip>), ReplayCorpusError> {
    let archetypes = deck_library
        .get("archetypes")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            ReplayCorpusError::Json("deck_library missing archetypes object".to_string())
        })?;
    let mut rows = Vec::new();
    let mut skips = Vec::new();

    for (archetype_name, archetype) in archetypes {
        let meta_share = archetype
            .pointer("/stats/meta_share")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let Some(decklists) = archetype.get("decklists").and_then(Value::as_array) else {
            continue;
        };
        for decklist in decklists {
            let deck_id = decklist
                .get("deck_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let deck = match parse_decklist(decklist.get("decklist")) {
                Ok(deck) => deck,
                Err(reason) => {
                    skips.push(ReplayCorpusSkip {
                        id: deck_id,
                        reason,
                    });
                    continue;
                }
            };
            if let Some(missing) = deck.iter().find(|id| !card_data.contains_key(*id)) {
                skips.push(ReplayCorpusSkip {
                    id: deck_id,
                    reason: format!("card {missing} missing from cards.json"),
                });
                continue;
            }
            if let Some(unimplemented) = deck.iter().find(|id| !implemented_card_ids.contains(*id))
            {
                skips.push(ReplayCorpusSkip {
                    id: deck_id,
                    reason: format!("card {unimplemented} has no registered effect"),
                });
                continue;
            }
            rows.push(DeckCandidate {
                archetype_name: archetype_name.clone(),
                deck_id,
                deck,
                meta_share,
            });
        }
    }

    rows.sort_by(|a, b| {
        b.meta_share
            .partial_cmp(&a.meta_share)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.archetype_name.cmp(&b.archetype_name))
            .then_with(|| a.deck_id.cmp(&b.deck_id))
    });
    Ok((rows, skips))
}

fn parse_decklist(value: Option<&Value>) -> Result<Vec<String>, String> {
    match value {
        Some(Value::String(raw)) => serde_json::from_str::<Vec<String>>(raw)
            .map_err(|err| format!("decklist string is not a JSON card-id array: {err}")),
        Some(Value::Array(_)) => string_array(value)
            .ok_or_else(|| "decklist array must contain only string card IDs".to_string()),
        Some(_) => Err("decklist must be a JSON string or array".to_string()),
        None => Err("missing decklist".to_string()),
    }
}

fn parse_training_actions(actions: &[Value]) -> Result<Vec<VerificationReplayAction>, String> {
    let mut out = Vec::with_capacity(actions.len());
    for (idx, action) in actions.iter().enumerate() {
        let player_id = action
            .get("player_id")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("action {idx} missing player_id"))?;
        let seat: PlayerId = match player_id {
            0 => 0,
            1 => 0,
            2 => 1,
            other => return Err(format!("action {idx} has invalid player_id {other}")),
        };
        let raw_action = action
            .get("action_id")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("action {idx} missing action_id"))?;
        if raw_action > u16::MAX as u64 {
            return Err(format!("action {idx} action_id {raw_action} exceeds u16"));
        }
        out.push(VerificationReplayAction {
            seat,
            action_id: raw_action as u16,
        });
    }
    Ok(out)
}

fn find_seed(artifact: &Value) -> Option<u64> {
    [
        "/run/runner_seed",
        "/run/game_seed",
        "/run/seed",
        "/recording/seed",
        "/seed",
    ]
    .iter()
    .find_map(|path| artifact.pointer(path).and_then(Value::as_u64))
}

fn string_array(value: Option<&Value>) -> Option<Vec<String>> {
    value.and_then(Value::as_array).and_then(|items| {
        items
            .iter()
            .map(|item| item.as_str().map(str::to_string))
            .collect()
    })
}

fn seat_balanced_seed(base_seed: u64, index: usize) -> u64 {
    2 * (base_seed + index as u64) + (index as u64 % 2)
}

fn skip(id: impl Into<String>, reason: impl Into<String>) -> ReplayCorpusSkip {
    ReplayCorpusSkip {
        id: id.into(),
        reason: reason.into(),
    }
}

fn retire(reason: impl Into<String>) -> ReplayCorpusRetirement {
    ReplayCorpusRetirement {
        source_file: None,
        reason: reason.into(),
    }
}

fn describe_divergence(divergence: &Divergence) -> String {
    match &divergence.kind {
        DivergenceKind::MaskMiss { sample_legal_ids } => format!(
            "illegal action {} by seat {} at step {} in phase {} (sample legal action ids: {:?})",
            divergence.action_id,
            divergence.actor,
            divergence.step,
            divergence.phase,
            sample_legal_ids
        ),
        DivergenceKind::Actor { expected, recorded } => format!(
            "actor mismatch at step {}: recording seat {}, replay seat {}",
            divergence.step, recorded, expected
        ),
        DivergenceKind::Memory {
            recorded,
            replayed,
            my_player_id,
        } => format!(
            "memory mismatch @ step {}: recorded {} (P{} perspective), engine {}",
            divergence.step, recorded, my_player_id, replayed
        ),
        DivergenceKind::Phase { recorded, replayed } => format!(
            "phase divergence at step {}: recorded {}, replayed {}",
            divergence.step, recorded, replayed
        ),
        DivergenceKind::Winner { recorded, replayed } => format!(
            "winner divergence at step {}: recorded {:?}, replayed {:?}",
            divergence.step, recorded, replayed
        ),
        DivergenceKind::RevealKind { message } => {
            format!(
                "reveal-kind divergence at step {}: {message}",
                divergence.step
            )
        }
        DivergenceKind::RevealExhausted { message } => format!(
            "reveal-exhausted divergence at step {}: {message}",
            divergence.step
        ),
        DivergenceKind::SelectionResolution { reason } => format!(
            "selection-resolution divergence at step {}: {reason}",
            divergence.step
        ),
    }
}

fn describe_step_divergence(divergence: &DivergenceReport) -> String {
    format!(
        "replay divergence at step {} for {}: recorded {}, replayed {}",
        divergence.step, divergence.field, divergence.recorded, divergence.replayed
    )
}

fn pretty_recording(recording: &VerificationReplayRecording) -> Result<String, ReplayCorpusError> {
    serde_json::to_string_pretty(recording)
        .map(|text| format!("{text}\n"))
        .map_err(|err| ReplayCorpusError::Json(err.to_string()))
}

fn is_replay_json_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".replay.json"))
}

fn retired_file_name(path: &Path) -> String {
    let name = file_name_string(path);
    if let Some(stem) = name.strip_suffix(".replay.json") {
        format!("{stem}.retired.json")
    } else {
        format!("{}.retired.json", slug(&name))
    }
}

fn file_name_string(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("recording.replay.json")
        .to_string()
}

fn slug(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "unknown".to_string()
    } else {
        trimmed.to_string()
    }
}

#[derive(Debug, Clone)]
struct DeckCandidate {
    archetype_name: String,
    deck_id: String,
    deck: Vec<String>,
    meta_share: f64,
}
