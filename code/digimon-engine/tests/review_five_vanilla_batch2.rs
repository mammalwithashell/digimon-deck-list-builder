//! Independent BT4-014 review through the production registry and native data.
//! Set FOCUSED_FIVE_VANILLA_R4_FULL to replay the pinned full R4 database;
//! without it, the portable exact-row subset is used. No manual DSL registration.

use std::collections::HashMap;

use digimon_engine::action::{build_action_mask, encode_digivolve};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::GamePhase;
use sha2::{Digest, Sha256};

const SUBJECT: &str = "BT4-014";
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
            let text = std::fs::read_to_string(path).expect("original full R4 cards.json");
            let digest = format!("{:x}", Sha256::digest(text.as_bytes()));
            assert_eq!(
                digest,
                provenance["original_runtime_sha256"].as_str().unwrap(),
                "full R4 bytes must match the original witness"
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
fn production_registry_preserves_vermilimon_native_ordinary_evolution() {
    let runner = DebugRunner::builder()
        .with_card_data(original_data())
        .build();
    // Game::new supplies this registry: no dsl_card or with_registry override.
    assert!(runner.game.effect_registry.get(SUBJECT).is_some());
    let pack = digimon_engine::dsl_registry::from_embedded_cached().unwrap();
    let native = pack.lookup(SUBJECT).expect("native Vermilimon definition");
    assert!(native.effects.is_empty());
    assert!(native.alt_paths.is_empty());
    let row = runner
        .game
        .card_data
        .iter()
        .find(|card| card.card_id == SUBJECT)
        .unwrap();
    assert_eq!(native.dp, row.dp);
    assert_eq!(native.level, row.level);
    assert_eq!(native.cost, Some(i32::from(row.play_cost)));
    assert_eq!(
        row.evo_costs,
        [EvoCost {
            card_color: 0,
            level: 4,
            memory_cost: 3,
        }]
    );
}

#[test]
fn both_real_champion_bases_are_independent_choices_for_both_seats() {
    let data = original_data();
    for player in 0..2 {
        for chosen in 0..2 {
            let mut runner = DebugRunner::builder()
                .with_card_data(data.clone())
                .hand(player, &[SUBJECT])
                .deck(0, &["BT1-009"; 8])
                .deck(1, &["BT3-007"; 8])
                .build();
            runner.set_first_player(player);
            runner.set_turn(2);
            runner.set_phase(GamePhase::Main);
            runner.game.set_memory(8);
            for base in CHAMPIONS {
                runner.place_on_field(player, base, Some(0));
            }
            // A real red rookie is a discriminating ineligible level control.
            runner.place_on_field(player, "BT1-009", Some(0));
            let mask = build_action_mask(&runner.game, player);
            for target in 0..2 {
                assert_eq!(mask[encode_digivolve(0, target) as usize], 1.0);
            }
            assert_eq!(mask[encode_digivolve(0, 2) as usize], 0.0);
            let action = encode_digivolve(0, chosen);
            assert_eq!(
                build_action_mask(&runner.game, 1 - player)[action as usize],
                0.0
            );
            assert_eq!(mask[action as usize], 1.0);
            runner.game.decode_action(action, player);

            for (index, base) in CHAMPIONS.iter().enumerate() {
                let sources: Vec<_> = runner.game.players[player as usize].battle_area[index]
                    .card_sources
                    .iter()
                    .map(|card| card.card_id(&runner.game.card_data))
                    .collect();
                let expected = if index == chosen as usize {
                    vec![*base, SUBJECT]
                } else {
                    vec![*base]
                };
                assert_eq!(sources, expected);
            }
            let rookie = &runner.game.players[player as usize].battle_area[2];
            assert_eq!(rookie.card_sources.len(), 1);
            assert_eq!(rookie.top_card().card_id(&runner.game.card_data), "BT1-009");
            assert_eq!(runner.memory(), 5);
            assert_eq!(runner.hand_size(player), 1);
            assert_eq!(runner.deck_size(player), 7);
            assert_eq!(
                runner.dp_of(runner.perm_handle(player, chosen as usize)),
                Some(8000)
            );
            assert!(runner.pending_selection().is_none());
        }
    }
}
