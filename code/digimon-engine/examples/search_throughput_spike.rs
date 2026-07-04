//! Search-throughput spike (add-determinized-search task 0.1).
//!
//! Prices the engine-side cost of determinized MCTS before any search
//! exists: raw step throughput, `Game::clone` cost vs game depth,
//! reset-and-replay cost (the fork alternative), and a derived
//! sims-per-second upper bound (clone + rollout steps, NN eval excluded).
//! Run from repo root:
//!   cargo run --release --example search_throughput_spike --features dsl-yaml-loader -- 40000
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

/// Simple deterministic LCG so the random-legal policy needs no extra deps.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn pick(&mut self, mask: &[f32]) -> u16 {
        let legal: Vec<u16> = mask
            .iter()
            .enumerate()
            .filter(|(_, &v)| v > 0.5)
            .map(|(i, _)| i as u16)
            .collect();
        legal[(self.next() as usize) % legal.len()]
    }
}

struct DepthBucket {
    label: &'static str,
    lo: u64,
    hi: u64,
    clone_ns: f64,
    clone_n: u64,
    replay_ns: f64,
    replay_n: u64,
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

    // ---- Pass 1: greedy playouts — step throughput + clone/replay vs depth.
    let mut seed = 1u64;
    let mut runner = build(&d1, &d2, &card_data, seed);
    let mut history: Vec<u16> = Vec::new();
    let (mut steps, mut games) = (0u64, 1u64);
    let mut step_t = 0f64;
    let mut buckets = [
        DepthBucket { label: "early (<50 steps)", lo: 0, hi: 50, clone_ns: 0.0, clone_n: 0, replay_ns: 0.0, replay_n: 0 },
        DepthBucket { label: "mid   (50-150)    ", lo: 50, hi: 150, clone_ns: 0.0, clone_n: 0, replay_ns: 0.0, replay_n: 0 },
        DepthBucket { label: "late  (150+)      ", lo: 150, hi: u64::MAX, clone_ns: 0.0, clone_n: 0, replay_ns: 0.0, replay_n: 0 },
    ];

    let wall = Instant::now();
    while steps < target {
        if runner.game.game_over {
            seed += 1;
            games += 1;
            runner = build(&d1, &d2, &card_data, seed);
            history.clear();
        }
        let depth = history.len() as u64;

        // Sample fork costs every 10 steps so timing overhead stays small.
        if depth % 10 == 0 {
            let b = buckets
                .iter_mut()
                .find(|b| depth >= b.lo && depth < b.hi)
                .unwrap();
            let t = Instant::now();
            let forked = runner.game.clone();
            b.clone_ns += t.elapsed().as_nanos() as f64;
            b.clone_n += 1;
            std::hint::black_box(&forked);
            drop(forked);

            // Reset-and-replay to the same depth (the no-clone alternative):
            // rebuild with the same seed, re-apply the recorded actions.
            let t = Instant::now();
            let mut replayed = build(&d1, &d2, &card_data, seed);
            for &a in &history {
                replayed.step(a);
            }
            b.replay_ns += t.elapsed().as_nanos() as f64;
            b.replay_n += 1;
            std::hint::black_box(&replayed);
        }

        let mask = runner.get_action_mask();
        let a = greedy_action(&runner.game, &mask);
        let t = Instant::now();
        runner.step(a);
        step_t += t.elapsed().as_secs_f64();
        history.push(a);
        steps += 1;
    }
    let total = wall.elapsed().as_secs_f64();

    // ---- Pass 2: random-legal playouts — the rollout-shaped step cost
    // (greedy is the decision policy; search rollouts look more random).
    let mut lcg = Lcg(0x9E3779B97F4A7C15);
    let mut rseed = 1000u64;
    let mut rrunner = build(&d1, &d2, &card_data, rseed);
    let mut rhistory: Vec<u16> = Vec::new();
    let (mut rsteps, mut rgames) = (0u64, 1u64);
    let mut rstep_t = 0f64;
    let rtarget = target / 2;
    while rsteps < rtarget {
        if rrunner.game.game_over {
            rseed += 1;
            rgames += 1;
            rrunner = build(&d1, &d2, &card_data, rseed);
            rhistory.clear();
        }
        let depth = rhistory.len() as u64;
        // Random games run deeper than greedy ones — sample fork costs here
        // too so the mid/late depth buckets are populated.
        if depth % 10 == 0 {
            let b = buckets
                .iter_mut()
                .find(|b| depth >= b.lo && depth < b.hi)
                .unwrap();
            let t = Instant::now();
            let forked = rrunner.game.clone();
            b.clone_ns += t.elapsed().as_nanos() as f64;
            b.clone_n += 1;
            std::hint::black_box(&forked);
            drop(forked);

            let t = Instant::now();
            let mut replayed = build(&d1, &d2, &card_data, rseed);
            for &a in &rhistory {
                replayed.step(a);
            }
            b.replay_ns += t.elapsed().as_nanos() as f64;
            b.replay_n += 1;
            std::hint::black_box(&replayed);
        }
        let t = Instant::now();
        let mask = rrunner.get_action_mask();
        let a = lcg.pick(&mask);
        rrunner.step(a);
        rstep_t += t.elapsed().as_secs_f64();
        rhistory.push(a);
        rsteps += 1;
    }
    let rtotal = rstep_t;

    // ---- Report.
    let step_us = step_t * 1e6 / steps as f64;
    let rstep_us = rtotal * 1e6 / rsteps as f64; // includes mask build (rollouts need it)
    eprintln!("=== search_throughput_spike: {steps} greedy steps / {games} games / {total:.1}s ===");
    eprintln!("greedy playout : {:.1} steps/game, engine step {:.1} us => {:.0} steps/sec (step only)",
        steps as f64 / games as f64, step_us, 1e6 / step_us);
    eprintln!("random playout : {:.1} steps/game, {:.1} us/step incl. mask => {:.0} steps/sec",
        rsteps as f64 / rgames as f64, rstep_us, 1e6 / rstep_us);
    eprintln!("--- fork cost by depth (sampled every 10 steps) ---");
    for b in &buckets {
        if b.clone_n == 0 {
            continue;
        }
        let clone_us = b.clone_ns / b.clone_n as f64 / 1000.0;
        let replay_us = b.replay_ns / b.replay_n as f64 / 1000.0;
        eprintln!(
            "{}: clone {:9.1} us (n={:4})   reset-replay {:10.1} us (n={:4})   ratio {:6.0}x",
            b.label, clone_us, b.clone_n, replay_us, b.replay_n, replay_us / clone_us
        );
    }
    // MCTS sim = one clone + step to a leaf (~1 step for tree reuse,
    // more for rollout-style). Report both bounds, NN eval excluded.
    let mid = &buckets[1];
    if mid.clone_n > 0 {
        let clone_us = mid.clone_ns / mid.clone_n as f64 / 1000.0;
        let sim1 = clone_us + rstep_us;
        let sim10 = clone_us + 10.0 * rstep_us;
        eprintln!("--- derived (mid-game, NN eval EXCLUDED) ---");
        eprintln!("sim = clone + 1 step : {:.1} us => {:.0} sims/sec/core", sim1, 1e6 / sim1);
        eprintln!("sim = clone + 10 steps: {:.1} us => {:.0} sims/sec/core", sim10, 1e6 / sim10);
    }
}
