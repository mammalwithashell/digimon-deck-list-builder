//! Self-play driver integration test (add-determinized-search task 3.1).
//!
//! Runs a tiny real self-play generation — starter decks, UniformEvaluator,
//! minimal search budget — and validates the shard contract the Python
//! trainer depends on (design D9/D10):
//!   - games play to completion (or are honestly counted as truncated);
//!   - shard row/offset bookkeeping is consistent across all arrays;
//!   - every π target puts mass on >1 action (forced decisions must NOT
//!     be recorded) and only on actions legal-looking ids (< action_dim);
//!   - z ∈ {-1, 0, +1}, with 0 only for truncated games;
//!   - obs width matches the configured observation profile;
//!   - the manifest agrees with the arrays.
//!
//! Budget note: this is a plumbing test, not a strength test — a handful
//! of simulations per decision keeps debug-build wall-clock sane.
//!
//! Run: RUST_MIN_STACK=268435456 cargo test --test selfplay

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::search::{DeterminizedConfig, UniformEvaluator};
use digimon_engine::selfplay::{npy, run_self_play, SelfPlayConfig};
mod opponents;

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

fn load_card_data() -> HashMap<String, CardData> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cards.json");
    let text = std::fs::read_to_string(path).expect("read data/cards.json from repo root");
    CardData::load_from_str(&text).expect("parse cards.json")
}

#[test]
fn self_play_generation_writes_consistent_shards() {
    // Registry construction recurses deeply — big-stack thread (same
    // mitigation as clone_fuzz / the CLI main).
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(run_test)
        .expect("spawn")
        .join()
        .expect("test thread panicked");
}

fn run_test() {
    let card_data = load_card_data();
    let owned = |d: &[&str]| -> Vec<String> { d.iter().map(|s| s.to_string()).collect() };
    let decks = vec![owned(D1), owned(D2)];
    let names = vec!["ST-1".to_string(), "ST-5".to_string()];

    let out_dir =
        std::env::temp_dir().join(format!("digimon_selfplay_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out_dir);

    let mut config = SelfPlayConfig {
        policy_only: false,
        belief_prior: None,
        games: 2,
        game_start: 0,
        seed: 7,
        temperature_decisions: 6,
        decision_cap: 600,
        // Force a mid-run flush so the multi-shard path is exercised.
        shard_rows: 8,
        evaluation_batch_size: 2,
        reduced_simulations: None,
        full_search_fraction: 1.0,
        cheap_simulations: 64,
        search: DeterminizedConfig::default(),
    };
    config.search.worlds = 2;
    config.search.search.simulations = 6;

    let mut evaluator = UniformEvaluator;
    let stats = run_self_play(
        &card_data,
        &decks,
        &names,
        &mut evaluator,
        &config,
        &out_dir,
        "uniform",
    )
    .expect("self-play run failed");

    eprintln!(
        "selfplay test: {} games, {} rows, {} shards, {} truncated, wins {:?}, {} decisions ({} forced)",
        stats.games,
        stats.rows,
        stats.shards,
        stats.truncated_games,
        stats.wins,
        stats.decisions_total,
        stats.forced_decisions_total
    );

    assert_eq!(stats.games, 2);
    assert!(stats.rows > 0, "no records emitted");
    assert!(stats.shards >= 1);
    // With a 600-decision cap and playable starter decks, random-ish play
    // finishes; a truncation here means the driver or cap is broken.
    assert_eq!(stats.truncated_games, 0, "starter games must complete");
    assert_eq!(
        stats.wins[0] + stats.wins[1],
        2,
        "every completed game needs a winner"
    );

    // ── Manifest ↔ arrays consistency ──
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(out_dir.join("manifest.json")).expect("manifest.json"),
    )
    .expect("manifest parses");
    assert_eq!(manifest["schema"], "selfplay-shards-v1");
    let obs_dim = manifest["obs_dim"].as_u64().expect("obs_dim") as usize;
    let action_dim = manifest["action_dim"].as_u64().expect("action_dim") as usize;
    assert_eq!(action_dim, 2192, "action space contract");
    let shards = manifest["shards"].as_array().expect("shards array");
    assert_eq!(shards.len(), stats.shards);

    let mut rows_seen = 0usize;
    for shard in shards {
        let dir = out_dir.join(shard["dir"].as_str().expect("shard dir"));
        let rows = shard["rows"].as_u64().expect("shard rows") as usize;

        let obs = npy::read(&dir.join("obs.npy")).expect("obs.npy");
        assert_eq!(obs.shape, vec![rows, obs_dim], "obs shape");

        let offsets = npy::read(&dir.join("policy_offsets.npy"))
            .expect("offsets")
            .as_i64()
            .expect("i64");
        assert_eq!(offsets.len(), rows + 1);
        assert_eq!(offsets[0], 0);
        assert!(
            offsets.windows(2).all(|w| w[0] < w[1]),
            "offsets must be strictly increasing — an empty π row means a \
             forced decision was recorded"
        );

        let actions = npy::read(&dir.join("policy_actions.npy"))
            .expect("actions")
            .as_i32()
            .expect("i32");
        let visits = npy::read(&dir.join("policy_visits.npy"))
            .expect("visits")
            .as_f32()
            .expect("f32");
        assert_eq!(actions.len(), *offsets.last().unwrap() as usize);
        assert_eq!(visits.len(), actions.len());
        assert!(
            actions.iter().all(|&a| a >= 0 && (a as usize) < action_dim),
            "π action ids inside the action space"
        );
        // Per row: >1 candidate action (forced rows are excluded by
        // design) and non-negative visit mass.
        for w in offsets.windows(2) {
            let (s, e) = (w[0] as usize, w[1] as usize);
            assert!(
                e - s >= 2,
                "π row with a single candidate action — forced decisions \
                 must not be recorded"
            );
            assert!(visits[s..e].iter().all(|&v| v >= 0.0));
        }

        let z = npy::read(&dir.join("z.npy"))
            .expect("z")
            .as_f32()
            .expect("f32");
        assert_eq!(z.len(), rows);
        assert!(
            z.iter().all(|&v| v == 1.0 || v == -1.0),
            "no truncation in this run, so every z must be ±1"
        );

        let mover = npy::read(&dir.join("mover.npy"))
            .expect("mover")
            .as_u8()
            .expect("u8");
        assert!(mover.iter().all(|&m| m < 2));

        let game_id = npy::read(&dir.join("game_id.npy"))
            .expect("game_id")
            .as_i32()
            .expect("i32");
        assert!(game_id.iter().all(|&g| g >= 0 && g < 2));

        // Default budget policy: every non-forced decision is a full search,
        // so every row trains the policy head.
        let policy_weight = npy::read(&dir.join("policy_weight.npy"))
            .expect("policy_weight")
            .as_f32()
            .expect("f32");
        assert_eq!(policy_weight.len(), rows);
        assert!(
            policy_weight.iter().all(|&w| w == 1.0),
            "default config must mark every row as a policy target"
        );

        rows_seen += rows;
    }
    assert_eq!(rows_seen, stats.rows, "manifest rows match arrays");

    // Both seats must contribute records across the run (the two-seat
    // pipeline is the whole point — design D9).
    let mut movers_all = Vec::new();
    for shard in shards {
        let dir = out_dir.join(shard["dir"].as_str().unwrap());
        movers_all.extend(npy::read(&dir.join("mover.npy")).unwrap().as_u8().unwrap());
    }
    assert!(
        movers_all.contains(&0) && movers_all.contains(&1),
        "records must come from BOTH seats"
    );

    // Each original shard is a complete game (shard_rows=8). Re-running
    // global game 1 independently must reproduce all bytes. No worker
    // assignment may change decks, seeds or targets.
    config.games = 1;
    config.game_start = 1;
    let chunk_dir = out_dir.join("chunk-one");
    run_self_play(
        &card_data,
        &decks,
        &names,
        &mut evaluator,
        &config,
        &chunk_dir,
        "uniform",
    )
    .unwrap();
    for name in [
        "obs",
        "policy_offsets",
        "policy_actions",
        "policy_visits",
        "policy_weight",
        "z",
        "terminated",
        "mover",
        "game_id",
    ] {
        assert_eq!(
            std::fs::read(out_dir.join("shard-001").join(format!("{name}.npy"))).unwrap(),
            std::fs::read(chunk_dir.join("shard-000").join(format!("{name}.npy"))).unwrap(),
            "global game IDs must preserve serial output: {name}"
        );
    }
    let _ = std::fs::remove_dir_all(&out_dir);
}

/// Search-budget experiments: phase-dependent budgets and playout-cap
/// randomization must be recorded honestly — cheap (value-only) rows carry
/// `policy_weight` 0, full searches carry 1, the manifest reports the
/// per-phase search mix, and the cheap/full assignment is a deterministic
/// function of the master seed and global game ID.
#[test]
fn budget_policy_marks_cheap_rows_and_reports_phase_mix() {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(run_budget_test)
        .expect("spawn")
        .join()
        .expect("test thread panicked");
}

fn run_budget_test() {
    let card_data = load_card_data();
    let owned = |d: &[&str]| -> Vec<String> { d.iter().map(|s| s.to_string()).collect() };
    let decks = vec![owned(D1), owned(D2)];
    let names = vec!["ST-1".to_string(), "ST-5".to_string()];

    let out_dir =
        std::env::temp_dir().join(format!("digimon_selfplay_budget_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out_dir);

    let mut config = SelfPlayConfig {
        policy_only: false,
        belief_prior: None,
        games: 2,
        game_start: 0,
        seed: 11,
        temperature_decisions: 6,
        decision_cap: 600,
        shard_rows: 4096,
        evaluation_batch_size: 2,
        // Non-Main roots get at most 4 of the 6 simulations; half of all
        // non-forced decisions are cheap 2-simulation value-only searches.
        reduced_simulations: Some(4),
        full_search_fraction: 0.5,
        cheap_simulations: 2,
        search: DeterminizedConfig::default(),
    };
    config.search.worlds = 2;
    config.search.search.simulations = 6;

    let mut evaluator = UniformEvaluator;
    let stats = run_self_play(
        &card_data,
        &decks,
        &names,
        &mut evaluator,
        &config,
        &out_dir,
        "uniform",
    )
    .expect("self-play run failed");
    assert_eq!(stats.games, 2);
    assert_eq!(stats.truncated_games, 0, "starter games must complete");
    assert_eq!(
        stats.full_search_decisions + stats.cheap_search_decisions,
        stats.rows as u64,
        "every recorded row is either a full or a cheap search"
    );
    assert!(stats.full_search_decisions > 0 && stats.cheap_search_decisions > 0);
    assert!(stats.reduced_budget_decisions > 0, "games always contain non-Main roots");

    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(out_dir.join("manifest.json")).expect("manifest.json"),
    )
    .expect("manifest parses");
    assert_eq!(manifest["search"]["reduced_simulations"], 4);
    assert_eq!(manifest["search"]["cheap_simulations"], 2);
    assert!((manifest["search"]["full_search_fraction"].as_f64().unwrap() - 0.5).abs() < 1e-6);
    assert_eq!(manifest["full_search_decisions"], stats.full_search_decisions);
    assert_eq!(manifest["cheap_search_decisions"], stats.cheap_search_decisions);
    assert_eq!(manifest["reduced_budget_decisions"], stats.reduced_budget_decisions);

    let phase_stats = manifest["phase_stats"].as_object().expect("phase_stats object");
    assert!(phase_stats.contains_key("Main"), "Main roots are always searched");
    let searched_total: u64 = phase_stats
        .values()
        .map(|v| v["searched"].as_u64().expect("searched"))
        .sum();
    assert_eq!(searched_total, stats.rows as u64, "phase mix covers every searched decision");
    for stat in phase_stats.values() {
        assert!(stat["seconds"].as_f64().expect("seconds") >= 0.0);
        assert!(stat["simulations"].as_u64().expect("simulations") > 0);
    }

    let mut rows_seen = 0usize;
    let mut weights_all: Vec<f32> = Vec::new();
    for shard in manifest["shards"].as_array().expect("shards") {
        let dir = out_dir.join(shard["dir"].as_str().unwrap());
        let rows = shard["rows"].as_u64().unwrap() as usize;
        let weight = npy::read(&dir.join("policy_weight.npy"))
            .expect("policy_weight")
            .as_f32()
            .expect("f32");
        assert_eq!(weight.len(), rows);
        let visits = npy::read(&dir.join("policy_visits.npy"))
            .expect("visits")
            .as_f32()
            .expect("f32");
        let offsets = npy::read(&dir.join("policy_offsets.npy"))
            .expect("offsets")
            .as_i64()
            .expect("i64");
        for (row, w) in offsets.windows(2).zip(weight.iter()) {
            let (s, e) = (row[0] as usize, row[1] as usize);
            let total: f32 = visits[s..e].iter().sum();
            assert!(*w == 0.0 || *w == 1.0, "policy_weight is a 0/1 flag");
            // Cheap searches still record their (smaller) visit mass so the
            // row remains a valid value target with a legality mask.
            assert!(total > 0.0, "every recorded row keeps a non-empty visit mass");
            if *w == 0.0 {
                assert!(total <= 2.0, "cheap rows must not exceed cheap_simulations");
            }
        }
        weights_all.extend_from_slice(&weight);
        rows_seen += rows;
    }
    assert_eq!(rows_seen, stats.rows);
    assert!(weights_all.contains(&0.0) && weights_all.contains(&1.0));

    // The cheap/full assignment derives from (seed, global game ID) only:
    // regenerating game 1 alone reproduces the whole shard byte-for-byte.
    config.games = 1;
    config.game_start = 1;
    let chunk_dir = out_dir.join("chunk-one");
    run_self_play(
        &card_data,
        &decks,
        &names,
        &mut evaluator,
        &config,
        &chunk_dir,
        "uniform",
    )
    .unwrap();
    let game_id = npy::read(&out_dir.join("shard-000").join("game_id.npy"))
        .unwrap()
        .as_i32()
        .unwrap();
    let first_row_of_game_one = game_id.iter().position(|&g| g == 1).expect("game 1 rows");
    let full_weights = npy::read(&out_dir.join("shard-000").join("policy_weight.npy"))
        .unwrap()
        .as_f32()
        .unwrap();
    let chunk_weights = npy::read(&chunk_dir.join("shard-000").join("policy_weight.npy"))
        .unwrap()
        .as_f32()
        .unwrap();
    assert_eq!(&full_weights[first_row_of_game_one..], &chunk_weights[..]);
    let _ = std::fs::remove_dir_all(&out_dir);
}
