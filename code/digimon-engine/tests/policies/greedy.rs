//! Behavioral tests for the heuristic greedy opponent. Locks specific
//! decisions in place so future edits to the heuristic can be caught by
//! deterministic, small-scenario assertions.

use std::collections::HashMap;

use digimon_engine::action::space::{
    encode_attack, encode_digivolve, ACTION_SPACE_SIZE, MOVE_FROM_BREEDING, PASS, PLAY_HAND_START,
    SECURITY_TARGET,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::GamePhase;
use digimon_engine::permanent::Permanent;
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
        },
        "BT1-050": {
            "card_id": "BT1-050", "card_name_eng": "MetalGreymon",
            "card_effect_class_name": "BT1_050", "play_cost": 7, "dp": 7000,
            "level": 5, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Cyborg"], "form_eng": ["Ultimate"], "attribute_eng": ["Vaccine"],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "",
            "evo_costs": [{"card_color": 0, "level": 4, "memory_cost": 3}]
        },
        "BT1-080": {
            "card_id": "BT1-080", "card_name_eng": "Setup Tamer",
            "card_effect_class_name": "BT1_080", "play_cost": 3, "dp": -1,
            "level": -1, "card_kind": 1, "rarity": 0, "card_colors": [0],
            "type_eng": ["Tamer"], "form_eng": [], "attribute_eng": [],
            "effect_description_eng": "[Start of Your Turn] Gain 1 memory.",
            "inherited_effect_description_eng": "",
            "security_effect_description_eng": "",
            "evo_costs": []
        },
        "BT1-081": {
            "card_id": "BT1-081", "card_name_eng": "Search Agumon",
            "card_effect_class_name": "BT1_081", "play_cost": 3, "dp": 1000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Reptile"], "form_eng": ["Rookie"], "attribute_eng": ["Vaccine"],
            "effect_description_eng": "[On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card among them to your hand.",
            "inherited_effect_description_eng": "",
            "security_effect_description_eng": "",
            "evo_costs": []
        },
        "BT1-082": {
            "card_id": "BT1-082", "card_name_eng": "Vanilla Rookie",
            "card_effect_class_name": "BT1_082", "play_cost": 3, "dp": 2000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Reptile"], "form_eng": ["Rookie"], "attribute_eng": ["Vaccine"],
            "effect_description_eng": "",
            "inherited_effect_description_eng": "",
            "security_effect_description_eng": "",
            "evo_costs": []
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

fn move_hand_card_to_breeding(runner: &mut DebugRunner, hand_idx: usize) {
    let rookie = runner.game.players[0].hand.remove(hand_idx);
    runner.game.players[0].breeding_area = Some(Permanent::new(rookie, runner.game.turn_count));
    runner.game.current_phase = GamePhase::Breeding;
}

fn breeding_move_or_pass_mask() -> Vec<f32> {
    let mut mask = vec![0.0f32; ACTION_SPACE_SIZE];
    mask[MOVE_FROM_BREEDING as usize] = 1.0;
    mask[PASS as usize] = 1.0;
    mask
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

#[test]
fn main_phase_play_prefers_curve_starter_over_expensive_high_level() {
    let db = test_card_db();
    let mut runner = DebugRunner::builder()
        .with_card_data(db)
        .hand(0, &["BT1-010", "BT1-050"])
        .memory(10)
        .start();
    runner.game.current_phase = GamePhase::Main;

    let mut mask = vec![0.0f32; ACTION_SPACE_SIZE];
    mask[PLAY_HAND_START as usize] = 1.0;
    mask[(PLAY_HAND_START + 1) as usize] = 1.0;
    mask[PASS as usize] = 1.0;

    assert_eq!(
        greedy_action(&runner.game, &mask),
        PLAY_HAND_START,
        "greedy should establish a level-3 starter before hard-playing a higher-level Digimon"
    );
}

#[test]
fn main_phase_plays_searcher_before_keep_turn_digivolve() {
    let db = test_card_db();
    let mut runner = DebugRunner::builder()
        .with_card_data(db)
        .hand(0, &["BT1-081", "BT1-025"])
        .memory(10)
        .start();
    let base = runner.place_on_field(0, "BT1-010", Some(0));
    runner.game.current_phase = GamePhase::Main;

    let mut mask = vec![0.0f32; ACTION_SPACE_SIZE];
    mask[PLAY_HAND_START as usize] = 1.0;
    mask[encode_digivolve(1, base.index as u16) as usize] = 1.0;
    mask[PASS as usize] = 1.0;

    assert_eq!(
        greedy_action(&runner.game, &mask),
        PLAY_HAND_START,
        "greedy should play a searcher before spending the turn on a keep-turn digivolve"
    );
}

#[test]
fn main_phase_plays_tamer_before_keep_turn_digivolve() {
    let db = test_card_db();
    let mut runner = DebugRunner::builder()
        .with_card_data(db)
        .hand(0, &["BT1-080", "BT1-025"])
        .memory(10)
        .start();
    let base = runner.place_on_field(0, "BT1-010", Some(0));
    runner.game.current_phase = GamePhase::Main;

    let mut mask = vec![0.0f32; ACTION_SPACE_SIZE];
    mask[PLAY_HAND_START as usize] = 1.0;
    mask[encode_digivolve(1, base.index as u16) as usize] = 1.0;
    mask[PASS as usize] = 1.0;

    assert_eq!(
        greedy_action(&runner.game, &mask),
        PLAY_HAND_START,
        "greedy should establish a Tamer before spending the turn on a keep-turn digivolve"
    );
}

#[test]
fn main_phase_attacks_security_before_favorable_board_trade() {
    let db = test_card_db();
    let mut runner = DebugRunner::builder()
        .with_card_data(db)
        .security(1, &["BT1-010"])
        .start();
    let attacker = runner.place_on_field(0, "BT1-025", Some(0));
    let target = runner.place_on_field(1, "BT1-010", Some(0));
    runner.game.current_phase = GamePhase::Main;

    let security_attack = encode_attack(attacker.index as u16, SECURITY_TARGET);
    let board_attack = encode_attack(attacker.index as u16, target.index as u16);
    let mut mask = vec![0.0f32; ACTION_SPACE_SIZE];
    mask[security_attack as usize] = 1.0;
    mask[board_attack as usize] = 1.0;
    mask[PASS as usize] = 1.0;

    assert_eq!(
        greedy_action(&runner.game, &mask),
        security_attack,
        "greedy should prefer applying security pressure over a favorable board trade"
    );
}

#[test]
fn breeding_holds_vanilla_level3_without_evolution_in_hand() {
    let db = test_card_db();
    let mut runner = DebugRunner::builder()
        .with_card_data(db)
        .hand(0, &["BT1-082"])
        .start();
    move_hand_card_to_breeding(&mut runner, 0);

    assert_eq!(
        greedy_action(&runner.game, &breeding_move_or_pass_mask()),
        PASS,
        "greedy should not move a vanilla level-3 out of breeding when no matching evolution is in hand"
    );
}

#[test]
fn breeding_moves_resource_flow_level3_without_evolution_in_hand() {
    let db = test_card_db();
    let mut runner = DebugRunner::builder()
        .with_card_data(db)
        .hand(0, &["BT1-081"])
        .start();
    move_hand_card_to_breeding(&mut runner, 0);

    assert_eq!(
        greedy_action(&runner.game, &breeding_move_or_pass_mask()),
        MOVE_FROM_BREEDING,
        "greedy should move a level-3 with resource-flow text even without a matching evolution in hand"
    );
}

#[test]
fn breeding_moves_vanilla_level3_with_valid_evolution_in_hand() {
    let db = test_card_db();
    let mut runner = DebugRunner::builder()
        .with_card_data(db)
        .hand(0, &["BT1-082", "BT1-025"])
        .start();
    move_hand_card_to_breeding(&mut runner, 0);

    assert_eq!(
        greedy_action(&runner.game, &breeding_move_or_pass_mask()),
        MOVE_FROM_BREEDING,
        "greedy should move a vanilla level-3 when a matching evolution is available in hand"
    );
}
