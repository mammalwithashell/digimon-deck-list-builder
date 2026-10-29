//! Diagnostic only: compare exact-input caching with the same model, seed,
//! simulation budget and decision prefix. Capped games are NOT training data.
//!
//! search_performance <model> <decks> <out> <seed> <games> <decision-cap> <cache-entries> [eval-batch-size]
use digimon_engine::card_data::CardData;
use digimon_engine::inference::BatchedPolicyValueEvaluator;
use digimon_engine::observation::parse_observation_profile;
use digimon_engine::search::{EvalRequest, Evaluation, MemoizedEvaluator, PolicyValueFn};
use digimon_engine::selfplay::{run_self_play, SelfPlayConfig};
use std::path::Path;
use std::time::{Duration, Instant};

#[cfg(windows)]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

struct Timed<E> {
    inner: E,
    calls: u64,
    rows: u64,
    elapsed: Duration,
}
impl<E: PolicyValueFn> PolicyValueFn for Timed<E> {
    fn evaluate_batch(&mut self, requests: &[EvalRequest]) -> Vec<Evaluation> {
        let started = Instant::now();
        let results = self.inner.evaluate_batch(requests);
        self.elapsed += started.elapsed();
        self.calls += 1;
        self.rows += requests.len() as u64;
        results
    }
}

fn main() {
    // Registry construction needs the same stack budget as native self-play.
    std::thread::Builder::new()
        .stack_size(268435456)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}

fn run() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert!((7..=8).contains(&args.len()), "usage: search_performance <model> <decks> <out> <seed> <games> <decision-cap> <cache-entries> [eval-batch-size]");
    let out = Path::new(&args[2]);
    assert!(!out.exists(), "preserve existing diagnostic output");
    let decks_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let decks: Vec<Vec<String>> = decks_json
        .as_array()
        .unwrap()
        .iter()
        .map(|deck| {
            let cards = deck
                .as_array()
                .unwrap_or_else(|| deck["cards"].as_array().unwrap());
            cards
                .iter()
                .map(|id| id.as_str().unwrap().to_owned())
                .collect()
        })
        .collect();
    let data =
        CardData::load_from_str(&std::fs::read_to_string("data/cards.json").unwrap()).unwrap();
    let profile = parse_observation_profile("standard_lite_v2").unwrap();
    let evaluator =
        BatchedPolicyValueEvaluator::load_for_profile(Path::new(&args[0]), profile).unwrap();
    evaluator.require_terminal_value().unwrap();
    let mut evaluator = MemoizedEvaluator::new(
        Timed {
            inner: evaluator,
            calls: 0,
            rows: 0,
            elapsed: Duration::ZERO,
        },
        args[6].parse().unwrap(),
    );
    let mut config = SelfPlayConfig::default();
    config.seed = args[3].parse().unwrap();
    config.games = args[4].parse().unwrap();
    config.decision_cap = args[5].parse().unwrap();
    config.evaluation_batch_size = args.get(7).map(|s| s.parse().unwrap()).unwrap_or(1);
    config.search.worlds = 4;
    config.search.search.simulations = 320;
    config.search.search.observation_profile = profile;
    let started = Instant::now();
    let stats = run_self_play(&data, &decks, &[], &mut evaluator, &config, out, &args[0]).unwrap();
    let elapsed = started.elapsed().as_secs_f64();
    let timed = evaluator.inner();
    let report = serde_json::json!({
        "diagnostic_only": true, "seed": config.seed, "games": stats.games,
        "decision_cap": config.decision_cap, "simulations": 320, "worlds": 4,
        "rows": stats.rows, "decisions": stats.decisions_total, "truncated": stats.truncated_games,
        "elapsed_seconds_including_registry": elapsed, "model_calls": timed.calls,
        "model_rows": timed.rows, "model_seconds": timed.elapsed.as_secs_f64(),
        "cache_entries": evaluator.len(), "cache_capacity": args[6],
        "cache_hits": evaluator.hits(), "cache_misses": evaluator.misses(),
        "evaluation_batch_size": config.evaluation_batch_size,
    });
    std::fs::write(
        out.join("performance.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{report}");
}
