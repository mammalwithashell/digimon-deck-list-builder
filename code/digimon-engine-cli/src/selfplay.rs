//! `selfplay` subcommand — AlphaZero self-play data generation
//! (add-determinized-search task 3.1; design D9/D10).
//!
//! Thin front-end over `digimon_engine::selfplay::run_self_play`: resolves
//! the evaluator (a task-0.2 policy+value ONNX export, or `--uniform` for
//! cold-start / smoke runs), loads the deck pool, and hands off. Process-
//! level parallelism by design: run N copies with distinct `--seed` /
//! `--out` and the shard directories compose.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use digimon_engine::card_data::CardData;
use digimon_engine::inference::BatchedPolicyValueEvaluator;
use digimon_engine::search::{
    DeterminizationMode, DeterminizedConfig, MemoizedEvaluator, PolicyValueFn,
    UniformEvaluator,
};
use digimon_engine::selfplay::opponents::{HistoricalOpponents, OpponentManifest};
use digimon_engine::selfplay::{run_self_play_with_opponents, SelfPlayConfig};

use crate::winprob::resolve_profile;

/// Deck-pool file: either `[["ST1-01", ...], ...]` or
/// `[{"name": "...", "cards": ["ST1-01", ...]}, ...]`.
fn load_deck_pool(path: &std::path::Path) -> Result<(Vec<Vec<String>>, Vec<String>), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let v: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("parsing {}: {e}", path.display()))?;
    let arr = v
        .as_array()
        .ok_or_else(|| format!("{}: expected a top-level array", path.display()))?;
    let mut decks = Vec::with_capacity(arr.len());
    let mut names = Vec::with_capacity(arr.len());
    for (i, entry) in arr.iter().enumerate() {
        if let Some(cards) = entry.as_array() {
            let deck: Vec<String> = cards
                .iter()
                .map(|c| {
                    c.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| format!("deck {i}: non-string card id"))
                })
                .collect::<Result<_, _>>()?;
            names.push(format!("deck-{i}"));
            decks.push(deck);
        } else if let Some(obj) = entry.as_object() {
            let cards = obj
                .get("cards")
                .and_then(|c| c.as_array())
                .ok_or_else(|| format!("deck {i}: object entry needs a `cards` array"))?;
            let deck: Vec<String> = cards
                .iter()
                .map(|c| {
                    c.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| format!("deck {i}: non-string card id"))
                })
                .collect::<Result<_, _>>()?;
            names.push(
                obj.get("name")
                    .and_then(|n| n.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("deck-{i}")),
            );
            decks.push(deck);
        } else {
            return Err(format!(
                "deck {i}: expected a card-id array or {{name, cards}} object"
            ));
        }
    }
    if decks.is_empty() {
        return Err(format!("{}: empty deck pool", path.display()));
    }
    Ok((decks, names))
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    model: Option<PathBuf>,
    opponent_pool: Option<PathBuf>,
    uniform: bool,
    decks_path: PathBuf,
    games: u32,
    game_start: u32,
    sims: u32,
    worlds: u32,
    eval_batch_size: usize,
    mode: String,
    seed: u64,
    out: PathBuf,
    temperature_decisions: u32,
    decision_cap: u32,
    shard_rows: usize,
    reduced_sims: Option<u32>,
    full_search_fraction: f32,
    cheap_sims: u32,
    tensor_profile: Option<String>,
    card_data: HashMap<String, CardData>,
    admission_dir: PathBuf,
) -> ExitCode {
    if eval_batch_size == 0 {
        eprintln!("--eval-batch-size must be positive");
        return ExitCode::from(2);
    }
    if !(0.0..=1.0).contains(&full_search_fraction) {
        eprintln!("--full-search-fraction must lie in [0, 1]");
        return ExitCode::from(2);
    }
    if full_search_fraction < 1.0 && cheap_sims == 0 {
        eprintln!("--cheap-sims must be positive when --full-search-fraction < 1");
        return ExitCode::from(2);
    }
    if reduced_sims == Some(0) {
        eprintln!("--reduced-sims must be positive");
        return ExitCode::from(2);
    }
    // Opt in for a frozen deterministic MLP only. This never caches engine
    // legality or tree results; an exact observation+mask hit reuses only the
    // same network output. Zero retains the uncached reference path.
    let cache_entries = match std::env::var("DIGIMON_SEARCH_EVAL_CACHE_ENTRIES") {
        Ok(value) => match value.parse::<usize>() {
            Ok(entries) => entries,
            Err(_) => {
                eprintln!("DIGIMON_SEARCH_EVAL_CACHE_ENTRIES must be a non-negative integer");
                return ExitCode::from(2);
            }
        },
        Err(std::env::VarError::NotPresent) => 0,
        Err(err) => {
            eprintln!("cannot read DIGIMON_SEARCH_EVAL_CACHE_ENTRIES: {err}");
            return ExitCode::from(2);
        }
    };
    let (decks, deck_names) = match load_deck_pool(&decks_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error loading deck pool: {e}");
            return ExitCode::from(2);
        }
    };
    let admission = match crate::admission::validate_pool(&decks, &card_data, &admission_dir) {
        Ok(record) => record,
        Err(e) => {
            eprintln!("training admission rejected: {e}");
            return ExitCode::from(2);
        }
    };
    let mode = match mode.as_str() {
        "pimc" => DeterminizationMode::Pimc,
        "ismcts" | "is-mcts" => DeterminizationMode::IsMcts,
        other => {
            eprintln!("unknown --mode {other:?} (expected `pimc` or `ismcts`)");
            return ExitCode::from(2);
        }
    };

    // Evaluator + observation profile. With a model, the profile comes from
    // the export's meta (or the explicit flag) — feeding a model the wrong
    // profile is an input-shape error at best and silent garbage at worst.
    let (mut evaluator, model_desc, profile): (Box<dyn PolicyValueFn>, String, _) =
        match (&model, uniform) {
            (Some(_), true) => {
                eprintln!("--model and --uniform are mutually exclusive");
                return ExitCode::from(2);
            }
            (Some(path), false) => {
                let profile = match resolve_profile(path, tensor_profile.as_deref()) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("error resolving tensor profile: {e}");
                        return ExitCode::from(2);
                    }
                };
                match BatchedPolicyValueEvaluator::load_for_profile(path, profile) {
                    Ok(e) => {
                        if let Err(err) = e.require_terminal_value() {
                            eprintln!("{err}");
                            return ExitCode::from(2);
                        }
                        let evaluator: Box<dyn PolicyValueFn> = if cache_entries == 0 {
                            Box::new(e)
                        } else {
                            Box::new(MemoizedEvaluator::new(e, cache_entries))
                        };
                        (evaluator, path.to_string_lossy().to_string(), profile)
                    }
                    Err(e) => {
                        eprintln!(
                            "error loading policy+value model {} (needs a task-0.2 export \
                             with an inline `value` output): {e}",
                            path.display()
                        );
                        return ExitCode::from(2);
                    }
                }
            }
            (None, true) => {
                let profile = match tensor_profile.as_deref() {
                    Some(raw) => {
                        match digimon_engine::observation::parse_observation_profile(raw) {
                            Ok(p) => p,
                            Err(e) => {
                                eprintln!("bad --tensor-profile: {e}");
                                return ExitCode::from(2);
                            }
                        }
                    }
                    None => digimon_engine::observation::default_observation_profile(),
                };
                (Box::new(UniformEvaluator), "uniform".to_string(), profile)
            }
            (None, false) => {
                eprintln!("provide --model <pv.onnx> or --uniform");
                return ExitCode::from(2);
            }
        };

    let mut config = SelfPlayConfig {
        games,
        game_start,
        seed,
        temperature_decisions,
        decision_cap,
        shard_rows,
        evaluation_batch_size: eval_batch_size,
        reduced_simulations: reduced_sims,
        full_search_fraction,
        cheap_simulations: cheap_sims,
        search: DeterminizedConfig::default(),
    };
    config.search.mode = mode;
    config.search.worlds = worlds;
    config.search.search.simulations = sims;
    config.search.search.observation_profile = profile;
    let active_cache_entries = if uniform { 0 } else { cache_entries };
    let mut opponents = match opponent_pool {
        None => None,
        Some(path) => {
            let loaded = (|| -> Result<HistoricalOpponents, String> {
                let current = model.as_ref().ok_or("--opponent-pool requires --model")?;
                let manifest = OpponentManifest::load(&path, current)?;
                let expected = serde_json::to_value(
                    digimon_engine::inference::contract::ModelContract::for_profile(
                        &digimon_engine::observation::observation_layout(profile),
                    ),
                )
                .unwrap();
                let mut evaluators: Vec<Box<dyn PolicyValueFn>> = Vec::new();
                for entry in &manifest.opponents {
                    let meta: serde_json::Value = serde_json::from_slice(
                        &std::fs::read(format!("{}.meta.json", entry.path.display()))
                            .map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                    if expected
                        .as_object()
                        .unwrap()
                        .iter()
                        .any(|(k, v)| meta.get(k) != Some(v))
                        || meta["value_semantics"] != "terminal_outcome"
                    {
                        return Err(format!(
                            "opponent {} sidecar contract/value mismatch",
                            entry.id
                        ));
                    }
                    let e = BatchedPolicyValueEvaluator::load_for_profile(&entry.path, profile)
                        .map_err(|e| format!("opponent {}: {e}", entry.id))?;
                    e.require_terminal_value().map_err(|e| e.to_string())?;
                    evaluators.push(if cache_entries == 0 {
                        Box::new(e)
                    } else {
                        Box::new(MemoizedEvaluator::new(e, cache_entries))
                    });
                }
                Ok(HistoricalOpponents {
                    manifest,
                    evaluators,
                })
            })();
            match loaded {
                Ok(pool) => Some(pool),
                Err(e) => {
                    eprintln!("historical opponent pool rejected: {e}");
                    return ExitCode::from(2);
                }
            }
        }
    };

    eprintln!(
        "selfplay: {} games, {} decks, mode {:?}, {} worlds x {} total sims, profile {}, evaluator {}",
        games,
        decks.len(),
        config.search.mode,
        worlds,
        sims,
        profile.as_str(),
        model_desc
    );
    match reduced_sims {
        Some(reduced) => eprintln!("phase budget: Main roots {sims} sims, other roots {reduced} sims"),
        None => eprintln!("phase budget: every root {sims} sims"),
    }
    if full_search_fraction < 1.0 {
        eprintln!(
            "playout-cap randomization: full search with probability {full_search_fraction}, \
             else {cheap_sims} noise-free sims recorded with policy_weight 0"
        );
    }
    eprintln!("exact-input evaluator cache: {active_cache_entries} entries");
    eprintln!(
        "independent-world evaluation batch size: {eval_batch_size} (IS-MCTS remains serial)"
    );

    match run_self_play_with_opponents(
        &card_data,
        &decks,
        &deck_names,
        evaluator.as_mut(),
        &config,
        &out,
        &model_desc,
        opponents.as_mut(),
    ) {
        Ok(stats) => {
            let path = out.join("manifest.json");
            let stamped = (|| -> Result<(), Box<dyn std::error::Error>> {
                let mut manifest: serde_json::Value =
                    serde_json::from_str(&std::fs::read_to_string(&path)?)?;
                manifest["admission"] = admission;
                manifest["search"]["evaluator_cache_entries"] = active_cache_entries.into();
                std::fs::write(&path, serde_json::to_string_pretty(&manifest)?)?;
                Ok(())
            })();
            if let Err(e) = stamped {
                eprintln!("cannot stamp admission into manifest: {e}");
                return ExitCode::from(1);
            }
            eprintln!(
                "done: {} games ({} truncated), {} rows in {} shard(s), wins P0 {} / P1 {}, \
                 {} decisions ({} forced) → {}",
                stats.games,
                stats.truncated_games,
                stats.rows,
                stats.shards,
                stats.wins[0],
                stats.wins[1],
                stats.decisions_total,
                stats.forced_decisions_total,
                out.display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("self-play run failed: {e}");
            ExitCode::from(1)
        }
    }
}
