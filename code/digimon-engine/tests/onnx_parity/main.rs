use std::path::PathBuf;

use digimon_engine::inference::load_policy;
use serde::Deserialize;

#[derive(Deserialize)]
struct Shapes {
    obs_size: usize,
    action_size: usize,
    hidden_size: usize,
}

#[derive(Deserialize)]
struct MlpExpected {
    action: usize,
}

#[derive(Deserialize)]
struct LstmExpected {
    step1_action: usize,
    step2_action: usize,
}

#[derive(Deserialize)]
struct Expected {
    shapes: Shapes,
    obs: Vec<f32>,
    mask: Vec<f32>,
    mlp: MlpExpected,
    lstm: LstmExpected,
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn load_expected() -> Expected {
    let json = std::fs::read_to_string(fixtures_dir().join("expected.json"))
        .expect("read expected.json (run digimon-engine/tests/fixtures/generate_fixtures.py)");
    serde_json::from_str(&json).expect("parse expected.json")
}

#[test]
fn mlp_parity_with_python() {
    let expected = load_expected();
    assert_eq!(expected.obs.len(), expected.shapes.obs_size);
    assert_eq!(expected.mask.len(), expected.shapes.action_size);

    let mut policy = load_policy(&fixtures_dir().join("mlp_tiny.onnx")).expect("load MLP policy");
    let action = policy
        .predict(&expected.obs, &expected.mask)
        .expect("MLP predict");
    assert_eq!(
        action, expected.mlp.action,
        "Rust MLP argmax ({action}) diverged from Python baseline ({})",
        expected.mlp.action
    );
}

#[test]
fn lstm_parity_with_python_and_threads_state() {
    let expected = load_expected();
    let mut policy = load_policy(&fixtures_dir().join("lstm_tiny.onnx")).expect("load LSTM policy");

    let step1 = policy
        .predict(&expected.obs, &expected.mask)
        .expect("LSTM step 1");
    assert_eq!(
        step1, expected.lstm.step1_action,
        "Rust LSTM step-1 argmax ({step1}) diverged from Python baseline ({})",
        expected.lstm.step1_action
    );

    let step2 = policy
        .predict(&expected.obs, &expected.mask)
        .expect("LSTM step 2");
    assert_eq!(
        step2, expected.lstm.step2_action,
        "Rust LSTM step-2 argmax ({step2}) diverged from Python baseline ({}); \
         state threading likely broken",
        expected.lstm.step2_action
    );
}

#[test]
fn lstm_reset_matches_fresh_policy() {
    let expected = load_expected();
    let mut policy = load_policy(&fixtures_dir().join("lstm_tiny.onnx")).expect("load LSTM policy");

    let fresh_action = policy
        .predict(&expected.obs, &expected.mask)
        .expect("fresh LSTM predict");

    let _ = policy
        .predict(&expected.obs, &expected.mask)
        .expect("LSTM step 2 advances state");
    policy.reset();
    let after_reset_action = policy
        .predict(&expected.obs, &expected.mask)
        .expect("LSTM predict after reset");

    assert_eq!(
        fresh_action, after_reset_action,
        "reset() did not return policy to its fresh state"
    );
}

#[test]
fn mlp_hidden_size_matches_fixture() {
    let expected = load_expected();
    assert_eq!(expected.shapes.hidden_size, 4);
}
