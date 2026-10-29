//! AlphaZero self-play driver (add-determinized-search task 3.1).
//!
//! Plays full games with [`determinized_search`] deciding BOTH seats —
//! each decision searches from the MOVER's infoset (viewer = mover), so
//! neither seat ever consumes the other's hidden information and the
//! P1-perspective bug that retired `opponent="self-play"` cannot arise by
//! construction (design D9).
//!
//! Per decision (design D10):
//! - **forced** (exactly one legal action): applied directly, no search,
//!   no record — a one-hot π on a choiceless node carries no signal;
//! - otherwise run the search, record `(obs, sparse π = root visit
//!   counts)` for the mover, then pick the action: visit-proportional
//!   sampling while `decisions_seen < temperature_decisions` (τ=1), argmax
//!   after (first-max tie-break, mask order);
//! - at game end, backfill `z = ±1` per record from its mover's side;
//!   games truncated by `decision_cap` get `z = 0` and are counted in the
//!   manifest (an honesty signal — see D10).
//!
//! Output: `shard-NNN/` directories of `.npy` arrays + one `manifest.json`
//! (schema `selfplay-shards-v1`), flushed on game boundaries whenever the
//! accumulated rows exceed `shard_rows`. π is sparse (offsets / actions /
//! visits) because a dense 2192-wide target would be ~25% extra bytes of
//! mostly zeros (design D9). Visit counts are RAW — the trainer
//! normalizes, keeping the total-simulation information available.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::action::build_action_mask;
use crate::card_data::CardData;
use crate::card_registry::CardRegistry;
use crate::enums::{GamePhase, PlayerId};
use crate::game::Game;
use crate::observation::build_observation_tensor_with_mask;
use crate::rules::Rules;
use crate::search::{
    determinized_search_batched, DeterminizationMode, DeterminizedConfig, PolicyValueFn,
};

use super::npy;

/// Tunables for one self-play run. `search.search.seed` is ignored — the
/// driver derives a fresh deterministic seed per decision from `seed`, the
/// game index, and the decision index.
#[derive(Debug, Clone)]
pub struct SelfPlayConfig {
    /// Games to play.
    pub games: u32,
    /// Global first game index, independent of worker assignment.
    pub game_start: u32,
    /// Master seed: deck sampling, game shuffles, temperature sampling,
    /// and per-decision search seeds all derive from it.
    pub seed: u64,
    /// τ=1 visit-proportional action sampling for this many NON-FORCED
    /// decisions per game; argmax afterwards.
    pub temperature_decisions: u32,
    /// Hard cap on decisions per game (forced ones included). Truncated
    /// games are scored z = 0 and counted.
    pub decision_cap: u32,
    /// Flush a shard once at least this many rows have accumulated
    /// (checked on game boundaries so z backfill stays shard-local).
    pub shard_rows: usize,
    /// Maximum independent PIMC worlds evaluated together. One is the serial
    /// reference; IS-MCTS remains serial regardless of this setting.
    pub evaluation_batch_size: usize,
    /// Phase-dependent budget: TOTAL simulations for roots that are NOT an
    /// unconstrained Main-phase decision (pending selections, mulligan,
    /// breeding, block/counter/alliance windows). `None` searches every
    /// root with `search.search.simulations`. Never raises the budget.
    pub reduced_simulations: Option<u32>,
    /// Playout-cap randomization: probability that a non-forced decision
    /// receives the full search and becomes a policy target
    /// (`policy_weight` 1). `1.0` is the reference behaviour.
    pub full_search_fraction: f32,
    /// TOTAL simulations for the cheap searches drawn when
    /// `full_search_fraction < 1`. Cheap searches run without root noise
    /// and their rows carry `policy_weight` 0 (value target only).
    pub cheap_simulations: u32,
    /// The determinized-search settings (mode, worlds, sims, PUCT,
    /// Dirichlet, observation profile).
    pub search: DeterminizedConfig,
}

impl Default for SelfPlayConfig {
    fn default() -> Self {
        Self {
            games: 16,
            game_start: 0,
            seed: 0,
            temperature_decisions: 16,
            decision_cap: 600,
            shard_rows: 4096,
            evaluation_batch_size: 1,
            reduced_simulations: None,
            full_search_fraction: 1.0,
            cheap_simulations: 64,
            search: DeterminizedConfig::default(),
        }
    }
}

/// Aggregate statistics for a completed run (also serialized into the
/// manifest).
#[derive(Debug, Clone, Default)]
pub struct SelfPlayStats {
    pub games: u32,
    pub rows: usize,
    pub truncated_games: u32,
    pub wins: [u32; 2],
    pub decisions_total: u64,
    pub forced_decisions_total: u64,
    /// Searched decisions that received the full budget (policy targets).
    pub full_search_decisions: u64,
    /// Searched decisions that received the cheap value-only budget.
    pub cheap_search_decisions: u64,
    /// Searched decisions whose root was not an unconstrained Main-phase
    /// decision and therefore ran under `reduced_simulations`.
    pub reduced_budget_decisions: u64,
    pub shards: usize,
}

/// Per-root-phase search mix for the manifest: how many searches a phase
/// received, their wall seconds and simulations actually run. Observational
/// only — never fed back into the search.
#[derive(Debug, Clone, Default)]
struct PhaseStat {
    searched: u64,
    seconds: f64,
    simulations: u64,
}

/// One training row, pre-z.
struct PendingRecord {
    obs: Vec<f32>,
    actions: Vec<u16>,
    /// Sparse policy target parallel to `actions`: the root visit counts
    /// conditional on the chosen plan prefix (`SearchResult::training_visits`).
    policy: Vec<f32>,
    mover: PlayerId,
    game_id: u32,
    /// 1.0 when the row's search was a full search (policy target), 0.0 for
    /// a cheap value-only search.
    policy_weight: f32,
    /// Filled at game end.
    z: f32,
    terminated: bool,
}

/// Rows accumulated toward the next shard flush.
struct ShardBuffer {
    records: Vec<PendingRecord>,
    obs_dim: usize,
    action_dim: usize,
    next_shard: usize,
    written: Vec<(String, usize)>,
}

impl ShardBuffer {
    fn flush(&mut self, out_dir: &Path) -> Result<(), String> {
        let _span = crate::search::profiling::span("shard_io");
        if self.records.is_empty() {
            return Ok(());
        }
        let rows = self.records.len();
        let dir = out_dir.join(format!("shard-{:03}", self.next_shard));
        std::fs::create_dir_all(&dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;

        let mut obs = Vec::with_capacity(rows * self.obs_dim);
        let mut offsets: Vec<i64> = Vec::with_capacity(rows + 1);
        let mut actions: Vec<i32> = Vec::new();
        let mut visits: Vec<f32> = Vec::new();
        let mut policy_weight: Vec<f32> = Vec::with_capacity(rows);
        let mut z: Vec<f32> = Vec::with_capacity(rows);
        let mut terminated: Vec<u8> = Vec::with_capacity(rows);
        let mut mover: Vec<u8> = Vec::with_capacity(rows);
        let mut game_id: Vec<i32> = Vec::with_capacity(rows);
        offsets.push(0);
        for r in &self.records {
            assert_eq!(r.obs.len(), self.obs_dim, "obs width drifted inside a run");
            obs.extend_from_slice(&r.obs);
            actions.extend(r.actions.iter().map(|&a| a as i32));
            visits.extend_from_slice(&r.policy);
            offsets.push(actions.len() as i64);
            policy_weight.push(r.policy_weight);
            z.push(r.z);
            terminated.push(u8::from(r.terminated));
            mover.push(r.mover);
            game_id.push(r.game_id as i32);
        }

        npy::write_f32_2d(&dir.join("obs.npy"), rows, self.obs_dim, &obs)?;
        npy::write_i64_1d(&dir.join("policy_offsets.npy"), &offsets)?;
        npy::write_i32_1d(&dir.join("policy_actions.npy"), &actions)?;
        npy::write_f32_1d(&dir.join("policy_visits.npy"), &visits)?;
        npy::write_f32_1d(&dir.join("policy_weight.npy"), &policy_weight)?;
        npy::write_f32_1d(&dir.join("z.npy"), &z)?;
        npy::write_u8_1d(&dir.join("terminated.npy"), &terminated)?;
        npy::write_u8_1d(&dir.join("mover.npy"), &mover)?;
        npy::write_i32_1d(&dir.join("game_id.npy"), &game_id)?;

        self.written
            .push((format!("shard-{:03}", self.next_shard), rows));
        self.next_shard += 1;
        self.records.clear();
        Ok(())
    }
}

/// Split-mix style seed derivation so every (game, decision) search and
/// every sampling stream is deterministic from the master seed alone.
pub(super) fn derive_seed(master: u64, stream: u64, index: u64) -> u64 {
    let mut x = master
        .wrapping_add(stream.wrapping_mul(0x9E3779B97F4A7C15))
        .wrapping_add(index.wrapping_mul(0xBF58476D1CE4E5B9));
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^ (x >> 31)
}

/// The mover a state is waiting on (same rule as the search core and the
/// clone-fuzz spike).
fn decision_player(game: &Game) -> PlayerId {
    crate::search::decision_player(game)
}

/// Play `config.games` self-play games and write `(obs, π, z)` shards +
/// `manifest.json` under `out_dir`. `decks` is the sampling pool (index
/// pairs drawn per game, mirror matches allowed); `deck_names` is parallel
/// (for the manifest) and may be empty. `model_desc` documents the
/// evaluator in the manifest (e.g. the ONNX path, or `"uniform"`).
pub fn run_self_play(
    card_data: &HashMap<String, CardData>,
    decks: &[Vec<String>],
    deck_names: &[String],
    evaluator: &mut dyn PolicyValueFn,
    config: &SelfPlayConfig,
    out_dir: &Path,
    model_desc: &str,
) -> Result<SelfPlayStats, String> {
    run_self_play_with_opponents(
        card_data, decks, deck_names, evaluator, config, out_dir, model_desc, None,
    )
}

/// Optional historical opponent seats; only the current actor emits training rows.
#[allow(clippy::too_many_arguments)]
pub fn run_self_play_with_opponents(
    card_data: &HashMap<String, CardData>,
    decks: &[Vec<String>],
    deck_names: &[String],
    evaluator: &mut dyn PolicyValueFn,
    config: &SelfPlayConfig,
    out_dir: &Path,
    model_desc: &str,
    mut opponents: Option<&mut super::opponents::HistoricalOpponents>,
) -> Result<SelfPlayStats, String> {
    if let Some(pool) = &opponents {
        pool.validate()?;
    }
    crate::search::profiling::reset(std::env::var("DIGIMON_SEARCH_PROFILE").as_deref() == Ok("1"));
    let game_end = config
        .game_start
        .checked_add(config.games)
        .filter(|&end| end <= i32::MAX as u32)
        .ok_or("global game range exceeds signed shard game IDs")?;
    if config.evaluation_batch_size == 0 {
        return Err("evaluation batch size must be positive".into());
    }
    if !(0.0..=1.0).contains(&config.full_search_fraction) {
        return Err("full_search_fraction must lie in [0, 1]".into());
    }
    if config.full_search_fraction < 1.0 && config.cheap_simulations == 0 {
        return Err("cheap_simulations must be positive when full_search_fraction < 1".into());
    }
    if config.reduced_simulations == Some(0) {
        return Err("reduced_simulations must be positive".into());
    }
    if decks.is_empty() {
        return Err("empty deck pool".into());
    }
    std::fs::create_dir_all(out_dir).map_err(|e| format!("creating {}: {e}", out_dir.display()))?;

    // One registry for the whole run (registry construction is the deep /
    // expensive part; HeadlessRunner rebuilds it per game, we must not).
    let registry =
        crate::search::profiling::measure("registry", || CardRegistry::from_cards(card_data));
    let profile = config.search.search.observation_profile;

    let mut deck_rng = StdRng::seed_from_u64(derive_seed(config.seed, 0, 0));
    let mut stats = SelfPlayStats::default();
    let mut buffer = ShardBuffer {
        records: Vec::new(),
        obs_dim: 0,
        action_dim: 0,
        next_shard: 0,
        written: Vec::new(),
    };
    let mut game_decks: Vec<(usize, usize)> = Vec::with_capacity(config.games as usize);
    let mut phase_stats: BTreeMap<String, PhaseStat> = BTreeMap::new();
    let mut opponent_games = Vec::new();
    let started = std::time::Instant::now();
    let trace_decisions = std::env::var_os("DIGIMON_SELFPLAY_TRACE_DECISIONS").is_some();

    // Preserve the original sequential deck-sampling stream exactly.
    for _ in 0..config.game_start {
        let _ = deck_rng.gen_range(0..decks.len());
        let _ = deck_rng.gen_range(0..decks.len());
    }
    for game_idx in config.game_start..game_end {
        let di = deck_rng.gen_range(0..decks.len());
        let dj = deck_rng.gen_range(0..decks.len());
        game_decks.push((di, dj));

        let deck_lists = vec![decks[di].clone(), decks[dj].clone()];
        let game_seed = derive_seed(config.seed, 1, game_idx as u64);
        eprintln!("game {game_idx} start: seed {game_seed}, decks {di} vs {dj}");
        let mut game = Game::new(&deck_lists, card_data, Rules::standard(), Some(game_seed))
            .map_err(|e| format!("game {game_idx} ({di} vs {dj}): {e}"))?;
        // Same contract as HeadlessRunner: turn 1 is exposed during the
        // mulligan window.
        game.turn_count = 1;
        let first_player = game.turn_order[0];
        let historical = opponents
            .as_ref()
            .and_then(|pool| pool.manifest.assignment(config.seed, game_idx));
        let learner = historical.map(|_| first_player ^ (game_idx % 2) as u8);
        let mut searched_by_seat = [0u32; 2];
        let mut rows_by_seat = [0usize; 2];

        let mut temp_rng = StdRng::seed_from_u64(derive_seed(config.seed, 2, game_idx as u64));
        // Playout-cap stream: its own index space (high bit set) inside the
        // temperature stream so it can never alias the per-game temperature
        // seed, and it is only consumed when the fraction is below 1 — the
        // reference configuration's randomness is untouched.
        let mut cap_rng =
            StdRng::seed_from_u64(derive_seed(config.seed, 2, (game_idx as u64) | (1 << 63)));
        let mut decisions = 0u32;
        let mut non_forced = 0u32;
        let game_first_record = buffer.records.len();

        while !game.game_over && decisions < config.decision_cap {
            let pid = decision_player(&game);
            if trace_decisions {
                eprintln!(
                    "game {game_idx} decision {decisions}: player {pid}, phase {:?}, memory {}",
                    game.current_phase, game.memory
                );
                let trace = serde_json::json!({"game": game_idx, "game_seed": game_seed,
                    "decision": decisions, "search_seed": derive_seed(config.seed, 3 + game_idx as u64, decisions as u64),
                    "scenario": game.to_scenario()});
                std::fs::write(
                    out_dir.join("current-decision.json"),
                    serde_json::to_vec(&trace).unwrap(),
                )
                .map_err(|e| format!("writing decision trace: {e}"))?;
            }
            let mask = build_action_mask(&game, pid);
            if buffer.action_dim == 0 {
                buffer.action_dim = mask.len();
            }
            let legal: Vec<u16> = mask
                .iter()
                .enumerate()
                .filter(|(_, &v)| v > 0.5)
                .map(|(i, _)| i as u16)
                .collect();
            match legal.len() {
                0 => {
                    return Err(format!(
                        "game {game_idx} seed {game_seed}: empty action mask for player {pid} \
                         at decision {decisions} (phase {:?}) — mask/engine invariant broken",
                        game.current_phase
                    ));
                }
                1 => {
                    // Forced: apply without search, emit no record (D10).
                    if trace_decisions {
                        eprintln!(
                            "game {game_idx} decision {decisions}: forced action {}",
                            legal[0]
                        );
                    }
                    game.decode_action(legal[0], pid);
                    decisions += 1;
                    stats.forced_decisions_total += 1;
                    continue;
                }
                _ => {}
            }

            // Budget allocation (search-budget experiments). The full budget
            // belongs to unconstrained Main-phase roots; every other root —
            // a pending selection, the mulligan, breeding, a block/counter/
            // alliance window — runs under `reduced_simulations` when set.
            // Independently, playout-cap randomization gives the decision
            // the full search with probability `full_search_fraction`, else
            // a cheap noise-free search whose row is a value target only.
            let main_root = game.pending_selection.is_none()
                && game.mulligan_current_player().is_none()
                && game.current_phase == GamePhase::Main;
            let full_search = config.full_search_fraction >= 1.0
                || cap_rng.gen::<f32>() < config.full_search_fraction;
            let mut search_cfg = config.search.clone();
            search_cfg.search.seed =
                derive_seed(config.seed, 3 + game_idx as u64, decisions as u64);
            if !main_root {
                if let Some(reduced) = config.reduced_simulations {
                    search_cfg.search.simulations = search_cfg.search.simulations.min(reduced);
                    stats.reduced_budget_decisions += 1;
                }
            }
            if full_search {
                stats.full_search_decisions += 1;
            } else {
                search_cfg.search.simulations =
                    search_cfg.search.simulations.min(config.cheap_simulations);
                search_cfg.search.dirichlet_epsilon = 0.0;
                stats.cheap_search_decisions += 1;
            }
            let search_started = std::time::Instant::now();
            let acting_evaluator: &mut dyn PolicyValueFn = match (historical, learner) {
                (Some(index), Some(current)) if current != pid => {
                    opponents.as_mut().unwrap().evaluators[index].as_mut()
                }
                _ => &mut *evaluator,
            };
            searched_by_seat[pid as usize] += 1;
            let result = determinized_search_batched(
                &registry,
                &game,
                pid,
                acting_evaluator,
                &search_cfg,
                config.evaluation_batch_size,
            );
            {
                let stat = phase_stats
                    .entry(format!("{:?}", game.current_phase))
                    .or_default();
                stat.searched += 1;
                stat.seconds += search_started.elapsed().as_secs_f64();
                stat.simulations += result.simulations_run as u64;
            }

            // Pick a complete physical commitment once. Navigation is only
            // its transport route; never re-search between its pages.
            let total_visits: u32 = result.root_plans.iter().map(|(_, v, _)| v).sum();
            let route = if total_visits == 0 {
                result.root_plans[temp_rng.gen_range(0..result.root_plans.len())]
                    .0
                    .clone()
            } else if non_forced < config.temperature_decisions {
                let mut ticket = temp_rng.gen_range(0..total_visits);
                let mut chosen = &result.root_plans[0].0;
                for (plan, visits, _) in &result.root_plans {
                    if ticket < *visits {
                        chosen = plan;
                        break;
                    }
                    ticket -= visits;
                }
                chosen.clone()
            } else {
                result
                    .best_plan()
                    .expect("search must return a legal plan")
                    .to_vec()
            };
            for (index, &action) in route.iter().enumerate() {
                if decisions >= config.decision_cap {
                    break;
                }
                assert_eq!(
                    decision_player(&game),
                    pid,
                    "search route changed player before commitment"
                );
                let next_mask;
                let step_mask = if index == 0 {
                    &mask
                } else {
                    next_mask = build_action_mask(&game, pid);
                    &next_mask
                };
                assert!(
                    step_mask[action as usize] > 0.5,
                    "search route must respect the live mask"
                );
                if step_mask.iter().filter(|&&m| m > 0.5).count() > 1 {
                    // The temperature clock covers BOTH actors even when only
                    // the current actor's rows are retained for training.
                    non_forced += 1;
                    if learner.is_none_or(|current| current == pid) {
                        let obs = build_observation_tensor_with_mask(
                            &game, pid, &registry, profile, step_mask,
                        );
                        if buffer.obs_dim == 0 {
                            buffer.obs_dim = obs.len();
                        }
                        // Shards also encode the complete engine legality mask.
                        // Rejected/backward page moves remain legal with ZERO
                        // target mass, so fitting can learn not to browse in circles.
                        let (actions, visits): (Vec<u16>, Vec<u32>) = result
                            .training_visits(&route[..index], step_mask)
                            .into_iter()
                            .unzip();
                        buffer.records.push(PendingRecord {
                            obs,
                            actions,
                            policy: visits.into_iter().map(|v| v as f32).collect(),
                            mover: pid,
                            game_id: game_idx,
                            policy_weight: if full_search { 1.0 } else { 0.0 },
                            z: 0.0,
                            terminated: false,
                        });
                        rows_by_seat[pid as usize] += 1;
                    }
                } else {
                    stats.forced_decisions_total += 1;
                }
                if trace_decisions {
                    eprintln!("game {game_idx} decision {decisions}: search action {action}");
                }
                game.decode_action(action, pid);
                decisions += 1;
            }
        }

        // z backfill for this game's records.
        let truncated = !game.game_over
            || matches!(
                game.terminal_outcome_reason,
                Some(
                    crate::game::TerminalOutcomeReason::StepLimit
                        | crate::game::TerminalOutcomeReason::Crash
                )
            );
        for r in &mut buffer.records[game_first_record..] {
            r.terminated = !truncated;
        }
        if truncated {
            stats.truncated_games += 1; // z stays 0.0 (D10)
        } else if let Some(w) = game.winner {
            stats.wins[w as usize % 2] += 1;
            for r in &mut buffer.records[game_first_record..] {
                r.z = if r.mover == w { 1.0 } else { -1.0 };
            }
        }
        stats.games += 1;
        stats.decisions_total += decisions as u64;
        if let Some(pool) = &opponents {
            opponent_games.push(serde_json::json!({
                "game_id": game_idx, "game_seed": game_seed,
                "opponent_id": historical.map(|i| pool.manifest.opponents[i].id.as_str()).unwrap_or("current"),
                "learner_seat": learner, "first_player": first_player,
                "winner": game.winner, "truncated": truncated,
                "searched_decisions_by_seat": searched_by_seat, "training_rows_by_seat": rows_by_seat,
            }));
        }
        let progress = serde_json::json!({"games": stats.games, "requested_games": config.games,
            "truncated_games": stats.truncated_games, "wins": stats.wins,
            "elapsed_seconds": started.elapsed().as_secs_f64(), "decisions": stats.decisions_total});
        std::fs::write(
            out_dir.join("progress.json"),
            serde_json::to_vec_pretty(&progress).unwrap(),
        )
        .map_err(|e| format!("writing progress: {e}"))?;
        eprintln!("game {game_idx} complete: {} decisions, winner {:?}, truncated {truncated}, elapsed {:.1}s", decisions, game.winner, started.elapsed().as_secs_f64());

        if buffer.records.len() >= config.shard_rows {
            stats.rows += buffer.records.len();
            buffer.flush(out_dir)?;
        }
    }

    stats.rows += buffer.records.len();
    buffer.flush(out_dir)?;
    stats.shards = buffer.written.len();

    let mut manifest = serde_json::json!({
        "schema": "selfplay-shards-v1",
        "model": model_desc,
        "obs_dim": buffer.obs_dim,
        "action_dim": buffer.action_dim,
        "observation_profile": profile.as_str(),
        "model_contract": crate::inference::contract::ModelContract::for_profile(&crate::observation::observation_layout(profile)),
        "value_target": "terminal_outcome",
        "search": {
            "mode": format!("{:?}", config.search.mode),
            "worlds": config.search.worlds,
            "simulations": config.search.search.simulations,
            "c_puct": config.search.search.c_puct,
            "dirichlet_alpha": config.search.search.dirichlet_alpha,
            "dirichlet_epsilon": config.search.search.dirichlet_epsilon,
            "max_decisions": config.search.search.max_decisions,
            "evaluation_batch_size_requested": config.evaluation_batch_size,
            "evaluation_batch_size": if config.search.mode == DeterminizationMode::IsMcts { 1 }
                else { config.evaluation_batch_size.min(config.search.worlds.max(1).min(config.search.search.simulations.max(1)) as usize) },
            "reduced_simulations": config.reduced_simulations,
            "full_search_fraction": config.full_search_fraction,
            "cheap_simulations": config.cheap_simulations,
        },
        "temperature_decisions": config.temperature_decisions,
        "decision_cap": config.decision_cap,
        "profiling": crate::search::profiling::snapshot(),
        "phase_stats": phase_stats
            .iter()
            .map(|(phase, stat)| {
                (
                    phase.clone(),
                    serde_json::json!({
                        "searched": stat.searched,
                        "seconds": stat.seconds,
                        "simulations": stat.simulations,
                    }),
                )
            })
            .collect::<serde_json::Map<String, serde_json::Value>>(),
        "game_start": config.game_start,
        "seed": config.seed,
        "games": stats.games,
        "rows": stats.rows,
        "truncated_games": stats.truncated_games,
        "wins": { "player0": stats.wins[0], "player1": stats.wins[1] },
        "decisions_total": stats.decisions_total,
        "forced_decisions_total": stats.forced_decisions_total,
        "full_search_decisions": stats.full_search_decisions,
        "cheap_search_decisions": stats.cheap_search_decisions,
        "reduced_budget_decisions": stats.reduced_budget_decisions,
        "deck_names": deck_names,
        "game_decks": game_decks.iter().map(|(a, b)| vec![*a, *b]).collect::<Vec<_>>(),
        "shards": buffer
            .written
            .iter()
            .map(|(d, r)| serde_json::json!({ "dir": d, "rows": r }))
            .collect::<Vec<_>>(),
        "z_semantics": "±1 terminal outcome from mover perspective; 0 = draw or truncation; terminated.npy distinguishes them, exclude truncated rows from value training",
        "policy_semantics": "raw root visit counts over the page's legal actions, conditional on the chosen plan prefix (sparse via offsets); normalize before use",
        "policy_weight_semantics": "1.0 = full search, the row trains the policy head; 0.0 = cheap value-only search, exclude the row from the policy loss (its visits still define the legality mask)",
    });
    if let Some(pool) = &opponents {
        manifest["opponent_pool"] = serde_json::to_value(&pool.manifest).unwrap();
        manifest["opponent_games"] = serde_json::to_value(opponent_games).unwrap();
        manifest["training_actor_semantics"] =
            "both current actors in self-play; current actor only against history".into();
    }
    let manifest_path = out_dir.join("manifest.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .map_err(|e| format!("writing {}: {e}", manifest_path.display()))?;

    Ok(stats)
}
