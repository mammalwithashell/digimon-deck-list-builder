//! `dcgo-replay` — replay DCGO JSONL recordings through `digimon-engine` and
//! validate parity.
//!
//! Phase 1 scope:
//!  - Consumes JSONL recordings produced by the DCGO mod's `GameRecorder`
//!    (see `DCGO/Assets/Scripts/Script/Recording/GameRecorder.cs`).
//!  - Replays the action stream through `HeadlessRunner`, asserting mask
//!    legality at each step and winner-match at game-end.
//!  - Reports failures as one of three structured kinds (illegal action,
//!    winner mismatch, actor mismatch). Engine errors surface as a fourth.
//!  - Aggregates failures across a corpus into a parity report keyed by
//!    failure kind and by card identity (best-effort attribution).
//!
//! Out of scope (Phase 2/3):
//!  - Opaque-opponent-deck recordings (`opp_deck_post_shuffle == null`).
//!    These require the engine's opaque-deck mode, which is in tasks 6.x.
//!  - Per-prompt selection encoding. The DCGO recorder currently emits
//!    `encoder_failure` rows for every selection (see task 3.5 fallback);
//!    the harness halts cleanly with a `PartialPass` when it hits one,
//!    reporting the step it stopped at.

pub mod recording;
pub mod replay;
pub mod report;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use digimon_engine::card_data::CardData;

pub use recording::{
    parse_jsonl, GameEnd, GameStart, RecordingV1, Row, SchemaError, SUPPORTED_SCHEMA_VERSION,
};
pub use replay::{
    replay_recording, ActorMismatch, IllegalAction, ReplayConfig, ReplayFail, ReplayOutcome,
    WinnerMismatch,
};
pub use report::{aggregate, ParityReport, PerCardEntry};

/// Load `data/cards.json` (or the given path) into the engine's card-data
/// table. `None` triggers the same upward-search-from-CWD behavior the CLI
/// has always had; callers that already know the path should use
/// [`load_card_data_at`] instead.
pub fn load_card_data(cards_json: Option<&Path>) -> Result<HashMap<String, CardData>, String> {
    let path = match cards_json {
        Some(p) => p.to_path_buf(),
        None => default_cards_json_path()
            .ok_or_else(|| "no --cards-json provided and no data/cards.json found".to_string())?,
    };
    load_card_data_at(&path)
}

/// Load card data from a known path. This is the primitive `load_card_data`
/// builds on; use it directly when the caller already has a concrete
/// `cards.json` path (e.g. `dcgo-harness triage --cards-json ...`) and has no
/// use for the CWD-search fallback.
pub fn load_card_data_at(path: impl AsRef<Path>) -> Result<HashMap<String, CardData>, String> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|e| format!("reading {}: {}", path.display(), e))?;
    let text =
        std::str::from_utf8(&bytes).map_err(|e| format!("cards.json is not valid UTF-8: {}", e))?;
    CardData::load_from_str(text).map_err(|e| format!("parsing cards.json: {}", e))
}

/// Walk up from CWD looking for `data/cards.json`. Mirrors the engine CLI's
/// resolver so the harness works without flags in a developer's typical
/// repo-root invocation.
pub fn default_cards_json_path() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    for _ in 0..6 {
        let candidate = dir.join("data").join("cards.json");
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

/// Collect every `.jsonl` file under `path` (one level deep) or the single
/// file at `path`, in a deterministic (sorted) order.
///
/// Shared by both `dcgo-replay` (single/directory replay) and
/// `dcgo-harness triage` (corpus triage) so there is exactly one definition
/// of "what counts as a recording in this corpus" and one guarantee about
/// its ordering. The sort is not just tidiness: `dcgo-harness triage`
/// attributes each divergence cluster to the FIRST recording that exhibited
/// it, so an unsorted `read_dir` order would make a cluster's "repro" line
/// flip between runs of an otherwise-identical corpus.
pub fn collect_recording_paths(path: &Path) -> Result<Vec<PathBuf>, String> {
    if !path.exists() {
        return Err(format!("input path does not exist: {}", path.display()));
    }
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }

    // Directory mode — one level deep. We don't recurse arbitrarily so a
    // user can co-locate misc files next to a `recordings/` subdir without
    // surprises.
    let mut out = Vec::new();
    let entries = fs::read_dir(path).map_err(|e| format!("read_dir {}: {}", path.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry: {}", e))?;
        let p = entry.path();
        if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            out.push(p);
        }
    }
    out.sort(); // deterministic processing order
    Ok(out)
}
