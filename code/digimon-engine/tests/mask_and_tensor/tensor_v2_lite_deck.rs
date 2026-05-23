use std::collections::HashMap;

use digimon_engine::action::{build_action_mask, ACTION_SPACE_SIZE};
use digimon_engine::card_data::CardData;
use digimon_engine::card_registry::CardRegistry;
use digimon_engine::game::Game;
use digimon_engine::observation::{build_observation_tensor, parse_observation_profile};
use digimon_engine::rules::Rules;
use digimon_engine::tensor_profiles::profile_by_id;

fn card_db() -> HashMap<String, CardData> {
    let json = r#"{
        "EGG-A": {
            "card_id": "EGG-A", "card_name_eng": "Egg A",
            "card_effect_class_name": "EGG_A", "play_cost": 0, "dp": -1,
            "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
            "type_eng": [], "form_eng": [], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": [],
            "index": 40
        },
        "EGG-B": {
            "card_id": "EGG-B", "card_name_eng": "Egg B",
            "card_effect_class_name": "EGG_B", "play_cost": 0, "dp": -1,
            "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [1],
            "type_eng": [], "form_eng": [], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": [],
            "index": 50
        },
        "CARD-A": {
            "card_id": "CARD-A", "card_name_eng": "Card A",
            "card_effect_class_name": "CARD_A", "play_cost": 3, "dp": 2000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": [], "form_eng": [], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": [],
            "index": 20
        },
        "CARD-B": {
            "card_id": "CARD-B", "card_name_eng": "Card B",
            "card_effect_class_name": "CARD_B", "play_cost": 4, "dp": 3000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [1],
            "type_eng": [], "form_eng": [], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": [],
            "index": 10
        },
        "CARD-C": {
            "card_id": "CARD-C", "card_name_eng": "Card C",
            "card_effect_class_name": "CARD_C", "play_cost": 5, "dp": 4000,
            "level": 4, "card_kind": 0, "rarity": 0, "card_colors": [2],
            "type_eng": [], "form_eng": [], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": [],
            "index": 30
        },
        "OPP-A": {
            "card_id": "OPP-A", "card_name_eng": "Opponent A",
            "card_effect_class_name": "OPP_A", "play_cost": 3, "dp": 1000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [3],
            "type_eng": [], "form_eng": [], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": [],
            "index": 60
        },
        "OPP-B": {
            "card_id": "OPP-B", "card_name_eng": "Opponent B",
            "card_effect_class_name": "OPP_B", "play_cost": 3, "dp": 1000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [4],
            "type_eng": [], "form_eng": [], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": [],
            "index": 70
        }
    }"#;
    CardData::load_from_str(json).unwrap()
}

fn deck_one() -> Vec<String> {
    let mut deck = Vec::new();
    deck.extend(std::iter::repeat("CARD-A".to_string()).take(4));
    deck.extend(std::iter::repeat("CARD-B".to_string()).take(2));
    deck.extend(std::iter::repeat("CARD-C".to_string()).take(44));
    deck.extend(std::iter::repeat("EGG-A".to_string()).take(4));
    deck.push("EGG-B".to_string());
    deck
}

fn deck_two() -> Vec<String> {
    let mut deck = Vec::new();
    deck.extend(std::iter::repeat("OPP-A".to_string()).take(25));
    deck.extend(std::iter::repeat("OPP-B".to_string()).take(25));
    deck.extend(std::iter::repeat("EGG-B".to_string()).take(5));
    deck
}

fn tensor_for(
    deck1: Vec<String>,
    deck2: Vec<String>,
    seed: u64,
    observer: u8,
) -> (Vec<f32>, CardRegistry) {
    let cards = card_db();
    let registry = CardRegistry::from_cards(&cards);
    let game = Game::new(&[deck1, deck2], &cards, Rules::standard(), Some(seed)).unwrap();
    let profile = parse_observation_profile("standard_lite_deck_v2").unwrap();
    let tensor = build_observation_tensor(&game, observer, &registry, profile);
    (tensor, registry)
}

fn decklist_rows(tensor: &[f32]) -> Vec<[f32; 8]> {
    let profile = profile_by_id("standard_lite_deck_v2").unwrap();
    let section = profile.section("own_original_decklist").unwrap();
    (0..55)
        .map(|row| {
            let base = section.start + row * 8;
            [
                tensor[base],
                tensor[base + 1],
                tensor[base + 2],
                tensor[base + 3],
                tensor[base + 4],
                tensor[base + 5],
                tensor[base + 6],
                tensor[base + 7],
            ]
        })
        .collect()
}

#[test]
fn lite_deck_v2_collapses_original_deck_copies_into_sorted_rows() {
    let (tensor, registry) = tensor_for(deck_one(), deck_two(), 1, 0);
    let rows = decklist_rows(&tensor);

    assert_eq!(tensor.len(), 8850);
    assert_eq!(rows[0][0], 1.0);
    assert_eq!(rows[0][1], registry.get_index("CARD-B") as f32);
    assert_eq!(rows[0][2], 2.0 / 4.0);
    assert_eq!(rows[0][3], 1.0);
    assert_eq!(rows[0][4], 0.0);

    assert_eq!(rows[1][0], 1.0);
    assert_eq!(rows[1][1], registry.get_index("CARD-A") as f32);
    assert_eq!(rows[1][2], 4.0 / 4.0);

    assert_eq!(rows[2][0], 1.0);
    assert_eq!(rows[2][1], registry.get_index("CARD-C") as f32);
    assert_eq!(rows[2][2], 44.0 / 4.0);

    assert_eq!(rows[3][0], 1.0);
    assert_eq!(rows[3][1], registry.get_index("EGG-A") as f32);
    assert_eq!(rows[3][2], 4.0 / 4.0);
    assert_eq!(rows[3][3], 0.0);
    assert_eq!(rows[3][4], 1.0);

    assert_eq!(rows[4][0], 1.0);
    assert_eq!(rows[4][1], registry.get_index("EGG-B") as f32);
    assert_eq!(rows[4][2], 1.0 / 4.0);
    assert_eq!(rows[4][3], 0.0);
    assert_eq!(rows[4][4], 1.0);

    assert_eq!(rows[5], [0.0; 8]);
}

#[test]
fn lite_deck_v2_decklist_section_is_stable_across_shuffle_and_mulligan() {
    let cards = card_db();
    let registry = CardRegistry::from_cards(&cards);
    let profile = parse_observation_profile("standard_lite_deck_v2").unwrap();
    let game_a = Game::new(
        &[deck_one(), deck_two()],
        &cards,
        Rules::standard(),
        Some(1),
    )
    .unwrap();
    let mut game_b = Game::new(
        &[deck_one(), deck_two()],
        &cards,
        Rules::standard(),
        Some(2),
    )
    .unwrap();

    while let Some(player) = game_b.mulligan_current_player() {
        game_b.accept_mulligan(player, false).unwrap();
    }

    let tensor_a = build_observation_tensor(&game_a, 0, &registry, profile);
    let tensor_b = build_observation_tensor(&game_b, 0, &registry, profile);
    let section = profile_by_id("standard_lite_deck_v2")
        .unwrap()
        .section("own_original_decklist")
        .unwrap();

    assert_eq!(
        &tensor_a[section.start..section.start + section.len],
        &tensor_b[section.start..section.start + section.len]
    );

    assert_eq!(build_action_mask(&game_a, 0).len(), ACTION_SPACE_SIZE);
}

#[test]
fn lite_deck_v2_does_not_encode_opponent_original_decklist() {
    let (tensor, registry) = tensor_for(deck_one(), deck_two(), 1, 0);
    let opponent_index = registry.get_index("OPP-A") as f32;

    for row in decklist_rows(&tensor) {
        assert_ne!(row[1], opponent_index);
    }
}
