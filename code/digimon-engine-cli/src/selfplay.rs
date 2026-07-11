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
    DeterminizationMode, DeterminizedConfig, PolicyValueFn, UniformEvaluator,
};
use digimon_engine::selfplay::{run_self_play, SelfPlayConfig};

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
    uniform: bool,
    decks_path: PathBuf,
    games: u32,
    sims: u32,
    worlds: u32,
    mode: String,
    seed: u64,
    out: PathBuf,
    temperature_decisions: u32,
    decision_cap: u32,
    shard_rows: usize,
    tensor_profile: Option<String>,
    card_data: HashMap<String, CardData>,
) -> ExitCode {
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
                match BatchedPolicyValueEvaluator::load(path) {
                    Ok(e) => (Box::new(e), path.to_string_lossy().to_string(), profile),
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

    let (decks, deck_names) = match load_deck_pool(&decks_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error loading deck pool: {e}");
            return ExitCode::from(2);
        }
    };

    let mut config = SelfPlayConfig {
        games,
        seed,
        temperature_decisions,
        decision_cap,
        shard_rows,
        search: DeterminizedConfig::default(),
    };
    config.search.mode = mode;
    config.search.worlds = worlds;
    config.search.search.simulations = sims;
    config.search.search.observation_profile = profile;

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

    match run_self_play(
        &card_data,
        &decks,
        &deck_names,
        evaluator.as_mut(),
        &config,
        &out,
        &model_desc,
    ) {
        Ok(stats) => {
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
