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
use crate::format::{self, FormatDescriptor, RarityPolicy};

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
        CardData::load_from_str(CARDS_JSON).expect("cards.json is malformed (compiled-in resource)")
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
// Banlists, the EDEN anomaly protocol, and the format descriptors all live in
// `data/deck_formats.json` and are parsed by the `format` module — the single
// source of truth shared with the hosted API. There are no longer any
// hardcoded card lists or per-format helpers here; validation is derived
// generically from a `FormatDescriptor` (see `validate_deck_for_descriptor`).
fn is_common_or_uncommon(card: &CardSummary) -> bool {
    matches!(card.rarity, Rarity::C | Rarity::U)
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
    let id = if game_mode.is_empty() {
        "standard"
    } else {
        game_mode
    };
    let fmt = format::descriptor(id)
        .ok_or_else(|| format!("Unsupported deck validation game_mode: {game_mode}"))?;
    Ok(validate_deck_for_descriptor(card_ids, fmt))
}

/// Validate a flat card-id list against the Standard format.
///
/// Checks (order preserved so error messages come out identically):
///   1. Unknown card warnings
///   2. Main = deck_size, Egg <= egg_max
///   3. Per-card copy limits from `max_count_in_deck`
///   4. Restricted list (banned → error, restricted → limit enforced) + singleton
///   5. Rarity policy (Pauper rarity gate / EDEN anomaly protocol)
///   6. Choice-group exclusivity
pub fn validate_deck(card_ids: &[String]) -> DeckValidationResult {
    let fmt = format::descriptor("standard").expect("standard format must exist");
    validate_deck_for_descriptor(card_ids, fmt)
}

/// Validate a flat card-id list against a named Rust game mode.
pub fn validate_deck_for_mode(
    card_ids: &[String],
    mode: GameMode,
) -> Result<DeckValidationResult, String> {
    let fmt = format::descriptor_for_mode(mode)
        .ok_or_else(|| format!("Unsupported deck validation game mode: {mode:?}"))?;
    Ok(validate_deck_for_descriptor(card_ids, fmt))
}

/// Generic deck validation derived entirely from a `FormatDescriptor` — no
/// per-format branches. The descriptor carries deck sizes, the rarity policy,
/// the card restriction (bans/limits/choice groups), and the singleton flag.
pub fn validate_deck_for_descriptor(
    card_ids: &[String],
    fmt: &FormatDescriptor,
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

    let deck_size = u32::from(fmt.deck_size);
    let egg_max = u32::from(fmt.egg_max);
    if main_count != deck_size {
        errors.push(format!(
            "Main deck must be exactly {deck_size} cards (got {main_count})"
        ));
    }
    if egg_count > egg_max {
        errors.push(format!(
            "Digi-Egg deck must be 0-{egg_max} cards (got {egg_count})"
        ));
    }

    // Iterate counts in sorted order so error messages are deterministic.
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

    // Format restriction (banlist / restricted limits) + singleton cap.
    for (card_id, count) in &sorted_counts {
        let count = **count;
        let name = db
            .get(card_id.as_str())
            .map(|e| e.card_name_eng.as_str())
            .unwrap_or(card_id.as_str());
        let restriction_limit = fmt
            .restriction
            .card_limits
            .get(card_id.as_str())
            .map(|l| u32::from(*l));
        let mut flagged = false;
        if let Some(limit) = restriction_limit {
            if limit == 0 {
                errors.push(format!("{card_id} ({name}) is banned"));
                flagged = true;
            } else if count > limit {
                errors.push(format!(
                    "{card_id} ({name}): {count} copies exceeds restricted limit of {limit}"
                ));
                flagged = true;
            }
        }
        // Singleton: at most one copy of any card. Skip when the restriction
        // branch above already reported a violation for this card so we don't
        // double-report (e.g. an EDEN "limited to 1" card under EDEN Singleton).
        if fmt.singleton && count > 1 && !flagged {
            errors.push(format!(
                "{card_id} ({name}): {count} copies ({} is singleton — max 1)",
                fmt.name
            ));
        }
    }

    // Rarity policy — generic over the descriptor's policy.
    match fmt.rarity_policy {
        RarityPolicy::All => {}
        RarityPolicy::CommonUncommon => {
            for (card_id, _) in &sorted_counts {
                let Some(entity) = db.get(card_id.as_str()) else {
                    continue;
                };
                if entity.card_kind == 3 || is_common_or_uncommon(entity) {
                    continue;
                }
                errors.push(format!(
                    "{} ({}): rarity {} is not legal in {} format",
                    card_id,
                    entity.card_name_eng,
                    entity.rarity.code(),
                    fmt.name
                ));
            }
        }
        RarityPolicy::EdenAnomaly => {
            let anomaly = format::anomaly_protocol();
            let mut anomaly_count = 0u32;
            for (card_id, count) in &sorted_counts {
                let Some(entity) = db.get(card_id.as_str()) else {
                    continue;
                };
                if entity.card_kind == 3 || is_common_or_uncommon(entity) {
                    continue;
                }
                let name_lower = entity.card_name_eng.to_ascii_lowercase();
                if anomaly.matches(card_id, entity.card_kind, entity.rarity, &name_lower) {
                    anomaly_count += **count;
                    continue;
                }
                errors.push(format!(
                    "{} ({}): rarity is not legal in {} format",
                    card_id, entity.card_name_eng, fmt.name
                ));
            }
            if anomaly_count > anomaly.max_total {
                errors.push(format!(
                    "EDEN Anomaly Protocol allows at most {} total rare/promo Tamers, Memory Boosts, Training Boosts, and Scrambles (got {anomaly_count})",
                    anomaly.max_total
                ));
            }
        }
    }

    // Choice-group exclusivity.
    let deck_ids_set: HashSet<&str> = card_ids.iter().map(String::as_str).collect();
    for (group_a, group_b) in &fmt.restriction.choice_groups {
        let has_a = group_a
            .iter()
            .any(|cid| deck_ids_set.contains(cid.as_str()));
        let has_b = group_b
            .iter()
            .any(|cid| deck_ids_set.contains(cid.as_str()));
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

// ─── Per-card legality ─────────────────────────────────────────────────

/// Per-card legality under a format — the single-card projection of the same
/// logic `validate_deck_for_descriptor` applies. `max_copies` is the effective
/// cap (restriction limit / singleton / default, clamped to the card's
/// intrinsic `max_count_in_deck`). `reason` explains an illegal or constrained
/// card; the deck-level anomaly *total* cap is reported as a constraint here,
/// not a hard rejection (only a full deck can exceed it).
#[derive(Debug, Clone, Serialize)]
pub struct CardLegality {
    pub legal: bool,
    pub max_copies: u32,
    pub reason: Option<String>,
}

/// Resolve legality for a single card under a named game mode.
pub fn card_legality(card_id: &str, game_mode: &str) -> Result<CardLegality, String> {
    let id = if game_mode.is_empty() {
        "standard"
    } else {
        game_mode
    };
    let fmt = format::descriptor(id)
        .ok_or_else(|| format!("Unsupported deck validation game_mode: {game_mode}"))?;
    Ok(card_legality_for_descriptor(card_id, fmt))
}

/// Per-card legality under a resolved format descriptor.
pub fn card_legality_for_descriptor(card_id: &str, fmt: &FormatDescriptor) -> CardLegality {
    let db = card_database();
    let restriction_limit = fmt
        .restriction
        .card_limits
        .get(card_id)
        .map(|l| u32::from(*l));

    // Banned dominates everything.
    if restriction_limit == Some(0) {
        return CardLegality {
            legal: false,
            max_copies: 0,
            reason: Some(format!("Banned in {}", fmt.name)),
        };
    }

    let entity = db.get(card_id);
    let intrinsic = entity.map(|e| e.max_count_in_deck).unwrap_or(4);
    let base = restriction_limit.unwrap_or(u32::from(fmt.default_max_copies));
    let mut max_copies = base.min(intrinsic);
    if fmt.singleton {
        max_copies = max_copies.min(1);
    }

    let Some(entity) = entity else {
        // Unknown card — validator only warns, so treat as legal here.
        return CardLegality {
            legal: true,
            max_copies,
            reason: None,
        };
    };

    let is_egg = entity.card_kind == 3;
    let mut reason: Option<String> = None;

    // Rarity policy gate.
    match fmt.rarity_policy {
        RarityPolicy::All => {}
        RarityPolicy::CommonUncommon => {
            if !is_egg && !is_common_or_uncommon(entity) {
                return CardLegality {
                    legal: false,
                    max_copies: 0,
                    reason: Some(format!(
                        "Rarity {} not legal in {} format",
                        entity.rarity.code(),
                        fmt.name
                    )),
                };
            }
        }
        RarityPolicy::EdenAnomaly => {
            if !is_egg && !is_common_or_uncommon(entity) {
                let anomaly = format::anomaly_protocol();
                let name_lower = entity.card_name_eng.to_ascii_lowercase();
                if anomaly.matches(card_id, entity.card_kind, entity.rarity, &name_lower) {
                    reason = Some(format!(
                        "Counts toward the EDEN Anomaly limit (max {})",
                        anomaly.max_total
                    ));
                } else {
                    return CardLegality {
                        legal: false,
                        max_copies: 0,
                        reason: Some(format!("Rarity not legal in {} format", fmt.name)),
                    };
                }
            }
        }
    }

    // Restriction / singleton copy-cap note (only if not already an anomaly note).
    if reason.is_none() {
        if let Some(limit) = restriction_limit {
            reason = Some(format!("Restricted to {limit} in {}", fmt.name));
        } else if fmt.singleton {
            reason = Some(format!("{} is singleton — max 1", fmt.name));
        }
    }

    CardLegality {
        legal: true,
        max_copies,
        reason,
    }
}

/// Legality for every card on the tested allowlist under a game mode, keyed by
/// card id. Lets the deck builder filter/badge the whole pool in one call.
pub fn card_legality_bulk(game_mode: &str) -> Result<HashMap<String, CardLegality>, String> {
    let id = if game_mode.is_empty() {
        "standard"
    } else {
        game_mode
    };
    let fmt = format::descriptor(id)
        .ok_or_else(|| format!("Unsupported deck validation game_mode: {game_mode}"))?;
    let out = tested_cards_set()
        .iter()
        .map(|cid| (cid.clone(), card_legality_for_descriptor(cid, fmt)))
        .collect();
    Ok(out)
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
