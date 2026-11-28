//! Shared ONNX valuation, MCTS inspection, and adversary-aware line scoring.
//!
//! Used by both PyO3 (`digimon_engine.inspect_position`) and
//! `digimon-engine-mcp` (`inspect_position`, `score_line`, `verify_puzzle`),
//! as well as the `search_exam` CLI example. Supports both stateless MLP
//! `.onnx` graphs and recurrent LSTM `.onnx` + `.value.onnx` sidecars via
//! [`crate::search::load_search_evaluator_for_profile`], or an already-loaded
//! `&mut dyn PolicyValueFn` from a cached model registry.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use ort::session::Session;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::action::build_action_mask;
use crate::action::explain::{explain_action, legal_decoded_actions, ActionExplanation};
use crate::action::space::PASS;
use crate::card_registry::CardRegistry;
use crate::determinize::{
    beliefs::{CandidateDeck, PriorSnapshot, WeightedDeckPrior},
    closed::materialize_closed_with_diagnostics,
};
use crate::enums::PlayerId;
use crate::events::GameEvent;
use crate::game::Game;
use crate::observation::{
    build_observation_tensor_with_mask, parse_observation_profile, ObservationProfileId,
};
use crate::policies::greedy_action;
use crate::search::{
    decision_player, determinized_search_batched, determinized_search_sampled,
    load_search_evaluator_for_profile, DeterminizedConfig, EvalRequest, Mcts, PolicyValueFn,
    SearchConfig,
};

use super::InferenceError;

fn default_accept_optional() -> bool {
    true
}

/// Declarative adversary interrupt rule (`studio-puzzle-v1` / `search-exam-v1`
/// optional `refutation_triggers` array).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RefutationTrigger {
    #[serde(default)]
    pub source_card_id: Option<String>,
    #[serde(default)]
    pub phase: Option<String>,
    #[serde(default = "default_accept_optional")]
    pub accept_optional: bool,
    #[serde(default)]
    pub prefer_label_contains: Option<String>,
}

/// Metadata inspected when loading an ONNX policy+value evaluator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedModelInfo {
    /// `"mlp"` or `"lstm"`.
    pub kind: String,
    /// Resolved observation profile validated against `digimon_contract`.
    pub profile: ObservationProfileId,
    /// `"terminal_outcome"` or `"shaped_return"`.
    pub value_semantics: String,
}

/// Parameters for [`inspect_position_with_evaluator`] / [`inspect_position_from_path`].
#[derive(Debug, Clone)]
pub struct InspectPositionParams {
    pub profile: ObservationProfileId,
    pub simulations: u32,
    pub worlds: u32,
    pub seed: u64,
    pub regime: String,
    pub top_k: usize,
    pub expectations: Vec<Value>,
}

impl Default for InspectPositionParams {
    fn default() -> Self {
        Self {
            profile: ObservationProfileId::StandardLiteV2,
            simulations: 0,
            worlds: 1,
            seed: 0,
            regime: "open_list".to_string(),
            top_k: 12,
            expectations: Vec::new(),
        }
    }
}

/// Parameters for [`score_line_with_evaluator`].
#[derive(Debug, Clone)]
pub struct ScoreLineParams {
    pub actions: Vec<u16>,
    /// When true (or when `adversary_sims > 0` or `!refutation_triggers.is_empty()`),
    /// any step where `decision_player != root_mover` is resolved automatically by
    /// the adversary rather than consuming caller-supplied `actions`.
    pub auto_adversary: bool,
    pub adversary_sims: u32,
    pub adversary_seed: u64,
    pub refutation_triggers: Vec<RefutationTrigger>,
    pub max_adversary_steps: usize,
}

impl Default for ScoreLineParams {
    fn default() -> Self {
        Self {
            actions: Vec::new(),
            auto_adversary: false,
            adversary_sims: 0,
            adversary_seed: 0,
            refutation_triggers: Vec::new(),
            max_adversary_steps: 64,
        }
    }
}

/// Per-step readout produced by [`score_line_with_evaluator`].
#[derive(Debug, Clone, Serialize)]
pub struct ScoreLineStep {
    pub step_index: usize,
    pub mover: PlayerId,
    pub phase: String,
    pub action_id: u16,
    pub label: String,
    pub auto_adversary: bool,
    pub policy_value_for_root_mover: f32,
    pub prior_prob: f32,
    pub events_emitted: Vec<Value>,
}

/// Complete line evaluation produced by [`score_line_with_evaluator`].
#[derive(Debug, Clone, Serialize)]
pub struct ScoreLineResult {
    pub root_mover: PlayerId,
    pub root_policy_value: f32,
    pub steps: Vec<ScoreLineStep>,
    pub final_winner: Option<PlayerId>,
    #[serde(skip)]
    pub final_game: Game,
}

/// Inspect ONNX session metadata and load a `Box<dyn PolicyValueFn>` supporting
/// both stateless MLP `.onnx` and recurrent LSTM `.onnx` + `.value.onnx` sidecars.
pub fn load_inspector_evaluator(
    path: &Path,
    profile_override: Option<ObservationProfileId>,
) -> Result<(Box<dyn PolicyValueFn>, LoadedModelInfo), InferenceError> {
    let session = Session::builder()
        .map_err(|e| InferenceError::LoadFailed(e.to_string()))?
        .with_intra_threads(1)
        .map_err(|e| InferenceError::LoadFailed(e.to_string()))?
        .commit_from_file(path)
        .map_err(|e| InferenceError::LoadFailed(e.to_string()))?;

    let detected_profile = session
        .metadata()
        .ok()
        .and_then(|m| m.custom("digimon_contract"))
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| {
            v.get("observation_profile")
                .and_then(Value::as_str)
                .and_then(|s| parse_observation_profile(s).ok())
        });

    let profile = profile_override
        .or(detected_profile)
        .unwrap_or(ObservationProfileId::StandardLiteV2);

    let session_value_sem = session
        .metadata()
        .ok()
        .and_then(|m| m.custom("digimon_value_semantics"));
    drop(session);

    let eval = load_search_evaluator_for_profile(path, profile, false, 0)?;
    let is_lstm = eval.is_recurrent();
    let kind = if is_lstm { "lstm" } else { "mlp" }.to_string();

    let mut is_terminal = session_value_sem.as_deref() == Some("terminal_outcome");
    if !is_terminal && is_lstm {
        if let Some(companion) = find_companion_value_path(path) {
            if let Ok(val_sess) = Session::builder()
                .map_err(|_| ())
                .and_then(|b| b.with_intra_threads(1).map_err(|_| ()))
                .and_then(|mut b| b.commit_from_file(&companion).map_err(|_| ()))
            {
                if val_sess
                    .metadata()
                    .ok()
                    .and_then(|m| m.custom("digimon_value_semantics"))
                    .as_deref()
                    == Some("terminal_outcome")
                {
                    is_terminal = true;
                }
            }
        }
    }
    if !is_terminal {
        let meta_path = PathBuf::from(format!("{}.meta.json", path.display()));
        if let Ok(raw) = std::fs::read_to_string(&meta_path) {
            if let Ok(meta) = serde_json::from_str::<Value>(&raw) {
                if meta.get("value_semantics").and_then(Value::as_str) == Some("terminal_outcome") {
                    is_terminal = true;
                }
            }
        }
    }

    let value_semantics = if is_terminal {
        "terminal_outcome"
    } else {
        "shaped_return"
    }
    .to_string();

    Ok((
        eval,
        LoadedModelInfo {
            kind,
            profile,
            value_semantics,
        },
    ))
}

fn find_companion_value_path(actor_path: &Path) -> Option<PathBuf> {
    let default_path = actor_path.with_extension("value.onnx");
    if default_path.exists() {
        return Some(default_path);
    }
    let meta_path = PathBuf::from(format!("{}.meta.json", actor_path.display()));
    let raw = std::fs::read_to_string(&meta_path).ok()?;
    let meta: Value = serde_json::from_str(&raw).ok()?;
    let name = meta.get("value_head").and_then(Value::as_str)?;
    if name == "inline" {
        return None;
    }
    let candidate = actor_path.parent().unwrap_or_else(|| Path::new(".")).join(name);
    candidate.exists().then_some(candidate)
}

/// Evaluate a position by loading an ONNX model (MLP or LSTM + `.value.onnx`) from disk.
pub fn inspect_position_from_path(
    game: &Game,
    registry: &CardRegistry,
    model_path: &Path,
    profile_override: Option<ObservationProfileId>,
    simulations: u32,
    worlds: u32,
    seed: u64,
    regime: &str,
    top_k: usize,
    expectations: Vec<Value>,
) -> Result<Value, String> {
    let (mut eval, info) = load_inspector_evaluator(model_path, profile_override)
        .map_err(|e| format!("failed to load ONNX model: {e}"))?;
    let params = InspectPositionParams {
        profile: info.profile,
        simulations,
        worlds,
        seed,
        regime: regime.to_string(),
        top_k,
        expectations,
    };
    inspect_position_with_evaluator(game, registry, eval.as_mut(), &params)
}

/// Evaluate a position using an already-loaded [`PolicyValueFn`] evaluator.
pub fn inspect_position_with_evaluator(
    game: &Game,
    registry: &CardRegistry,
    eval: &mut dyn PolicyValueFn,
    params: &InspectPositionParams,
) -> Result<Value, String> {
    if game.game_over {
        return Err("Game is already over".to_string());
    }
    let viewer = decision_player(game);
    let mask = build_action_mask(game, viewer);
    let legal_ids: Vec<u16> = mask
        .iter()
        .enumerate()
        .filter_map(|(i, &m)| (m > 0.5).then_some(i as u16))
        .collect();
    if legal_ids.is_empty() {
        return Err("No legal actions available at this position".to_string());
    }

    let obs = build_observation_tensor_with_mask(game, viewer, registry, params.profile, &mask);
    let root_state = eval.root_recurrent_state(viewer);
    let (raw_out, _) = eval.evaluate_batch_recurrent(
        &[EvalRequest {
            obs,
            mask: mask.clone(),
        }],
        &[root_state],
    );
    let first_eval = raw_out
        .into_iter()
        .next()
        .ok_or_else(|| "evaluator returned empty batch".to_string())?;
    let policy_value = first_eval.value;

    let policy_entropy: f32 = legal_ids
        .iter()
        .map(|&aid| {
            first_eval
                .policy
                .get(aid as usize)
                .copied()
                .unwrap_or(0.0)
        })
        .filter(|&p| p > 0.0)
        .map(|p| -p * p.ln())
        .sum::<f32>()
        .max(0.0);

    let mut scored_policy: Vec<(u16, f32)> = legal_ids
        .iter()
        .map(|&aid| {
            (
                aid,
                first_eval
                    .policy
                    .get(aid as usize)
                    .copied()
                    .unwrap_or(0.0),
            )
        })
        .collect();
    scored_policy.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });

    let top_k = params.top_k.max(1);
    let policy_actions: Vec<Value> = scored_policy
        .iter()
        .take(top_k)
        .map(|&(aid, prob)| {
            let label = explain_action(game, viewer, aid).label;
            json!({
                "action_id": aid,
                "label": label,
                "prob": prob,
            })
        })
        .collect();

    let mut chosen_action_id = scored_policy[0].0;
    let mut chosen_plan: Vec<u16> = vec![chosen_action_id];
    let mut mode = "policy_only";
    let mut search_value: Option<f32> = None;
    let mut simulations_run: u32 = 0;
    let mut root_plans_json: Vec<Value> = Vec::new();
    let mut root_q_json: Vec<Value> = Vec::new();
    let mut plan_shares: Vec<(Vec<u16>, f64)> = Vec::new();

    if params.simulations > 0 {
        mode = "search";
        let mut config = DeterminizedConfig::default();
        config.worlds = params.worlds.max(1);
        config.search.simulations = params.simulations;
        config.search.dirichlet_epsilon = 0.0;
        config.search.observation_profile = params.profile;
        config.search.seed = params.seed;

        let search_res = if params.regime == "perfect_info" {
            Mcts::new(registry).search(game, eval, &config.search)
        } else {
            determinized_search_batched(registry, game, viewer, eval, &config, 4)
        };
        search_value = Some(search_res.root_value);
        simulations_run = search_res.simulations_run;

        let total_visits: u32 = search_res.root_plans.iter().map(|p| p.1).sum();
        let mut plans = search_res.root_plans.clone();
        plans.sort_by(|a, b| b.1.cmp(&a.1));

        if let Some(best) = search_res.best_plan() {
            if !best.is_empty() {
                chosen_plan = best.to_vec();
                chosen_action_id = best[0];
            }
        }

        for (plan, visits, _q_val) in &plans {
            let share = if total_visits == 0 {
                0.0
            } else {
                *visits as f64 / total_visits as f64
            };
            plan_shares.push((plan.clone(), share));
        }

        for (plan, visits, q_val) in plans.iter().take(top_k) {
            let share = if total_visits == 0 {
                0.0
            } else {
                *visits as f64 / total_visits as f64
            };
            let prior_prob = search_res
                .root_priors
                .iter()
                .find(|(p, _, _)| p == plan)
                .map(|(_, raw_p, _)| *raw_p)
                .unwrap_or(0.0);
            let labels: Vec<String> = plan
                .iter()
                .enumerate()
                .map(|(idx, &aid)| {
                    if idx == 0 {
                        explain_action(game, viewer, aid).label
                    } else {
                        format!("Action {aid}")
                    }
                })
                .collect();
            root_plans_json.push(json!({
                "actions": plan,
                "labels": labels,
                "visits": visits,
                "share": share,
                "q_value": q_val,
                "prior_prob": prior_prob,
            }));
        }

        for &(aid, q) in &search_res.root_q {
            root_q_json.push(json!({
                "action_id": aid,
                "label": explain_action(game, viewer, aid).label,
                "q_value": q,
            }));
        }
    } else {
        for &(aid, prob) in &scored_policy {
            plan_shares.push((vec![aid], prob as f64));
        }
    }

    let expectation_results =
        evaluate_expectations(&params.expectations, &plan_shares, &chosen_plan, chosen_action_id);

    Ok(json!({
        "mode": mode,
        "regime": params.regime,
        "viewer": (viewer as u64) + 1,
        "legal_count": legal_ids.len(),
        "policy_value": policy_value,
        "policy_entropy": policy_entropy,
        "policy_actions": policy_actions,
        "search_value": search_value,
        "simulations_run": simulations_run,
        "root_plans": root_plans_json,
        "root_q": root_q_json,
        "chosen_action_id": chosen_action_id,
        "chosen_plan": chosen_plan,
        "chosen_label": explain_action(game, viewer, chosen_action_id).label,
        "expectation_results": expectation_results,
    }))
}

/// Grade Studio / search-exam expectations against observed plan visit shares and chosen plan.
pub fn evaluate_expectations(
    expectations: &[Value],
    plan_shares: &[(Vec<u16>, f64)],
    chosen_plan: &[u16],
    chosen_action_id: u16,
) -> Vec<Value> {
    let mut expectation_results = Vec::new();
    for exp in expectations {
        let kind = exp.get("kind").and_then(Value::as_str).unwrap_or("");
        let plan: Vec<u16> = exp
            .get("plan")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as u16))
                    .collect()
            })
            .unwrap_or_default();
        let observed_share = plan_shares
            .iter()
            .find(|(p, _)| *p == plan)
            .map(|(_, s)| *s)
            .unwrap_or(0.0);
        let (passed, detail) = match kind {
            "visits" | "visit_share_at_least" => {
                let min_share = exp
                    .get("min_share")
                    .or_else(|| exp.get("share"))
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let ok = observed_share >= min_share;
                (
                    ok,
                    format!(
                        "observed {:.1}% vs required >= {:.1}%",
                        observed_share * 100.0,
                        min_share * 100.0
                    ),
                )
            }
            "avoid" | "not_chosen" => {
                let ok =
                    chosen_plan != plan.as_slice() && !(plan.len() == 1 && plan[0] == chosen_action_id);
                (ok, format!("chosen={chosen_plan:?}, avoided={plan:?}"))
            }
            "choose" | "chosen" => {
                let ok =
                    chosen_plan == plan.as_slice() || (plan.len() == 1 && plan[0] == chosen_action_id);
                (ok, format!("chosen={chosen_plan:?}, expected={plan:?}"))
            }
            other => (false, format!("unknown expectation kind {other:?}")),
        };
        expectation_results.push(json!({
            "kind": kind,
            "plan": plan,
            "passed": passed,
            "observed_share": observed_share,
            "detail": detail,
        }));
    }
    expectation_results
}

/// Parameters for [`verify_belief_puzzle_with_evaluator`].
#[derive(Debug, Clone)]
pub struct VerifyBeliefParams {
    pub profile: ObservationProfileId,
    pub simulations: u32,
    pub worlds: u32,
    pub seeds: u64,
    pub base_seed: u64,
    pub batch: usize,
    pub expectations: Vec<Value>,
}

impl Default for VerifyBeliefParams {
    fn default() -> Self {
        Self {
            profile: ObservationProfileId::StandardLiteV2,
            simulations: 128,
            worlds: 16,
            seeds: 4,
            base_seed: 0,
            batch: 4,
            expectations: Vec::new(),
        }
    }
}

/// Parse a `"prior"` JSON object (`format_id`, `population`, `candidates`,
/// `unseen_mass`, `fallback_attempts`) into a validated [`WeightedDeckPrior`].
pub fn parse_prior_from_json(
    prior_val: &Value,
    provenance: &str,
) -> Result<WeightedDeckPrior, String> {
    let obj = prior_val
        .as_object()
        .ok_or_else(|| "puzzle 'prior' must be a JSON object".to_string())?;
    let format_id = obj
        .get("format_id")
        .and_then(Value::as_str)
        .unwrap_or("standard");
    let pop_val = obj
        .get("population")
        .ok_or_else(|| "prior missing 'population' array".to_string())?;
    let population: BTreeSet<String> = serde_json::from_value(pop_val.clone())
        .map_err(|e| format!("invalid prior.population: {e}"))?;
    let cand_val = obj
        .get("candidates")
        .ok_or_else(|| "prior missing 'candidates' array".to_string())?;
    let candidates: Vec<CandidateDeck> = serde_json::from_value(cand_val.clone())
        .map_err(|e| format!("invalid prior.candidates: {e}"))?;
    let unseen_mass = obj
        .get("unseen_mass")
        .and_then(Value::as_f64)
        .unwrap_or(0.02);
    let fallback_attempts = obj
        .get("fallback_attempts")
        .and_then(Value::as_u64)
        .unwrap_or(16) as usize;
    let snapshot = PriorSnapshot::new(
        format_id,
        provenance,
        population,
        candidates,
        unseen_mass,
        fallback_attempts,
    )?;
    WeightedDeckPrior::new(snapshot)
}

fn format_plan_label(game: &Game, viewer: PlayerId, plan: &[u16]) -> String {
    if plan.is_empty() {
        return String::new();
    }
    if plan.len() == 1 {
        return explain_action(game, viewer, plan[0]).label;
    }
    let mut labels = Vec::with_capacity(plan.len());
    for (idx, &aid) in plan.iter().enumerate() {
        if idx == 0 {
            labels.push(explain_action(game, viewer, aid).label);
        } else {
            labels.push(format!("Action {aid}"));
        }
    }
    labels.join(" -> ")
}

/// Evaluate an imperfect-information position under both `closed-list` PIMC
/// (`KnowledgeLedger` + [`WeightedDeckPrior`] via [`materialize_closed_with_diagnostics`])
/// and `open-list` PIMC across `params.seeds` seeds and `params.worlds` worlds,
/// reporting `closed_value_mean`, `open_value_mean`, `information_gap = open_value_mean - closed_value_mean`,
/// `closed_visit_shares`, and `expectation_results`.
pub fn verify_belief_puzzle_with_evaluator(
    game: &Game,
    registry: &CardRegistry,
    eval: &mut dyn PolicyValueFn,
    viewer: PlayerId,
    prior: &WeightedDeckPrior,
    params: &VerifyBeliefParams,
) -> Result<Value, String> {
    if game.game_over {
        return Err("Game is already over".to_string());
    }
    if decision_player(game) != viewer {
        return Err(format!(
            "decision_player ({}) does not match belief puzzle viewer ({viewer})",
            decision_player(game)
        ));
    }
    let memory = game.belief_knowledge(viewer).map_err(|e| {
        format!(
            "belief_knowledge unavailable for viewer {viewer} (requires Turn-1 legal action prefix with enable_belief_knowledge()): {e}"
        )
    })?;

    let seeds = params.seeds.max(1);
    let worlds = params.worlds.max(1);
    let simulations = params.simulations.max(1);
    let batch = params.batch.max(1);

    let mut closed_val_sum = 0.0f64;
    let mut open_val_sum = 0.0f64;
    let mut plan_stats: BTreeMap<Vec<u16>, (f64, f64, u64)> = BTreeMap::new();
    let mut exp_pass_counts = vec![0u64; params.expectations.len()];
    let mut exp_obs_share_sum = vec![0.0f64; params.expectations.len()];
    let mut exp_first_detail = vec![String::new(); params.expectations.len()];

    for k in 0..seeds {
        let mut config = DeterminizedConfig::default();
        config.worlds = worlds;
        config.search.simulations = simulations;
        config.search.dirichlet_epsilon = 0.0;
        config.search.observation_profile = params.profile;
        config.search.seed = params.base_seed.wrapping_add(k.wrapping_mul(0x5851f42d4c957f2d));

        let closed_res = determinized_search_sampled(
            registry,
            game,
            viewer,
            eval,
            &config,
            batch,
            &mut |w_seed| {
                materialize_closed_with_diagnostics(game, viewer, memory, prior, w_seed)
                    .map(|(w, _)| w)
            },
        )?;
        let open_res = determinized_search_batched(registry, game, viewer, eval, &config, batch);

        closed_val_sum += closed_res.root_value as f64;
        open_val_sum += open_res.root_value as f64;

        let total_visits: u32 = closed_res.root_plans.iter().map(|p| p.1).sum();
        let mut seed_plan_shares = Vec::with_capacity(closed_res.root_plans.len());
        for (plan, visits, q_val) in &closed_res.root_plans {
            let share = if total_visits == 0 {
                0.0
            } else {
                *visits as f64 / total_visits as f64
            };
            seed_plan_shares.push((plan.clone(), share));
            let entry = plan_stats.entry(plan.clone()).or_insert((0.0, 0.0, 0));
            entry.0 += share;
            entry.1 += *q_val as f64;
            entry.2 += 1;
        }

        let chosen_plan: Vec<u16> = closed_res
            .best_plan()
            .map(|p| p.to_vec())
            .unwrap_or_default();
        let chosen_action_id = chosen_plan.first().copied().unwrap_or(0);
        let seed_evals = evaluate_expectations(
            &params.expectations,
            &seed_plan_shares,
            &chosen_plan,
            chosen_action_id,
        );
        for (i, er) in seed_evals.iter().enumerate() {
            if er.get("passed").and_then(Value::as_bool) == Some(true) {
                exp_pass_counts[i] += 1;
            }
            exp_obs_share_sum[i] += er
                .get("observed_share")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            if k == 0 {
                exp_first_detail[i] = er
                    .get("detail")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
            }
        }
    }

    let closed_value_mean = (closed_val_sum / seeds as f64) as f32;
    let open_value_mean = (open_val_sum / seeds as f64) as f32;
    let information_gap = open_value_mean - closed_value_mean;

    let mut closed_visit_shares_vec: Vec<(Vec<u16>, f64, f32)> = plan_stats
        .into_iter()
        .map(|(plan, (share_sum, q_sum, q_cnt))| {
            let mean_share = share_sum / seeds as f64;
            let mean_q = if q_cnt > 0 {
                (q_sum / q_cnt as f64) as f32
            } else {
                0.0
            };
            (plan, mean_share, mean_q)
        })
        .collect();
    closed_visit_shares_vec.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });

    let closed_visit_shares: Vec<Value> = closed_visit_shares_vec
        .into_iter()
        .map(|(plan, mean_share, mean_q)| {
            let label = format_plan_label(game, viewer, &plan);
            json!({
                "plan": plan,
                "label": label,
                "mean_share": mean_share,
                "mean_q": mean_q,
            })
        })
        .collect();

    let mut expectation_results = Vec::with_capacity(params.expectations.len());
    for (i, exp) in params.expectations.iter().enumerate() {
        expectation_results.push(json!({
            "kind": exp.get("kind").and_then(Value::as_str).unwrap_or(""),
            "plan": exp.get("plan").cloned().unwrap_or_else(|| json!([])),
            "label": exp.get("label").and_then(Value::as_str).unwrap_or(""),
            "passed": exp_pass_counts[i] == seeds,
            "passed_seeds": exp_pass_counts[i],
            "total_seeds": seeds,
            "observed_share": exp_obs_share_sum[i] / seeds as f64,
            "detail": exp_first_detail[i],
        }));
    }

    let passed = expectation_results
        .iter()
        .all(|r| r.get("passed").and_then(Value::as_bool) == Some(true));

    Ok(json!({
        "ok": true,
        "passed": passed,
        "closed_value_mean": closed_value_mean,
        "open_value_mean": open_value_mean,
        "information_gap": information_gap,
        "closed_visit_shares": closed_visit_shares,
        "expectation_results": expectation_results,
    }))
}

/// Match `refutation_triggers` at an opponent interrupt / decision node (`decision_player != solver`).
pub fn match_refutation_trigger(
    game: &Game,
    mover: PlayerId,
    triggers: &[RefutationTrigger],
) -> Option<u16> {
    if triggers.is_empty() {
        return None;
    }
    let legal = legal_decoded_actions(game, mover);
    if legal.is_empty() {
        return None;
    }
    let phase_name = format!("{:?}", game.current_phase);
    let prompt_source_card_id = prompt_source_card_id(game);
    let non_pass: Vec<&ActionExplanation> =
        legal.iter().filter(|a| a.action_id != PASS).collect();

    for rule in triggers {
        if let Some(ref req_phase) = rule.phase {
            if req_phase != &phase_name {
                continue;
            }
        }
        let candidate_non_pass: Vec<&ActionExplanation> = match &rule.source_card_id {
            Some(src_id) => {
                let by_action: Vec<&ActionExplanation> = non_pass
                    .iter()
                    .copied()
                    .filter(|a| effective_action_card_id(game, a) == Some(src_id.as_str()))
                    .collect();
                if !by_action.is_empty() {
                    by_action
                } else if prompt_source_card_id.as_deref() == Some(src_id.as_str()) {
                    non_pass.clone()
                } else {
                    continue;
                }
            }
            None => non_pass.clone(),
        };

        if let Some(ref sub) = rule.prefer_label_contains {
            let needle = sub.to_lowercase();
            if !needle.is_empty() {
                if let Some(found) = candidate_non_pass
                    .iter()
                    .find(|a| a.label.to_lowercase().contains(&needle))
                {
                    return Some(found.action_id);
                }
            }
        }

        if rule.accept_optional {
            if let Some(first) = candidate_non_pass.first() {
                return Some(first.action_id);
            }
        } else if legal.iter().any(|a| a.action_id == PASS) {
            return Some(PASS);
        }
    }
    None
}

fn prompt_source_card_id(game: &Game) -> Option<String> {
    let sel = game.pending_selection.as_ref()?;
    if let Some(cd) = game.card_data_for_handle(sel.source_card) {
        return Some(cd.card_id.clone());
    }
    let handle = sel.source_permanent?;
    let perm = game.player(handle.player).battle_area.get(handle.index as usize)?;
    Some(perm.top_card().card_id(&game.card_data).to_string())
}

fn effective_action_card_id<'a>(game: &'a Game, act: &'a ActionExplanation) -> Option<&'a str> {
    if let Some(sel) = game.pending_selection.as_ref() {
        if let Some(choices) = sel.effect_choices.as_ref() {
            if let Some(entry) = choices.iter().find(|c| c.action_id == act.action_id) {
                if let Some(ch) = entry.source_card {
                    if let Some(cd) = game.card_data_for_handle(ch) {
                        return Some(cd.card_id.as_str());
                    }
                }
            }
        }
    }
    act.card_id.as_deref()
}

/// Step a cloned `Game` through `params.actions` (preserving the caller's live
/// game and evaluator root recurrent states), resolving opponent interrupts
/// (`decision_player != root_mover`) either from `params.actions` or
/// automatically via `refutation_triggers` / bounded adversary MCTS.
pub fn score_line_with_evaluator(
    game: &Game,
    registry: &CardRegistry,
    eval: &mut dyn PolicyValueFn,
    profile: ObservationProfileId,
    params: &ScoreLineParams,
) -> Result<ScoreLineResult, String> {
    let mut g = game.clone();
    let root_mover = decision_player(&g);
    let mut line_states = [eval.root_recurrent_state(0), eval.root_recurrent_state(1)];

    let root_policy_value = if g.game_over {
        terminal_value_for(&g, root_mover)
    } else {
        let mask = build_action_mask(&g, root_mover);
        let obs = build_observation_tensor_with_mask(&g, root_mover, registry, profile, &mask);
        let st_in = line_states.get(root_mover as usize).and_then(|s| s.clone());
        let (ev, _) = eval.evaluate_batch_recurrent(&[EvalRequest { obs, mask }], &[st_in]);
        ev.first().map(|r| r.value).unwrap_or(0.0)
    };

    let use_auto_adversary = params.auto_adversary
        || params.adversary_sims > 0
        || !params.refutation_triggers.is_empty();
    let max_adv = if params.max_adversary_steps == 0 {
        64
    } else {
        params.max_adversary_steps
    };

    let mut action_cursor = 0usize;
    let mut adv_steps = 0usize;
    let mut steps = Vec::new();
    let hard_cap = params.actions.len().saturating_add(max_adv).saturating_add(64);

    while !g.game_over && steps.len() < hard_cap {
        let mover = decision_player(&g);
        let mask = build_action_mask(&g, mover);
        let legal_ids: Vec<u16> = mask
            .iter()
            .enumerate()
            .filter_map(|(i, &m)| (m > 0.5).then_some(i as u16))
            .collect();
        if legal_ids.is_empty() {
            break;
        }

        let pre_obs = build_observation_tensor_with_mask(&g, mover, registry, profile, &mask);
        let st_in = line_states.get(mover as usize).and_then(|s| s.clone());
        let (pre_ev, st_out) = eval.evaluate_batch_recurrent(
            &[EvalRequest {
                obs: pre_obs,
                mask: mask.clone(),
            }],
            &[st_in],
        );
        let pre_row = pre_ev
            .into_iter()
            .next()
            .ok_or_else(|| "evaluator returned empty batch during score_line".to_string())?;

        let (action_id, is_auto_adv) = if mover == root_mover {
            if action_cursor >= params.actions.len() {
                break;
            }
            let aid = params.actions[action_cursor];
            action_cursor += 1;
            (aid, false)
        } else if !use_auto_adversary {
            if action_cursor >= params.actions.len() {
                break;
            }
            let aid = params.actions[action_cursor];
            action_cursor += 1;
            (aid, false)
        } else {
            if adv_steps >= max_adv {
                break;
            }
            adv_steps += 1;
            let chosen = choose_adversary_action(
                &g,
                mover,
                &mask,
                &legal_ids,
                &pre_row.policy,
                registry,
                eval,
                profile,
                params,
                steps.len(),
            );
            (chosen, true)
        };

        if (action_id as usize) >= mask.len() || mask[action_id as usize] <= 0.5 {
            return Err(format!(
                "action {} is not legal for player {} in phase {:?} at step {}",
                action_id,
                mover,
                g.current_phase,
                steps.len()
            ));
        }

        if legal_ids.len() > 1 {
            if let Some(next_st) = st_out.into_iter().next().flatten() {
                if let Some(slot) = line_states.get_mut(mover as usize) {
                    *slot = Some(next_st);
                }
            }
        }

        let prior_prob = pre_row
            .policy
            .get(action_id as usize)
            .copied()
            .unwrap_or(0.0);
        let phase = format!("{:?}", g.current_phase);
        let label = explain_action(&g, mover, action_id).label;
        let before_seq = g.event_seq;

        g.decode_action(action_id, mover);

        let events_emitted = collect_events_since(&g, before_seq);
        let policy_value_for_root_mover = if g.game_over {
            terminal_value_for(&g, root_mover)
        } else {
            let next_mover = decision_player(&g);
            let next_mask = build_action_mask(&g, next_mover);
            if next_mask.iter().any(|&m| m > 0.5) {
                let next_obs = build_observation_tensor_with_mask(
                    &g,
                    next_mover,
                    registry,
                    profile,
                    &next_mask,
                );
                let next_st = line_states.get(next_mover as usize).and_then(|s| s.clone());
                let (post_ev, _) = eval.evaluate_batch_recurrent(
                    &[EvalRequest {
                        obs: next_obs,
                        mask: next_mask,
                    }],
                    &[next_st],
                );
                let raw_v = post_ev.first().map(|r| r.value).unwrap_or(0.0);
                if next_mover == root_mover {
                    raw_v
                } else {
                    -raw_v
                }
            } else {
                0.0
            }
        };

        let step_index = steps.len();
        steps.push(ScoreLineStep {
            step_index,
            mover,
            phase,
            action_id,
            label,
            auto_adversary: is_auto_adv,
            policy_value_for_root_mover,
            prior_prob,
            events_emitted,
        });
    }

    let final_winner = g.winner;
    Ok(ScoreLineResult {
        root_mover,
        root_policy_value,
        steps,
        final_winner,
        final_game: g,
    })
}

#[allow(clippy::too_many_arguments)]
fn choose_adversary_action(
    game: &Game,
    mover: PlayerId,
    mask: &[f32],
    legal_ids: &[u16],
    prior_policy: &[f32],
    registry: &CardRegistry,
    eval: &mut dyn PolicyValueFn,
    profile: ObservationProfileId,
    params: &ScoreLineParams,
    step_index: usize,
) -> u16 {
    if legal_ids.len() == 1 {
        return legal_ids[0];
    }
    if let Some(trig_action) = match_refutation_trigger(game, mover, &params.refutation_triggers) {
        return trig_action;
    }
    if params.adversary_sims > 0 {
        let mut cfg = SearchConfig::default();
        cfg.simulations = params.adversary_sims;
        cfg.dirichlet_epsilon = 0.0;
        cfg.observation_profile = profile;
        cfg.seed = params
            .adversary_seed
            .wrapping_add((step_index as u64).wrapping_mul(0x9E3779B97F4A7C15));
        let search_res = Mcts::new(registry).search(game, eval, &cfg);
        if let Some(best) = search_res.best_plan().and_then(|p| p.first().copied()) {
            if mask.get(best as usize).copied().unwrap_or(0.0) > 0.5 {
                return best;
            }
        }
    }

    let greedy = greedy_action(game, mask);
    let mut best_aid = legal_ids[0];
    let mut best_prob = -1.0f32;
    let mut all_equal = true;
    let first_prob = prior_policy
        .get(legal_ids[0] as usize)
        .copied()
        .unwrap_or(0.0);
    for &aid in legal_ids {
        let p = prior_policy.get(aid as usize).copied().unwrap_or(0.0);
        if (p - first_prob).abs() > 1e-6 {
            all_equal = false;
        }
        if p > best_prob {
            best_prob = p;
            best_aid = aid;
        }
    }
    if all_equal && mask.get(greedy as usize).copied().unwrap_or(0.0) > 0.5 {
        greedy
    } else {
        best_aid
    }
}

fn terminal_value_for(game: &Game, perspective: PlayerId) -> f32 {
    match game.winner {
        Some(w) if w == perspective => 1.0,
        Some(_) => -1.0,
        None => 0.0,
    }
}

fn collect_events_since(game: &Game, before_seq: u64) -> Vec<Value> {
    let total = game.event_seq;
    let delta = total.saturating_sub(before_seq) as usize;
    let len = game.events.len();
    let start = len.saturating_sub(delta);
    game.events[start..]
        .iter()
        .map(|e: &GameEvent| {
            serde_json::to_value(e).unwrap_or_else(|err| {
                json!({
                    "type": "SerializationError",
                    "error": err.to_string(),
                    "debug": format!("{e:?}"),
                })
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card_data::CardData;
    use crate::rules::Rules;
    use crate::search::{Evaluation, RecurrentState};

    struct MockRecurrentEval {
        root: [Option<RecurrentState>; 2],
        seen_step_counters: Vec<f32>,
    }

    impl MockRecurrentEval {
        fn new() -> Self {
            let init = |v: f32| {
                Some(RecurrentState {
                    h_pi: vec![v],
                    c_pi: vec![0.0],
                    h_vf: vec![v],
                    c_vf: vec![0.0],
                })
            };
            Self {
                root: [init(10.0), init(20.0)],
                seen_step_counters: Vec::new(),
            }
        }
    }

    impl PolicyValueFn for MockRecurrentEval {
        fn evaluate_batch(&mut self, requests: &[EvalRequest]) -> Vec<Evaluation> {
            let (ev, _) = self.evaluate_batch_recurrent(requests, &vec![None; requests.len()]);
            ev
        }

        fn evaluate_batch_recurrent(
            &mut self,
            requests: &[EvalRequest],
            states_in: &[Option<RecurrentState>],
        ) -> (Vec<Evaluation>, Vec<Option<RecurrentState>>) {
            let mut evals = Vec::with_capacity(requests.len());
            let mut next_states = Vec::with_capacity(requests.len());
            for (i, req) in requests.iter().enumerate() {
                let h_val = states_in
                    .get(i)
                    .and_then(|s| s.as_ref())
                    .and_then(|s| s.h_pi.first().copied())
                    .unwrap_or(0.0);
                self.seen_step_counters.push(h_val);
                let legal = req.mask.iter().filter(|&&m| m > 0.5).count();
                let p = if legal > 0 { 1.0 / legal as f32 } else { 0.0 };
                let policy = req
                    .mask
                    .iter()
                    .map(|&m| if m > 0.5 { p } else { 0.0 })
                    .collect();
                evals.push(Evaluation {
                    policy,
                    value: (h_val * 0.01).clamp(-1.0, 1.0),
                });
                next_states.push(Some(RecurrentState {
                    h_pi: vec![h_val + 1.0],
                    c_pi: vec![0.0],
                    h_vf: vec![h_val + 1.0],
                    c_vf: vec![0.0],
                }));
            }
            (evals, next_states)
        }

        fn is_recurrent(&self) -> bool {
            true
        }

        fn root_recurrent_state(&self, player: PlayerId) -> Option<RecurrentState> {
            self.root.get(player as usize).and_then(|s| s.clone())
        }
    }

    fn make_simple_game() -> (Game, CardRegistry) {
        let cards_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/cards.json");
        let cards = CardData::load_from_file(&cards_path).expect("load cards.json");
        let creg = CardRegistry::from_cards(&cards);
        let mut deck = vec!["ST1-01".to_string(); 4];
        deck.extend(vec!["ST1-03".to_string(); 50]);
        let mut g =
            Game::new(&[deck.clone(), deck], &cards, Rules::standard(), Some(42)).unwrap();
        while let Some(p) = g.mulligan_current_player() {
            let _ = g.accept_mulligan(p, true);
        }
        let _ = g.enable_belief_knowledge();
        (g, creg)
    }

    #[test]
    fn load_inspector_evaluator_reads_mlp_lite_fixture() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/mlp_lite_value.onnx");
        let (_, info) = load_inspector_evaluator(&fixture, None).expect("load fixture");
        assert_eq!(info.kind, "mlp");
        assert_eq!(info.profile, ObservationProfileId::StandardLiteV2);
        assert_eq!(info.value_semantics, "shaped_return");
    }

    #[test]
    fn score_line_threads_recurrent_state_without_mutating_root_or_game() {
        let (game, creg) = make_simple_game();
        let mut eval = MockRecurrentEval::new();
        let orig_root_0 = eval.root_recurrent_state(0);
        let orig_root_1 = eval.root_recurrent_state(1);
        let orig_turn = game.turn_count;
        let orig_phase = game.current_phase;

        let insp = inspect_position_with_evaluator(
            &game,
            &creg,
            &mut eval,
            &InspectPositionParams::default(),
        )
        .expect("inspect_position_with_evaluator");
        assert_eq!(insp["mode"], "policy_only");
        assert!(insp["policy_entropy"].as_f64().unwrap() >= 0.0);

        let mover = decision_player(&game);
        let mask = build_action_mask(&game, mover);
        let first_legal = mask
            .iter()
            .position(|&m| m > 0.5)
            .expect("at least one legal action") as u16;

        let res = score_line_with_evaluator(
            &game,
            &creg,
            &mut eval,
            ObservationProfileId::StandardLiteV2,
            &ScoreLineParams {
                actions: vec![first_legal],
                auto_adversary: true,
                adversary_sims: 0,
                adversary_seed: 0,
                refutation_triggers: Vec::new(),
                max_adversary_steps: 4,
            },
        )
        .expect("score_line_with_evaluator");

        assert!(!res.steps.is_empty());
        // Live game and evaluator root states are untouched.
        assert_eq!(game.turn_count, orig_turn);
        assert_eq!(game.current_phase, orig_phase);
        assert_eq!(eval.root_recurrent_state(0), orig_root_0);
        assert_eq!(eval.root_recurrent_state(1), orig_root_1);
    }
}

