//! NEW two-model information-regime evaluator. Qualification is required before scoring.
//! Existing checkpoint_search_eval and frozen campaigns are not modified.
use digimon_engine::{
    card_data::CardData,
    card_registry::CardRegistry,
    determinize::beliefs::{PriorSnapshot, WeightedDeckPrior},
    game::Game,
    inference::BatchedPolicyValueEvaluator,
    observation::ObservationProfileId,
    rules::Rules,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;
#[path = "../../digimon-engine-cli/src/admission.rs"]
mod admission;
#[path = "search_quality/frozen_modes.rs"]
mod modes;
#[path = "search_quality/telemetry.rs"]
mod telemetry;
#[path = "search_quality/request_driver.rs"]
mod request_driver;
use request_driver::num;
const SCHEMA: &str = "frozen-information-regime-eval-v1";

fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn hash(path: impl AsRef<Path>) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}
fn regime(settings: &Value) -> modes::InformationRegime {
    assert_eq!(settings["kind"], "search");
    assert_eq!(num(settings, "simulations"), 320);
    assert_eq!(num(settings, "worlds"), 4);
    assert_eq!(num(settings, "batch"), 4);
    assert_eq!(settings["root_noise"].as_f64(), Some(0.));
    serde_json::from_value(settings["information_regime"].clone())
        .expect("explicit known-list or closed-list regime required")
}
fn run() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 2, "request output");
    let out = Path::new(&args[1]);
    assert!(!out.exists(), "preserve original result");
    let request = read(&args[0]);
    assert_eq!(request["schema"], SCHEMA);
    let plan = read(request["plan"].as_str().unwrap());
    assert_eq!(plan["schema"], SCHEMA);
    let candidate_mode = regime(&request["candidate"]);
    let opponent_mode = regime(&request["opponent"]);
    let runtime_hash = hash(std::env::current_exe().unwrap());
    assert_eq!(runtime_hash, plan["native_sha256"].as_str().unwrap());
    assert_eq!(request["runtime_sha256"], plan["native_sha256"]);
    for (path, key) in [
        ("cards_json", "cards_sha256"),
        ("model", "model_sha256"),
        ("opponent_model", "opponent_model_sha256"),
        ("prior", "prior_file_sha256"),
    ] {
        assert_eq!(
            hash(plan[path].as_str().unwrap()),
            plan[key].as_str().unwrap(),
            "frozen {path} mismatch"
        );
    }
    let data = CardData::load_from_str(
        &std::fs::read_to_string(plan["cards_json"].as_str().unwrap()).unwrap(),
    )
    .unwrap();
    let decks: Vec<Vec<String>> = plan["decks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| serde_json::from_value(d["cards"].clone()).unwrap())
        .collect();
    admission::validate_pool(
        &decks,
        &data,
        Path::new(plan["cards_json"].as_str().unwrap())
            .parent()
            .unwrap(),
    )
    .unwrap();
    let prior = WeightedDeckPrior::new(
        serde_json::from_value::<PriorSnapshot>(read(plan["prior"].as_str().unwrap())).unwrap(),
    )
    .unwrap();
    admission::validate_population(
        &prior.snapshot().population.iter().cloned().collect::<Vec<_>>(),
        &data,
        Path::new(plan["cards_json"].as_str().unwrap()).parent().unwrap(),
    )
    .expect("fallback population must pass this runtime's admission gates");
    assert_eq!(
        prior.candidate_count(),
        21,
        "training prior must contain 21 distinct lists"
    );
    assert_eq!(
        prior.snapshot_hash(),
        plan["prior_snapshot_hash"].as_str().unwrap()
    );
    assert_eq!(request["prior_snapshot_hash"], plan["prior_snapshot_hash"]);
    assert!(
        prior
            .snapshot()
            .population
            .iter()
            .all(|id| data.contains_key(id)),
        "unloaded fallback card"
    );
    let mut candidate = BatchedPolicyValueEvaluator::load_for_profile(
        Path::new(plan["model"].as_str().unwrap()),
        ObservationProfileId::StandardLiteV2,
    )
    .unwrap();
    let mut opponent = BatchedPolicyValueEvaluator::load_for_profile(
        Path::new(plan["opponent_model"].as_str().unwrap()),
        ObservationProfileId::StandardLiteV2,
    )
    .unwrap();
    candidate.require_terminal_value().unwrap();
    opponent.require_terminal_value().unwrap();
    let registry = CardRegistry::from_cards(&data);
    let indices: Vec<usize> = serde_json::from_value(request["deck_indices"].clone()).unwrap();
    assert_eq!(indices.len(), 2);
    let pair: Vec<_> = indices.iter().map(|i| decks[*i].clone()).collect();
    let cap = num(&plan, "decision_cap");
    assert!((1..=20000).contains(&cap));
    request_driver::run_pair(
        out, &request, &plan, &runtime_hash, Some(&prior), &prior.snapshot_hash(),
        &registry, &mut candidate, &mut opponent, candidate_mode, opponent_mode,
        cap, SCHEMA,
        |_| Game::new(&pair, &data, Rules::standard(), Some(num(&request, "game_seed"))).unwrap(),
    );
}
fn main() {
    std::thread::Builder::new()
        .stack_size(268435456)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}
