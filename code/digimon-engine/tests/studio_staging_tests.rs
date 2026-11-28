//! Unit tests for Board-State Studio staging & snapshot extensions
//! (`egg_deck`, `linked_cards`, `face_up_security`, legacy fixture
//! compatibility, and staged search-exam reconstruction).

use digimon_engine::card_data::CardData;
use digimon_engine::game::Game;
use digimon_engine::rules::Rules;
use digimon_engine::search::decision_player;
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;

fn load_cards() -> HashMap<String, CardData> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/cards.json");
    CardData::load_from_file(&path).expect("load data/cards.json")
}

fn filler_deck() -> Vec<String> {
    let mut d = vec!["ST1-01".to_string(); 4];
    d.extend(vec!["ST1-03".to_string(); 50]);
    d
}

#[test]
fn stage_and_snapshot_round_trips_egg_deck_linked_cards_and_face_up_security() {
    let cards = load_cards();
    let decks = vec![filler_deck(), filler_deck()];
    let mut g = Game::new(&decks, &cards, Rules::standard(), Some(7)).unwrap();
    while let Some(p) = g.mulligan_current_player() {
        let _ = g.accept_mulligan(p, true);
    }

    let fixture = json!({
        "state": {
            "memory": 3,
            "phase": "Main",
            "turn": 4,
            "first_player": 1
        },
        "zones": {
            "1": {
                "hand": ["ST1-03", "ST1-07"],
                "egg_deck": ["ST1-01", "ST1-01"],
                "security": [
                    {"card_id": "ST1-03", "face_up": true},
                    {"card_id": "ST1-07", "face_up": false},
                    {"card_id": "ST1-11", "face_up": true}
                ],
                "field": [
                    {
                        "stack": ["ST1-03", "ST1-07"],
                        "is_suspended": true,
                        "turn_played": 2,
                        "linked_cards": ["ST1-11"]
                    }
                ],
                "breeding": ["ST1-01", "ST1-03"]
            },
            "2": {
                "hand": ["ST1-03"],
                "egg_deck": ["ST1-01"],
                "security": ["ST1-03", "ST1-07"],
                "face_up_security": [1],
                "field": []
            }
        }
    });

    g.apply_scenario(&fixture).expect("apply_scenario succeeds");

    // Verify P1 state directly.
    let p0 = &g.players[0];
    assert_eq!(p0.digitama_deck.len(), 2);
    assert_eq!(p0.battle_area.len(), 1);
    assert_eq!(p0.battle_area[0].card_sources.len(), 2);
    assert_eq!(p0.battle_area[0].linked_cards.len(), 1);
    assert_eq!(
        g.card_data[p0.battle_area[0].linked_cards[0].data_index].card_id,
        "ST1-11"
    );
    assert!(p0.battle_area[0].is_suspended);
    assert_eq!(p0.battle_area[0].turn_played, 2);
    // Security top (vec tail) and bottom (vec index 0) are face-up; middle is face-down.
    assert_eq!(p0.security.len(), 3);
    assert!(p0.face_up_security.contains(&p0.security[2].card_index));
    assert!(!p0.face_up_security.contains(&p0.security[1].card_index));
    assert!(p0.face_up_security.contains(&p0.security[0].card_index));

    // Verify P2 face_up_security by index.
    let p1 = &g.players[1];
    assert_eq!(p1.security.len(), 2);
    assert!(!p1.face_up_security.contains(&p1.security[1].card_index));
    assert!(p1.face_up_security.contains(&p1.security[0].card_index));

    // Capture via to_scenario() and re-apply onto a fresh game.
    let captured = g.to_scenario();
    assert_eq!(captured["zones"]["1"]["egg_deck"], json!(["ST1-01", "ST1-01"]));
    assert_eq!(
        captured["zones"]["1"]["field"][0]["linked_cards"],
        json!(["ST1-11"])
    );
    assert_eq!(captured["zones"]["1"]["face_up_security"], json!([0, 2]));
    assert_eq!(captured["zones"]["2"]["face_up_security"], json!([1]));

    let mut g2 = Game::new(&decks, &cards, Rules::standard(), Some(99)).unwrap();
    while let Some(p) = g2.mulligan_current_player() {
        let _ = g2.accept_mulligan(p, true);
    }
    g2.apply_scenario(&captured).expect("re-apply captured scenario");

    let p0_2 = &g2.players[0];
    assert_eq!(p0_2.digitama_deck.len(), 2);
    assert_eq!(p0_2.battle_area[0].linked_cards.len(), 1);
    assert!(p0_2.face_up_security.contains(&p0_2.security[2].card_index));
    assert!(!p0_2.face_up_security.contains(&p0_2.security[1].card_index));
    assert!(p0_2.face_up_security.contains(&p0_2.security[0].card_index));
    assert_eq!(g2.memory, 3);
    assert_eq!(g2.turn_count, 4);
}

#[test]
fn legacy_scenario_fixture_without_new_fields_still_applies() {
    let cards = load_cards();
    let decks = vec![filler_deck(), filler_deck()];
    let mut g = Game::new(&decks, &cards, Rules::standard(), Some(1)).unwrap();
    while let Some(p) = g.mulligan_current_player() {
        let _ = g.accept_mulligan(p, true);
    }

    let legacy = json!({
        "state": {
            "memory": 0,
            "phase": "Main",
            "turn": 2,
            "first_player": 2
        },
        "zones": {
            "1": {
                "hand": ["ST1-03"],
                "security": ["ST1-03", "ST1-07"],
                "field": [{ "stack": ["ST1-03"], "is_suspended": false, "turn_played": 0 }]
            },
            "2": {
                "hand": ["ST1-07"],
                "security": ["ST1-03"]
            }
        }
    });

    g.apply_scenario(&legacy).expect("legacy scenario succeeds");
    assert!(g.players[0].face_up_security.is_empty());
    assert!(g.players[0].battle_area[0].linked_cards.is_empty());
    assert_eq!(decision_player(&g), 1);
}
