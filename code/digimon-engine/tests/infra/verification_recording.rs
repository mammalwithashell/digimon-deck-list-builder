use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::recorder::{
    cards_json_content_hash, GameRecorder, ReplayDeckRefs, VERIFICATION_REPLAY_FORMAT_VERSION,
};
use digimon_engine::HeadlessRunner;

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
    for _ in 0..4 {
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

#[test]
fn cards_hash_is_sha256_over_exact_cards_json_bytes() {
    assert_eq!(
        cards_json_content_hash(b"abc"),
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn headless_runner_emits_canonical_replay_recording() {
    let db = test_card_db();
    let deck = test_deck();
    let mut runner = HeadlessRunner::new(
        deck.clone(),
        deck.clone(),
        &db,
        false,
        true,
        false,
        Some(17),
    )
    .unwrap();

    while runner.mulligan_current_player().is_some() {
        runner.step(0);
    }
    runner.step(62);

    let recording = runner
        .get_verification_replay_recording("sha256:test-cards")
        .expect("recording enabled");

    assert_eq!(recording.format_version, VERIFICATION_REPLAY_FORMAT_VERSION);
    assert_eq!(recording.cards_hash, "sha256:test-cards");
    assert_eq!(recording.seed, Some(17));
    assert_eq!(recording.actions.len(), recording.digests.len());
    assert_eq!(recording.final_digest, *recording.digests.last().unwrap());
    assert_eq!(
        recording.deck_refs.player1.inline_cards(),
        Some(deck.as_slice())
    );
    assert_eq!(
        recording.deck_refs.player2.inline_cards(),
        Some(deck.as_slice())
    );
    assert!(
        recording.actions[0].seat < 2,
        "seat is zero-based and within player count"
    );
    assert_eq!(recording.actions[0].action_id, 0);

    let json = serde_json::to_value(&recording).unwrap();
    assert_eq!(json["format_version"], 1);
    assert!(json["actions"][0].get("seat").is_some());
    assert!(json["actions"][0].get("player_id").is_none());
    assert_eq!(
        json["digests"].as_array().unwrap().len(),
        recording.actions.len()
    );
}

#[test]
fn debug_runner_recorded_action_hook_emits_digests() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "Test"))
        .memory(3)
        .start();
    let mut recorder = GameRecorder::new(false);

    runner.decode_recorded_action(&mut recorder, 0, 62);

    let recording = recorder.to_verification_replay(
        &runner.game,
        ReplayDeckRefs::inline(Vec::new(), Vec::new()),
        Some(99),
        "sha256:test-cards",
    );

    assert_eq!(recording.actions.len(), 1);
    assert_eq!(recording.digests, vec![runner.game.verification_digest()]);
    assert_eq!(recording.final_digest, runner.game.verification_digest());
}

#[test]
fn current_replay_protocol_round_trips_and_unknown_versions_are_rejected() {
    use digimon_engine::runners::replay::{ReplaySession, VerificationReplayAdapter};
    use digimon_engine::recorder::VERIFICATION_ENGINE_SCHEMA_VERSION;
    let db = test_card_db();
    let deck = test_deck();
    let mut runner = HeadlessRunner::new(deck.clone(), deck, &db, false, true, false, Some(17)).unwrap();
    while runner.mulligan_current_player().is_some() { runner.step(0); }
    runner.step(62);
    let recording = runner.get_verification_replay_recording("sha256:test-cards").unwrap();
    assert_eq!(recording.engine_schema_version, VERIFICATION_ENGINE_SCHEMA_VERSION);
    assert_eq!(recording.engine_schema_version, 3);
    let report = ReplaySession::verify_verification_recording("current-digest", recording.clone(), &db).unwrap();
    assert!(report.divergence.is_none(), "{report:?}");
    let mut unknown = recording.clone(); unknown.engine_schema_version = 999;
    assert!(VerificationReplayAdapter::from_recording(unknown, &db).is_err());
    let mut wrong = recording.clone(); wrong.engine_schema_version = 1;
    let report = ReplaySession::verify_verification_recording("wrong-digest-version", wrong, &db).unwrap();
    assert!(report.divergence.is_some(), "no fallback to another digest version");
    let mut unknown_format = recording; unknown_format.format_version = 999;
    assert!(VerificationReplayAdapter::from_recording(unknown_format, &db).is_err());
}
