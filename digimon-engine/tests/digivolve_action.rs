use std::collections::HashMap;

use digimon_engine::action::space::{encode_digivolve, BREEDING_TARGET, DNA_DIGIVOLVE_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::enums::*;
use digimon_engine::game::Game;
use digimon_engine::permanent::Permanent;
use digimon_engine::rules::Rules;
use digimon_engine::selection::SelectionKind;

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
    for _ in 0..4 { deck.push("BT1-001".to_string()); }
    for _ in 0..6 { deck.push("BT1-010".to_string()); }
    for _ in 0..6 { deck.push("BT1-025".to_string()); }
    for _ in 0..4 { deck.push("BT1-085".to_string()); }
    for _ in 0..4 { deck.push("BT1-093".to_string()); }
    deck
}

/// Build a game with the turn player in Main phase, with a single
/// `base_card_id` permanent on the field at slot 0.
fn game_with_perm_on_field(base_card_id: &str) -> (Game, PlayerId) {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::standard();
    let mut game = Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap();
    game.start_game();
    game.enter_main_phase();

    let tp = game.turn_player();
    let data_idx = game.card_data.iter().position(|d| d.card_id == base_card_id).unwrap();
    let card_idx = game.next_card_index();
    let card = CardSource::new(data_idx, tp, card_idx);
    let turn = game.turn_count;
    game.player_mut(tp).battle_area.push(Permanent::new(card, turn));
    (game, tp)
}

fn push_to_hand(game: &mut Game, player_id: PlayerId, card_id: &str) -> usize {
    let data_idx = game.card_data.iter().position(|d| d.card_id == card_id).unwrap();
    let card_idx = game.next_card_index();
    let card = CardSource::new(data_idx, player_id, card_idx);
    game.player_mut(player_id).hand.push(card);
    game.player(player_id).hand.len() - 1
}

#[test]
fn decode_digivolve_basic_from_hand() {
    let (mut game, tp) = game_with_perm_on_field("BT1-010"); // Agumon Lv3
    let hand_idx = push_to_hand(&mut game, tp, "BT1-025");    // Greymon Lv4

    // Pre-set positive memory so paying cost (2) doesn't cross zero and
    // trigger end_turn (which would flip the seesaw sign).
    game.memory = 3;
    let mem_before = game.memory;
    let hand_before = game.player(tp).hand.len();

    let action = encode_digivolve(hand_idx as u16, 0);
    assert!(game.decode_action(action, tp), "decode_action should succeed");

    let perm = &game.player(tp).battle_area[0];
    assert_eq!(perm.card_sources.len(), 2, "stack grew by 1");
    assert_eq!(perm.level(&game.card_data), Some(4));
    assert_eq!(mem_before - game.memory, 2, "paid Greymon evo memory_cost=2");
    assert_eq!(perm.turn_digivolved, game.turn_count);
    // -1 for card consumed, +1 for draw after digivolve
    assert_eq!(game.player(tp).hand.len(), hand_before);
}

#[test]
fn decode_digivolve_rejects_out_of_range_hand() {
    let (mut game, tp) = game_with_perm_on_field("BT1-010");

    let action = encode_digivolve(99, 0);
    assert!(!game.decode_action(action, tp));
}

#[test]
fn decode_digivolve_onto_breeding() {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::standard();
    let mut game = Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap();
    game.start_game();
    let tp = game.turn_player();

    // Hatch a Koromon (Lv2) into breeding.
    assert!(game.hatch(tp));
    assert!(game.player(tp).breeding_area.is_some());

    game.enter_main_phase();

    // Agumon has evo_cost {level=2, card_color=0=red, memory_cost=1} per
    // our local test_card_db.
    let hand_idx = push_to_hand(&mut game, tp, "BT1-010");

    // Pre-set positive memory so cost (1) doesn't cross zero.
    game.memory = 3;
    let mem_before = game.memory;
    let hand_before = game.player(tp).hand.len();

    let action = encode_digivolve(hand_idx as u16, BREEDING_TARGET);
    assert!(game.decode_action(action, tp), "decode_action should succeed");

    let breeding = game.player(tp).breeding_area.as_ref().unwrap();
    assert_eq!(breeding.card_sources.len(), 2);
    assert_eq!(breeding.level(&game.card_data), Some(3));
    assert_eq!(mem_before - game.memory, 1);
    assert_eq!(game.player(tp).hand.len(), hand_before);
}

#[test]
fn decode_dna_digivolve_installs_material_selection() {
    // BT1-010 Agumon has no dna_costs in the JSON. Mutate the card_data
    // for this test to add dna_costs so Agumon can "DNA-digivolve" from
    // two Lv2 Koromons. This exercises only the initiation path; the
    // execution callback is a TODO.
    let (mut game, tp) = {
        let db = test_card_db();
        let deck = test_deck();
        let rules = Rules::standard();
        let mut game = Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap();
        game.start_game();
        game.enter_main_phase();
        let tp = game.turn_player();
        (game, tp)
    };

    // Put two Koromons on the field.
    let koromon_idx = game.card_data.iter().position(|d| d.card_id == "BT1-001").unwrap();
    for _ in 0..2 {
        let card_idx = game.next_card_index();
        let card = CardSource::new(koromon_idx, tp, card_idx);
        let turn = game.turn_count;
        game.player_mut(tp).battle_area.push(Permanent::new(card, turn));
    }

    // Give Agumon a DNA cost of two Lv2 Red materials.
    let agumon_idx = game.card_data.iter().position(|d| d.card_id == "BT1-010").unwrap();
    {
        use digimon_engine::card_data::{DnaCost, DnaRequirement};
        let meta = &mut game.card_data[agumon_idx];
        meta.dna_costs = vec![DnaCost {
            memory_cost: 0,
            requirement1: DnaRequirement {
                level: 2,
                card_color: Some(CardColor::Red),
                name_contains: String::new(),
                text_contains: String::new(),
            },
            requirement2: DnaRequirement {
                level: 2,
                card_color: Some(CardColor::Red),
                name_contains: String::new(),
                text_contains: String::new(),
            },
        }];
    }

    let hand_idx = push_to_hand(&mut game, tp, "BT1-010");

    let action = DNA_DIGIVOLVE_START + hand_idx as u16;
    assert!(game.decode_action(action, tp), "DNA initiation should succeed");

    assert_eq!(game.current_phase, GamePhase::SelectMaterial);
    let sel = game
        .pending_selection
        .as_ref()
        .expect("pending selection installed");
    assert_eq!(sel.kind, SelectionKind::Material);
    assert_eq!(sel.selecting_player, tp);
    assert!(
        !sel.valid_action_ids.is_empty(),
        "at least one first-material target"
    );
}
