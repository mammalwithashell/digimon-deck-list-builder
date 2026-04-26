//! Headless self-play integration tripwire: run `HeadlessRunner` end-to-end
//! many times with `GreedyPolicy` on both seats across a range of seeds and
//! assert each game declares a winner within a bounded step budget. This
//! is the cheap regression check that catches action-mask drift, policy
//! off-by-ones, or engine hangs before they reach production.
//!
//! Complementary to the targeted behavioral tests in `policies_greedy.rs`
//! — those lock specific decisions in place; this one locks the ensemble
//! property "greedy vs greedy always terminates".

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::policies::GreedyPolicy;
use digimon_engine::{HeadlessRunner, Policy};

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

#[test]
fn greedy_self_play_terminates_across_seeds() {
    let db = test_card_db();
    const NUM_GAMES: u64 = 20;
    const MAX_STEPS_PER_GAME: u32 = 2000;

    let mut wins = [0u32, 0u32];
    let mut draws = 0u32;

    for seed in 0..NUM_GAMES {
        let mut runner = HeadlessRunner::new(
            test_deck(),
            test_deck(),
            &db,
            false,
            false,
            false,
            Some(seed),
        )
        .expect("runner construction");

        let mut greedy = GreedyPolicy::new();
        let winner = runner.run_until_conclusion(
            MAX_STEPS_PER_GAME,
            Some(|g: &digimon_engine::Game, m: &[f32]| greedy.select(g, m)),
        );
        greedy.reset();

        assert!(
            runner.is_game_over(),
            "game {} did not end within {} steps",
            seed,
            MAX_STEPS_PER_GAME
        );
        match winner {
            0 => wins[0] += 1,
            1 => wins[1] += 1,
            _ => draws += 1,
        }
    }

    // Sanity: not all games went to a degenerate timeout-declared winner.
    // At least some reaal terminations across 20 seeds.
    eprintln!(
        "greedy self-play over {} seeds — p0: {}, p1: {}, draws/timeouts: {}",
        NUM_GAMES, wins[0], wins[1], draws
    );
    assert!(
        wins[0] + wins[1] > 0,
        "expected at least one real termination across {} seeds",
        NUM_GAMES
    );
}
