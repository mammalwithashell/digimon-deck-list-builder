//! Behavioural parity tests for `deck_tools` — every assertion targets a
//! semantics that has to match `digimon_gym/engine/data/deck_loader.py` so
//! desktop and hosted API can't diverge on deck validation.

use digimon_engine::deck_tools::{
    card_database, card_legality, classify_parsed, is_card_tested, out_of_set_cards, parse_deck,
    parse_text, parse_tts, summarize_deck, tested_cards_sorted, validate_deck,
    validate_deck_for_game_mode, validate_deck_for_mode,
};
use digimon_engine::{build_registry, CardData, GameMode, HeadlessRunner, Rarity};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const STARTER_DECKS_JSON: &str = include_str!("../../../../data/starter_decks.json");
const CARDS_JSON: &str = include_str!("../../../../data/cards.json");

#[derive(Debug, Deserialize)]
struct StarterDecksFixture {
    starter_decks: BTreeMap<String, StarterDeckFixture>,
}

#[derive(Debug, Deserialize)]
struct StarterDeckFixture {
    id: String,
    name: String,
    decklist: Vec<String>,
}

fn starter_decks_fixture() -> StarterDecksFixture {
    serde_json::from_str(STARTER_DECKS_JSON).expect("starter_decks.json parses")
}

fn st3_starter_deck() -> Vec<String> {
    starter_decks_fixture()
        .starter_decks
        .get("ST-3")
        .expect("ST-3 fixture exists")
        .decklist
        .clone()
}

fn full_card_data() -> HashMap<String, CardData> {
    CardData::load_from_str(CARDS_JSON).expect("cards.json parses")
}

const DECK_LIBRARY_JSON: &str = include_str!("../../../../data/deck_library.json");

// ─── Card ID pattern ───────────────────────────────────────────────────

#[test]
fn parse_text_expands_quantities() {
    let raw = "// Deck list\n3 Agumon BT1-001\n1 Greymon AD1-001\n";
    let ids = parse_text(raw).unwrap();
    assert_eq!(
        ids,
        vec![
            "BT1-001".to_string(),
            "BT1-001".to_string(),
            "BT1-001".to_string(),
            "AD1-001".to_string(),
        ]
    );
}

#[test]
fn parse_text_skips_invalid_lines() {
    let raw = "// header\nthis is garbage\n2 Vanilla BT24-017\n";
    let ids = parse_text(raw).unwrap();
    assert_eq!(ids.len(), 2);
    assert!(ids.iter().all(|c| c == "BT24-017"));
}

#[test]
fn parse_text_errors_on_empty_or_invalid_only() {
    assert!(parse_text("// just a comment").is_err());
    assert!(parse_text("").is_err());
}

#[test]
fn parse_tts_accepts_json_array_and_filters_noise() {
    let raw = r#"["Exported from https://digimoncard.io", "BT24-017", "BT24-017", "not a card"]"#;
    let ids = parse_tts(raw).unwrap();
    assert_eq!(ids, vec!["BT24-017".to_string(), "BT24-017".to_string()]);
}

#[test]
fn parse_tts_rejects_non_array_json() {
    assert!(parse_tts(r#"{"nope": true}"#).is_err());
    assert!(parse_tts("not even json").is_err());
}

#[test]
fn parse_deck_auto_detects_format() {
    // TTS when input starts with `[`
    let ids = parse_deck(r#"["BT24-017"]"#).unwrap();
    assert_eq!(ids, vec!["BT24-017".to_string()]);

    // Text otherwise
    let ids = parse_deck("1 Whatever BT24-017").unwrap();
    assert_eq!(ids, vec!["BT24-017".to_string()]);
}

#[test]
fn parse_deck_falls_through_to_text_when_tts_parse_fails() {
    // Starts with `[` but isn't valid JSON — Python parser falls through
    // to text parsing in this case, and we match that behaviour.
    let ids = parse_deck("[this is text\n1 Agumon BT1-001").unwrap();
    assert_eq!(ids, vec!["BT1-001".to_string()]);
}

#[test]
fn parse_deck_empty_is_error() {
    assert!(parse_deck("   ").is_err());
}

// ─── Card database loader ─────────────────────────────────────────────

#[test]
fn card_database_loads_real_cards_json() {
    let db = card_database();
    // cards.json is ~4000 entries — allow wide bounds in case the file grows.
    assert!(db.len() > 1000, "card DB too small: {}", db.len());
    // AD1-001 is Greymon (Digimon, card_kind=0) with a Digi-Egg counterpart.
    let greymon = db.get("AD1-001").expect("AD1-001 should exist");
    assert_eq!(greymon.card_kind, 0);
    assert_eq!(greymon.card_name_eng, "Greymon");
    assert_eq!(greymon.rarity, Rarity::R);
}

#[test]
fn card_database_defaults_max_copies_to_4() {
    let db = card_database();
    for entry in db.values().take(100) {
        // Default is 4; overrides are rare and always <= 50.
        assert!(
            entry.max_count_in_deck >= 1 && entry.max_count_in_deck <= 50,
            "{}: unexpected max_count {}",
            entry.card_id,
            entry.max_count_in_deck
        );
    }
}

// ─── Tested cards ─────────────────────────────────────────────────────

#[test]
fn tested_cards_sorted_is_sorted_and_nonempty() {
    let cards = tested_cards_sorted();
    assert!(!cards.is_empty());
    let mut sorted = cards.clone();
    sorted.sort();
    assert_eq!(cards, sorted);
}

#[test]
fn out_of_set_preserves_order_and_deduplicates() {
    // Use card IDs that definitely aren't tested to exercise the filter.
    let probe = vec![
        "ZZZ-999".to_string(),
        "ZZZ-999".to_string(),
        "ZZY-998".to_string(),
    ];
    let out = out_of_set_cards(&probe);
    assert_eq!(out, vec!["ZZZ-999".to_string(), "ZZY-998".to_string()]);
}

#[test]
fn tested_cards_allowlist_is_consistent_with_membership_helper() {
    let sorted = tested_cards_sorted();
    if let Some(first) = sorted.first() {
        assert!(is_card_tested(first));
    }
    assert!(!is_card_tested("ZZZ-999"));
}

#[test]
fn tested_card_metadata_covers_allowlist_with_display_fields() {
    let metas = digimon_engine::deck_tools::tested_card_metadata();
    let allowlist = tested_cards_sorted();
    // One metadata entry per allowlisted card (every allowlisted card has
    // a cards.json entry by construction — build_tested_cards.py
    // intersects with cards.json), sorted by card ID.
    assert_eq!(metas.len(), allowlist.len());
    for (meta, card_id) in metas.iter().zip(allowlist.iter()) {
        assert_eq!(&meta.card_id, card_id);
        assert!(!meta.name.is_empty(), "{card_id} has no name");
        assert!(!meta.card_type.is_empty(), "{card_id} has no card_type");
        assert!(!meta.colors.is_empty(), "{card_id} has no colors");
    }
    // Spot-check a known card: BT12-050 Stingmon, Green Digimon Lv.4.
    let stingmon = metas
        .iter()
        .find(|m| m.card_id == "BT12-050")
        .expect("BT12-050 should be implemented");
    assert_eq!(stingmon.name, "Stingmon");
    assert_eq!(stingmon.card_type, "Digimon");
    assert_eq!(stingmon.colors, vec!["Green".to_string()]);
    assert_eq!(stingmon.level, Some(4));
}

#[test]
fn st3_starter_deck_fixture_has_canonical_counts() {
    let fixture = starter_decks_fixture();
    let st3 = fixture
        .starter_decks
        .get("ST-3")
        .expect("ST-3 fixture exists");
    assert_eq!(st3.id, "ST-3");
    assert_eq!(st3.name, "Starter Deck Heaven's Yellow");

    let counts = summarize_deck(&st3.decklist);
    let expected = HashMap::from([
        ("ST3-01".to_string(), 4_u32),
        ("ST3-02".to_string(), 4),
        ("ST3-03".to_string(), 4),
        ("ST3-04".to_string(), 4),
        ("ST3-05".to_string(), 2),
        ("ST3-06".to_string(), 4),
        ("ST3-07".to_string(), 4),
        ("ST3-08".to_string(), 4),
        ("ST3-09".to_string(), 4),
        ("ST3-10".to_string(), 2),
        ("ST3-11".to_string(), 2),
        ("ST3-12".to_string(), 4),
        ("ST3-13".to_string(), 4),
        ("ST3-14".to_string(), 2),
        ("ST3-15".to_string(), 4),
        ("ST3-16".to_string(), 2),
    ]);

    assert_eq!(st3.decklist.len(), 54);
    assert_eq!(counts, expected);
    assert!(out_of_set_cards(&st3.decklist).is_empty());

    let validation = validate_deck(&st3.decklist);
    assert!(
        validation.is_valid,
        "ST-3 starter deck should validate cleanly: {:?}",
        validation.errors
    );
}

#[test]
fn st3_starter_deck_cards_are_registered_as_implemented() {
    let implemented: std::collections::HashSet<String> =
        build_registry().registered_card_ids().into_iter().collect();
    for card_id in summarize_deck(&st3_starter_deck()).keys() {
        assert!(
            implemented.contains(card_id.as_str()),
            "{card_id} should be registered by the Rust card registry"
        );
    }
}

#[test]
fn st3_starter_deck_initializes_headless_runner() {
    let deck = st3_starter_deck();
    let card_data = full_card_data();
    let runner = HeadlessRunner::new(
        deck.clone(),
        deck,
        &card_data,
        false,
        false,
        false,
        Some(3),
    );

    assert!(
        runner.is_ok(),
        "Rust headless runner should accept the canonical ST-3 deck: {:?}",
        runner.err()
    );
}

// ─── classify_parsed ──────────────────────────────────────────────────

#[test]
fn classify_parsed_separates_eggs_and_unknown() {
    // Build a list mixing a known Digimon, a known Digi-Egg (card_kind=3),
    // and an unknown id. Have to pick real entries from the DB so the
    // classifier lands the right buckets.
    let db = card_database();
    let egg_id = db
        .values()
        .find(|c| c.card_kind == 3)
        .expect("a Digi-Egg must exist in the DB")
        .card_id
        .clone();
    let digimon_id = db
        .values()
        .find(|c| c.card_kind == 0)
        .expect("a Digimon must exist")
        .card_id
        .clone();
    let parsed = classify_parsed(vec![
        egg_id.clone(),
        digimon_id.clone(),
        digimon_id.clone(),
        "ZZZ-999".to_string(),
        "ZZZ-999".to_string(), // duplicate unknown → only one warning
    ]);
    assert_eq!(parsed.egg_deck, vec![egg_id]);
    assert_eq!(
        parsed
            .main_deck
            .iter()
            .filter(|c| *c == &digimon_id)
            .count(),
        2
    );
    assert!(parsed.main_deck.contains(&"ZZZ-999".to_string()));
    assert_eq!(parsed.warnings.len(), 1);
    assert!(parsed.warnings[0].contains("ZZZ-999"));
}

// ─── Validation ───────────────────────────────────────────────────────

/// Build a minimum legal 50-card main deck + 0 eggs from a card that has
/// no restrictions and is not banned. Pick one vanilla Digimon that
/// allows 4 copies.
fn make_legal_deck() -> Vec<String> {
    let db = card_database();
    // `card_database()` returns a HashMap with non-deterministic iteration
    // order, so sort by card_id before picking candidates — otherwise a
    // restricted card (e.g. P-030) can land in the first 13 hits and the
    // validator rejects the resulting 4-copy deck.
    let mut candidates: Vec<&str> = db
        .values()
        .filter(|c| {
            c.card_kind == 0 && c.max_count_in_deck >= 4 && !is_banned_or_restricted(&c.card_id)
        })
        .map(|c| c.card_id.as_str())
        .collect();
    candidates.sort_unstable();
    // 50 / 4 = 12 remainder 2; but repeating the same id 50x breaks copy
    // limit (max 4). Instead pick 13 distinct legal ids, 4 copies each =
    // 52 — trim back to 50.
    let ids: Vec<&str> = candidates.into_iter().take(13).collect();
    assert_eq!(
        ids.len(),
        13,
        "need 13 unrestricted 4-copy Digimon in the DB"
    );
    let mut deck = Vec::with_capacity(50);
    for cid in &ids {
        for _ in 0..4 {
            if deck.len() < 50 {
                deck.push((*cid).to_string());
            }
        }
    }
    assert_eq!(deck.len(), 50);
    deck
}

fn make_legal_pauper_deck() -> Vec<String> {
    let db = card_database();
    let mut candidates: Vec<&str> = db
        .values()
        .filter(|c| {
            c.card_kind == 0
                && c.max_count_in_deck >= 4
                && matches!(c.rarity, Rarity::C | Rarity::U)
                && !is_banned_or_restricted(&c.card_id)
        })
        .map(|c| c.card_id.as_str())
        .collect();
    candidates.sort_unstable();
    let ids: Vec<&str> = candidates.into_iter().take(13).collect();
    assert_eq!(
        ids.len(),
        13,
        "need 13 unrestricted 4-copy common/uncommon Digimon in the DB"
    );
    let mut deck = Vec::with_capacity(50);
    for cid in &ids {
        for _ in 0..4 {
            if deck.len() < 50 {
                deck.push((*cid).to_string());
            }
        }
    }
    assert_eq!(deck.len(), 50);
    deck
}

fn is_banned_or_restricted(card_id: &str) -> bool {
    // Mirror of `deck_tools.rs::BANNED_CARDS` and `RESTRICTED_CARDS`. These
    // constants are private to the `deck_tools` module, so duplicating
    // them here is the price of the test-side filter — keep them in sync
    // when the validator's lists change.
    const BANNED: &[&str] = &["BT2-090", "BT5-109", "EX5-065"];
    const RESTRICTED: &[&str] = &[
        "BT1-090", "BT10-009", "BT11-033", "BT11-064", "BT13-012", "BT13-110", "BT14-002",
        "BT14-084", "BT15-057", "BT15-102", "BT16-011", "BT17-069", "BT19-040", "BT2-047",
        "BT2-069", "BT3-054", "BT3-103", "BT4-104", "BT4-111", "BT6-100", "BT6-104", "BT7-038",
        "BT7-064", "BT7-069", "BT7-072", "BT7-107", "BT9-098", "BT9-099", "EX1-021", "EX1-068",
        "EX2-039", "EX2-070", "EX3-057", "EX4-006", "EX4-019", "EX4-030", "EX5-015", "EX5-018",
        "EX5-062", "P-008", "P-025", "P-029", "P-030", "P-123", "P-130", "ST2-13", "ST9-09",
    ];
    const CHOICE_GROUP_MEMBERS: &[&str] =
        &["EX2-007", "EX7-064", "BT20-037", "BT17-035", "EX8-037"];
    BANNED.contains(&card_id)
        || RESTRICTED.contains(&card_id)
        || CHOICE_GROUP_MEMBERS.contains(&card_id)
}

fn is_eden_restricted_or_choice_member(card_id: &str) -> bool {
    const IDS: &[&str] = &[
        "BT3-097", "BT9-103", "BT16-011", "EX4-008", "EX6-042", "BT12-092", "EX2-007", "BT15-082",
        "BT1-107", "BT1-112", "BT2-039", "BT2-047", "BT2-069", "BT3-054", "BT3-058", "BT3-103",
        "BT4-098", "BT4-109", "BT5-062", "BT6-085", "BT6-079", "BT6-100", "BT7-014", "BT7-025",
        "BT7-021", "BT7-038", "BT7-064", "BT7-072", "BT7-075", "BT7-107", "BT8-095", "BT8-097",
        "BT9-109", "BT11-064", "BT13-012", "BT14-002", "BT14-060", "BT14-093", "BT16-024",
        "BT17-023", "BT17-065", "BT17-067", "BT17-075", "BT17-092", "BT18-087", "BT19-070",
        "EX3-067", "EX5-048", "ST1-09", "EX4-015", "EX5-065",
    ];
    IDS.contains(&card_id)
}

fn make_eden_legal_deck() -> Vec<String> {
    let db = card_database();
    let mut candidates: Vec<&str> = db
        .values()
        .filter(|c| {
            c.card_kind == 0
                && matches!(c.rarity, Rarity::C | Rarity::U)
                && c.max_count_in_deck >= 4
                && !is_eden_restricted_or_choice_member(&c.card_id)
        })
        .map(|c| c.card_id.as_str())
        .collect();
    candidates.sort_unstable();
    let ids: Vec<&str> = candidates.into_iter().take(13).collect();
    assert_eq!(ids.len(), 13, "need 13 EDEN-legal 4-copy Digimon");
    let mut deck = Vec::with_capacity(50);
    for cid in &ids {
        for _ in 0..4 {
            if deck.len() < 50 {
                deck.push((*cid).to_string());
            }
        }
    }
    deck
}

#[test]
fn validate_deck_accepts_a_legal_50_card_deck() {
    let deck = make_legal_deck();
    let result = validate_deck(&deck);
    assert!(
        result.is_valid,
        "expected legal deck, got errors: {:?}",
        result.errors
    );
}

#[test]
fn validate_deck_rejects_wrong_main_size() {
    let mut deck = make_legal_deck();
    deck.pop();
    let result = validate_deck(&deck);
    assert!(!result.is_valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("Main deck must be")));
}

#[test]
fn validate_deck_rejects_too_many_egg_cards() {
    let db = card_database();
    // Eggs cap at 4 copies each — use two distinct egg IDs so we can land
    // 6 total without tripping a per-card limit error.
    let eggs: Vec<String> = db
        .values()
        .filter(|c| c.card_kind == 3 && c.max_count_in_deck >= 4)
        .take(2)
        .map(|c| c.card_id.clone())
        .collect();
    assert!(eggs.len() >= 2, "need two distinct Digi-Eggs in the DB");
    let mut deck = make_legal_deck();
    // 4 of egg[0] + 2 of egg[1] = 6 total eggs, both under per-card cap.
    for _ in 0..4 {
        deck.push(eggs[0].clone());
    }
    for _ in 0..2 {
        deck.push(eggs[1].clone());
    }
    let result = validate_deck(&deck);
    assert!(!result.is_valid);
    assert!(result.errors.iter().any(|e| e.contains("Digi-Egg deck")));
}

#[test]
fn validate_deck_rejects_banned_card() {
    let mut deck = make_legal_deck();
    // Replace one of the valid ids with a banned one.
    deck[0] = "BT2-090".to_string();
    let result = validate_deck(&deck);
    assert!(!result.is_valid);
    assert!(result.errors.iter().any(|e| e.contains("is banned")));
}

#[test]
fn validate_deck_rejects_restricted_limit_exceeded() {
    let mut deck = make_legal_deck();
    // BT1-090 is restricted to 1 copy. Stuff two in and drop two of the
    // other legal cards to stay at 50.
    deck[0] = "BT1-090".to_string();
    deck[1] = "BT1-090".to_string();
    let result = validate_deck(&deck);
    assert!(!result.is_valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("BT1-090") && e.contains("restricted limit")));
}

#[test]
fn validate_deck_enforces_choice_groups() {
    // Choice group: {EX2-007} vs {EX7-064}. Having 1 of each should error.
    let mut deck = make_legal_deck();
    deck[0] = "EX2-007".to_string();
    deck[1] = "EX7-064".to_string();
    let result = validate_deck(&deck);
    assert!(!result.is_valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("Choice restriction violated")));
}

#[test]
fn validate_deck_warns_on_unknown_cards() {
    let mut deck = make_legal_deck();
    deck[0] = "ZZZ-999".to_string();
    let result = validate_deck(&deck);
    // Unknown → warning, not an error (matches Python).
    assert!(result.warnings.iter().any(|w| w.contains("ZZZ-999")));
}

#[test]
fn validate_deck_for_pauper_accepts_common_uncommon_deck() {
    let deck = make_legal_pauper_deck();
    let result = validate_deck_for_game_mode(&deck, "pauper").unwrap();
    assert!(
        result.is_valid,
        "expected pauper deck to be legal, got errors: {:?}",
        result.errors
    );
}

#[test]
fn validate_deck_for_pauper_rejects_rare_or_higher() {
    let mut deck = make_legal_pauper_deck();
    deck[0] = "AD1-001".to_string(); // Greymon, rarity R.
    let result = validate_deck_for_game_mode(&deck, "pauper").unwrap();
    assert!(!result.is_valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("AD1-001") && e.contains("rarity R")));
}

#[test]
fn validate_deck_for_pauper_accepts_game_mode_enum() {
    let deck = make_legal_pauper_deck();
    let result = validate_deck_for_mode(&deck, GameMode::Pauper).unwrap();
    assert!(
        result.is_valid,
        "expected pauper deck to be legal, got errors: {:?}",
        result.errors
    );
}

#[test]
fn standard_format_allows_rare_cards() {
    let mut deck = make_legal_pauper_deck();
    deck[0] = "AD1-001".to_string(); // Greymon, rarity R.
    let result = validate_deck_for_game_mode(&deck, "standard").unwrap();
    assert!(
        result.is_valid,
        "standard format should not reject rarity R, got errors: {:?}",
        result.errors
    );
}

#[test]
fn validate_eden_accepts_common_uncommon_deck() {
    let deck = make_eden_legal_deck();
    let result = validate_deck_for_game_mode(&deck, "eden").unwrap();
    assert!(result.is_valid, "EDEN deck errors: {:?}", result.errors);
}

#[test]
fn validate_eden_rejects_non_anomaly_rare() {
    let mut deck = make_eden_legal_deck();
    deck[0] = "AD1-001".to_string(); // Rare Digimon, not an anomaly category.
    let result = validate_deck_for_game_mode(&deck, "eden").unwrap();
    assert!(!result.is_valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("AD1-001") && e.contains("EDEN format")));
}

#[test]
fn validate_eden_allows_four_anomaly_cards() {
    let mut deck = make_eden_legal_deck();
    for slot in deck.iter_mut().take(4) {
        *slot = "P-103".to_string(); // Promo Training option.
    }
    let result = validate_deck_for_game_mode(&deck, "eden").unwrap();
    assert!(result.is_valid, "EDEN anomaly errors: {:?}", result.errors);
}

#[test]
fn validate_eden_rejects_fifth_anomaly_card() {
    let mut deck = make_eden_legal_deck();
    for slot in deck.iter_mut().take(4) {
        *slot = "P-103".to_string();
    }
    deck[4] = "LM-027".to_string(); // Promo Scramble option.
    let result = validate_deck_for_game_mode(&deck, "eden").unwrap();
    assert!(!result.is_valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("EDEN Anomaly Protocol") && e.contains("got 5")));
}

#[test]
fn validate_eden_uses_custom_banlist_and_pair() {
    let mut banned_deck = make_eden_legal_deck();
    banned_deck[0] = "BT3-097".to_string();
    let banned = validate_deck_for_game_mode(&banned_deck, "eden").unwrap();
    assert!(!banned.is_valid);
    assert!(banned
        .errors
        .iter()
        .any(|e| e.contains("BT3-097") && e.contains("banned")));

    let mut pair_deck = make_eden_legal_deck();
    pair_deck[0] = "EX4-015".to_string();
    pair_deck[1] = "EX5-065".to_string();
    let pair = validate_deck_for_game_mode(&pair_deck, "eden").unwrap();
    assert!(!pair.is_valid);
    assert!(pair
        .errors
        .iter()
        .any(|e| e.contains("Choice restriction violated")));
}

#[test]
fn validate_eden_singleton_rejects_multiple_copies() {
    // make_eden_legal_deck() runs 4-ofs, which EDEN Singleton forbids.
    let deck = make_eden_legal_deck();
    let result = validate_deck_for_game_mode(&deck, "eden_singleton").unwrap();
    assert!(!result.is_valid);
    assert!(
        result.errors.iter().any(|e| e.contains("singleton")),
        "expected a singleton violation, got: {:?}",
        result.errors
    );
}

#[test]
fn validate_eden_singleton_accepts_a_50_distinct_card_deck() {
    // 50 distinct EDEN-legal Digimon, one copy each — singleton-legal.
    let db = card_database();
    let mut candidates: Vec<&str> = db
        .values()
        .filter(|c| {
            c.card_kind == 0
                && matches!(c.rarity, Rarity::C | Rarity::U)
                && !is_eden_restricted_or_choice_member(&c.card_id)
        })
        .map(|c| c.card_id.as_str())
        .collect();
    candidates.sort_unstable();
    let deck: Vec<String> = candidates.into_iter().take(50).map(String::from).collect();
    assert_eq!(deck.len(), 50, "need 50 distinct EDEN-legal C/U Digimon");
    let result = validate_deck_for_game_mode(&deck, "eden_singleton").unwrap();
    assert!(
        result.is_valid,
        "expected singleton deck to be legal, got errors: {:?}",
        result.errors
    );
}

#[test]
fn validate_eden_singleton_applies_eden_banlist() {
    let mut deck = make_eden_legal_deck();
    deck[0] = "BT3-097".to_string(); // EDEN-banned.
    let result = validate_deck_for_game_mode(&deck, "eden_singleton").unwrap();
    assert!(!result.is_valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("BT3-097") && e.contains("banned")));
}

#[test]
fn card_legality_reports_banned_card() {
    let leg = card_legality("BT2-090", "standard").unwrap(); // Matt Ishida, banned.
    assert!(!leg.legal);
    assert_eq!(leg.max_copies, 0);
    assert!(leg.reason.unwrap_or_default().to_lowercase().contains("banned"));
}

#[test]
fn card_legality_pauper_rejects_rare() {
    let leg = card_legality("AD1-001", "pauper").unwrap(); // Greymon, rarity R.
    assert!(!leg.legal);
    assert_eq!(leg.max_copies, 0);
}

#[test]
fn card_legality_eden_anomaly_card_is_legal_but_flagged() {
    let leg = card_legality("P-103", "eden").unwrap(); // Promo Training option (anomaly).
    assert!(leg.legal, "anomaly card should be legal: {leg:?}");
    assert!(leg
        .reason
        .unwrap_or_default()
        .to_lowercase()
        .contains("anomaly"));
}

#[test]
fn card_legality_eden_singleton_caps_at_one() {
    // Any otherwise-legal card is capped at a single copy under singleton.
    let leg = card_legality("P-103", "eden_singleton").unwrap();
    assert!(leg.legal);
    assert_eq!(leg.max_copies, 1);
}

#[test]
fn st2_cocytus_blue_starter_artifact_has_official_counts_and_valid_sizes() {
    let root: serde_json::Value =
        serde_json::from_str(DECK_LIBRARY_JSON).expect("deck_library.json parses");
    let decklist = root["archetypes"]["ST-2 Cocytus Blue"]["decklists"][0]["decklist"]
        .as_str()
        .expect("ST-2 decklist uses deck_library string convention");
    let deck: Vec<String> = serde_json::from_str(decklist).expect("ST-2 decklist parses");

    let counts = summarize_deck(&deck);
    let expected = [
        ("ST2-01", 4u32),
        ("ST2-02", 4),
        ("ST2-03", 4),
        ("ST2-04", 4),
        ("ST2-05", 4),
        ("ST2-06", 2),
        ("ST2-07", 4),
        ("ST2-08", 4),
        ("ST2-09", 4),
        ("ST2-10", 2),
        ("ST2-11", 2),
        ("ST2-12", 4),
        ("ST2-13", 4),
        ("ST2-14", 4),
        ("ST2-15", 2),
        ("ST2-16", 2),
    ];
    for (card_id, count) in expected {
        assert_eq!(counts.get(card_id), Some(&count), "{card_id} count");
    }
    assert_eq!(deck.len(), 54);

    let parsed = classify_parsed(deck.clone());
    assert_eq!(parsed.egg_deck.len(), 4);
    assert_eq!(parsed.main_deck.len(), 50);

    let result = validate_deck_for_game_mode(&deck, "no_restriction").unwrap();
    assert!(
        result.is_valid,
        "starter artifact should satisfy construction sizes/copy limits in no_restriction mode: {:?}",
        result.errors
    );
}

// ─── summarize_deck ───────────────────────────────────────────────────

#[test]
fn summarize_deck_counts_duplicates() {
    let deck = vec!["A-1".to_string(), "A-1".to_string(), "B-2".to_string()];
    let counts = summarize_deck(&deck);
    assert_eq!(counts.get("A-1"), Some(&2));
    assert_eq!(counts.get("B-2"), Some(&1));
}
