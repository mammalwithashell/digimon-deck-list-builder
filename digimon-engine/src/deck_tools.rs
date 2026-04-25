//! Deck list parsing, validation, and tested-cards allowlist.
//!
//! Port of `digimon_gym/engine/data/deck_loader.py` and
//! `digimon_gym/engine/data/tested_cards.py` — behaviour must stay
//! byte-for-byte compatible so the desktop app's deck UX matches what the
//! hosted API accepts. The authoritative data files live under `data/`
//! at repo root and are `include_str!`d at compile time so both engines
//! read the exact same bytes.
//!
//! Responsibilities:
//! - Parse TTS (JSON array) + digimoncard.io text deck exports
//! - Validate deck size, copy limits, banned/restricted list, choice groups
//! - Expose the alpha-release "tested cards" allowlist used as a gate on
//!   out-of-scope card IDs

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;

use serde::Deserialize;

// Data files — `include_str!` bakes the bytes into the crate, so there's no
// separate resource-bundling step for desktop builds.
const CARDS_JSON: &str = include_str!("../../data/cards.json");
const TESTED_CARDS_JSON: &str = include_str!("../../data/tested_cards.json");

/// Minimal card metadata — only the fields validation + parsing touch.
/// The full cards.json entry has many more keys (DP, effect text, etc.);
/// ignoring them via `#[serde(default)]` keeps deserialization resilient
/// to future additions.
#[derive(Debug, Clone)]
pub struct CardSummary {
    pub card_id: String,
    pub card_name_eng: String,
    /// Matches Python `CardKind`: 0=Digimon, 1=Tamer, 2=Option, 3=DigiEgg.
    pub card_kind: u8,
    /// Max copies allowed per deck. Defaults to 4; a handful of cards
    /// override it in the data file (e.g. starter-deck restrictions).
    pub max_count_in_deck: u32,
}

#[derive(Deserialize)]
struct CardEntryRaw {
    card_id: String,
    #[serde(default)]
    card_name_eng: String,
    card_kind: u8,
    #[serde(default = "default_max_count")]
    max_count_in_deck: u32,
}

fn default_max_count() -> u32 {
    4
}

#[derive(Deserialize)]
struct TestedCardsFile {
    card_ids: Vec<String>,
}

/// Lazily-parsed card database — keyed by card_id, mirrors
/// `CardDatabase.get_card()` lookups in Python.
pub fn card_database() -> &'static HashMap<String, CardSummary> {
    static CELL: OnceLock<HashMap<String, CardSummary>> = OnceLock::new();
    CELL.get_or_init(|| {
        let raw: HashMap<String, CardEntryRaw> = serde_json::from_str(CARDS_JSON)
            .expect("cards.json is malformed (compiled-in resource)");
        raw.into_iter()
            .map(|(k, v)| {
                (
                    k,
                    CardSummary {
                        card_id: v.card_id,
                        card_name_eng: v.card_name_eng,
                        card_kind: v.card_kind,
                        max_count_in_deck: v.max_count_in_deck,
                    },
                )
            })
            .collect()
    })
}

/// Full alpha allowlist as a set for O(1) membership checks.
pub fn tested_cards_set() -> &'static HashSet<String> {
    static CELL: OnceLock<HashSet<String>> = OnceLock::new();
    CELL.get_or_init(|| {
        let f: TestedCardsFile = serde_json::from_str(TESTED_CARDS_JSON)
            .expect("tested_cards.json is malformed (compiled-in resource)");
        f.card_ids.into_iter().collect()
    })
}

/// Sorted allowlist — matches the `list_tested_cards()` endpoint shape,
/// which Python sorts before returning so the UI renders deterministically.
pub fn tested_cards_sorted() -> Vec<String> {
    let mut ids: Vec<String> = tested_cards_set().iter().cloned().collect();
    ids.sort();
    ids
}

pub fn is_card_tested(card_id: &str) -> bool {
    tested_cards_set().contains(card_id)
}

/// Distinct card IDs from `card_ids` that are NOT on the tested allowlist.
/// Order is first-seen and duplicates are collapsed — matches Python's
/// `out_of_set_cards` so per-card error messages line up.
pub fn out_of_set_cards<I, S>(card_ids: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let tested = tested_cards_set();
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for cid in card_ids {
        let cid = cid.as_ref();
        if tested.contains(cid) || seen.contains(cid) {
            continue;
        }
        seen.insert(cid.to_string());
        out.push(cid.to_string());
    }
    out
}

// ─── Restricted list ───────────────────────────────────────────────────
//
// Source: https://digimoncard.io/restricted-list. Must stay in sync with
// `deck_loader.py::_BANNED / _RESTRICTED / _CHOICE_GROUPS`. Keeping the
// entries in the same order as the Python file makes it obvious when one
// side drifts out of step.

const BANNED_CARDS: &[&str] = &[
    "BT2-090",   // Matt Ishida
    "BT5-109",   // Mega Digimon Fusion!
    "EX5-065",   // Sayo & Koh
];

const RESTRICTED_CARDS: &[&str] = &[
    "BT1-090",  "BT10-009", "BT11-033", "BT11-064", "BT13-012", "BT13-110",
    "BT14-002", "BT14-084", "BT15-057", "BT15-102", "BT16-011", "BT17-069",
    "BT19-040", "BT2-047",  "BT2-069",  "BT3-054",  "BT3-103",  "BT4-104",
    "BT4-111",  "BT6-100",  "BT6-104",  "BT7-038",  "BT7-064",  "BT7-069",
    "BT7-072",  "BT7-107",  "BT9-098",  "BT9-099",  "EX1-021",  "EX1-068",
    "EX2-039",  "EX2-070",  "EX3-057",  "EX4-006",  "EX4-019",  "EX4-030",
    "EX5-015",  "EX5-018",  "EX5-062",  "P-008",    "P-025",    "P-029",
    "P-030",    "P-123",    "P-130",    "ST2-13",   "ST9-09",
];

/// Choice groups: cards from side A and side B can't coexist in the same
/// deck. Currently two groups:
///   1. Mother D-Reaper vs Shoto Kazama
///   2. Chaosmon: Valdur Arm vs Taomon + Sakuyamon (X Antibody)
const CHOICE_GROUPS: &[(&[&str], &[&str])] = &[
    (&["EX2-007"], &["EX7-064"]),
    (&["BT20-037"], &["BT17-035", "EX8-037"]),
];

fn card_limit(card_id: &str) -> Option<u32> {
    if BANNED_CARDS.contains(&card_id) {
        return Some(0);
    }
    if RESTRICTED_CARDS.contains(&card_id) {
        return Some(1);
    }
    None
}

// ─── Parsers ───────────────────────────────────────────────────────────

/// Matches standard Digimon TCG card IDs: BT24-017, P-103, LM-027, ST1-01,
/// EX8-037. Regex equivalent: `^[A-Z]{1,3}\d*-\d+$`. Hand-rolled so we
/// don't pull in a regex crate just for one pattern.
fn is_card_id(s: &str) -> bool {
    let bytes = s.as_bytes();
    let len = bytes.len();
    if len < 3 {
        return false;
    }
    let mut i = 0;
    // 1..=3 uppercase ASCII letters
    let letters_start = i;
    while i < len && bytes[i].is_ascii_uppercase() {
        i += 1;
    }
    let letters_count = i - letters_start;
    if !(1..=3).contains(&letters_count) {
        return false;
    }
    // Zero or more digits
    while i < len && bytes[i].is_ascii_digit() {
        i += 1;
    }
    // Required hyphen
    if i >= len || bytes[i] != b'-' {
        return false;
    }
    i += 1;
    // One or more digits to end
    if i >= len {
        return false;
    }
    while i < len {
        if !bytes[i].is_ascii_digit() {
            return false;
        }
        i += 1;
    }
    true
}

/// Parse a TTS (Tabletop Simulator) deck export — JSON array of card ID
/// strings. Non-card-ID entries (e.g. export headers) are filtered out.
pub fn parse_tts(raw: &str) -> Result<Vec<String>, String> {
    let data: serde_json::Value = serde_json::from_str(raw)
        .map_err(|e| format!("Invalid TTS JSON: {e}"))?;
    let arr = data
        .as_array()
        .ok_or_else(|| "TTS format expects a JSON array".to_string())?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        if let Some(s) = item.as_str() {
            if is_card_id(s) {
                out.push(s.to_string());
            }
        }
    }
    Ok(out)
}

/// Parse a digimoncard.io text deck export (lines like `4 Medusamon
/// BT24-017`). Comments (starting with `//`) are skipped. Lines whose
/// first token is not a number or whose last token isn't a valid card ID
/// are silently ignored — matches Python's permissive parser.
pub fn parse_text(raw: &str) -> Result<Vec<String>, String> {
    let mut card_ids = Vec::new();
    for raw_line in raw.trim().split('\n') {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 2 {
            continue;
        }
        let Ok(quantity) = tokens[0].parse::<u32>() else {
            continue;
        };
        let card_id = tokens[tokens.len() - 1];
        if !is_card_id(card_id) {
            continue;
        }
        for _ in 0..quantity {
            card_ids.push(card_id.to_string());
        }
    }
    if card_ids.is_empty() {
        return Err("No valid card entries found in text format input".to_string());
    }
    Ok(card_ids)
}

/// Auto-detect format and parse a deck list string. Tries TTS first when
/// the input starts with `[`, falls back to text. Error message mirrors
/// Python's so error-display code on the frontend doesn't branch.
pub fn parse_deck(raw: &str) -> Result<Vec<String>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("Empty deck string".to_string());
    }
    if raw.starts_with('[') {
        if let Ok(ids) = parse_tts(raw) {
            return Ok(ids);
        }
    }
    if let Ok(ids) = parse_text(raw) {
        return Ok(ids);
    }
    Err(
        "Could not parse deck list. Expected either:\n  \
         - TTS format: JSON array like [\"BT24-017\", \"BT24-017\", ...]\n  \
         - Text format: lines like '4 Medusamon BT24-017'"
            .to_string(),
    )
}

pub fn summarize_deck(card_ids: &[String]) -> HashMap<String, u32> {
    let mut counts: HashMap<String, u32> = HashMap::new();
    for cid in card_ids {
        *counts.entry(cid.clone()).or_insert(0) += 1;
    }
    counts
}

// ─── Validation ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct DeckValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Validate a flat card-id list against game rules + the restricted list.
///
/// Checks (order matches Python so error messages come out identically):
///   1. Unknown card warnings
///   2. Main = 50, Egg <= 5
///   3. Per-card copy limits from `max_count_in_deck`
///   4. Restricted list (banned → error, restricted → limit enforced)
///   5. Choice-group exclusivity
pub fn validate_deck(card_ids: &[String]) -> DeckValidationResult {
    let db = card_database();
    let counts = summarize_deck(card_ids);

    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Classify into main/egg counts, accumulate unknowns.
    let mut main_count: u32 = 0;
    let mut egg_count: u32 = 0;
    let mut unknown_ids: Vec<String> = Vec::new();

    let distinct: HashSet<&String> = card_ids.iter().collect();
    for card_id in &distinct {
        let n = counts.get(card_id.as_str()).copied().unwrap_or(0);
        match db.get(card_id.as_str()) {
            None => {
                unknown_ids.push((*card_id).clone());
                main_count += n; // treat unknowns as main for size check
            }
            Some(entity) if entity.card_kind == 3 => {
                egg_count += n;
            }
            Some(_) => {
                main_count += n;
            }
        }
    }

    unknown_ids.sort();
    for uid in &unknown_ids {
        warnings.push(format!(
            "Unknown card ID: {uid} (not in card database)"
        ));
    }

    if main_count != 50 {
        errors.push(format!(
            "Main deck must be exactly 50 cards (got {main_count})"
        ));
    }
    if egg_count > 5 {
        errors.push(format!(
            "Digi-Egg deck must be 0-5 cards (got {egg_count})"
        ));
    }

    // Iterate counts in sorted order so error messages are deterministic
    // across runs — Python's Counter happens to be insertion-ordered but
    // a HashMap in Rust isn't, and the frontend shows the first error.
    let sorted_counts: BTreeMap<&String, &u32> = counts.iter().collect();
    for (card_id, count) in &sorted_counts {
        let count = **count;
        if let Some(entity) = db.get(card_id.as_str()) {
            if count > entity.max_count_in_deck {
                errors.push(format!(
                    "{} ({}): {} copies exceeds max {} per deck",
                    card_id, entity.card_name_eng, count, entity.max_count_in_deck
                ));
            }
        }
    }

    for (card_id, count) in &sorted_counts {
        let count = **count;
        if let Some(limit) = card_limit(card_id.as_str()) {
            let name = db
                .get(card_id.as_str())
                .map(|e| e.card_name_eng.as_str())
                .unwrap_or(card_id.as_str());
            if limit == 0 {
                errors.push(format!("{card_id} ({name}) is banned"));
            } else if count > limit {
                errors.push(format!(
                    "{card_id} ({name}): {count} copies exceeds restricted limit of {limit}"
                ));
            }
        }
    }

    let deck_ids_set: HashSet<&str> = card_ids.iter().map(String::as_str).collect();
    for (group_a, group_b) in CHOICE_GROUPS {
        let has_a = group_a.iter().any(|cid| deck_ids_set.contains(cid));
        let has_b = group_b.iter().any(|cid| deck_ids_set.contains(cid));
        if has_a && has_b {
            errors.push(format!(
                "Choice restriction violated: cannot include cards from [{}] and [{}] in the same deck",
                group_a.join(", "),
                group_b.join(", "),
            ));
        }
    }

    DeckValidationResult {
        is_valid: errors.is_empty(),
        errors,
        warnings,
    }
}

// ─── Split a parsed flat list into {main, egg, warnings} ──────────────

pub struct ParsedDeck {
    pub main_deck: Vec<String>,
    pub egg_deck: Vec<String>,
    pub warnings: Vec<String>,
}

/// Mirror of `POST /decks/parse`: given a parsed flat list (one entry per
/// copy), bucket into main/egg using the card DB and surface "unknown
/// card" warnings. Unknown cards fall into main — matches Python.
pub fn classify_parsed(card_ids: Vec<String>) -> ParsedDeck {
    let db = card_database();
    let mut main_deck = Vec::new();
    let mut egg_deck = Vec::new();
    let mut warnings = Vec::new();
    let mut seen_unknown: HashSet<String> = HashSet::new();

    for card_id in card_ids {
        match db.get(&card_id) {
            None => {
                if seen_unknown.insert(card_id.clone()) {
                    warnings.push(format!(
                        "Unknown card: {card_id} (not in card database)"
                    ));
                }
                main_deck.push(card_id);
            }
            Some(entity) if entity.card_kind == 3 => egg_deck.push(card_id),
            Some(_) => main_deck.push(card_id),
        }
    }

    ParsedDeck {
        main_deck,
        egg_deck,
        warnings,
    }
}

/// Expand a `{card_id -> count}` map into a flat list of card IDs.
/// Mirrors Python `deck_loader.expand_deck_dict`. Iteration order over
/// the map is unspecified; callers that care about order should sort
/// the returned vec themselves.
pub fn expand_deck_dict(counts: &HashMap<String, u32>) -> Vec<String> {
    let mut out = Vec::with_capacity(counts.values().map(|c| *c as usize).sum());
    for (card_id, count) in counts {
        for _ in 0..*count {
            out.push(card_id.clone());
        }
    }
    out
}

/// Resolve the ONNX models directory. Honors the `ONNX_MODELS_DIR` env
/// var; falls back to `models` relative to the working directory.
/// Mirrors Python `digimon_gym.engine.model_utils.get_models_dir`.
pub fn get_models_dir() -> std::path::PathBuf {
    std::env::var("ONNX_MODELS_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("models"))
}
