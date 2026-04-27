use std::collections::HashMap;

use digimon_engine::action::explain::{explain_action, ActionKind, ActionZone};
use digimon_engine::action::space::{
    encode_attack, encode_digivolve, BREEDING_TARGET, EFFECTS_PER_PERMANENT,
    FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, HATCH, PASS, SECURITY_TARGET,
};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardKind, GamePhase};
use digimon_engine::game::Game;
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
    for _ in 0..5 {
        deck.push("BT1-001".to_string());
    }
    for _ in 0..25 {
        deck.push("BT1-010".to_string());
    }
    for _ in 0..25 {
        deck.push("BT1-025".to_string());
    }
    deck
}

fn playable_game() -> Game {
    let db = test_card_db();
    let decks = vec![test_deck(), test_deck()];
    let mut game = Game::new(&decks, &db, Rules::standard(), Some(42)).unwrap();
    game.start_game();
    while let Some(p) = game.mulligan_current_player() {
        game.accept_mulligan(p, true).unwrap();
    }
    game.enter_main_phase();
    game.set_memory(5);
    game
}

#[test]
fn explains_main_phase_play_from_hand_with_card_context() {
    let game = playable_game();
    let pid = game.turn_player();
    let hand_idx = game
        .player(pid)
        .hand
        .iter()
        .position(|c| c.card_kind(&game.card_data) == CardKind::Digimon)
        .unwrap();

    let explanation = explain_action(&game, pid, hand_idx as u16);

    assert_eq!(explanation.action_id, hand_idx as u16);
    assert_eq!(explanation.player_id, pid);
    assert_eq!(explanation.kind, ActionKind::Play);
    assert_eq!(explanation.source_zone, Some(ActionZone::Hand));
    assert_eq!(explanation.source_index, Some(hand_idx as u16));
    assert!(explanation.label.contains("Play"));
    assert!(explanation.card_id.is_some());
}

#[test]
fn explains_breeding_hatch_and_pass() {
    let db = test_card_db();
    let decks = vec![test_deck(), test_deck()];
    let mut game = Game::new(&decks, &db, Rules::standard(), Some(42)).unwrap();
    game.start_game();
    while let Some(p) = game.mulligan_current_player() {
        game.accept_mulligan(p, true).unwrap();
    }
    assert_eq!(game.current_phase, GamePhase::Breeding);
    let pid = game.turn_player();

    let hatch = explain_action(&game, pid, HATCH);
    assert_eq!(hatch.kind, ActionKind::Hatch);
    assert_eq!(hatch.label, "Hatch from egg deck");

    let pass = explain_action(&game, pid, PASS);
    assert_eq!(pass.kind, ActionKind::Pass);
    assert_eq!(pass.label, "Pass / decline");
}

#[test]
fn explains_attack_security_target() {
    let mut game = playable_game();
    let pid = game.turn_player();
    let hand_idx = game
        .player(pid)
        .hand
        .iter()
        .position(|c| c.card_kind(&game.card_data) == CardKind::Digimon)
        .unwrap();
    game.play_from_hand(pid, hand_idx).unwrap();

    let action = encode_attack(0, SECURITY_TARGET);
    let explanation = explain_action(&game, pid, action);

    assert_eq!(explanation.kind, ActionKind::Attack);
    assert_eq!(explanation.source_zone, Some(ActionZone::Battle));
    assert_eq!(explanation.source_index, Some(0));
    assert_eq!(explanation.target_zone, Some(ActionZone::Security));
    assert!(explanation.label.contains("attacks security"));
}

#[test]
fn explains_digivolve_onto_breeding() {
    let game = playable_game();
    let pid = game.turn_player();
    let action = encode_digivolve(0, 14);

    let explanation = explain_action(&game, pid, action);

    assert_eq!(explanation.kind, ActionKind::Digivolve);
    assert_eq!(explanation.source_zone, Some(ActionZone::Hand));
    assert_eq!(explanation.source_index, Some(0));
    assert_eq!(explanation.target_zone, Some(ActionZone::Breeding));
    assert_eq!(explanation.target_index, None);
}

#[test]
fn explains_breeding_field_effect_with_breeding_card_context() {
    let mut game = playable_game();
    let pid = game.turn_player();
    assert!(
        game.hatch(pid),
        "test setup should hatch an egg into breeding"
    );

    let action =
        FIELD_EFFECT_START + BREEDING_TARGET * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN;
    let explanation = explain_action(&game, pid, action);

    assert_eq!(explanation.kind, ActionKind::FieldEffect);
    assert_eq!(explanation.source_zone, Some(ActionZone::Breeding));
    assert_eq!(explanation.source_index, None);
    assert_ne!(explanation.source_index, Some(BREEDING_TARGET));
    assert_eq!(explanation.target_zone, None);
    assert_eq!(explanation.target_index, None);
    assert_eq!(explanation.card_id.as_deref(), Some("BT1-001"));
    assert_eq!(explanation.card_name.as_deref(), Some("Koromon"));
    assert!(explanation.label.contains("breeding area"));
}
