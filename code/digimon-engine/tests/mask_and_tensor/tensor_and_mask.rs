use std::collections::HashMap;

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::*;
use digimon_engine::card_data::CardData;
use digimon_engine::card_registry::CardRegistry;
use digimon_engine::enums::*;
use digimon_engine::game::Game;
use digimon_engine::rules::Rules;
use digimon_engine::tensor::{
    build_tensor, compute_positions, FIELD_SLOTS, MAX_HAND, SLOT_SIZE, TENSOR_SIZE,
};

/// Same minimal card db as engine_core tests.
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
            "effect_description_eng": "Delete 1 of your opponent's Digimon.",
            "inherited_effect_description_eng": "",
            "security_effect_description_eng": "Delete 1 of your opponent's Digimon.",
            "evo_costs": []
        }
    }"#;
    CardData::load_from_str(json).unwrap()
}

fn test_deck() -> Vec<String> {
    let mut deck = Vec::new();
    for _ in 0..4 {
        deck.push("BT1-001".to_string());
    }
    for _ in 0..6 {
        deck.push("BT1-010".to_string());
    }
    for _ in 0..6 {
        deck.push("BT1-025".to_string());
    }
    for _ in 0..4 {
        deck.push("BT1-085".to_string());
    }
    for _ in 0..4 {
        deck.push("BT1-093".to_string());
    }
    deck
}

#[test]
fn tensor_size_matches_constant() {
    let db = test_card_db();
    let registry = CardRegistry::from_cards(&db);
    let deck = test_deck();
    let game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(42)).unwrap();

    let t = build_tensor(&game, 0, &registry);
    assert_eq!(t.len(), TENSOR_SIZE);
    assert_eq!(t.len(), 1375);
}

#[test]
fn tensor_global_section() {
    let db = test_card_db();
    let registry = CardRegistry::from_cards(&db);
    let deck = test_deck();
    let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(42)).unwrap();
    game.start_game();

    let t = build_tensor(&game, 0, &registry);

    // [0] turn count / 30, clamped
    assert!((t[0] - 1.0 / 30.0).abs() < 1e-6);
    // [1] phase = Breeding (3.0)
    assert_eq!(t[1], GamePhase::Breeding.tensor_value());
    // [2] memory normalized — game starts at 0
    assert_eq!(t[2], 0.0);
    // [3-9] reserved
    for i in 3..10 {
        assert_eq!(t[i], 0.0);
    }
}

#[test]
fn tensor_hand_card_ids() {
    let db = test_card_db();
    let registry = CardRegistry::from_cards(&db);
    let deck = test_deck();
    let game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(42)).unwrap();

    let t = build_tensor(&game, 0, &registry);

    // My hand starts at OFF_MY_HAND = 1130. After Game::new (deals 5 cards),
    // hand has 5 cards. Each slot should have a non-zero card ID.
    let hand_start = 1130;
    let me = game.player(0);
    for i in 0..me.hand.len() {
        let id = registry.get_index(&me.hand[i].card_id(&game.card_data));
        assert!(
            id > 0,
            "Hand card {} should have non-zero registry index",
            i
        );
        assert_eq!(t[hand_start + i], id as f32);
    }
    // Slots beyond hand size should be 0
    for i in me.hand.len()..MAX_HAND {
        assert_eq!(t[hand_start + i], 0.0);
    }
}

#[test]
fn tensor_battle_area_after_play() {
    let db = test_card_db();
    let registry = CardRegistry::from_cards(&db);
    let deck = test_deck();
    let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(42)).unwrap();
    game.start_game();
    game.enter_main_phase();

    let tp = game.turn_player();
    // Find an Agumon (Lv3) in hand and play it
    let agumon_idx = game
        .player(tp)
        .hand
        .iter()
        .position(|c| c.card_id(&game.card_data) == "BT1-010");

    if let Some(idx) = agumon_idx {
        game.play_from_hand(tp, idx);

        let t = build_tensor(&game, tp, &registry);
        // My battle area starts at 10. First slot (+0) = top card ID
        let agumon_id = registry.get_index("BT1-010") as f32;
        assert_eq!(t[10], agumon_id);
        // +1 = DP normalized = 2000 / 30000
        assert!((t[11] - 2000.0 / 30000.0).abs() < 1e-6);
        // +2 = suspended = 0
        assert_eq!(t[12], 0.0);
        // +6 = source count = 1
        assert_eq!(t[16], 1.0);
        // First source card_id at +7
        assert_eq!(t[17], agumon_id);
    }
}

#[test]
fn tensor_perspective_mirroring() {
    let db = test_card_db();
    let registry = CardRegistry::from_cards(&db);
    let deck = test_deck();
    let game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(42)).unwrap();

    let t_p0 = build_tensor(&game, 0, &registry);
    let t_p1 = build_tensor(&game, 1, &registry);

    // P0's hand starts at 1130, P1's hand also starts at 1130 from P1's view.
    // Both hands have 5 cards drawn from same shuffled deck (same seed),
    // so contents may differ if shuffle is per-player; but the structure should be valid.
    // Check both tensors are full-sized.
    assert_eq!(t_p0.len(), TENSOR_SIZE);
    assert_eq!(t_p1.len(), TENSOR_SIZE);
}

#[test]
fn position_split_covers_tensor() {
    let (cards, scalars) = compute_positions();
    assert_eq!(cards.len() + scalars.len(), TENSOR_SIZE);

    // No duplicates within or across
    let mut combined: Vec<usize> = cards.iter().chain(scalars.iter()).copied().collect();
    combined.sort();
    combined.dedup();
    assert_eq!(combined.len(), TENSOR_SIZE);

    // Card positions count: per slot has 1 (top) + 11 sources = 12 card slots.
    // 14 my-battle slots + 14 opp-battle slots + 1 my-breeding + 1 opp-breeding = 30 slots × 12 = 360
    // + 20 my-hand + 20 opp-hand = 40
    // + 45 my-trash + 45 opp-trash = 90
    // + 10 my-security + 10 opp-security = 20
    // + 10 revealed = 10
    // Total: 360 + 40 + 90 + 20 + 10 = 520
    assert_eq!(cards.len(), 520);
    assert_eq!(scalars.len(), TENSOR_SIZE - 520);
}

// ─── Action mask tests ────────────────────────────────────────────────

#[test]
fn mask_size_correct() {
    let db = test_card_db();
    let deck = test_deck();
    let game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(42)).unwrap();

    let mask = build_action_mask(&game, 0);
    assert_eq!(mask.len(), ACTION_SPACE_SIZE);
    assert_eq!(mask.len(), 2168);
}

#[test]
fn mask_breeding_phase() {
    let db = test_card_db();
    let deck = test_deck();
    let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(42)).unwrap();
    game.start_game();

    assert_eq!(game.current_phase, GamePhase::Breeding);

    let tp = game.turn_player();
    let mask = build_action_mask(&game, tp);

    // Hatch should be allowed (breeding empty, eggs in deck)
    assert_eq!(mask[HATCH as usize], 1.0);
    // Move from breeding NOT allowed (breeding is empty)
    assert_eq!(mask[MOVE_FROM_BREEDING as usize], 0.0);
    // Pass allowed
    assert_eq!(mask[PASS as usize], 1.0);
}

#[test]
fn mask_breeding_after_hatch() {
    let db = test_card_db();
    let deck = test_deck();
    let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(42)).unwrap();
    game.start_game();

    let tp = game.turn_player();
    game.hatch(tp);

    let mask = build_action_mask(&game, tp);

    // Hatch no longer allowed (breeding occupied)
    assert_eq!(mask[HATCH as usize], 0.0);
    // Move from breeding NOT allowed yet (Lv2 egg < Lv3)
    assert_eq!(mask[MOVE_FROM_BREEDING as usize], 0.0);
}

#[test]
fn mask_main_phase_play_cards() {
    let db = test_card_db();
    let deck = test_deck();
    let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(42)).unwrap();
    game.start_game();
    game.enter_main_phase();
    game.set_memory(5);

    let tp = game.turn_player();
    let mask = build_action_mask(&game, tp);

    // Pass always available in main
    assert_eq!(mask[PASS as usize], 1.0);

    // At least one play action should be legal (we have memory 5 and cards costing 0-8)
    let has_legal_play = (0..PLAY_HAND_END as usize).any(|i| mask[i] > 0.0);
    assert!(
        has_legal_play,
        "Should have at least one playable card with memory 5"
    );

    // Specifically: an Agumon (cost 3) at memory 5 → 5 - 3 = 2 ≥ -10, legal
    let me = game.player(tp);
    for (i, card) in me.hand.iter().enumerate() {
        if card.card_id(&game.card_data) == "BT1-010" {
            assert_eq!(mask[i], 1.0, "Agumon at index {} should be playable", i);
        }
    }
}

#[test]
fn mask_main_no_attack_with_summoning_sickness() {
    let db = test_card_db();
    let deck = test_deck();
    let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(42)).unwrap();
    game.start_game();
    game.enter_main_phase();

    let tp = game.turn_player();

    // Play Agumon
    let agumon_idx = game
        .player(tp)
        .hand
        .iter()
        .position(|c| c.card_id(&game.card_data) == "BT1-010");
    if let Some(idx) = agumon_idx {
        game.play_from_hand(tp, idx);

        let mask = build_action_mask(&game, tp);
        // Just-played Agumon (slot 0) should NOT be able to attack security
        let sec_atk = encode_attack(0, SECURITY_TARGET) as usize;
        assert_eq!(
            mask[sec_atk], 0.0,
            "Just-played Agumon should have summoning sickness"
        );
    }
}

#[test]
fn mask_action_space_size_constant() {
    assert_eq!(ACTION_SPACE_SIZE, 2168);
    // Validate that no encode function exceeds the action space
    assert!(encode_attack(19, 14) < ACTION_SPACE_SIZE as u16);
    assert!(encode_digivolve(29, 14) < ACTION_SPACE_SIZE as u16);
}

// Sanity: SLOT_SIZE and FIELD_SLOTS make sense
#[test]
fn tensor_layout_constants() {
    assert_eq!(SLOT_SIZE, 40);
    assert_eq!(FIELD_SLOTS, 14);
    assert_eq!(MAX_HAND, 20);
}
