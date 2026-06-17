//! The `starter` format must accept the as-printed ST-2 starter deck
//! (ST2-13 x4), which the `standard` banlist Limits to 1 copy.

use digimon_engine::deck_tools::validate_deck_for_game_mode;

const STARTER_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/starter_decks.json"));

fn st2_cards() -> Vec<String> {
    let v: serde_json::Value = serde_json::from_str(STARTER_JSON).expect("parse starter_decks.json");
    let decks = v["starter_decks"].as_array().expect("starter_decks array");
    let st2 = decks
        .iter()
        .find(|d| d["id"] == "starter_st2_cocytus_blue")
        .expect("ST-2 deck present");
    let mut cards: Vec<String> = Vec::new();
    for key in ["main_deck", "egg_deck"] {
        for c in st2[key].as_array().expect("deck array") {
            cards.push(c.as_str().expect("card id string").to_string());
        }
    }
    cards
}

#[test]
fn st2_starter_deck_is_legal_in_starter_format() {
    let cards = st2_cards();
    let res = validate_deck_for_game_mode(&cards, "starter").expect("starter format exists");
    assert!(
        res.is_valid,
        "ST-2 starter deck should be legal in the starter format; errors: {:?}",
        res.errors
    );
}

#[test]
fn st2_starter_deck_is_illegal_in_standard_due_to_limited_card() {
    let cards = st2_cards();
    let res = validate_deck_for_game_mode(&cards, "standard").expect("standard format exists");
    assert!(
        !res.is_valid,
        "ST-2 deck runs ST2-13 x4, over the standard Limited cap of 1"
    );
    assert!(
        res.errors.iter().any(|e| e.contains("ST2-13")),
        "expected a limit error mentioning ST2-13; got {:?}",
        res.errors
    );
}
