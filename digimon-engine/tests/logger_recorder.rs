//! Logger + recorder integration tests. Ports
//! `digimon_gym/engine/loggers.py` and `digimon_gym/engine/recording.py`
//! parity checks.

use std::collections::HashMap;

use digimon_engine::action::space::encode_digivolve;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::game::Game;
use digimon_engine::logger::{GameLogger, SilentLogger, VerboseLogger};
use digimon_engine::permanent::Permanent;
use digimon_engine::rules::Rules;

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
            "security_effect_description_eng": "",
            "evo_costs": [{"card_color": 0, "level": 2, "memory_cost": 1}]
        },
        "BT1-025": {
            "card_id": "BT1-025", "card_name_eng": "Greymon",
            "card_effect_class_name": "BT1_025", "play_cost": 5, "dp": 5000,
            "level": 4, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Dinosaur"], "form_eng": ["Champion"], "attribute_eng": ["Vaccine"],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "",
            "evo_costs": [{"card_color": 0, "level": 3, "memory_cost": 2}]
        },
        "BT1-085": {
            "card_id": "BT1-085", "card_name_eng": "Tai Kamiya",
            "card_effect_class_name": "BT1_085", "play_cost": 2, "dp": -1,
            "level": -1, "card_kind": 1, "rarity": 0, "card_colors": [0],
            "type_eng": ["DigiDestined"], "form_eng": [], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "BT1-093": {
            "card_id": "BT1-093", "card_name_eng": "Gaia Force",
            "card_effect_class_name": "BT1_093", "play_cost": 8, "dp": -1,
            "level": -1, "card_kind": 2, "rarity": 0, "card_colors": [0],
            "type_eng": [], "form_eng": [], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        }
    }"#;
    CardData::load_from_str(json).unwrap()
}

fn test_deck() -> Vec<String> {
    let mut deck = Vec::new();
    for _ in 0..4 { deck.push("BT1-001".to_string()); }
    for _ in 0..6 { deck.push("BT1-010".to_string()); }
    for _ in 0..6 { deck.push("BT1-025".to_string()); }
    for _ in 0..4 { deck.push("BT1-085".to_string()); }
    for _ in 0..4 { deck.push("BT1-093".to_string()); }
    deck
}

fn fresh_game() -> Game {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::standard();
    Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap()
}

#[test]
fn silent_logger_discards() {
    let mut l = SilentLogger;
    l.log("hello");
    l.log_verbose("world");
    assert!(l.get_logs().is_empty());
    l.clear();
    assert!(l.get_logs().is_empty());
}

#[test]
fn verbose_logger_buffers_and_prefixes_verbose() {
    let mut l = VerboseLogger::new();
    l.log("hello");
    l.log_verbose("world");
    let logs = l.get_logs();
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0], "hello");
    assert_eq!(logs[1], "[VERBOSE] world");
    l.clear();
    assert!(l.get_logs().is_empty());
}

#[test]
fn game_rejects_out_of_range_digivolve_and_logs_reason() {
    let mut game = fresh_game();
    game.set_logger(Box::new(VerboseLogger::new()));
    game.start_game();
    game.enter_main_phase();
    let tp = game.turn_player();

    // Seat a permanent so field_index 0 is valid; hand index 99 is not.
    let data_idx = game
        .card_data
        .iter()
        .position(|d| d.card_id == "BT1-010")
        .unwrap();
    let card_idx = game.next_card_index();
    let card = CardSource::new(data_idx, tp, card_idx);
    let turn = game.turn_count;
    game.player_mut(tp).battle_area.push(Permanent::new(card, turn));

    let action = encode_digivolve(20, 0);
    assert!(!game.decode_action(action, tp));

    let logs = game.logger.get_logs();
    assert!(
        logs.iter()
            .any(|l| l.contains("[Rejected]") && l.contains("hand index 20")),
        "expected a '[Rejected] ... hand index 20' message, got logs: {:?}",
        logs
    );
}

#[test]
fn game_default_logger_is_silent() {
    let mut game = fresh_game();
    game.start_game();
    game.enter_main_phase();
    let tp = game.turn_player();

    let action = encode_digivolve(20, 0);
    let _ = game.decode_action(action, tp);

    // Default SilentLogger returns empty logs.
    assert!(game.logger.get_logs().is_empty());
}
