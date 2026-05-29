//! Tests for §3.3 (face-down security visibility) and §3.4 (revealed_cards
//! slot) in `docs/RUST_PYTHON_PARITY.md`.
//!
//! §3.3 — my-security must be 0.0 for face-down cards. Writing card IDs
//! for face-down security leaks hidden information to the RL agent.
//! §3.4 — `game.revealed_cards` feeds the 10-slot revealed-cards section
//! at `OFF_REVEALED` and is cleared on turn rotation.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::card_registry::CardRegistry;
use digimon_engine::enums::GamePhase;
use digimon_engine::game::Game;
use digimon_engine::rules::Rules;
// v1-pinned test: hidden-info checks (face-down security, revealed-cards
// slot) are v1-layout-specific. Use the relocated v1 builder + v1 constants
// directly so this file is robust to the engine default flipping.
use digimon_engine::tensor_profiles::standard::v1::{
    MAX_REVEALED, MAX_SECURITY, OFF_MY_SECURITY, OFF_OPP_SECURITY, OFF_REVEALED,
};
use digimon_engine::tensor_v1::build_tensor_standard_compact_v1 as build_tensor;

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
    for _ in 0..6 {
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

fn fresh_game() -> (Game, CardRegistry) {
    let db = test_card_db();
    let registry = CardRegistry::from_cards(&db);
    let deck = test_deck();
    let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(42)).unwrap();
    game.start_game(); // triggers mulligan finalize -> setup_security
    (game, registry)
}

#[test]
fn my_security_is_zero_by_default() {
    let (game, registry) = fresh_game();
    let me = game.player(0);
    assert!(!me.security.is_empty(), "precondition: security was laid");
    assert!(
        me.face_up_security.is_empty(),
        "precondition: security starts face-down"
    );

    let t = build_tensor(&game, 0, &registry);
    for i in 0..MAX_SECURITY {
        assert_eq!(
            t[OFF_MY_SECURITY + i],
            0.0,
            "face-down security slot {} must be 0.0 (no hidden info leak)",
            i
        );
    }
}

#[test]
fn opp_security_is_zero_by_default() {
    let (game, registry) = fresh_game();
    let t = build_tensor(&game, 0, &registry);
    for i in 0..MAX_SECURITY {
        assert_eq!(t[OFF_OPP_SECURITY + i], 0.0);
    }
}

#[test]
fn my_security_visible_when_face_up() {
    let (mut game, registry) = fresh_game();

    // Flip the 0th security card face-up.
    let (sec_index, expected_id) = {
        let me = game.player(0);
        let card = &me.security[0];
        (
            card.card_index,
            registry.get_index(&card.card_id(&game.card_data)) as f32,
        )
    };
    game.players[0].face_up_security.insert(sec_index);

    let t = build_tensor(&game, 0, &registry);

    // Slot 0 shows the card's registry index.
    assert_eq!(t[OFF_MY_SECURITY], expected_id);
    // All other slots remain face-down.
    for i in 1..MAX_SECURITY {
        assert_eq!(
            t[OFF_MY_SECURITY + i],
            0.0,
            "non-flipped security slot {} must stay 0.0",
            i
        );
    }
}

#[test]
fn revealed_cards_populates_offset() {
    let (mut game, registry) = fresh_game();

    // Synthesize a revealed card by cloning the top of P0's deck.
    let revealed = game.players[0]
        .deck
        .last()
        .cloned()
        .expect("deck non-empty");
    let expected_id = registry.get_index(&revealed.card_id(&game.card_data)) as f32;
    game.revealed_cards.push(revealed);

    let t = build_tensor(&game, 0, &registry);

    assert_eq!(t[OFF_REVEALED], expected_id);
    for i in 1..MAX_REVEALED {
        assert_eq!(
            t[OFF_REVEALED + i],
            0.0,
            "unfilled reveal slot {} must be 0.0",
            i
        );
    }
}

#[test]
fn revealed_cards_cleared_on_turn_rotation() {
    let (mut game, registry) = fresh_game();

    let revealed = game.players[0]
        .deck
        .last()
        .cloned()
        .expect("deck non-empty");
    game.revealed_cards.push(revealed);

    // Sanity: reveal populated before rotation.
    let t_before = build_tensor(&game, 0, &registry);
    assert!(t_before[OFF_REVEALED] > 0.0);

    // Drive through Main → EndTurn → next player's Breeding. No EOT-keyword
    // permanents exist, so `end_turn` falls straight through to
    // `rotate_turn_player` where reveal clearing happens.
    game.enter_main_phase();
    assert_eq!(game.current_phase, GamePhase::Main);
    game.end_turn();

    assert!(
        game.revealed_cards.is_empty(),
        "revealed_cards must be cleared on turn rotation"
    );

    let t_after = build_tensor(&game, 0, &registry);
    for i in 0..MAX_REVEALED {
        assert_eq!(
            t_after[OFF_REVEALED + i],
            0.0,
            "reveal slot {} must be zero after rotation",
            i
        );
    }
}
