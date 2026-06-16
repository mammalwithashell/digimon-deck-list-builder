//! Tauri commands for deck parsing / validation / alpha-pool allowlist.
//!
//! Thin wrappers over `digimon_engine::deck_tools`; response shapes match
//! the hosted API's `/decks/parse`, `/decks/validate`, and
//! `/decks/tested-cards` endpoints so `frontend/src/api/deckApi.ts` can
//! dispatch either backend without branching on the response parser.

use std::collections::HashMap;

use digimon_engine::deck_tools::{
    card_legality as engine_card_legality, card_legality_bulk as engine_card_legality_bulk,
    classify_parsed, out_of_set_cards, parse_deck, summarize_deck, tested_card_metadata,
    tested_cards_sorted, validate_deck_for_game_mode, CardMeta,
};
use digimon_engine::format::{self, RarityPolicy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseDeckDto {
    pub main_deck: Vec<String>,
    pub egg_deck: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateDeckDto {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub summary: HashMap<String, u32>,
    pub total_cards: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestedCardsDto {
    pub card_ids: Vec<String>,
    pub card_count: usize,
}

/// Parse a raw deck string (TTS JSON or digimoncard.io text) and classify
/// cards into main / egg / unknown — mirrors `POST /decks/parse`.
#[tauri::command]
pub fn rust_parse_deck(deck: String) -> Result<ParseDeckDto, String> {
    let ids = parse_deck(&deck)?;
    let parsed = classify_parsed(ids);
    Ok(ParseDeckDto {
        main_deck: parsed.main_deck,
        egg_deck: parsed.egg_deck,
        warnings: parsed.warnings,
    })
}

/// Validate a deck described as two arrays (main + egg). Also applies the
/// alpha-pool gate — any card lacking behavioural test coverage becomes a
/// hard error so out-of-scope cards can't reach the engine. Response shape
/// matches the Python endpoint exactly.
#[tauri::command]
pub fn rust_validate_deck_raw(
    main_deck: Vec<String>,
    egg_deck: Vec<String>,
    game_mode: Option<String>,
) -> Result<ValidateDeckDto, String> {
    let mut card_ids = Vec::with_capacity(main_deck.len() + egg_deck.len());
    card_ids.extend_from_slice(&main_deck);
    card_ids.extend_from_slice(&egg_deck);
    if card_ids.is_empty() {
        return Err("Provide deck or main_deck/egg_deck".to_string());
    }

    let result =
        validate_deck_for_game_mode(&card_ids, game_mode.as_deref().unwrap_or("standard"))?;
    let summary = summarize_deck(&card_ids);
    let total_cards = card_ids.len();

    let mut errors = result.errors;
    let out_of_pool = out_of_set_cards(&card_ids);
    let mut is_valid = result.is_valid;
    if !out_of_pool.is_empty() {
        for cid in &out_of_pool {
            errors.push(format!(
                "Card {cid} is not available in the alpha release (no test coverage)"
            ));
        }
        is_valid = false;
    }

    Ok(ValidateDeckDto {
        is_valid,
        errors,
        warnings: result.warnings,
        summary,
        total_cards,
    })
}

/// Return the sorted alpha-release allowlist. Desktop deck-builder uses
/// this to grey out unsupported cards in the collection browser.
#[tauri::command]
pub fn rust_list_tested_cards() -> Result<TestedCardsDto, String> {
    let ids = tested_cards_sorted();
    let count = ids.len();
    Ok(TestedCardsDto {
        card_ids: ids,
        card_count: count,
    })
}

/// Full display metadata for every implemented (allowlisted) card, sorted
/// by card ID — mirrors `GET /decks/card-database`. The deck builder seeds
/// its browse pool from this so the whole implemented pool is visible
/// offline, instead of whatever a remote search happened to return.
#[tauri::command]
pub fn rust_card_database() -> Result<Vec<CardMeta>, String> {
    Ok(tested_card_metadata().to_vec())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub deck_size: u16,
    pub egg_max: u8,
    pub rarity_policy: String,
    pub singleton: bool,
    pub default_max_copies: u8,
    pub playable: bool,
}

/// All deck formats from the engine registry, in declaration order — the
/// deck-builder format selector and the play/format catalog are built from
/// this so the frontend never hardcodes a (drifting) format list. Mirrors
/// the hosted API's `GET /decks/formats`.
#[tauri::command]
pub fn rust_list_formats() -> Vec<FormatDto> {
    format::list_formats()
        .iter()
        .map(|f| FormatDto {
            id: f.id.clone(),
            name: f.name.clone(),
            description: f.description.clone(),
            deck_size: f.deck_size,
            egg_max: f.egg_max,
            rarity_policy: match f.rarity_policy {
                RarityPolicy::All => "all",
                RarityPolicy::CommonUncommon => "common_uncommon",
                RarityPolicy::EdenAnomaly => "eden_anomaly",
            }
            .to_string(),
            singleton: f.singleton,
            default_max_copies: f.default_max_copies,
            playable: f.playable,
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardLegalityDto {
    pub legal: bool,
    pub max_copies: u32,
    pub reason: Option<String>,
}

/// Per-card legality under a game mode — drives the deck-builder pool
/// "format-legal only" filter and per-card ban/limit/anomaly badges. Mirrors
/// `GET /decks/card-legality`.
#[tauri::command]
pub fn rust_card_legality(card_id: String, game_mode: Option<String>) -> Result<CardLegalityDto, String> {
    let mode = game_mode.unwrap_or_else(|| "standard".to_string());
    let leg = engine_card_legality(&card_id, &mode)?;
    Ok(CardLegalityDto {
        legal: leg.legal,
        max_copies: leg.max_copies,
        reason: leg.reason,
    })
}

/// Legality for every tested-allowlist card under a game mode, keyed by card
/// id — one call drives the deck-builder pool legality filter + badges.
#[tauri::command]
pub fn rust_card_legality_bulk(
    game_mode: Option<String>,
) -> Result<HashMap<String, CardLegalityDto>, String> {
    let mode = game_mode.unwrap_or_else(|| "standard".to_string());
    let map = engine_card_legality_bulk(&mode)?;
    Ok(map
        .into_iter()
        .map(|(k, leg)| {
            (
                k,
                CardLegalityDto {
                    legal: leg.legal,
                    max_copies: leg.max_copies,
                    reason: leg.reason,
                },
            )
        })
        .collect())
}
