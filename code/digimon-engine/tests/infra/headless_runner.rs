//! Smoke tests for the `HeadlessRunner` — matches behavior of
//! `digimon_gym.engine.runners.headless_game.HeadlessGame`.

use std::collections::HashMap;

use digimon_engine::action::ACTION_SPACE_SIZE;
use digimon_engine::card_data::CardData;
use digimon_engine::tensor::TENSOR_SIZE;
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

fn test_deck() -> Vec<String> {
    let mut deck = Vec::new();
    for _ in 0..4 {
        deck.push("BT1-001".to_string());
    }
    for _ in 0..10 {
        deck.push("BT1-010".to_string());
    }
    for _ in 0..10 {
        deck.push("BT1-025".to_string());
    }
    deck
}

#[test]
fn new_runner_starts_in_mulligan() {
    let db = test_card_db();
    let d1 = test_deck();
    let d2 = test_deck();
    let runner = HeadlessRunner::new(d1, d2, &db, false, false, false, Some(42)).unwrap();

    // Mulligan pending for both players until they decide.
    assert!(runner.mulligan_current_player().is_some());
    assert!(!runner.is_game_over());
    assert_eq!(runner.winner_id(), u8::MAX);
}

#[test]
fn mask_and_tensor_sizes_match_layout() {
    let db = test_card_db();
    let runner =
        HeadlessRunner::new(test_deck(), test_deck(), &db, false, false, false, Some(1)).unwrap();

    assert_eq!(runner.get_action_mask().len(), ACTION_SPACE_SIZE);
    assert_eq!(runner.get_board_tensor(None).len(), TENSOR_SIZE);
    assert_eq!(runner.get_board_tensor(Some(0)).len(), TENSOR_SIZE);
    assert_eq!(runner.get_board_tensor(Some(1)).len(), TENSOR_SIZE);
}

#[test]
fn step_is_noop_after_game_over() {
    let db = test_card_db();
    let mut runner =
        HeadlessRunner::new(test_deck(), test_deck(), &db, false, false, false, Some(7)).unwrap();

    runner.game.declare_winner(1);
    assert!(runner.is_game_over());

    // Stepping after game_over must not panic or mutate phase away from GameOver.
    runner.step(62);
    assert!(runner.is_game_over());
    assert_eq!(runner.winner_id(), 1);
}

#[test]
fn default_policy_reaches_conclusion() {
    let db = test_card_db();
    let mut runner =
        HeadlessRunner::new(test_deck(), test_deck(), &db, false, false, false, Some(99)).unwrap();

    // No explicit policy — runner falls back to PASS-everything.
    // The game should terminate within the turn cap, either by deck-out
    // (decks are only 24 cards) or by the tiebreaker (declare player 0).
    let winner = runner.run_until_conclusion::<fn(&_, &[f32]) -> u16>(2000, None);
    assert!(runner.is_game_over());
    assert_ne!(winner, u8::MAX, "tiebreaker should declare a winner");
}

#[test]
fn mulligan_accept_advances() {
    let db = test_card_db();
    let mut runner =
        HeadlessRunner::new(test_deck(), test_deck(), &db, false, false, false, Some(3)).unwrap();

    let first = runner.mulligan_current_player().expect("mulligan pending");
    runner.accept_mulligan(first, true).unwrap();
    let second = runner.mulligan_current_player().expect("second decision");
    assert_ne!(first, second);
    runner.accept_mulligan(second, true).unwrap();

    // Both kept → mulligan complete, turn 1 has begun.
    assert!(runner.mulligan_current_player().is_none());
    assert_eq!(runner.game.turn_count, 1);
}
