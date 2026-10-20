use std::collections::{HashMap, HashSet};
use std::fs;

use digimon_engine::card_data::CardData;
use digimon_engine::recorder::{ReplayDeckRef, VerificationReplayRecording};
use digimon_engine::replay_corpus::{
    bless_replay_corpus_dir, bless_verification_recording, convert_training_recording_artifact,
    generate_greedy_replay_corpus, regenerate_greedy_replay, verify_replay_corpus_dir,
    write_replay_corpus_entries,
    BlessOutcome, ConversionOutcome, ReplayCorpusConfig,
};
use digimon_engine::runners::replay::ReplaySession;

fn test_card_db() -> HashMap<String, CardData> {
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

fn deck_library_fixture() -> String {
    let deck = serde_json::to_string(&test_deck()).unwrap();
    format!(
        r#"{{
            "archetypes": {{
                "Starter A": {{
                    "stats": {{ "meta_share": 0.75 }},
                    "decklists": [
                        {{ "deck_id": "starter_a", "source": "test", "decklist": {deck:?} }}
                    ]
                }},
                "Starter B": {{
                    "stats": {{ "meta_share": 0.25 }},
                    "decklists": [
                        {{ "deck_id": "starter_b", "source": "test", "decklist": {deck:?} }}
                    ]
                }},
                "Unimplemented": {{
                    "stats": {{ "meta_share": 1.0 }},
                    "decklists": [
                        {{ "deck_id": "bad", "source": "test", "decklist": "[\"BT99-999\"]" }}
                    ]
                }}
            }}
        }}"#
    )
}

fn generated_recording(max_steps_per_game: u32, seed: u64) -> VerificationReplayRecording {
    let db = test_card_db();
    let implemented: HashSet<String> = ["ST1-01".to_string(), "ST1-03".to_string()].into();
    generate_greedy_replay_corpus(
        &deck_library_fixture(),
        b"{cards fixture}",
        &db,
        &implemented,
        ReplayCorpusConfig {
            game_count: 1,
            max_steps_per_game,
            base_seed: seed,
        },
    )
    .expect("generate corpus")
    .entries
    .into_iter()
    .next()
    .expect("one generated replay")
    .recording
}

#[test]
fn generated_greedy_corpus_is_sized_replayable_and_seeded() {
    let db = test_card_db();
    let implemented: HashSet<String> = ["ST1-01".to_string(), "ST1-03".to_string()].into();
    let config = ReplayCorpusConfig {
        game_count: 2,
        max_steps_per_game: 6,
        base_seed: 100,
    };

    let corpus = generate_greedy_replay_corpus(
        &deck_library_fixture(),
        b"{cards fixture}",
        &db,
        &implemented,
        config,
    )
    .expect("generate corpus");

    assert_eq!(corpus.entries.len(), 2);
    assert!(corpus.skips.iter().any(|skip| skip.id == "bad"));

    for (idx, entry) in corpus.entries.iter().enumerate() {
        assert_eq!(
            entry.recording.seed,
            Some(2 * (100 + idx as u64) + (idx as u64 % 2))
        );
        assert_eq!(entry.recording.actions.len(), entry.recording.digests.len());
        assert_eq!(
            entry.recording.final_digest,
            *entry.recording.digests.last().unwrap()
        );
        assert!(entry.name.ends_with(".replay.json"));
        assert!(matches!(
            entry.recording.deck_refs.player1,
            ReplayDeckRef::Inline { .. }
        ));
        assert!(matches!(
            entry.recording.deck_refs.player2,
            ReplayDeckRef::Inline { .. }
        ));

        let mut session =
            ReplaySession::from_verification_recording(entry.recording.clone(), &db, true).unwrap();
        session.run_to_completion();
        assert!(
            session.divergences().is_empty(),
            "generated corpus entry must replay without divergence"
        );
    }
}

#[test]
fn training_recording_converter_reports_reconstructible_and_skipped_artifacts() {
    let deck = test_deck();
    let artifact = serde_json::json!({
        "run": { "runner_seed": 42 },
        "recording": {
            "initial_state": {
                "player1": { "deck_list": deck },
                "player2": { "deck_list": test_deck() }
            },
            "actions": [
                { "player_id": 1, "action_id": 0 },
                { "player_id": 2, "action_id": 0 },
                { "player_id": 1, "action_id": 62 }
            ]
        }
    });

    let db = test_card_db();
    let outcome = convert_training_recording_artifact(&artifact, "sha256:test-cards", &db)
        .expect("conversion result");
    match outcome {
        ConversionOutcome::Converted(recording) => {
            assert_eq!(recording.seed, Some(42));
            assert_eq!(recording.actions.len(), 3);
            assert_eq!(recording.actions[0].seat, 0);
            assert_eq!(recording.actions[1].seat, 1);
        }
        ConversionOutcome::Skipped(skip) => panic!("expected converted artifact, got {skip:?}"),
    }

    let skipped = convert_training_recording_artifact(
        &serde_json::json!({
            "run": {},
            "recording": { "initial_state": null, "actions": [] }
        }),
        "sha256:test-cards",
        &db,
    )
    .expect("skip result");
    assert!(matches!(skipped, ConversionOutcome::Skipped(_)));
}

#[test]
fn corpus_writer_materializes_pretty_replay_json_files() {
    let db = test_card_db();
    let implemented: HashSet<String> = ["ST1-01".to_string(), "ST1-03".to_string()].into();
    let corpus = generate_greedy_replay_corpus(
        &deck_library_fixture(),
        b"{cards fixture}",
        &db,
        &implemented,
        ReplayCorpusConfig {
            game_count: 1,
            max_steps_per_game: 4,
            base_seed: 200,
        },
    )
    .expect("generate corpus");

    let out_dir =
        std::env::temp_dir().join(format!("digimon_replay_corpus_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&out_dir);
    let paths =
        write_replay_corpus_entries(&out_dir, &corpus.entries).expect("write corpus entries");
    assert_eq!(paths.len(), 1);

    let text = fs::read_to_string(&paths[0]).expect("read generated replay");
    assert!(text.contains("\"format_version\""));
    let parsed: digimon_engine::recorder::VerificationReplayRecording =
        serde_json::from_str(&text).expect("replay json parses");
    assert_eq!(
        parsed.actions.len(),
        corpus.entries[0].recording.actions.len()
    );

    fs::remove_dir_all(out_dir).expect("clean temp corpus directory");
}

#[test]
fn verify_corpus_dir_reports_first_replay_divergence_without_mutation() {
    let db = test_card_db();
    let root =
        std::env::temp_dir().join(format!("digimon_replay_verify_test_{}", std::process::id()));
    let goldens_dir = root.join("goldens");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&goldens_dir).expect("create goldens dir");

    let good = generated_recording(6, 250);
    fs::write(
        goldens_dir.join("good.replay.json"),
        format!("{}\n", serde_json::to_string_pretty(&good).unwrap()),
    )
    .expect("write good replay");

    let mut stale = good.clone();
    stale.digests[0] = stale.digests[0].wrapping_add(1);
    fs::write(
        goldens_dir.join("stale.replay.json"),
        format!("{}\n", serde_json::to_string_pretty(&stale).unwrap()),
    )
    .expect("write stale replay");

    let summary = verify_replay_corpus_dir(&goldens_dir, &db).expect("verify corpus");
    assert_eq!(summary.checked, 2);
    assert_eq!(summary.divergent.len(), 1);
    let divergence = summary.divergent[0].divergence.as_ref().unwrap();
    assert_eq!(divergence.game, "stale.replay.json");
    assert_eq!(divergence.step, 1);

    let stale_text =
        fs::read_to_string(goldens_dir.join("stale.replay.json")).expect("read stale replay");
    assert!(
        stale_text.contains(&stale.digests[0].to_string()),
        "verify must not rewrite stale replay files"
    );

    fs::remove_dir_all(root).expect("clean temp verify dir");
}

#[test]
fn bless_workflow_regenerates_digest_stream_without_changing_actions() {
    let db = test_card_db();
    let mut recording = generated_recording(6, 300);
    let original_actions = recording.actions.clone();
    recording.digests = vec![1; recording.actions.len()];
    recording.final_digest = 2;

    let outcome =
        bless_verification_recording(recording.clone(), &db).expect("bless verification replay");
    let BlessOutcome::Blessed(blessed) = outcome else {
        panic!("expected digest bless, got {outcome:?}");
    };

    assert_eq!(blessed.actions, original_actions);
    assert_eq!(blessed.digests.len(), blessed.actions.len());
    assert_ne!(blessed.digests, recording.digests);
    assert_eq!(blessed.final_digest, *blessed.digests.last().unwrap());

    let mut session =
        ReplaySession::from_verification_recording(blessed, &db, true).expect("replay blessed");
    session.run_to_completion();
    assert!(session.divergences().is_empty());
}

#[test]
fn bless_workflow_retires_unreplayable_actions_with_review_reason() {
    let db = test_card_db();
    let mut recording = generated_recording(6, 400);
    recording.actions[0].action_id = u16::MAX;

    let outcome = bless_verification_recording(recording, &db).expect("bless result");
    let BlessOutcome::Retired(retired) = outcome else {
        panic!("expected retired replay, got {outcome:?}");
    };

    assert!(retired.reason.contains("illegal action"));
    assert!(retired.reason.contains("step 0"));
}

#[test]
fn bless_corpus_dir_rewrites_changed_replays_and_retires_unreplayable_files() {
    let db = test_card_db();
    let root =
        std::env::temp_dir().join(format!("digimon_replay_bless_test_{}", std::process::id()));
    let goldens_dir = root.join("goldens");
    let retired_dir = root.join("retired");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&goldens_dir).expect("create goldens dir");

    let mut stale = generated_recording(6, 500);
    stale.digests = vec![7; stale.actions.len()];
    stale.final_digest = 8;
    fs::write(
        goldens_dir.join("stale.replay.json"),
        format!("{}\n", serde_json::to_string_pretty(&stale).unwrap()),
    )
    .expect("write stale replay");

    let mut illegal = generated_recording(6, 600);
    illegal.actions[0].action_id = u16::MAX;
    fs::write(
        goldens_dir.join("illegal.replay.json"),
        format!("{}\n", serde_json::to_string_pretty(&illegal).unwrap()),
    )
    .expect("write illegal replay");

    let summary =
        bless_replay_corpus_dir(&goldens_dir, &retired_dir, &db).expect("bless corpus dir");

    assert_eq!(summary.updated, 1);
    assert_eq!(summary.unchanged, 0);
    assert_eq!(summary.retired.len(), 1);
    assert!(!goldens_dir.join("illegal.replay.json").exists());
    assert!(retired_dir.join("illegal.retired.json").exists());

    let stale_text =
        fs::read_to_string(goldens_dir.join("stale.replay.json")).expect("read blessed replay");
    let blessed_stale: VerificationReplayRecording =
        serde_json::from_str(&stale_text).expect("parse blessed replay");
    assert_ne!(blessed_stale.digests, stale.digests);
    assert_eq!(
        blessed_stale.final_digest,
        *blessed_stale.digests.last().unwrap()
    );

    let retired_text =
        fs::read_to_string(retired_dir.join("illegal.retired.json")).expect("read retired replay");
    assert!(retired_text.contains("\"reason\""));
    assert!(retired_text.contains("illegal action"));

    fs::remove_dir_all(root).expect("clean temp bless dir");
}

#[test]
fn legacy_schema_rehash_stays_legacy_and_replays_cleanly() {
    use digimon_engine::recorder::VerificationDigestVersion;
    let db = test_card_db();
    let mut legacy = generated_recording(6, 801);
    legacy.engine_schema_version = 1;
    let mut session = ReplaySession::from_verification_recording(legacy.clone(), &db, false).unwrap();
    let mut expected = Vec::new();
    while !session.is_complete() {
        session.step();
        assert!(!session.is_paused());
        expected.push(session.game.verification_digest_with_version(VerificationDigestVersion::LegacyV2));
    }
    legacy.digests = expected.clone(); legacy.final_digest = *expected.last().unwrap();
    let report = ReplaySession::verify_verification_recording("legacy-positive", legacy.clone(), &db).unwrap();
    assert!(report.divergence.is_none(), "{report:?}");
    legacy.digests.fill(0); legacy.final_digest = 0;
    let BlessOutcome::Blessed(blessed) = bless_verification_recording(legacy, &db).unwrap() else { panic!("legacy should stay replayable"); };
    assert_eq!(blessed.engine_schema_version, 1);
    assert_eq!(blessed.digests, expected);
    assert_eq!(blessed.final_digest, *expected.last().unwrap());
    assert!(ReplaySession::verify_verification_recording("legacy-rehash", blessed, &db).unwrap().divergence.is_none());
}

#[test]
fn unsupported_versions_never_retire_or_rewrite_corpus_files() {
    let db = test_card_db();
    for (format, schema) in [(1, 3), (2, 2), (1, 999)] {
        let root = std::env::temp_dir().join(format!("digimon_unknown_protocol_{}_{}_{}_{}", std::process::id(), format, schema, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let goldens = root.join("goldens"); let retired = root.join("retired");
        fs::create_dir_all(&goldens).unwrap();
        let mut recording = generated_recording(6, 802);
        recording.format_version = format; recording.engine_schema_version = schema;
        let bytes = serde_json::to_vec_pretty(&recording).unwrap();
        let file = goldens.join("future.replay.json"); fs::write(&file, &bytes).unwrap();
        assert!(bless_replay_corpus_dir(&goldens, &retired, &db).is_err());
        assert_eq!(fs::read(&file).unwrap(), bytes);
        assert!(!retired.join("future.retired.json").exists());
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn regenerate_rerecords_one_golden_from_its_inline_decks_and_seed() {
    let original = generated_recording(12, 40_521_420);
    let db = test_card_db();
    // Same decks, seed and length budget: the same greedy game, byte for byte.
    let again = regenerate_greedy_replay(&original, b"{cards fixture}", &db, None)
        .expect("inline decks + seed regenerate");
    assert_eq!(again, original);
    assert_eq!(again.actions.len(), original.actions.len());
    // An explicit budget overrides the original action count.
    let shorter = regenerate_greedy_replay(&original, b"{cards fixture}", &db, Some(4)).unwrap();
    assert_eq!(shorter.actions.len(), 4.min(original.actions.len()));
    assert_eq!(shorter.actions[..], original.actions[..shorter.actions.len()]);
    // Never guessed: a decklist ref or a missing seed is refused.
    let mut by_id = original.clone();
    by_id.deck_refs.player1 = ReplayDeckRef::Decklist { id: "starter_a".to_string() };
    assert!(regenerate_greedy_replay(&by_id, b"{cards fixture}", &db, None).is_err());
    let mut unseeded = original;
    unseeded.seed = None;
    assert!(regenerate_greedy_replay(&unseeded, b"{cards fixture}", &db, None).is_err());
}
