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

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::action::build_action_mask;
use crate::card_data::CardData;
use crate::card_registry::CardRegistry;
use crate::enums::PlayerId;
use crate::game::Game;
use crate::observation::build_observation_tensor;
use crate::rules::Rules;
use crate::search::{determinized_search, DeterminizedConfig, PolicyValueFn};

use super::npy;

/// Tunables for one self-play run. `search.search.seed` is ignored — the
/// driver derives a fresh deterministic seed per decision from `seed`, the
/// game index, and the decision index.
#[derive(Debug, Clone)]
pub struct SelfPlayConfig {
    /// Games to play.
    pub games: u32,
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
    /// The determinized-search settings (mode, worlds, sims, PUCT,
    /// Dirichlet, observation profile).
    pub search: DeterminizedConfig,
}

impl Default for SelfPlayConfig {
    fn default() -> Self {
        Self {
            games: 16,
            seed: 0,
            temperature_decisions: 16,
            decision_cap: 600,
            shard_rows: 4096,
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
    pub shards: usize,
}

/// One training row, pre-z.
struct PendingRecord {
    obs: Vec<f32>,
    actions: Vec<u16>,
    visits: Vec<u32>,
    mover: PlayerId,
    game_id: u32,
    /// Filled at game end.
    z: f32,
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
        let mut z: Vec<f32> = Vec::with_capacity(rows);
        let mut mover: Vec<u8> = Vec::with_capacity(rows);
        let mut game_id: Vec<i32> = Vec::with_capacity(rows);
        offsets.push(0);
        for r in &self.records {
            assert_eq!(r.obs.len(), self.obs_dim, "obs width drifted inside a run");
            obs.extend_from_slice(&r.obs);
            actions.extend(r.actions.iter().map(|&a| a as i32));
            visits.extend(r.visits.iter().map(|&v| v as f32));
            offsets.push(actions.len() as i64);
            z.push(r.z);
            mover.push(r.mover);
            game_id.push(r.game_id as i32);
        }

        npy::write_f32_2d(&dir.join("obs.npy"), rows, self.obs_dim, &obs)?;
        npy::write_i64_1d(&dir.join("policy_offsets.npy"), &offsets)?;
        npy::write_i32_1d(&dir.join("policy_actions.npy"), &actions)?;
        npy::write_f32_1d(&dir.join("policy_visits.npy"), &visits)?;
        npy::write_f32_1d(&dir.join("z.npy"), &z)?;
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
fn derive_seed(master: u64, stream: u64, index: u64) -> u64 {
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
    if decks.is_empty() {
        return Err("empty deck pool".into());
    }
    std::fs::create_dir_all(out_dir).map_err(|e| format!("creating {}: {e}", out_dir.display()))?;

    // One registry for the whole run (registry construction is the deep /
    // expensive part; HeadlessRunner rebuilds it per game, we must not).
    let registry = CardRegistry::from_cards(card_data);
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

    for game_idx in 0..config.games {
        let di = deck_rng.gen_range(0..decks.len());
        let dj = deck_rng.gen_range(0..decks.len());
        game_decks.push((di, dj));

        let deck_lists = vec![decks[di].clone(), decks[dj].clone()];
        let game_seed = derive_seed(config.seed, 1, game_idx as u64);
        let mut game = Game::new(&deck_lists, card_data, Rules::standard(), Some(game_seed))
            .map_err(|e| format!("game {game_idx} ({di} vs {dj}): {e}"))?;
        // Same contract as HeadlessRunner: turn 1 is exposed during the
        // mulligan window.
        game.turn_count = 1;

        let mut temp_rng = StdRng::seed_from_u64(derive_seed(config.seed, 2, game_idx as u64));
        let mut decisions = 0u32;
        let mut non_forced = 0u32;
        let game_first_record = buffer.records.len();

        while !game.game_over && decisions < config.decision_cap {
            let pid = decision_player(&game);
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
                    game.decode_action(legal[0], pid);
                    decisions += 1;
                    stats.forced_decisions_total += 1;
                    continue;
                }
                _ => {}
            }

            let mut search_cfg = config.search.clone();
            search_cfg.search.seed =
                derive_seed(config.seed, 3 + game_idx as u64, decisions as u64);
            let result = determinized_search(&registry, &game, pid, evaluator, &search_cfg);

            // Record BEFORE acting: obs is the pre-action state the π
            // target was computed for.
            let obs = build_observation_tensor(&game, pid, &registry, profile);
            if buffer.obs_dim == 0 {
                buffer.obs_dim = obs.len();
            }
            let (actions, visits): (Vec<u16>, Vec<u32>) =
                result.root_visits.iter().copied().unzip();
            let total_visits: u32 = visits.iter().sum();

            let action = if total_visits == 0 {
                // Degenerate budget (0 sims): fall back to uniform-legal.
                legal[temp_rng.gen_range(0..legal.len())]
            } else if non_forced < config.temperature_decisions {
                // τ = 1: sample proportional to visit counts.
                let mut ticket = temp_rng.gen_range(0..total_visits);
                let mut chosen = actions[0];
                for (a, v) in actions.iter().zip(visits.iter()) {
                    if ticket < *v {
                        chosen = *a;
                        break;
                    }
                    ticket -= v;
                }
                chosen
            } else {
                // Argmax, first-max tie-break in mask order.
                let mut best = (0u32, actions[0]);
                for (a, v) in actions.iter().zip(visits.iter()) {
                    if *v > best.0 {
                        best = (*v, *a);
                    }
                }
                best.1
            };

            buffer.records.push(PendingRecord {
                obs,
                actions,
                visits,
                mover: pid,
                game_id: game_idx,
                z: 0.0,
            });

            game.decode_action(action, pid);
            decisions += 1;
            non_forced += 1;
        }

        // z backfill for this game's records.
        let truncated = !game.game_over;
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

        if buffer.records.len() >= config.shard_rows {
            stats.rows += buffer.records.len();
            buffer.flush(out_dir)?;
        }
    }

    stats.rows += buffer.records.len();
    buffer.flush(out_dir)?;
    stats.shards = buffer.written.len();

    let manifest = serde_json::json!({
        "schema": "selfplay-shards-v1",
        "model": model_desc,
        "obs_dim": buffer.obs_dim,
        "action_dim": buffer.action_dim,
        "observation_profile": profile.as_str(),
        "search": {
            "mode": format!("{:?}", config.search.mode),
            "worlds": config.search.worlds,
            "simulations": config.search.search.simulations,
            "c_puct": config.search.search.c_puct,
            "dirichlet_alpha": config.search.search.dirichlet_alpha,
            "dirichlet_epsilon": config.search.search.dirichlet_epsilon,
            "max_decisions": config.search.search.max_decisions,
        },
        "temperature_decisions": config.temperature_decisions,
        "decision_cap": config.decision_cap,
        "seed": config.seed,
        "games": stats.games,
        "rows": stats.rows,
        "truncated_games": stats.truncated_games,
        "wins": { "player0": stats.wins[0], "player1": stats.wins[1] },
        "decisions_total": stats.decisions_total,
        "forced_decisions_total": stats.forced_decisions_total,
        "deck_names": deck_names,
        "game_decks": game_decks.iter().map(|(a, b)| vec![*a, *b]).collect::<Vec<_>>(),
        "shards": buffer
            .written
            .iter()
            .map(|(d, r)| serde_json::json!({ "dir": d, "rows": r }))
            .collect::<Vec<_>>(),
        "z_semantics": "±1 terminal outcome from the record's mover perspective; 0 = decision-cap truncated game",
        "policy_semantics": "raw root visit counts over root-legal actions (sparse via offsets); normalize before use",
    });
    let manifest_path = out_dir.join("manifest.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .map_err(|e| format!("writing {}: {e}", manifest_path.display()))?;

    Ok(stats)
}
