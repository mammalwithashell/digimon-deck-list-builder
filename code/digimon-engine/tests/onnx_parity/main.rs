use std::path::PathBuf;

use digimon_engine::inference::{load_policy, BatchedPolicyValueEvaluator};
use serde::Deserialize;

#[path = "../search/support.rs"]
mod search_support;

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
struct MlpValueExpected {
    batch_obs: Vec<Vec<f32>>,
    masks: Vec<Vec<f32>>,
    logits: Vec<Vec<f32>>,
    values: Vec<f32>,
}

#[derive(Deserialize)]
struct Expected {
    shapes: Shapes,
    obs: Vec<f32>,
    mask: Vec<f32>,
    mlp: MlpExpected,
    lstm: LstmExpected,
    mlp_value: MlpValueExpected,
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
fn batched_evaluator_parity_with_python() {
    let expected = load_expected();
    let mv = &expected.mlp_value;
    let mut evaluator =
        BatchedPolicyValueEvaluator::load(&fixtures_dir().join("mlp_tiny_value.onnx"))
            .expect("load value-head MLP");

    let obs_refs: Vec<&[f32]> = mv.batch_obs.iter().map(|r| r.as_slice()).collect();
    let mask_refs: Vec<&[f32]> = mv.masks.iter().map(|r| r.as_slice()).collect();
    let results = evaluator
        .evaluate_batch(&obs_refs, &mask_refs)
        .expect("evaluate batch");
    assert_eq!(results.len(), mv.batch_obs.len());

    for (i, res) in results.iter().enumerate() {
        // Value parity with the Python baseline.
        assert!(
            (res.value - mv.values[i]).abs() < 1e-5,
            "row {i}: value {} != python {}",
            res.value,
            mv.values[i]
        );
        // Policy respects the mask exactly and normalizes over legal actions.
        let mut sum = 0.0f32;
        for (a, (&p, &m)) in res.policy.iter().zip(mv.masks[i].iter()).enumerate() {
            if m < 0.5 {
                assert_eq!(p, 0.0, "row {i}: illegal action {a} has mass {p}");
            }
            sum += p;
        }
        assert!((sum - 1.0).abs() < 1e-5, "row {i}: policy sums to {sum}");
        // Policy argmax agrees with the masked argmax of the Python logits.
        let rust_best = res
            .policy
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        let py_best = mv.logits[i]
            .iter()
            .zip(mv.masks[i].iter())
            .enumerate()
            .map(|(a, (&l, &m))| (a, if m < 0.5 { f32::NEG_INFINITY } else { l }))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap()
            .0;
        assert_eq!(rust_best, py_best, "row {i}: policy argmax diverged");
    }

    // Rows differ (row 2 is a scaled copy of row 1) — catches layout bugs.
    assert_ne!(results[0].value, results[1].value);
}

#[test]
fn batched_evaluator_rejects_policy_only_graph() {
    // The plain MLP fixture has no `value` output — the evaluator must
    // refuse it rather than silently returning garbage values.
    let err = BatchedPolicyValueEvaluator::load(&fixtures_dir().join("mlp_tiny.onnx"));
    assert!(err.is_err());
}

#[test]
fn search_evaluator_honors_selected_profile_and_rejects_mismatches() {
    use digimon_engine::observation::{observation_layout, ObservationProfileId};
    let lite = ObservationProfileId::StandardLiteV2;
    let path = fixtures_dir().join("mlp_lite_value.onnx");
    let mut evaluator = BatchedPolicyValueEvaluator::load_for_profile(&path, lite)
        .expect("non-default profile model must load");
    assert!(
        evaluator.require_terminal_value().is_err(),
        "a PPO-shaped value export must not silently become a terminal search critic"
    );
    let obs = vec![0.0; observation_layout(lite).tensor_size];
    let mask = vec![1.0; digimon_engine::action::ACTION_SPACE_SIZE];
    assert_eq!(
        evaluator.evaluate_batch(&[&obs], &[&mask]).unwrap().len(),
        1
    );
    assert!(evaluator.evaluate_batch(&[&obs[..8]], &[&mask]).is_err());
    assert!(BatchedPolicyValueEvaluator::load(&path).is_err());
    assert!(BatchedPolicyValueEvaluator::load_for_profile(
        &fixtures_dir().join("mlp_tiny_value.onnx"),
        lite
    )
    .is_err());
}

#[test]
fn mlp_hidden_size_matches_fixture() {
    let expected = load_expected();
    assert_eq!(expected.shapes.hidden_size, 4);
}

#[test]
fn independent_tree_batching_matches_serial_onnx_search() {
    use digimon_engine::card_registry::CardRegistry;
    use digimon_engine::observation::ObservationProfileId;
    use digimon_engine::runners::HeadlessRunner;
    use digimon_engine::search::{Mcts, SearchConfig};
    search_support::run_big_stack(|| {
        let data = search_support::load_card_data();
        let registry = CardRegistry::from_cards(&data);
        let owned = |d: &[&str]| d.iter().map(|s| s.to_string()).collect();
        let roots: Vec<_> = [7, 11, 21]
            .into_iter()
            .map(|seed| {
                HeadlessRunner::new(
                    owned(search_support::ST1_DECK),
                    owned(search_support::ST5_DECK),
                    &data,
                    false,
                    false,
                    false,
                    Some(seed),
                )
                .unwrap()
                .game
            })
            .collect();
        let profile = ObservationProfileId::StandardLiteV2;
        let configs: Vec<_> = [19, 41, 63]
            .into_iter()
            .map(|seed| SearchConfig {
                simulations: 32,
                seed,
                observation_profile: profile,
                ..Default::default()
            })
            .collect();
        let path = fixtures_dir().join("mlp_lite_value.onnx");
        let mut serial = BatchedPolicyValueEvaluator::load_for_profile(&path, profile).unwrap();
        let mut batched = BatchedPolicyValueEvaluator::load_for_profile(&path, profile).unwrap();
        let mcts = Mcts::new(&registry);
        let expected: Vec<_> = roots
            .iter()
            .zip(&configs)
            .map(|(r, c)| mcts.search(r, &mut serial, c))
            .collect();
        assert_eq!(mcts.search_batch(&roots, &mut batched, &configs), expected);
    });
}
