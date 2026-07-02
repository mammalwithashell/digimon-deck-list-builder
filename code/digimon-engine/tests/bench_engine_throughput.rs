//! Bare-engine throughput benchmark + per-phase breakdown (no Python/PyO3/NN).
//! Plays full ST-1 vs ST-1 games and reports games/sec, steps/sec, and where the
//! per-step time goes (mask-build vs engine-step vs policy), greedy AND random.
//! Run: `cargo test --release --test bench_engine_throughput -- --nocapture`.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use digimon_engine::card_data::CardData;
use digimon_engine::policies::GreedyPolicy;
use digimon_engine::{HeadlessRunner, Policy};

const ST1: &[&str] = &[
    "ST1-01", "ST1-01", "ST1-01", "ST1-01", "ST1-02", "ST1-02", "ST1-02", "ST1-02", "ST1-03",
    "ST1-03", "ST1-03", "ST1-03", "ST1-04", "ST1-04", "ST1-04", "ST1-04", "ST1-05", "ST1-05",
    "ST1-05", "ST1-05", "ST1-06", "ST1-06", "ST1-06", "ST1-06", "ST1-07", "ST1-07", "ST1-08",
    "ST1-08", "ST1-08", "ST1-08", "ST1-09", "ST1-09", "ST1-09", "ST1-09", "ST1-10", "ST1-10",
    "ST1-11", "ST1-11", "ST1-12", "ST1-12", "ST1-12", "ST1-12", "ST1-13", "ST1-13", "ST1-13",
    "ST1-13", "ST1-14", "ST1-14", "ST1-14", "ST1-14", "ST1-15", "ST1-15", "ST1-16", "ST1-16",
];

const NUM_GAMES: u64 = 200;
const MAX_STEPS: u32 = 2000;

fn load_db() -> HashMap<String, CardData> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cards.json");
    let json = std::fs::read_to_string(path).expect("read cards.json");
    CardData::load_from_str(&json).expect("parse cards.json")
}

fn deck() -> Vec<String> {
    ST1.iter().map(|s| s.to_string()).collect()
}

#[derive(Default)]
struct Phase {
    construct: Duration,
    mask: Duration,
    policy: Duration,
    step: Duration,
    steps: u64,
}

fn xorshift(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

/// Run NUM_GAMES with a per-phase-timed manual loop. `use_greedy=false` picks a
/// uniform-random legal action instead (isolates the engine step from policy cost).
fn run(db: &HashMap<String, CardData>, use_greedy: bool) -> Phase {
    let mut p = Phase::default();
    let mut rng: u64 = 0x9E3779B97F4A7C15;
    for seed in 0..NUM_GAMES {
        let tc = Instant::now();
        let mut runner =
            HeadlessRunner::new(deck(), deck(), db, false, false, false, Some(seed + 1)).unwrap();
        p.construct += tc.elapsed();
        let mut greedy = GreedyPolicy::new();
        let mut steps = 0u32;
        while !runner.is_game_over() && steps < MAX_STEPS {
            let tm = Instant::now();
            let mask = runner.get_action_mask();
            p.mask += tm.elapsed();

            let tp = Instant::now();
            let action = if use_greedy {
                greedy.select(runner.game(), &mask)
            } else {
                let legal: Vec<u16> = mask
                    .iter()
                    .enumerate()
                    .filter(|(_, &v)| v > 0.5)
                    .map(|(i, _)| i as u16)
                    .collect();
                if legal.is_empty() {
                    0
                } else {
                    legal[(xorshift(&mut rng) as usize) % legal.len()]
                }
            };
            p.policy += tp.elapsed();

            let ts = Instant::now();
            runner.step(action);
            p.step += ts.elapsed();
            steps += 1;
        }
        p.steps += steps as u64;
    }
    p
}

fn report(label: &str, p: &Phase) {
    let total = p.construct + p.mask + p.policy + p.step;
    let secs = total.as_secs_f64();
    let pct = |d: Duration| 100.0 * d.as_secs_f64() / secs;
    let us_per_step = |d: Duration| d.as_secs_f64() * 1e6 / p.steps as f64;
    eprintln!("==== {label} ====");
    eprintln!(
        "  {} games, {} steps ({:.1}/game) in {:.2}s -> {:.1} games/sec, {:.0} steps/sec",
        NUM_GAMES,
        p.steps,
        p.steps as f64 / NUM_GAMES as f64,
        secs,
        NUM_GAMES as f64 / secs,
        p.steps as f64 / secs,
    );
    eprintln!(
        "  construct : {:>5.1}%  ({:.1} us/step-equiv)",
        pct(p.construct),
        us_per_step(p.construct)
    );
    eprintln!(
        "  mask build: {:>5.1}%  ({:.1} us/step)",
        pct(p.mask),
        us_per_step(p.mask)
    );
    eprintln!(
        "  policy    : {:>5.1}%  ({:.1} us/step)",
        pct(p.policy),
        us_per_step(p.policy)
    );
    eprintln!(
        "  engine step:{:>5.1}%  ({:.1} us/step)  <-- effect resolution",
        pct(p.step),
        us_per_step(p.step)
    );
}

#[test]
fn bench_bare_engine_throughput() {
    let db = load_db();
    // warm up
    let _ = run(&db, true);

    let greedy = run(&db, true);
    let random = run(&db, false);

    eprintln!("\n######## BARE-ENGINE THROUGHPUT + PER-PHASE BREAKDOWN ########");
    report("GREEDY self-play (ST-1, real cards.json, release)", &greedy);
    report(
        "RANDOM self-play (isolates engine step from policy)",
        &random,
    );
    eprintln!("#############################################################\n");
}
