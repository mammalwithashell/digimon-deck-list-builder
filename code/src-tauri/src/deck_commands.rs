//! Tauri commands for deck parsing / validation / alpha-pool allowlist.
//!
//! Thin wrappers over `digimon_engine::deck_tools`; response shapes match
//! the hosted API's `/decks/parse`, `/decks/validate`, and
//! `/decks/tested-cards` endpoints so `frontend/src/api/deckApi.ts` can
//! dispatch either backend without branching on the response parser.

use std::collections::HashMap;

use digimon_engine::deck_tools::{
    classify_parsed, out_of_set_cards, parse_deck, summarize_deck, tested_cards_sorted,
    validate_deck_for_game_mode,
};
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
