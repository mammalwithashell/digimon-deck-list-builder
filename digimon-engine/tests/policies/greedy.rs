//! Behavioral tests for the heuristic greedy opponent. Locks specific
//! decisions in place so future edits to the heuristic can be caught by
//! deterministic, small-scenario assertions.

use std::collections::HashMap;

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::policies::greedy_action;
use digimon_engine::HeadlessRunner;

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
    let empty = vec![0.0f32; 2168];
    assert_eq!(greedy_action(&runner.game, &empty), PASS);
}

#[test]
fn mulligan_with_level3_keeps() {
    // With every Rookie slot filled with a level-3 Digimon (Agumon), the
    // opening hand is guaranteed to contain one — the heuristic must pick
    // keep (action 0).
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

    let mask = runner.get_action_mask();
    assert_eq!(
        greedy_action(&runner.game, &mask),
        0,
        "greedy should keep (action 0) when hand contains level-3"
    );
}

#[test]
fn mulligan_without_level3_mulligans() {
    // No level-3 Digimon — only DigiEggs and Champions — forces mulligan
    // (action 1). Champions are unplayable without a Rookie to digivolve
    // from, so the heuristic prefers to redraw.
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

    let mask = runner.get_action_mask();
    assert_eq!(
        greedy_action(&runner.game, &mask),
        1,
        "greedy should mulligan (action 1) when hand has no level-3"
    );
}
