//! Independent batch-1 review: production registration and real-card composition.
//!
//! By default this uses the portable exact-row fixture. Set
//! FOCUSED_FIVE_VANILLA_R4_FULL to the original 4,483-row R4 cards.json to
//! repeat the full-database review; its digest and all five rows are verified.
//! No DSL fixture compilation or manual effect-registry insertion is used.

use std::collections::HashMap;

use digimon_engine::action::{build_action_mask, encode_digivolve};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::GamePhase;
use sha2::{Digest, Sha256};

const CARDS: [&str; 4] = ["BT1-009", "BT1-014", "BT3-007", "BT4-010"];
const ROOKIES: [&str; 2] = ["BT1-009", "BT3-007"];
const CHAMPIONS: [&str; 2] = ["BT1-014", "BT4-010"];

fn original_data() -> HashMap<String, CardData> {
    let fixture = include_str!("fixtures/focused_five_vanilla_r4.json");
    let provenance: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/focused_five_vanilla_r4.provenance.json"
    ))
    .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(fixture.as_bytes())),
        provenance["fixture_sha256"].as_str().unwrap()
    );
    let text = match std::env::var("FOCUSED_FIVE_VANILLA_R4_FULL") {
        Ok(path) => {
            let text = std::fs::read_to_string(&path).expect("original full R4 cards.json");
            let digest = format!("{:x}", Sha256::digest(text.as_bytes()));
            assert_eq!(
                digest,
                provenance["original_runtime_sha256"].as_str().unwrap(),
                "full-database review requires the original R4 bytes"
            );
            let full: serde_json::Value = serde_json::from_str(&text).unwrap();
            let subset: serde_json::Value = serde_json::from_str(fixture).unwrap();
            assert_eq!(
                full.as_object().unwrap().len() as u64,
                provenance["original_runtime_record_count"]
                    .as_u64()
                    .unwrap()
            );
            for (id, row) in subset.as_object().unwrap() {
                assert_eq!(&full[id], row, "fixture row drift for {id}");
            }
            eprintln!("review source: full original R4; rows=4483; sha256={digest}");
            text
        }
        Err(std::env::VarError::NotPresent) => fixture.to_owned(),
        Err(err) => panic!("invalid full R4 path: {err}"),
    };
    CardData::load_from_str(&text).expect("original R4 runtime data")
}

#[test]
fn ordinary_production_registry_contains_all_four_native_definitions() {
    // Omitting dsl_card is intentional: DebugRunner must retain Game::new's
    // production registry rather than manually registering the subject cards.
    let runner = DebugRunner::builder()
        .with_card_data(original_data())
        .build();
    let pack = digimon_engine::dsl_registry::from_embedded_cached().unwrap();
    for id in CARDS {
        assert!(runner.game.effect_registry.get(id).is_some(), "{id}");
        let native = pack.lookup(id).expect("native embedded definition");
        assert!(native.effects.is_empty(), "{id}");
        assert!(native.alt_paths.is_empty(), "{id}");
        let row = runner
            .game
            .card_data
            .iter()
            .find(|c| c.card_id == id)
            .unwrap();
        assert_eq!(native.dp, row.dp, "{id}");
        assert_eq!(native.level, row.level, "{id}");
        assert_eq!(native.cost, Some(i32::from(row.play_cost)), "{id}");
    }
}

#[test]
fn native_rookies_and_champions_keep_all_four_distinct_evolution_choices() {
    let data = original_data();
    for player in 0..2 {
        for hand_index in 0..2 {
            for target in 0..2 {
                let mut runner = DebugRunner::builder()
                    .with_card_data(data.clone())
                    .hand(player, &CHAMPIONS)
                    .deck(0, &[ROOKIES[0]; 8])
                    .deck(1, &[ROOKIES[1]; 8])
                    .build();
                runner.set_first_player(player);
                runner.set_turn(2);
                runner.set_phase(GamePhase::Main);
                runner.game.set_memory(8);
                for id in ROOKIES {
                    runner.place_on_field(player, id, Some(0));
                }

                let mask = build_action_mask(&runner.game, player);
                for hand in 0..2 {
                    for field in 0..2 {
                        assert_eq!(mask[encode_digivolve(hand, field) as usize], 1.0);
                    }
                }
                let action = encode_digivolve(hand_index, target);
                assert_eq!(
                    build_action_mask(&runner.game, 1 - player)[action as usize],
                    0.0
                );
                runner.game.decode_action(action, player);

                for (index, stack) in runner.game.players[player as usize]
                    .battle_area
                    .iter()
                    .enumerate()
                {
                    let sources: Vec<_> = stack
                        .card_sources
                        .iter()
                        .map(|card| card.card_id(&runner.game.card_data))
                        .collect();
                    let expected = if index == target as usize {
                        vec![ROOKIES[index], CHAMPIONS[hand_index as usize]]
                    } else {
                        vec![ROOKIES[index]]
                    };
                    assert_eq!(sources, expected);
                }
                assert_eq!(runner.memory(), if hand_index == 0 { 6 } else { 7 });
                assert_eq!(runner.hand_size(player), 2);
                assert_eq!(runner.deck_size(player), 7);
                assert!(runner.pending_selection().is_none());
            }
        }
    }
}
