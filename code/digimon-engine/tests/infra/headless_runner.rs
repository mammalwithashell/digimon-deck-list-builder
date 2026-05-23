//! Smoke tests for the `HeadlessRunner` — matches behavior of
//! `digimon_gym.engine.runners.headless_game.HeadlessGame`.

use std::collections::HashMap;

use digimon_engine::action::ACTION_SPACE_SIZE;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::observation::parse_observation_profile;
use digimon_engine::tensor::TENSOR_SIZE;
use digimon_engine::{HeadlessRunner, TerminalOutcomeReason};

use crate::dsl_card_data::card_data_from_compiled;

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

fn sample_runner() -> HeadlessRunner {
    let db = test_card_db();
    HeadlessRunner::new(test_deck(), test_deck(), &db, false, false, false, Some(1)).unwrap()
}

fn sample_runner_with_observation_profile(profile_id: &str) -> HeadlessRunner {
    let db = test_card_db();
    let profile = parse_observation_profile(profile_id).unwrap();
    HeadlessRunner::new_with_observation_profile(
        test_deck(),
        test_deck(),
        &db,
        false,
        false,
        false,
        Some(1),
        profile,
    )
    .unwrap()
}

fn dsl_slice_card_db() -> HashMap<String, CardData> {
    ["BT15-003", "BT17-007", "BT17-015", "BT22-084", "AD1-025"]
        .into_iter()
        .map(|id| (id.to_string(), card_data_from_compiled(id)))
        .collect()
}

fn dsl_slice_deck() -> Vec<String> {
    let mut deck = Vec::new();
    deck.extend(std::iter::repeat("BT15-003".to_string()).take(4));
    deck.extend(std::iter::repeat("BT17-007".to_string()).take(12));
    deck.extend(std::iter::repeat("BT17-015".to_string()).take(8));
    deck.extend(std::iter::repeat("BT22-084".to_string()).take(8));
    deck.extend(std::iter::repeat("AD1-025".to_string()).take(8));
    deck
}

#[test]
fn runner_default_observation_profile_is_standard_lite_v2() {
    let runner = sample_runner();

    assert_eq!(runner.observation_profile_id(), "standard_lite_v2");
    assert_eq!(runner.observation_layout().tensor_size, 8410);
    assert_eq!(runner.get_board_tensor(None).len(), 8410);
}

#[test]
fn runner_can_use_standard_compact_v1_observation_profile() {
    let runner = sample_runner_with_observation_profile("standard_compact_v1");

    assert_eq!(runner.observation_profile_id(), "standard_compact_v1");
    assert_eq!(runner.observation_layout().tensor_size, TENSOR_SIZE);
    assert_eq!(runner.get_board_tensor(None).len(), TENSOR_SIZE);
}

#[test]
fn runner_can_use_standard_lite_deck_v2_observation_profile() {
    let runner = sample_runner_with_observation_profile("standard_lite_deck_v2");

    assert_eq!(runner.observation_profile_id(), "standard_lite_deck_v2");
    assert_eq!(runner.observation_layout().tensor_size, 8850);
    assert_eq!(runner.get_action_mask().len(), ACTION_SPACE_SIZE);
    assert_eq!(runner.get_board_tensor(None).len(), 8850);
}

#[test]
fn runner_test_setter_accepts_observation_profile_aliases() {
    let mut runner = sample_runner();

    runner.set_observation_profile_for_test("v2_lite").unwrap();

    assert_eq!(runner.observation_profile_id(), "standard_lite_v2");
    assert_eq!(runner.get_board_tensor(None).len(), 8410);
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
    assert_eq!(
        runner.get_board_tensor(None).len(),
        runner.observation_layout().tensor_size
    );
    assert_eq!(
        runner.get_board_tensor(Some(0)).len(),
        runner.observation_layout().tensor_size
    );
    assert_eq!(
        runner.get_board_tensor(Some(1)).len(),
        runner.observation_layout().tensor_size
    );
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
fn declared_winner_exposes_terminal_outcome_reason() {
    let mut runner = sample_runner();

    runner.game.declare_winner(1);

    assert!(runner.is_game_over());
    assert_eq!(runner.winner_id(), 1);
    assert_eq!(
        runner.terminal_outcome_reason(),
        Some(TerminalOutcomeReason::EngineDeclared)
    );
}

#[test]
fn deckout_exposes_terminal_outcome_reason() {
    let mut runner = sample_runner();
    let first = runner.mulligan_current_player().expect("mulligan pending");
    runner.accept_mulligan(first, true).unwrap();
    let second = runner.mulligan_current_player().expect("second decision");
    runner.accept_mulligan(second, true).unwrap();

    runner.game.player_mut(0).deck.clear();
    runner.game.player_mut(1).deck.clear();
    runner.run_until_conclusion::<fn(&_, &[f32]) -> u16>(20, None);

    assert!(runner.is_game_over());
    assert_eq!(
        runner.terminal_outcome_reason(),
        Some(TerminalOutcomeReason::DeckOut)
    );
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

#[test]
fn headless_match_runs_with_embedded_dsl_omnimon_slice() {
    let db = dsl_slice_card_db();
    let mut runner = HeadlessRunner::new(
        dsl_slice_deck(),
        dsl_slice_deck(),
        &db,
        false,
        false,
        false,
        Some(123),
    )
    .unwrap();

    assert!(
        runner
            .game
            .effects_for_card("BT22-084", CardHandle(0))
            .is_some(),
        "Game::new should register embedded DSL effects"
    );

    while let Some(player) = runner.mulligan_current_player() {
        runner.accept_mulligan(player, true).unwrap();
    }

    let winner = runner.run_until_conclusion::<fn(&_, &[f32]) -> u16>(1000, None);
    assert!(runner.is_game_over());
    assert_ne!(winner, u8::MAX);
}
