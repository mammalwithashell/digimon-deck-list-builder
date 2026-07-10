use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use digimon_engine::card_data::CardData;
use digimon_engine::recorder::VerificationReplayRecording;
use digimon_engine::runners::replay::ReplaySession;
use digimon_engine::HeadlessRunner;

const DIGEST_HELPER_ENV: &str = "DIGIMON_VERIFICATION_DIGEST_HELPER";
const DIGEST_STREAM_PREFIX: &str = "DIGIMON_DIGEST_STREAM_JSON=";

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

fn verification_recording() -> VerificationReplayRecording {
    let db = test_card_db();
    let deck = test_deck();
    let mut runner =
        HeadlessRunner::new(deck.clone(), deck, &db, false, true, false, Some(20260708))
            .expect("headless runner constructs");

    runner.step(0);
    runner.step(0);
    for _ in 0..4 {
        runner.step(62);
    }

    runner
        .get_verification_replay_recording("sha256:test-cards")
        .expect("verification recording enabled")
}

fn replay_digest_stream(recording: VerificationReplayRecording) -> Vec<u64> {
    let db = test_card_db();
    let mut session = ReplaySession::from_verification_recording(recording, &db, true)
        .expect("verification replay session constructs");
    let mut digests = Vec::new();

    while !session.is_complete() && !session.is_game_over() && !session.is_paused() {
        let step = session.step();
        assert!(
            step.divergences.is_empty(),
            "valid deterministic replay must not diverge: {:?}",
            step.divergences
        );
        assert!(
            session.divergences().is_empty(),
            "valid deterministic replay must not record mask divergences: {:?}",
            session.divergences()
        );
        digests.push(session.game.verification_digest());
    }

    assert!(session.is_complete(), "fixture should replay to completion");
    digests
}

#[test]
fn verification_replay_digests_are_process_deterministic() {
    let recording = verification_recording();
    let first = replay_digest_stream(recording.clone());
    let second = replay_digest_stream(recording.clone());

    assert_eq!(
        first, recording.digests,
        "replay matches recorded digest stream"
    );
    assert_eq!(first, second, "two in-process replays are byte-identical");

    let output = Command::new(std::env::current_exe().expect("current test exe"))
        .arg("determinism_guard::verification_replay_digest_helper")
        .arg("--exact")
        .arg("--ignored")
        .arg("--nocapture")
        .env(DIGEST_HELPER_ENV, "1")
        .output()
        .expect("spawn digest helper test process");

    assert!(
        output.status.success(),
        "digest helper failed: status={:?}\nstdout={}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find(|line| line.starts_with(DIGEST_STREAM_PREFIX))
        .unwrap_or_else(|| panic!("digest helper did not print stream marker; stdout={stdout}"));
    let cross_process: Vec<u64> =
        serde_json::from_str(&line[DIGEST_STREAM_PREFIX.len()..]).unwrap();

    assert_eq!(
        first, cross_process,
        "cross-process replay digest stream is byte-identical"
    );
}

#[test]
#[ignore = "subprocess helper for verification_replay_digests_are_process_deterministic"]
fn verification_replay_digest_helper() {
    if std::env::var(DIGEST_HELPER_ENV).ok().as_deref() != Some("1") {
        return;
    }
    let stream = replay_digest_stream(verification_recording());
    println!(
        "{}{}",
        DIGEST_STREAM_PREFIX,
        serde_json::to_string(&stream).unwrap()
    );
}

#[test]
fn rng_entropy_sources_are_limited_to_documented_engine_sites() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_dir = manifest_dir.join("src");
    let mut files = Vec::new();
    collect_rust_files(&src_dir, &mut files);

    let allowed = [
        ("src/game/setup.rs", "from_entropy"),
        ("src/policies/random.rs", "from_entropy"),
    ];
    let mut offenders = Vec::new();

    for path in files {
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("must read {}: {e}", path.display()));
        for (idx, line) in text.lines().enumerate() {
            for needle in ["from_entropy", "thread_rng"] {
                if !line.contains(needle) {
                    continue;
                }
                let rel = path
                    .strip_prefix(&manifest_dir)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if !allowed
                    .iter()
                    .any(|(file, allowed_needle)| rel == *file && needle == *allowed_needle)
                {
                    offenders.push(format!("{}:{}: {}", rel, idx + 1, line.trim()));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "unexpected entropy/thread RNG sites in engine source:\n{}",
        offenders.join("\n")
    );
}

fn collect_rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("must read {}: {e}", dir.display())) {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}
