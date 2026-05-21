//! Behavioral tests for `GreedyPolicy`. Covers mulligan heuristic and
//! end-to-end self-play via `HeadlessRunner` — a cheap tripwire that
//! catches hangs or obviously-illegal action selection.

use std::collections::HashMap;

use digimon_engine::action::space::{ACTION_SPACE_SIZE, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::policies::{GreedyPolicy, Policy};
use digimon_engine::{HeadlessRunner, RandomPolicy};

fn test_card_db() -> HashMap<String, CardData> {
    let json = r#"{
        "BT1-001": {
            "card_id": "BT1-001", "card_name_eng": "Koromon",
            "card_effect_class_name": "BT1_001", "play_cost": 0, "dp": -1,
            "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
            "type_eng": ["Lesser"], "form_eng": ["In-Training"], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "BT1-010": {
            "card_id": "BT1-010", "card_name_eng": "Agumon",
            "card_effect_class_name": "BT1_010", "play_cost": 3, "dp": 2000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Reptile"], "form_eng": ["Rookie"], "attribute_eng": ["Vaccine"],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "BT1-025": {
            "card_id": "BT1-025", "card_name_eng": "Greymon",
            "card_effect_class_name": "BT1_025", "play_cost": 5, "dp": 5000,
            "level": 4, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Dinosaur"], "form_eng": ["Champion"], "attribute_eng": ["Vaccine"],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "",
            "evo_costs": [{"card_color": 0, "level": 3, "memory_cost": 2}]
        }
    }"#;
    CardData::load_from_str(json).unwrap()
}

fn deck_with_level3() -> Vec<String> {
    let mut d = Vec::new();
    for _ in 0..4 {
        d.push("BT1-001".to_string());
    }
    for _ in 0..10 {
        d.push("BT1-010".to_string());
    }
    for _ in 0..10 {
        d.push("BT1-025".to_string());
    }
    d
}

#[test]
fn empty_mask_returns_pass() {
    let db = test_card_db();
    let runner = HeadlessRunner::new(
        deck_with_level3(),
        deck_with_level3(),
        &db,
        false,
        false,
        false,
        Some(1),
    )
    .unwrap();
    let mut greedy = GreedyPolicy::new();
    let empty = vec![0.0f32; ACTION_SPACE_SIZE];
    let action = greedy.select(&runner.game, &empty);
    assert_eq!(action, PASS);
}

#[test]
fn mulligan_with_level3_keeps() {
    // With at least one level-3 Digimon in the opening hand, the policy
    // must pick action 0 (keep). Decks are all Agumon (level 3) to make
    // the opening hand guaranteed to contain one.
    let db = test_card_db();
    let mut all_agumon = Vec::new();
    for _ in 0..4 {
        all_agumon.push("BT1-001".to_string());
    }
    for _ in 0..20 {
        all_agumon.push("BT1-010".to_string());
    }
    let runner = HeadlessRunner::new(
        all_agumon.clone(),
        all_agumon,
        &db,
        false,
        false,
        false,
        Some(7),
    )
    .unwrap();

    let mut greedy = GreedyPolicy::new();
    let mask = runner.get_action_mask();
    let action = greedy.select(&runner.game, &mask);
    assert_eq!(
        action, 0,
        "greedy should keep (action 0) when hand contains level-3"
    );
}

#[test]
fn mulligan_without_level3_mulligans() {
    // Deck has only level-2 (Koromon, DigiEgg) and level-4 (Greymon) — no
    // level-3, so greedy should mulligan. Note: Koromon is a DigiEgg, it
    // lives in the digitama deck, not main deck. For main deck use only
    // level-4 Greymon which has no level-3 promotion target in hand.
    let db = test_card_db();
    let mut no_level3 = Vec::new();
    for _ in 0..4 {
        no_level3.push("BT1-001".to_string()); // egg
    }
    for _ in 0..20 {
        no_level3.push("BT1-025".to_string()); // level 4
    }
    let runner = HeadlessRunner::new(
        no_level3.clone(),
        no_level3,
        &db,
        false,
        false,
        false,
        Some(13),
    )
    .unwrap();

    let mut greedy = GreedyPolicy::new();
    let mask = runner.get_action_mask();
    let action = greedy.select(&runner.game, &mask);
    assert_eq!(
        action, 1,
        "greedy should mulligan (action 1) when hand has no level-3"
    );
}

#[test]
fn greedy_vs_greedy_completes() {
    // End-to-end: greedy on both seats runs to conclusion without panic
    // and declares a winner. This is the cheap tripwire that catches
    // obviously-illegal action selection.
    let db = test_card_db();
    let mut runner = HeadlessRunner::new(
        deck_with_level3(),
        deck_with_level3(),
        &db,
        false,
        false,
        false,
        Some(42),
    )
    .unwrap();

    let mut greedy = GreedyPolicy::new();
    let winner = runner.run_until_conclusion(
        2000,
        Some(|g: &digimon_engine::Game, m: &[f32]| greedy.select(g, m)),
    );
    assert_ne!(winner, u8::MAX, "game should declare a winner");
    assert!(runner.is_game_over());
}

#[test]
fn random_vs_random_completes() {
    let db = test_card_db();
    let mut runner = HeadlessRunner::new(
        deck_with_level3(),
        deck_with_level3(),
        &db,
        false,
        false,
        false,
        Some(99),
    )
    .unwrap();

    let mut rng_policy = RandomPolicy::new(Some(123));
    let winner = runner.run_until_conclusion(
        5000,
        Some(|g: &digimon_engine::Game, m: &[f32]| rng_policy.select(g, m)),
    );
    assert_ne!(winner, u8::MAX, "random vs random should declare a winner");
}
