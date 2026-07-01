//! Deck Creation — surface D (deck-validation). Ledger rows DC-1..DC-5.
//!
//! Asserted via `digimon_engine::deck_tools::validate_deck` (the flat card-id
//! list is bucketed into main deck vs Digi-Egg deck by `card_kind`):
//!   DC-1  identity is by card NUMBER; ≤4 copies per number.
//!   DC-2/3 Digi-Eggs only ever bucket to the egg deck (structural).
//!   DC-4  egg deck holds ≤5 cards.
//!   DC-5  the main deck must be exactly 50 on its own.

#![allow(unused_imports)]

use digimon_engine::deck_tools::{card_database, validate_deck};

/// 50-card legal main deck: 13 distinct ≤4-copy Digimon × 4 copies, trimmed to
/// 50 (mirrors the deck_tools fixture). `max_count_in_deck >= 4` excludes
/// banned (limit 0) / restricted (limit 1) cards.
fn legal_main_deck() -> Vec<String> {
    let db = card_database();
    let mut candidates: Vec<&str> = db
        .values()
        .filter(|c| c.card_kind == 0 && c.max_count_in_deck >= 4)
        .map(|c| c.card_id.as_str())
        .collect();
    candidates.sort_unstable();
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

/// Distinct Digi-Egg (card_kind == 3) ids, sorted for determinism.
fn distinct_eggs(n: usize) -> Vec<String> {
    let db = card_database();
    let mut eggs: Vec<String> = db
        .values()
        .filter(|c| c.card_kind == 3)
        .map(|c| c.card_id.clone())
        .collect();
    eggs.sort_unstable();
    assert!(eggs.len() >= n, "need {n} distinct Digi-Eggs in the DB");
    eggs.into_iter().take(n).collect()
}

/// Baseline: a 50-card main deck (no egg deck) is legal.
#[test]
fn dc_baseline_fifty_card_deck_is_legal() {
    let result = validate_deck(&legal_main_deck());
    assert!(
        result.is_valid,
        "a 50-card main deck must be legal; errors: {:?}",
        result.errors
    );
}

/// DC-5 — the main deck must be exactly 50: 49 or 51 cards is illegal.
#[test]
fn dc5_main_deck_must_be_exactly_fifty() {
    let mut short = legal_main_deck();
    short.pop();
    assert!(
        !validate_deck(&short).is_valid,
        "FAQ: a 49-card main deck is illegal (must be exactly 50)"
    );

    let mut long = legal_main_deck();
    long.push(long[0].clone()); // would be a 5th copy AND 51 cards
    assert!(
        !validate_deck(&long).is_valid,
        "FAQ: a 51-card main deck is illegal (must be exactly 50)"
    );
}

/// DC-1 — copy limit is per card NUMBER: a 5th copy of one number is illegal.
#[test]
fn dc1_at_most_four_copies_per_card_number() {
    let mut deck = legal_main_deck();
    // Replace the last slot with a 5th copy of the first id → 5 copies of one
    // number, still 50 cards total.
    let first = deck[0].clone();
    let last = deck.len() - 1;
    deck[last] = first;
    assert!(
        !validate_deck(&deck).is_valid,
        "FAQ: at most 4 copies of a card with the same card number"
    );
}

/// DC-4 — the Digi-Egg deck holds at most 5 cards. Six DISTINCT eggs (1 copy
/// each, so no copy-limit confound) appended to a legal 50 main → illegal.
#[test]
fn dc4_egg_deck_holds_at_most_five() {
    let mut deck = legal_main_deck();
    deck.extend(distinct_eggs(6));
    assert!(
        !validate_deck(&deck).is_valid,
        "FAQ: the Digi-Egg deck can hold at most 5 cards"
    );
}

/// DC-2/DC-3 (structural) — Digi-Eggs bucket to the egg deck, NOT the main 50:
/// a legal 50 main + up to 5 eggs is legal, and the eggs do not displace main
/// cards (you cannot smuggle a Digi-Egg into the main deck).
#[test]
fn dc2_eggs_bucket_to_egg_deck_not_main() {
    let mut deck = legal_main_deck();
    deck.extend(distinct_eggs(5));
    let result = validate_deck(&deck);
    assert!(
        result.is_valid,
        "50 main + 5 eggs is legal (eggs bucket separately); errors: {:?}",
        result.errors
    );
}
