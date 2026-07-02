//! Profiling harness: real-deck greedy game in pure Rust. Times the four
//! per-iteration components (construction, mask build, greedy, engine step)
//! separately so we can see the top-level breakdown without a sampling
//! profiler. Run from repo root:
//!   cargo run --release --example step_profile --features dsl-yaml-loader -- 40000
use digimon_engine::card_data::CardData;
use digimon_engine::policies::greedy_action;
use digimon_engine::runners::HeadlessRunner;
use std::time::Instant;

const D1: &[&str] = &[
    "ST1-01", "ST1-01", "ST1-01", "ST1-01", "ST1-02", "ST1-02", "ST1-02", "ST1-02", "ST1-03",
    "ST1-03", "ST1-03", "ST1-03", "ST1-04", "ST1-04", "ST1-04", "ST1-04", "ST1-05", "ST1-05",
    "ST1-05", "ST1-05", "ST1-06", "ST1-06", "ST1-06", "ST1-06", "ST1-07", "ST1-07", "ST1-07",
    "ST1-07", "ST1-08", "ST1-08", "ST1-09", "ST1-09", "ST1-09", "ST1-09", "ST1-10", "ST1-10",
    "ST1-10", "ST1-10", "ST1-11", "ST1-11", "ST1-12", "ST1-12", "ST1-13", "ST1-13", "ST1-14",
    "ST1-14", "ST1-14", "ST1-14", "ST1-15", "ST1-15", "ST1-16", "ST1-16", "ST1-16", "ST1-16",
];
const D2: &[&str] = &[
    "ST5-01", "ST5-01", "ST5-01", "ST5-01", "ST5-02", "ST5-02", "ST5-02", "ST5-02", "ST5-03",
    "ST5-03", "ST5-03", "ST5-03", "ST5-04", "ST5-04", "ST5-04", "ST5-04", "ST5-05", "ST5-05",
    "ST5-05", "ST5-05", "ST5-06", "ST5-06", "ST5-06", "ST5-06", "ST5-07", "ST5-07", "ST5-07",
    "ST5-07", "ST5-08", "ST5-08", "ST5-09", "ST5-09", "ST5-09", "ST5-09", "ST5-10", "ST5-10",
    "ST5-10", "ST5-10", "ST5-11", "ST5-11", "ST5-12", "ST5-12", "ST5-13", "ST5-13", "ST5-14",
    "ST5-14", "ST5-14", "ST5-14", "ST5-15", "ST5-15",
];

fn build(
    d1: &[String],
    d2: &[String],
    cd: &std::collections::HashMap<String, CardData>,
    seed: u64,
) -> HeadlessRunner {
    HeadlessRunner::new(
        d1.to_vec(),
        d2.to_vec(),
        cd,
        false,
        false,
        false,
        Some(seed),
    )
    .expect("build")
}

fn main() {
    let target: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(40_000);
    let text = std::fs::read_to_string("data/cards.json")
        .expect("run from repo root (needs data/cards.json)");
    let card_data = CardData::load_from_str(&text).expect("parse cards.json");
    let d1: Vec<String> = D1.iter().map(|s| s.to_string()).collect();
    let d2: Vec<String> = D2.iter().map(|s| s.to_string()).collect();

    let mut seed = 1u64;
    let mut runner = build(&d1, &d2, &card_data, seed);
    let (mut steps, mut games) = (0u64, 1u64);
    let (mut ctor_t, mut mask_t, mut greedy_t, mut step_t) = (0f64, 0f64, 0f64, 0f64);

    let wall = Instant::now();
    while steps < target {
        if runner.game.game_over {
            seed += 1;
            games += 1;
            let t = Instant::now();
            runner = build(&d1, &d2, &card_data, seed);
            ctor_t += t.elapsed().as_secs_f64();
        }
        let t = Instant::now();
        let mask = runner.get_action_mask();
        mask_t += t.elapsed().as_secs_f64();
        let t = Instant::now();
        let a = greedy_action(&runner.game, &mask);
        greedy_t += t.elapsed().as_secs_f64();
        let t = Instant::now();
        runner.step(a);
        step_t += t.elapsed().as_secs_f64();
        steps += 1;
    }
    let total = wall.elapsed().as_secs_f64();
    let ms = |x: f64| x * 1000.0;
    eprintln!(
        "=== {} steps / {} games / {:.1}s wall ({:.2} steps/game) ===",
        steps,
        games,
        total,
        steps as f64 / games as f64
    );
    eprintln!(
        "construction: {:8.1} ms ({:4.0}%) = {:.2} ms/game",
        ms(ctor_t),
        100.0 * ctor_t / total,
        ms(ctor_t) / games as f64
    );
    eprintln!(
        "engine step : {:8.1} ms ({:4.0}%) = {:.3} ms/step",
        ms(step_t),
        100.0 * step_t / total,
        ms(step_t) / steps as f64
    );
    eprintln!(
        "mask build  : {:8.1} ms ({:4.0}%) = {:.3} ms/step",
        ms(mask_t),
        100.0 * mask_t / total,
        ms(mask_t) / steps as f64
    );
    eprintln!(
        "greedy      : {:8.1} ms ({:4.0}%) = {:.3} ms/step",
        ms(greedy_t),
        100.0 * greedy_t / total,
        ms(greedy_t) / steps as f64
    );
}
