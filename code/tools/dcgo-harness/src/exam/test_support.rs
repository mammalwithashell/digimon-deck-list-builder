//! Fixtures shared by the exam module's unit tests. Declared behind
//! `#[cfg(test)]` in `mod.rs`, so none of this ships in the binary.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::enums::CardKind;

/// The ST-1 "Gaia Red" starter list as printed: 54 cards, of which the 4 copies
/// of `ST1-01` are the digi-egg deck and the other 50 are the main deck.
///
/// Kept as one flat list (rather than pre-split) so the split below is derived
/// from `cards.json`'s own `card_kind` rather than hard-coded here.
const ST1: &[&str] = &[
    "ST1-01", "ST1-01", "ST1-01", "ST1-01", "ST1-02", "ST1-02", "ST1-02", "ST1-02", "ST1-03",
    "ST1-03", "ST1-03", "ST1-03", "ST1-04", "ST1-04", "ST1-04", "ST1-04", "ST1-05", "ST1-05",
    "ST1-05", "ST1-05", "ST1-06", "ST1-06", "ST1-06", "ST1-06", "ST1-07", "ST1-07", "ST1-08",
    "ST1-08", "ST1-08", "ST1-08", "ST1-09", "ST1-09", "ST1-09", "ST1-09", "ST1-10", "ST1-10",
    "ST1-11", "ST1-11", "ST1-12", "ST1-12", "ST1-12", "ST1-12", "ST1-13", "ST1-13", "ST1-13",
    "ST1-13", "ST1-14", "ST1-14", "ST1-14", "ST1-14", "ST1-15", "ST1-15", "ST1-16", "ST1-16",
];

/// Loads the real card pool from `data/cards.json`.
///
/// Repo-root resolution matches the engine's own test precedent
/// (`code/digimon-engine/tests/bench_engine_throughput.rs`): relative to
/// `CARGO_MANIFEST_DIR`, with a `DIGIMON_REPO_ROOT` override for callers that
/// relocate the data dir. A failure here means the pool moved or changed —
/// tests never invent their own card data.
pub fn load_card_data() -> HashMap<String, CardData> {
    let root = std::env::var("DIGIMON_REPO_ROOT")
        .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../..").to_string());
    let path = format!("{root}/data/cards.json");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
    CardData::load_from_str(&text).expect("cards.json should parse")
}

/// The ST-1 starter deck, split into `(main_deck_50, egg_deck_4)`.
///
/// The split is taken from the loaded card pool (`CardKind::DigiEgg`) rather
/// than assumed, so a data change that reclassifies a card fails the assertion
/// below instead of silently producing an illegal deck.
///
/// `Game::new` does not validate deck legality, but DCGO gates battles on
/// `DeckData.IsValidDeckData()` (50 main, <= 5 egg), so any scenario meant to
/// mirror DCGO must use a tournament-legal list — this one is.
pub fn st1_decks() -> (Vec<String>, Vec<String>) {
    let card_data = load_card_data();
    let mut main = Vec::new();
    let mut egg = Vec::new();
    for id in ST1 {
        let data = card_data
            .get(*id)
            .unwrap_or_else(|| panic!("{id} missing from cards.json"));
        if data.card_kind == CardKind::DigiEgg {
            egg.push((*id).to_string());
        } else {
            main.push((*id).to_string());
        }
    }
    (main, egg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_pool_loads_and_st1_splits_50_4() {
        let card_data = load_card_data();
        assert!(
            !card_data.is_empty(),
            "cards.json loaded but produced an empty pool"
        );

        let (main, egg) = st1_decks();
        assert_eq!(main.len(), 50, "ST-1 main deck should be 50 cards");
        assert_eq!(egg.len(), 4, "ST-1 egg deck should be 4 cards");
        assert!(
            egg.iter().all(|id| id == "ST1-01"),
            "the ST-1 egg deck is 4x ST1-01, got {egg:?}"
        );
    }
}
