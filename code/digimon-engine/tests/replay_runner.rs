//! Round-trip integration tests for `ReplayRunner`.
//!
//! Tasks 3.11–3.14 of the `add-engine-debug-mcp` change:
//! - Round-trip parity (record a real game with `HeadlessRunner`, replay it,
//!   assert state matches).
//! - Verify-mode divergence detection.
//! - Seek-equivalence (`seek(N)` ≡ N sequential `step()` calls).
//! - Backward-seek correctness.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::runners::replay::{ReplayError, ReplayRunner};
use digimon_engine::HeadlessRunner;

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

fn test_deck() -> Vec<String> {
    std::iter::repeat("ST1-01".to_string())
        .take(5)
        .chain(std::iter::repeat("ST1-03".to_string()).take(45))
        .collect()
}

/// Record a game with N steps: 2 mulligan keeps + (N-2) passes. Returns
/// the recording JSON.
fn record_game(n_post_mulligan_steps: usize) -> serde_json::Value {
    let db = minimal_db();
    let deck = test_deck();
    let mut r = HeadlessRunner::new(
        deck.clone(),
        deck,
        &db,
        false, // verbose
        true,  // record_actions
        false, // record_tensors
        Some(42),
    )
    .expect("headless runner constructs");

    // Mulligan: both players keep.
    r.step(0);
    r.step(0);
    for _ in 0..n_post_mulligan_steps {
        r.step(62); // PASS
    }
    r.get_recording().expect("recording present")
}

#[test]
fn round_trip_construct_replay_no_divergence() {
    let db = minimal_db();
    let recording = record_game(5);
    let mut replay = ReplayRunner::new(recording, &db, true).expect("constructs from recording");

    // 5 replayable steps (the 2 mulligan actions are filtered out).
    assert_eq!(replay.total_steps(), 5);

    let mut total_divergences = 0;
    while !replay.is_complete() {
        let r = replay.step();
        total_divergences += r.divergences.len();
    }
    // With identical engine on both sides and no RNG-consuming effects in
    // this minimal deck, replay should produce zero divergences.
    assert_eq!(
        total_divergences, 0,
        "round-trip replay produced divergences"
    );
}

#[test]
fn run_to_completion_advances_through_all_actions() {
    let db = minimal_db();
    let recording = record_game(3);
    let mut replay = ReplayRunner::new(recording, &db, false).expect("constructs");
    let n = replay.total_steps();
    replay.run_to_completion();
    assert_eq!(replay.current_step(), n);
    assert!(replay.is_complete());
}

#[test]
fn seek_forward_equivalent_to_sequential_steps() {
    let db = minimal_db();
    let recording = record_game(4);

    // Path A: seek directly to step 3.
    let mut r_seek = ReplayRunner::new(recording.clone(), &db, false).unwrap();
    r_seek.seek(3).expect("seek 3");

    // Path B: call step() three times.
    let mut r_step = ReplayRunner::new(recording, &db, false).unwrap();
    for _ in 0..3 {
        r_step.step();
    }

    // Compare key observable state. Internal `Game` equality is not
    // trivially comparable (closures, RNG state), so we compare the
    // serializable surfaces that matter for debugging.
    assert_eq!(r_seek.current_step(), r_step.current_step());
    assert_eq!(r_seek.game.turn_count, r_step.game.turn_count);
    assert_eq!(r_seek.game.memory, r_step.game.memory);
    assert_eq!(r_seek.game.current_phase, r_step.game.current_phase);
    assert_eq!(r_seek.game.turn_player(), r_step.game.turn_player());
    for i in 0..2 {
        assert_eq!(r_seek.game.players[i].hand.len(), r_step.game.players[i].hand.len());
        assert_eq!(r_seek.game.players[i].deck.len(), r_step.game.players[i].deck.len());
        assert_eq!(
            r_seek.game.players[i].battle_area.len(),
            r_step.game.players[i].battle_area.len()
        );
    }
}

#[test]
fn seek_backward_rebuilds_from_initial() {
    let db = minimal_db();
    let recording = record_game(5);

    // Path A: step to 4, then seek back to 2.
    let mut r_back = ReplayRunner::new(recording.clone(), &db, false).unwrap();
    for _ in 0..4 {
        r_back.step();
    }
    r_back.seek(2).expect("backward seek to 2");

    // Path B: build fresh and step to 2.
    let mut r_fresh = ReplayRunner::new(recording, &db, false).unwrap();
    for _ in 0..2 {
        r_fresh.step();
    }

    assert_eq!(r_back.current_step(), 2);
    assert_eq!(r_back.current_step(), r_fresh.current_step());
    assert_eq!(r_back.game.turn_count, r_fresh.game.turn_count);
    assert_eq!(r_back.game.memory, r_fresh.game.memory);
    assert_eq!(r_back.game.current_phase, r_fresh.game.current_phase);
}

#[test]
fn verify_mode_detects_injected_memory_divergence() {
    let db = minimal_db();
    let mut recording = record_game(3);

    // Inject a synthetic divergence: corrupt the recorded memory_after of
    // the first replayable action so verify mode reports it.
    let actions = recording["actions"]
        .as_array()
        .unwrap()
        .clone();
    // Find the first non-mulligan action and bump memory_after.
    let mut new_actions = Vec::new();
    let mut bumped = false;
    for mut a in actions {
        if !bumped && a["phase"].as_str() != Some("Mulligan") {
            let cur = a["memory_after"].as_i64().unwrap_or(0);
            a["memory_after"] = serde_json::json!(cur + 999);
            bumped = true;
        }
        new_actions.push(a);
    }
    recording["actions"] = serde_json::Value::Array(new_actions);

    let mut replay = ReplayRunner::new(recording, &db, true).expect("constructs");
    let first = replay.step();
    let mem_div = first
        .divergences
        .iter()
        .find(|d| d.field == "memory_after");
    assert!(
        mem_div.is_some(),
        "expected memory_after divergence in step 1, got: {:?}",
        first.divergences
    );
}

#[test]
fn unknown_card_in_recording_errors() {
    let db = minimal_db();
    let mut recording = record_game(2);
    // Replace one card_id in player1's library with something not in `db`.
    recording["initial_state"]["player1"]["library_order"][0] =
        serde_json::json!("UNKNOWN-XYZ");
    match ReplayRunner::new(recording, &db, false) {
        Err(ReplayError::UnknownCard(ids)) => {
            assert!(ids.contains(&"UNKNOWN-XYZ".to_string()));
        }
        other => panic!("expected UnknownCard, got {:?}", other.err()),
    }
}
