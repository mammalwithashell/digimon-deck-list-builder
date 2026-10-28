use super::*;
use digimon_engine::search::{EvalRequest, Evaluation, PolicyValueFn};
use digimon_engine::selfplay::opponents::{HistoricalOpponents, OpponentEntry, OpponentManifest};
use digimon_engine::selfplay::run_self_play_with_opponents;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

fn manifest(probability: f64) -> OpponentManifest {
    OpponentManifest {
        schema: "search-opponent-pool-v1".into(),
        selfplay_probability: probability,
        opponents: vec![OpponentEntry {
            id: "old".into(),
            path: "old.onnx".into(),
            sha256: "a".repeat(64),
            metadata_sha256: "b".repeat(64),
            weight: 1.0,
        }],
    }
}

#[test]
fn assignments_are_weighted_paired_and_independent_of_chunks() {
    let mut pool = manifest(0.5);
    let mut other = pool.opponents[0].clone();
    other.id = "older".into();
    other.sha256 = "c".repeat(64);
    other.weight = 3.0;
    pool.opponents.push(other);
    pool.validate().unwrap();
    let assigned: Vec<_> = (0..2000).map(|i| pool.assignment(91, i)).collect();
    for i in (0..2000).step_by(2) {
        assert_eq!(assigned[i], assigned[i + 1]);
    }
    assert_eq!(
        assigned[77..121],
        (77..121)
            .map(|i| pool.assignment(91, i))
            .collect::<Vec<_>>()
    );
    let counts = [None, Some(0), Some(1)].map(|x| assigned.iter().filter(|&&a| a == x).count());
    assert!((850..1150).contains(&counts[0]));
    assert!(counts[2] > counts[1] * 2);
}

#[test]
fn invalid_pool_configuration_fails_closed() {
    for p in [f64::NAN, -0.1, 1.1] {
        assert!(manifest(p).validate().is_err());
    }
    let mut pool = manifest(0.0);
    pool.opponents[0].weight = 0.0;
    assert!(pool.validate().is_err());
    pool.opponents[0].weight = 1.0;
    pool.opponents.push(pool.opponents[0].clone());
    assert!(pool.validate().is_err());
    pool.opponents.clear();
    assert!(pool.validate().is_err());
}

#[test]
fn native_pool_loader_rejects_changed_assets_and_current_model_aliases() {
    use digimon_engine::selfplay::opponents::file_sha256;
    let root = std::env::temp_dir().join(format!("history-hashes-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let current = root.join("current.onnx");
    let old = root.join("old.onnx");
    let meta = root.join("old.onnx.meta.json");
    let path = root.join("pool.json");
    std::fs::write(&current, "current").unwrap();
    std::fs::write(&old, "older").unwrap();
    std::fs::write(&meta, "metadata").unwrap();
    let mut pool = manifest(0.5);
    pool.opponents[0].sha256 = file_sha256(&old).unwrap();
    pool.opponents[0].metadata_sha256 = file_sha256(&meta).unwrap();
    std::fs::write(&path, serde_json::to_vec(&pool).unwrap()).unwrap();
    assert_eq!(
        OpponentManifest::load(&path, &current).unwrap().opponents[0].path,
        old
    );
    std::fs::write(&old, "changed").unwrap();
    assert!(OpponentManifest::load(&path, &current)
        .unwrap_err()
        .contains("hash mismatch"));
    std::fs::write(&old, "older").unwrap();
    std::fs::write(&meta, "changed").unwrap();
    assert!(OpponentManifest::load(&path, &current)
        .unwrap_err()
        .contains("hash mismatch"));
    std::fs::write(&current, "older").unwrap();
    assert!(OpponentManifest::load(&path, &current)
        .unwrap_err()
        .contains("aliases"));
}

struct CountingUniform(Arc<AtomicUsize>);
impl PolicyValueFn for CountingUniform {
    fn evaluate_batch(&mut self, requests: &[EvalRequest]) -> Vec<Evaluation> {
        self.0.fetch_add(requests.len(), Ordering::Relaxed);
        UniformEvaluator.evaluate_batch(requests)
    }
}

fn receipt(path: &std::path::Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(path.join("manifest.json")).unwrap()).unwrap()
}

#[test]
fn historical_seats_keep_search_targets_and_filter_only_opponent_rows() {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(|| {
            let data = load_card_data();
            let decks = vec![
                D1.iter().map(|s| s.to_string()).collect(),
                D2.iter().map(|s| s.to_string()).collect(),
            ];
            let root =
                std::env::temp_dir().join(format!("historical-opponents-{}", std::process::id()));
            let mut config = SelfPlayConfig {
                games: 2,
                seed: 31,
                decision_cap: 40,
                shard_rows: 100000,
                evaluation_batch_size: 2,
                ..Default::default()
            };
            config.search.worlds = 2;
            config.search.search.simulations = 6;
            let baseline = root.join("baseline");
            run_self_play(
                &data,
                &decks,
                &[],
                &mut UniformEvaluator,
                &config,
                &baseline,
                "uniform",
            )
            .unwrap();
            let old_calls = Arc::new(AtomicUsize::new(0));
            let current_calls = Arc::new(AtomicUsize::new(0));
            let mut pool = HistoricalOpponents {
                manifest: manifest(0.0),
                evaluators: vec![Box::new(CountingUniform(old_calls.clone()))],
            };
            let history = root.join("history");
            run_self_play_with_opponents(
                &data,
                &decks,
                &[],
                &mut CountingUniform(current_calls.clone()),
                &config,
                &history,
                "uniform",
                Some(&mut pool),
            )
            .unwrap();
            assert!(old_calls.load(Ordering::Relaxed) > 0);
            assert!(current_calls.load(Ordering::Relaxed) > 0);
            let old = receipt(&baseline);
            let new = receipt(&history);
            for key in [
                "wins",
                "decisions_total",
                "forced_decisions_total",
                "game_decks",
                "search",
            ] {
                assert_eq!(old[key], new[key], "{key}");
            }
            let records = new["opponent_games"].as_array().unwrap();
            for (i, r) in records.iter().enumerate() {
                assert_eq!(
                    r["learner_seat"].as_u64().unwrap() ^ r["first_player"].as_u64().unwrap(),
                    i as u64
                );
                let learner = r["learner_seat"].as_u64().unwrap() as usize;
                assert_eq!(r["training_rows_by_seat"][1 - learner], 0);
            }
            let b = baseline.join("shard-000");
            let h = history.join("shard-000");
            let ids = npy::read(&b.join("game_id.npy")).unwrap().as_i32().unwrap();
            let movers = npy::read(&b.join("mover.npy")).unwrap().as_u8().unwrap();
            let keep: Vec<_> = ids
                .iter()
                .zip(&movers)
                .enumerate()
                .filter_map(|(i, (&g, &m))| (records[g as usize]["learner_seat"] == m).then_some(i))
                .collect();
            for file in [
                "obs.npy",
                "z.npy",
                "terminated.npy",
                "mover.npy",
                "game_id.npy",
            ] {
                let source = npy::read(&b.join(file)).unwrap();
                let target = npy::read(&h.join(file)).unwrap();
                let width = source.body.len() / ids.len();
                let expected: Vec<u8> = keep
                    .iter()
                    .flat_map(|&i| source.body[i * width..(i + 1) * width].to_vec())
                    .collect();
                assert_eq!(target.body, expected, "{file}");
            }
            let offsets = npy::read(&b.join("policy_offsets.npy"))
                .unwrap()
                .as_i64()
                .unwrap();
            for file in ["policy_actions.npy", "policy_visits.npy"] {
                let source = npy::read(&b.join(file)).unwrap();
                let expected: Vec<u8> = keep
                    .iter()
                    .flat_map(|&i| {
                        source.body[offsets[i] as usize * 4..offsets[i + 1] as usize * 4].to_vec()
                    })
                    .collect();
                assert_eq!(npy::read(&h.join(file)).unwrap().body, expected, "{file}");
            }
            // Split at an odd boundary: assignments and emitted targets stay identical.
            let mut combined = Vec::new();
            for i in 0..2 {
                config.games = 1;
                config.game_start = i;
                let split = root.join(format!("split-{i}"));
                run_self_play_with_opponents(
                    &data,
                    &decks,
                    &[],
                    &mut UniformEvaluator,
                    &config,
                    &split,
                    "uniform",
                    Some(&mut pool),
                )
                .unwrap();
                assert_eq!(receipt(&split)["opponent_games"][0], records[i as usize]);
                combined.extend(npy::read(&split.join("shard-000/obs.npy")).unwrap().body);
            }
            assert_eq!(combined, npy::read(&h.join("obs.npy")).unwrap().body);
            // Optional pool with 100% current-current must be byte-identical.
            config.games = 2;
            config.game_start = 0;
            pool.manifest.selfplay_probability = 1.0;
            let noop = root.join("noop");
            run_self_play_with_opponents(
                &data,
                &decks,
                &[],
                &mut UniformEvaluator,
                &config,
                &noop,
                "uniform",
                Some(&mut pool),
            )
            .unwrap();
            for entry in std::fs::read_dir(&b).unwrap() {
                let entry = entry.unwrap();
                assert_eq!(
                    std::fs::read(entry.path()).unwrap(),
                    std::fs::read(noop.join("shard-000").join(entry.file_name())).unwrap()
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
