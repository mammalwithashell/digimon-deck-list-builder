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

use serde::{Deserialize, Serialize};

use crate::enums::{GameMode, Rarity};
use crate::rules::CardRestriction;

// Data files — `include_str!` bakes the bytes into the crate, so there's no
// separate resource-bundling step for desktop builds.
const CARDS_JSON: &str = include_str!("../../../data/cards.json");
const TESTED_CARDS_JSON: &str = include_str!("../../../data/tested_cards.json");

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
    /// Matches Python `Rarity`: 0=C, 1=U, 2=R, 3=SR, 4=SEC, 5=P.
    pub rarity: Rarity,
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
    #[serde(default = "default_rarity")]
    rarity: u8,
    #[serde(default = "default_max_count")]
    max_count_in_deck: u32,
}

fn default_max_count() -> u32 {
    4
}

fn default_rarity() -> u8 {
    u8::MAX
}

fn parse_rarity(raw: u8, card_id: &str) -> Rarity {
    if raw == default_rarity() {
        return Rarity::NoRarity;
    }
    Rarity::from_u8(raw)
        .unwrap_or_else(|| panic!("cards.json has unknown rarity value {raw} for card {card_id}"))
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
                let rarity = parse_rarity(v.rarity, &k);
                (
                    k,
                    CardSummary {
                        card_id: v.card_id,
                        card_name_eng: v.card_name_eng,
                        card_kind: v.card_kind,
                        rarity,
                        max_count_in_deck: v.max_count_in_deck,
                    },
                )
            })
            .collect()
    })
}

/// Lazily-parsed full `CardData` map for the entire card pool baked into
/// the engine via `CARDS_JSON`. Used by hosts that need to call
/// `Game::new(decks, card_data, ...)` — the deck-tools `CardSummary`
/// surface above is too narrow for `Game::new` because it drops
/// `effect_class_name`, parsed `evo_costs`, etc.
///
/// Returns a clone-on-call `HashMap` rather than a `'static` reference
/// because callers like `Game::new` take ownership semantics over the
/// passed-in map. The underlying parse runs once and is cached.
pub fn full_card_data() -> HashMap<String, crate::card_data::CardData> {
    use crate::card_data::CardData;
    static CELL: OnceLock<HashMap<String, CardData>> = OnceLock::new();
    let parsed = CELL.get_or_init(|| {
        CardData::load_from_str(CARDS_JSON)
            .expect("cards.json is malformed (compiled-in resource)")
    });
    parsed.clone()
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
    "BT2-090", // Matt Ishida
    "BT5-109", // Mega Digimon Fusion!
    "EX5-065", // Sayo & Koh
];

const RESTRICTED_CARDS: &[&str] = &[
    "BT1-090", "BT10-009", "BT11-033", "BT11-064", "BT13-012", "BT13-110", "BT14-002", "BT14-084",
    "BT15-057", "BT15-102", "BT16-011", "BT17-069", "BT19-040", "BT2-047", "BT2-069", "BT3-054",
    "BT3-103", "BT4-104", "BT4-111", "BT6-100", "BT6-104", "BT7-038", "BT7-064", "BT7-069",
    "BT7-072", "BT7-107", "BT9-098", "BT9-099", "EX1-021", "EX1-068", "EX2-039", "EX2-070",
    "EX3-057", "EX4-006", "EX4-019", "EX4-030", "EX5-015", "EX5-018", "EX5-062", "P-008", "P-025",
    "P-029", "P-030", "P-123", "P-130", "ST2-13", "ST9-09",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckRuleset {
    Standard,
    NoRestriction,
    Pauper,
    Eden,
}

impl DeckRuleset {
    pub fn from_game_mode(game_mode: &str) -> Option<Self> {
        match game_mode {
            "standard" | "" => Some(Self::Standard),
            "no_restriction" => Some(Self::NoRestriction),
            "pauper" => Some(Self::Pauper),
            "eden" => Some(Self::Eden),
            _ => None,
        }
    }

    pub fn from_game_mode_enum(mode: GameMode) -> Option<Self> {
        match mode {
            GameMode::Standard => Some(Self::Standard),
            GameMode::Pauper => Some(Self::Pauper),
            GameMode::NoRestriction => Some(Self::NoRestriction),
            GameMode::Eden => Some(Self::Eden),
            GameMode::EdhCommander | GameMode::Titan => None,
        }
    }

    fn restriction(self) -> Option<&'static CardRestriction> {
        match self {
            Self::Standard => None,
            Self::NoRestriction => None,
            Self::Pauper => None,
            Self::Eden => Some(CardRestriction::eden_ref()),
        }
    }
}

fn format_card_limit(ruleset: DeckRuleset, card_id: &str) -> Option<u32> {
    match ruleset {
        DeckRuleset::Standard | DeckRuleset::Pauper => card_limit(card_id),
        DeckRuleset::NoRestriction => None,
        DeckRuleset::Eden => CardRestriction::eden_ref()
            .card_limits
            .get(card_id)
            .map(|limit| u32::from(*limit)),
    }
}

fn format_choice_groups(ruleset: DeckRuleset) -> Vec<(Vec<String>, Vec<String>)> {
    match ruleset {
        DeckRuleset::Standard | DeckRuleset::Pauper => CHOICE_GROUPS
            .iter()
            .map(|(a, b)| {
                (
                    a.iter().map(|s| (*s).to_string()).collect(),
                    b.iter().map(|s| (*s).to_string()).collect(),
                )
            })
            .collect(),
        DeckRuleset::NoRestriction => Vec::new(),
        DeckRuleset::Eden => ruleset
            .restriction()
            .expect("EDEN restriction must exist")
            .choice_groups
            .clone(),
    }
}

fn is_common_or_uncommon(card: &CardSummary) -> bool {
    matches!(card.rarity, Rarity::C | Rarity::U)
}

fn is_eden_anomaly(card: &CardSummary) -> bool {
    let name = card.card_name_eng.to_ascii_lowercase();
    match card.card_kind {
        // Rare or promo Tamers.
        1 => matches!(card.rarity, Rarity::R | Rarity::P),
        // Rare/promo/SR Memory Boosts, promo Trainings, and promo Scrambles.
        2 if name.contains("memory boost") => {
            matches!(card.rarity, Rarity::R | Rarity::SR | Rarity::P)
        }
        2 if name.contains("training") => card.rarity == Rarity::P,
        2 if name.contains("scramble") => card.rarity == Rarity::P,
        _ => false,
    }
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
    let data: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("Invalid TTS JSON: {e}"))?;
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
    Err("Could not parse deck list. Expected either:\n  \
         - TTS format: JSON array like [\"BT24-017\", \"BT24-017\", ...]\n  \
         - Text format: lines like '4 Medusamon BT24-017'"
        .to_string())
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

pub fn validate_deck_for_game_mode(
    card_ids: &[String],
    game_mode: &str,
) -> Result<DeckValidationResult, String> {
    let ruleset = DeckRuleset::from_game_mode(game_mode)
        .ok_or_else(|| format!("Unsupported deck validation game_mode: {game_mode}"))?;
    Ok(validate_deck_for_ruleset(card_ids, ruleset))
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
    validate_deck_for_ruleset(card_ids, DeckRuleset::Standard)
}

/// Validate a flat card-id list against a named Rust game mode.
pub fn validate_deck_for_mode(
    card_ids: &[String],
    mode: GameMode,
) -> Result<DeckValidationResult, String> {
    let ruleset = DeckRuleset::from_game_mode_enum(mode)
        .ok_or_else(|| format!("Unsupported deck validation game mode: {mode:?}"))?;
    Ok(validate_deck_for_ruleset(card_ids, ruleset))
}

pub fn validate_deck_for_ruleset(
    card_ids: &[String],
    ruleset: DeckRuleset,
) -> DeckValidationResult {
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
        warnings.push(format!("Unknown card ID: {uid} (not in card database)"));
    }

    if main_count != 50 {
        errors.push(format!(
            "Main deck must be exactly 50 cards (got {main_count})"
        ));
    }
    if egg_count > 5 {
        errors.push(format!("Digi-Egg deck must be 0-5 cards (got {egg_count})"));
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
        if let Some(limit) = format_card_limit(ruleset, card_id.as_str()) {
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

    if ruleset == DeckRuleset::Eden {
        let mut anomaly_count = 0u32;
        for (card_id, count) in &sorted_counts {
            let Some(entity) = db.get(card_id.as_str()) else {
                continue;
            };
            if entity.card_kind == 3 || is_common_or_uncommon(entity) {
                continue;
            }
            if is_eden_anomaly(entity) {
                anomaly_count += **count;
                continue;
            }
            errors.push(format!(
                "{} ({}): rarity is not legal in EDEN format",
                card_id, entity.card_name_eng
            ));
        }
        if anomaly_count > 4 {
            errors.push(format!(
                "EDEN Anomaly Protocol allows at most 4 total rare/promo Tamers, Memory Boosts, Training Boosts, and Scrambles (got {anomaly_count})"
            ));
        }
    }

    if ruleset == DeckRuleset::Pauper {
        for (card_id, _) in &sorted_counts {
            let Some(entity) = db.get(card_id.as_str()) else {
                continue;
            };
            if entity.card_kind == 3 || is_common_or_uncommon(entity) {
                continue;
            }
            errors.push(format!(
                "{} ({}): rarity {} is not legal in Pauper format",
                card_id,
                entity.card_name_eng,
                entity.rarity.code()
            ));
        }
    }

    let deck_ids_set: HashSet<&str> = card_ids.iter().map(String::as_str).collect();
    for (group_a, group_b) in format_choice_groups(ruleset) {
        let has_a = group_a
            .iter()
            .any(|cid| deck_ids_set.contains(cid.as_str()));
        let has_b = group_b
            .iter()
            .any(|cid| deck_ids_set.contains(cid.as_str()));
        if has_a && has_b {
            let prefix = if ruleset == DeckRuleset::Eden {
                "EDEN "
            } else {
                ""
            };
            errors.push(format!(
                "{prefix}Choice restriction violated: cannot include cards from [{}] and [{}] in the same deck",
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
                    warnings.push(format!("Unknown card: {card_id} (not in card database)"));
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

// ─── Rich card metadata for UI browsing ───────────────────────────────

/// Display-oriented card metadata for the deck-builder card pool.
/// Unlike `CardSummary` (validation-only fields) this carries everything
/// the browser grid + preview panel render: effect text, colors, level,
/// costs, DP, traits. Serialized verbatim over the Tauri `invoke()`
/// boundary and mirrored by the hosted API's `/decks/card-database`.
#[derive(Debug, Clone, Serialize)]
pub struct CardMeta {
    pub card_id: String,
    pub name: String,
    /// "Digimon" | "Tamer" | "Option" | "Digi-Egg"
    pub card_type: String,
    /// Color names in printed order (1-2 entries), e.g. ["Green"].
    pub colors: Vec<String>,
    pub level: Option<i64>,
    pub play_cost: Option<i64>,
    /// Memory cost of the first printed digivolution requirement.
    pub evolution_cost: Option<i64>,
    /// "C" | "U" | "R" | "SR" | "SEC" | "P" | "" (unknown).
    pub rarity: String,
    pub dp: Option<i64>,
    /// Form, e.g. "Rookie" (first `form_eng` entry).
    pub stage: String,
    /// Digimon types joined with "/", e.g. "Larva".
    pub digi_type: String,
    pub attribute: String,
    pub main_effect: String,
    pub inherited_effect: String,
    pub security_effect: String,
}

#[derive(Deserialize)]
struct EvoCostRaw {
    #[serde(default)]
    memory_cost: Option<i64>,
}

#[derive(Deserialize)]
struct CardMetaRaw {
    card_id: String,
    #[serde(default)]
    card_name_eng: String,
    card_kind: u8,
    #[serde(default = "default_rarity")]
    rarity: u8,
    #[serde(default)]
    card_colors: Vec<u8>,
    #[serde(default)]
    level: Option<i64>,
    #[serde(default)]
    play_cost: Option<i64>,
    #[serde(default)]
    dp: Option<i64>,
    #[serde(default)]
    type_eng: Vec<String>,
    #[serde(default)]
    form_eng: Vec<String>,
    #[serde(default)]
    attribute_eng: Vec<String>,
    #[serde(default)]
    effect_description_eng: String,
    #[serde(default)]
    inherited_effect_description_eng: String,
    #[serde(default)]
    security_effect_description_eng: String,
    #[serde(default)]
    evo_costs: Vec<EvoCostRaw>,
}

fn color_name(raw: u8) -> &'static str {
    // Matches `enums::CardColor` (Red=0 .. Purple=6).
    match raw {
        0 => "Red",
        1 => "Blue",
        2 => "Yellow",
        3 => "Green",
        4 => "White",
        5 => "Black",
        6 => "Purple",
        _ => "",
    }
}

fn kind_name(raw: u8) -> &'static str {
    match raw {
        0 => "Digimon",
        1 => "Tamer",
        2 => "Option",
        3 => "Digi-Egg",
        _ => "",
    }
}

fn rarity_name(raw: u8) -> &'static str {
    match raw {
        0 => "C",
        1 => "U",
        2 => "R",
        3 => "SR",
        4 => "SEC",
        5 => "P",
        _ => "",
    }
}

/// Display metadata for every card on the tested (implemented) allowlist,
/// sorted by card ID. Parsed once and cached for the process lifetime.
pub fn tested_card_metadata() -> &'static [CardMeta] {
    static CELL: OnceLock<Vec<CardMeta>> = OnceLock::new();
    CELL.get_or_init(|| {
        let raw: HashMap<String, CardMetaRaw> = serde_json::from_str(CARDS_JSON)
            .expect("cards.json is malformed (compiled-in resource)");
        let tested = tested_cards_set();
        let mut out: Vec<CardMeta> = raw
            .into_values()
            .filter(|entry| tested.contains(&entry.card_id))
            .map(|entry| CardMeta {
                name: entry.card_name_eng,
                card_type: kind_name(entry.card_kind).to_string(),
                colors: entry
                    .card_colors
                    .iter()
                    .map(|c| color_name(*c).to_string())
                    .filter(|n| !n.is_empty())
                    .collect(),
                level: entry.level,
                play_cost: entry.play_cost,
                evolution_cost: entry.evo_costs.first().and_then(|c| c.memory_cost),
                rarity: rarity_name(entry.rarity).to_string(),
                dp: entry.dp,
                stage: entry.form_eng.first().cloned().unwrap_or_default(),
                digi_type: entry.type_eng.join("/"),
                attribute: entry.attribute_eng.join("/"),
                main_effect: entry.effect_description_eng,
                inherited_effect: entry.inherited_effect_description_eng,
                security_effect: entry.security_effect_description_eng,
                card_id: entry.card_id,
            })
            .collect();
        out.sort_by(|a, b| a.card_id.cmp(&b.card_id));
        out
    })
}

/// Resolve the ONNX models directory. Honors the `ONNX_MODELS_DIR` env
/// var; falls back to `models` relative to the working directory.
/// Mirrors Python `digimon_gym.engine.model_utils.get_models_dir`.
pub fn get_models_dir() -> std::path::PathBuf {
    std::env::var("ONNX_MODELS_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("models"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_rarity_sentinel_maps_to_no_rarity() {
        assert_eq!(parse_rarity(default_rarity(), "TEST-001"), Rarity::NoRarity);
    }

    #[test]
    #[should_panic(expected = "cards.json has unknown rarity value 42 for card TEST-001")]
    fn invalid_rarity_value_panics() {
        let _ = parse_rarity(42, "TEST-001");
    }
}
