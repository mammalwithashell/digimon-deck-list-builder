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
use digimon_engine::recorder::GameRecorder;
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

fn fresh_game() -> (Game, Vec<String>) {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::standard();
    let game = Game::new(&[deck.clone(), deck.clone()], &db, rules, Some(42)).unwrap();
    (game, deck)
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
    let (mut game, _deck) = fresh_game();
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
    game.decode_action(action, tp);

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
    let (mut game, _deck) = fresh_game();
    game.start_game();
    game.enter_main_phase();
    let tp = game.turn_player();

    let action = encode_digivolve(20, 0);
    let _ = game.decode_action(action, tp);

    // Default SilentLogger returns empty logs.
    assert!(game.logger.get_logs().is_empty());
}

#[test]
fn recorder_captures_initial_state_after_start() {
    let (mut game, deck) = fresh_game();
    game.start_game();

    let mut rec = GameRecorder::new(false);
    rec.capture_initial_state(&game, (&deck, &deck));

    let init = rec.initial_state().expect("initial state captured");
    assert_eq!(init.first_player_id, game.turn_player());

    for p in [&init.player1, &init.player2] {
        assert_eq!(p.initial_hand.len(), 5, "opening hand is 5 cards");
        assert_eq!(p.security_order.len(), 5, "security is 5 cards after mulligan");
        assert_eq!(p.digitama_library_order.len(), 4, "egg deck has 4 eggs");
    }
}

#[test]
fn recorder_record_and_finalize_action_captures_deltas() {
    let (mut game, deck) = fresh_game();
    game.start_game();
    game.enter_main_phase();
    let tp = game.turn_player();

    let mut rec = GameRecorder::new(false);
    rec.capture_initial_state(&game, (&deck, &deck));

    let action_id = 62u16; // PASS
    let mem_before = game.memory;
    let turn_before = game.turn_count;

    let idx = rec.record_action(&game, action_id, tp);
    let _ = game.decode_action(action_id, tp);
    rec.finalize_action(idx, &game);

    assert_eq!(rec.actions().len(), 1);
    let a = &rec.actions()[0];
    assert_eq!(a.step_number, 1);
    assert_eq!(a.action_id, action_id);
    assert_eq!(a.player_id, tp);
    assert_eq!(a.memory_before, mem_before);
    assert_eq!(a.memory_after, game.memory);
    assert_eq!(a.turn_number, turn_before);
    assert_eq!(a.is_game_over, game.game_over);
    assert_eq!(a.winner_id, game.winner);
}

#[test]
fn recorder_to_json_has_expected_top_level_keys() {
    let (mut game, deck) = fresh_game();
    game.start_game();

    let mut rec = GameRecorder::new(false);
    rec.capture_initial_state(&game, (&deck, &deck));
    let json = rec.to_json();

    assert!(json.get("initial_state").is_some());
    assert!(json.get("actions").is_some());
    assert!(json["actions"].is_array());
    assert_eq!(json["actions"].as_array().unwrap().len(), 0);
    assert_eq!(json["total_actions"], serde_json::json!(0));
    assert_eq!(json["tensor_snapshots_count"], serde_json::json!(0));
    // "tensor_snapshots" is omitted when empty — matches Python's conditional
    // inclusion in GameRecorder.to_dict (recording.py:213-222).
    assert!(
        json.get("tensor_snapshots").is_none(),
        "tensor_snapshots should be absent when empty"
    );
}
