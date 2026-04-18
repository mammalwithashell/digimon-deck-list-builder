//! Recorder wires into HeadlessRunner::step. N steps → N recorded
//! actions, dict shape matches Python's `GameRecorder.to_dict`.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::HeadlessRunner;

/// Minimal inline card DB — mirrors the shape used in `infra/headless_runner.rs`
/// so these tests run without an external `cards.json` file.
fn minimal_db() -> HashMap<String, CardData> {
    let json = r#"{
        "ST1-01": {
            "card_id": "ST1-01", "card_name_eng": "Botamon",
            "card_effect_class_name": "ST1_01", "play_cost": 0, "dp": -1,
            "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
            "type_eng": ["Lesser"], "form_eng": ["In-Training"], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "ST1-03": {
            "card_id": "ST1-03", "card_name_eng": "Koromon",
            "card_effect_class_name": "ST1_03", "play_cost": 3, "dp": 2000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Lesser"], "form_eng": ["Rookie"], "attribute_eng": ["Free"],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        }
    }"#;
    CardData::load_from_str(json).unwrap()
}

/// Build a 50-card deck: 4× ST1-01 (DigiEgg / level 2) + 46× ST1-03.
/// The deck must be ≥ 50 cards total (4 eggs + 46 main); the engine
/// separates eggs (card_kind == 3) from main-deck cards automatically.
fn test_deck() -> Vec<String> {
    std::iter::repeat("ST1-01".to_string())
        .take(4)
        .chain(std::iter::repeat("ST1-03".to_string()).take(46))
        .collect()
}

#[test]
fn recording_has_expected_top_level_keys() {
    let db = minimal_db();
    let deck: Vec<String> = std::iter::repeat("ST1-01".to_string())
        .take(5)
        .chain(std::iter::repeat("ST1-03".to_string()).take(45))
        .collect();
    let mut r =
        HeadlessRunner::new(deck.clone(), deck, &db, false, true, false, Some(42)).unwrap();
    // Complete mulligan first (both players keep)
    r.step(0);
    r.step(0);
    // Three passes in Main phase
    r.step(62);
    r.step(62);
    r.step(62);

    let rec = r.get_recording().expect("recorder enabled → recording present");
    for key in [
        "initial_state",
        "actions",
        "total_actions",
        "tensor_snapshots_count",
    ] {
        assert!(rec.as_object().unwrap().contains_key(key), "missing {:?}", key);
    }
    // tensor_snapshots is omitted when empty (record_tensors=false)
    assert!(!rec.as_object().unwrap().contains_key("tensor_snapshots"));
    assert_eq!(rec["tensor_snapshots_count"], serde_json::json!(0));
    // 5 total actions (2 mulligan keeps + 3 passes)
    assert_eq!(rec["total_actions"], serde_json::json!(5));
    assert_eq!(rec["actions"].as_array().unwrap().len(), 5);
}

#[test]
fn recording_initial_state_has_both_players() {
    let db = minimal_db();
    let deck = test_deck();
    let mut r =
        HeadlessRunner::new(deck.clone(), deck, &db, false, true, false, Some(7)).unwrap();
    // Must call step() at least once so the lazy capture fires after mulligan.
    // Action 0 = mulligan-keep for player 1; step again for player 2.
    r.step(0); // player 1 keeps mulligan
    r.step(0); // player 2 keeps mulligan

    let rec = r.get_recording().expect("recorder enabled");
    let init = &rec["initial_state"];
    assert!(init["player1"].is_object());
    assert!(init["player2"].is_object());
    assert!(init["first_player_id"].is_i64());
    assert!(init["timestamp"].is_string());

    // Verify ISO-8601 timestamp format (YYYY-MM-DDTHH:MM:SS+00:00 = 25 chars).
    let ts = init["timestamp"].as_str().unwrap();
    assert_eq!(ts.len(), 25, "timestamp should be 25-char ISO-8601: {:?}", ts);
    assert_eq!(&ts[19..], "+00:00");

    for key in [
        "player_id",
        "deck_list",
        "library_order",
        "digitama_library_order",
        "security_order",
        "initial_hand",
    ] {
        assert!(
            init["player1"].as_object().unwrap().contains_key(key),
            "initial_state.player1 missing {:?}",
            key
        );
    }

    // Security must be populated (5 cards after mulligan) — guards Issue 1 regression.
    assert_eq!(
        init["player1"]["security_order"].as_array().unwrap().len(),
        5,
        "player1 security_order should have 5 cards"
    );
    assert_eq!(
        init["player2"]["security_order"].as_array().unwrap().len(),
        5,
        "player2 security_order should have 5 cards"
    );
}

#[test]
fn recording_returns_none_when_disabled() {
    let db = minimal_db();
    let deck = test_deck();
    // record_actions=false → no recorder
    let r =
        HeadlessRunner::new(deck.clone(), deck, &db, false, false, false, Some(1)).unwrap();
    assert!(r.get_recording().is_none());
}

#[test]
fn lazy_capture_fires_on_pre_action_window() {
    let db = minimal_db();
    let deck: Vec<String> = std::iter::repeat("ST1-01".to_string())
        .take(5)
        .chain(std::iter::repeat("ST1-03".to_string()).take(45))
        .collect();
    let mut r =
        HeadlessRunner::new(deck.clone(), deck, &db, false, true, false, Some(11)).unwrap();

    // Complete mulligan via step (both players keep)
    r.step(0);
    r.step(0);
    // By now mulligan is complete and initial_state should have been captured
    // in the post-action window of the second step.
    r.step(62); // PASS in Main phase

    let rec = r.get_recording().expect("recorder enabled");
    let init = &rec["initial_state"];
    assert!(init.is_object(), "initial_state must be populated after mulligan");
    assert_eq!(
        init["player1"]["security_order"].as_array().unwrap().len(),
        5
    );
    // Confirm the recorded action sequence includes the post-mulligan step.
    let actions = rec["actions"].as_array().unwrap();
    assert!(
        actions.iter().any(|a| a["action_id"] == serde_json::json!(62)),
        "expected a post-mulligan PASS action in actions[]"
    );
}

#[test]
fn record_tensors_produces_snapshots() {
    let db = minimal_db();
    let deck: Vec<String> = std::iter::repeat("ST1-01".to_string())
        .take(5)
        .chain(std::iter::repeat("ST1-03".to_string()).take(45))
        .collect();
    let mut r = HeadlessRunner::new(
        deck.clone(),
        deck,
        &db,
        false, // verbose
        true,  // record_actions
        true,  // record_tensors
        Some(3),
    )
    .unwrap();
    r.step(0);
    r.step(0);
    r.step(62);

    let rec = r.get_recording().expect("recorder enabled");
    let snapshots = rec["tensor_snapshots"]
        .as_array()
        .expect("tensor_snapshots array present");
    // At least one snapshot for the post-mulligan step
    assert!(
        !snapshots.is_empty(),
        "expected tensor snapshots, got {}",
        snapshots.len()
    );
    assert!(rec["tensor_snapshots_count"].as_i64().unwrap() >= 1);
    // Snapshot shape
    let s = &snapshots[0];
    assert!(s["tensor"].is_array());
    assert!(s["action_mask"].is_array());
    assert!(s["step"].is_i64());
    assert!(s["player_id"].is_i64());
}
