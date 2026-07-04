//! `winprob` subcommand — the review-v0 win-probability trace
//! (add-determinized-search task 0.5).
//!
//! Replays a native `GameRecorder` recording and, at every decision point,
//! evaluates the observation tensor of the player to move with a task-0.2
//! policy+value ONNX export (inline `value` output). Emits a sidecar JSON
//! with the per-decision value series + the policy prior of the played
//! action — the pre-search half of the task-5.1 annotation fields (grades
//! need search and land later).
//!
//! Value semantics: the SB3 value head predicts the expected DISCOUNTED
//! SHAPED RETURN for the viewer (the player to move), not a calibrated
//! win probability. The sidecar records this caveat.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use digimon_engine::card_data::CardData;
use digimon_engine::enums::PlayerId;
use digimon_engine::inference::BatchedPolicyValueEvaluator;
use digimon_engine::observation::{
    build_observation_tensor, default_observation_profile, parse_observation_profile,
    ObservationProfileId,
};
use digimon_engine::runners::ReplaySession;
use serde_json::{json, Value};

/// Resolve the observation profile the model expects. Precedence:
/// explicit `--tensor-profile` flag > the export's `<model>.meta.json`
/// (`observation_profile` field, written by `code/tools/export_onnx.py`) >
/// the engine default. Feeding a model tensors in a different profile than
/// it was exported with is an ONNX input-shape error at best and silent
/// garbage at worst.
fn resolve_profile(
    model_path: &std::path::Path,
    flag: Option<&str>,
) -> Result<ObservationProfileId, String> {
    if let Some(raw) = flag {
        return parse_observation_profile(raw);
    }
    let meta_path = std::path::PathBuf::from(format!("{}.meta.json", model_path.display()));
    if let Ok(text) = std::fs::read_to_string(&meta_path) {
        let meta: Value = serde_json::from_str(&text)
            .map_err(|e| format!("malformed {}: {e}", meta_path.display()))?;
        if let Some(raw) = meta.get("observation_profile").and_then(|v| v.as_str()) {
            return parse_observation_profile(raw).map_err(|e| {
                format!("{} names an unknown profile: {e}", meta_path.display())
            });
        }
    }
    eprintln!(
        "note: no --tensor-profile and no readable {} — using the engine default profile ({})",
        meta_path.display(),
        default_observation_profile().as_str()
    );
    Ok(default_observation_profile())
}

struct Decision {
    step: u64,
    turn: u64,
    phase: String,
    player_py: u64,
    action_id: u16,
    obs: Vec<f32>,
    mask: Vec<f32>,
    mask_empty: bool,
}

pub fn run(
    recording_path: PathBuf,
    model_path: PathBuf,
    out_path: Option<PathBuf>,
    batch: usize,
    tensor_profile: Option<String>,
    card_data: HashMap<String, CardData>,
) -> ExitCode {
    let profile = match resolve_profile(&model_path, tensor_profile.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error resolving tensor profile: {e}");
            return ExitCode::from(2);
        }
    };
    let text = match std::fs::read_to_string(&recording_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error reading recording {}: {}", recording_path.display(), e);
            return ExitCode::from(2);
        }
    };
    let mut recording: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error parsing recording JSON: {}", e);
            return ExitCode::from(2);
        }
    };
    // Eval sidecar envelope ({outcome, recording, run, ...}) — descend into
    // the wrapped GameRecorder payload.
    if recording.get("actions").is_none() {
        if let Some(inner) = recording.get("recording") {
            recording = inner.clone();
        }
    }
    let actions: Vec<Value> = match recording.get("actions").and_then(|a| a.as_array()) {
        Some(a) => a.clone(),
        None => {
            eprintln!("recording has no `actions` array (native GameRecorder format required)");
            return ExitCode::from(2);
        }
    };

    let mut evaluator = match BatchedPolicyValueEvaluator::load(&model_path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!(
                "error loading value-head model {} (needs a task-0.2 export with an inline `value` output): {}",
                model_path.display(),
                e
            );
            return ExitCode::from(2);
        }
    };

    let mut session = match ReplaySession::new(recording.clone(), &card_data, false) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error constructing replay session: {:?}", e);
            return ExitCode::from(2);
        }
    };
    // Same registry construction as HeadlessRunner (runners/headless.rs):
    // the tensor's card-index space comes from the full card-data map.
    let registry = digimon_engine::card_registry::CardRegistry::from_cards(&card_data);

    // Walk the recording: capture (obs, mask) for the mover BEFORE each
    // action is applied, then apply it. Mirror ReplaySession's replayable
    // filter exactly — mulligan rows are NOT replayed (initial_state is
    // post-mulligan), so they are not review decisions either; iterating
    // them would desync obs/mask attribution from the applied state.
    let replayable: Vec<&Value> = actions
        .iter()
        .filter(|a| a["phase"].as_str().unwrap_or("") != "Mulligan")
        .collect();
    let mut decisions: Vec<Decision> = Vec::with_capacity(replayable.len());
    for action in &replayable {
        let player_py = action["player_id"].as_u64().unwrap_or(1);
        let pid: PlayerId = player_py.saturating_sub(1) as PlayerId;
        let action_id = action["action_id"].as_u64().unwrap_or(0) as u16;
        let obs = build_observation_tensor(&session.game, pid, &registry, profile);
        let mut mask = digimon_engine::build_action_mask(&session.game, pid);
        let mask_empty = !mask.iter().any(|&m| m > 0.5);
        if mask_empty {
            // Shouldn't happen at a recorded decision; substitute uniform so
            // the evaluator's masked softmax stays defined, and flag it.
            mask.iter_mut().for_each(|m| *m = 1.0);
        }
        decisions.push(Decision {
            step: action["step"].as_u64().unwrap_or(0),
            turn: action["turn"].as_u64().unwrap_or(0),
            phase: action["phase"].as_str().unwrap_or("").to_string(),
            player_py,
            action_id,
            obs,
            mask,
            mask_empty,
        });
        let r = session.step();
        if r.is_game_over {
            break;
        }
    }

    // Batched evaluation.
    let mut values: Vec<f32> = Vec::with_capacity(decisions.len());
    let mut priors: Vec<f32> = Vec::with_capacity(decisions.len());
    for chunk in decisions.chunks(batch.max(1)) {
        let obs_refs: Vec<&[f32]> = chunk.iter().map(|d| d.obs.as_slice()).collect();
        let mask_refs: Vec<&[f32]> = chunk.iter().map(|d| d.mask.as_slice()).collect();
        match evaluator.evaluate_batch(&obs_refs, &mask_refs) {
            Ok(results) => {
                for (d, r) in chunk.iter().zip(results.iter()) {
                    values.push(r.value);
                    priors.push(r.policy.get(d.action_id as usize).copied().unwrap_or(0.0));
                }
            }
            Err(e) => {
                eprintln!("evaluation failed: {}", e);
                return ExitCode::from(2);
            }
        }
    }

    let winner_py = actions
        .iter()
        .rev()
        .find_map(|a| a.get("winner_id").and_then(|w| w.as_u64()));

    let out = out_path.unwrap_or_else(|| {
        let mut p = recording_path.clone();
        let stem = p
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "recording".into());
        p.set_file_name(format!("{stem}.winprob.json"));
        p
    });

    let records: Vec<Value> = decisions
        .iter()
        .zip(values.iter().zip(priors.iter()))
        .map(|(d, (v, p))| {
            let mut rec = json!({
                "step": d.step,
                "turn": d.turn,
                "phase": d.phase,
                "player_id": d.player_py,
                "action_id": d.action_id,
                "value": v,
                "played_prior": p,
            });
            if d.mask_empty {
                rec["mask_empty"] = json!(true);
            }
            rec
        })
        .collect();

    let sidecar = json!({
        "schema": "winprob-annotations-v1",
        "recording": recording_path.to_string_lossy(),
        "model": model_path.to_string_lossy(),
        "observation_profile": profile.as_str(),
        "value_semantics": "SB3 value head: expected discounted shaped return for the player to move (viewer-relative). NOT a calibrated win probability.",
        "winner_id": winner_py,
        "decisions": records,
    });
    match std::fs::write(&out, serde_json::to_string_pretty(&sidecar).unwrap()) {
        Ok(()) => {
            eprintln!(
                "wrote {} decisions ({} evaluated) to {}",
                decisions.len(),
                values.len(),
                out.display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error writing sidecar {}: {}", out.display(), e);
            ExitCode::from(2)
        }
    }
}
