//! Batched recurrent policy+value evaluator for MCTS search.
//!
//! Loads an exported actor LSTM (`<stem>.onnx`: `(obs, h_in, c_in) -> (logits, h_out, c_out)`)
//! alongside its companion critic LSTM (`<stem>.value.onnx`: `(obs, h_in, c_in) -> (value, h_out, c_out)`),
//! allowing MCTS nodes to thread per-player `(h_pi, c_pi, h_vf, c_vf)` hidden states along
//! search paths while keeping the live game's per-seat root hidden states synced across turns.

use std::path::{Path, PathBuf};

use ndarray::{Array2, Array3};
use ort::session::Session;
use ort::value::Value;

use crate::enums::PlayerId;
use crate::search::RecurrentState;

use super::evaluator::{masked_softmax, EvalResult};
use super::InferenceError;

pub struct BatchedLstmPolicyValueEvaluator {
    actor_session: Session,
    critic_session: Session,
    obs_size: usize,
    hidden_size: usize,
    supports_dynamic_batch: bool,
    terminal_value: bool,
    root_states: [Option<RecurrentState>; 2],
}

impl BatchedLstmPolicyValueEvaluator {
    pub fn load_for_profile(
        actor_path: &Path,
        profile: crate::observation::ObservationProfileId,
    ) -> Result<Self, InferenceError> {
        let layout = crate::observation::observation_layout(profile);
        let critic_path = resolve_companion_value_path(actor_path)?;

        let actor_session = Session::builder()
            .map_err(|e| InferenceError::LoadFailed(e.to_string()))?
            .with_intra_threads(1)
            .map_err(|e| InferenceError::LoadFailed(e.to_string()))?
            .commit_from_file(actor_path)
            .map_err(|e| InferenceError::LoadFailed(e.to_string()))?;
        super::contract::validate_session_contract(&actor_session, &layout)?;

        let actor_outputs: Vec<String> = actor_session
            .outputs()
            .iter()
            .map(|o| o.name().to_string())
            .collect();
        if !(actor_outputs.iter().any(|n| n == "logits")
            && actor_outputs.iter().any(|n| n == "h_out")
            && actor_outputs.iter().any(|n| n == "c_out"))
        {
            return Err(InferenceError::UnsupportedOutputs(actor_outputs));
        }

        let critic_session = Session::builder()
            .map_err(|e| InferenceError::LoadFailed(e.to_string()))?
            .with_intra_threads(1)
            .map_err(|e| InferenceError::LoadFailed(e.to_string()))?
            .commit_from_file(&critic_path)
            .map_err(|e| {
                InferenceError::LoadFailed(format!(
                    "companion LSTM value graph {}: {e}",
                    critic_path.display()
                ))
            })?;
        super::contract::validate_value_session_contract(&critic_session, &layout)?;

        let critic_outputs: Vec<String> = critic_session
            .outputs()
            .iter()
            .map(|o| o.name().to_string())
            .collect();
        if !(critic_outputs.iter().any(|n| n == "value")
            && critic_outputs.iter().any(|n| n == "h_out")
            && critic_outputs.iter().any(|n| n == "c_out"))
        {
            return Err(InferenceError::UnsupportedOutputs(critic_outputs));
        }

        let (actor_h, actor_dyn) = inspect_h_in(&actor_session)?;
        let (critic_h, critic_dyn) = inspect_h_in(&critic_session)?;
        if actor_h != critic_h {
            return Err(InferenceError::ShapeMismatch(format!(
                "actor hidden_size {actor_h} != critic hidden_size {critic_h}"
            )));
        }

        let has_terminal_meta = |s: &Session| {
            s.metadata()
                .ok()
                .and_then(|m| m.custom("digimon_value_semantics"))
                .is_some_and(|v| v == "terminal_outcome")
        };
        let terminal_value = has_terminal_meta(&critic_session) || has_terminal_meta(&actor_session);

        Ok(Self {
            actor_session,
            critic_session,
            obs_size: layout.tensor_size,
            hidden_size: actor_h,
            supports_dynamic_batch: actor_dyn && critic_dyn,
            terminal_value,
            root_states: [None, None],
        })
    }

    pub fn require_terminal_value(&self) -> Result<(), InferenceError> {
        if self.terminal_value {
            Ok(())
        } else {
            Err(InferenceError::LoadFailed(
                "search self-play requires terminal_outcome value calibration; shaped-return PPO values are not compatible"
                    .into(),
            ))
        }
    }

    pub fn hidden_size(&self) -> usize {
        self.hidden_size
    }

    pub fn root_state(&self, player: PlayerId) -> Option<RecurrentState> {
        self.root_states.get(player as usize).and_then(|s| s.clone())
    }

    pub fn reset_episode(&mut self) {
        self.root_states = [None, None];
    }

    pub fn observe_committed_step(
        &mut self,
        player: PlayerId,
        obs: &[f32],
        mask: &[f32],
    ) -> Result<(), InferenceError> {
        let idx = player as usize;
        if idx >= 2 {
            return Ok(());
        }
        let prev = self.root_states[idx].clone();
        let (_, mut outs) = self.evaluate_batch_recurrent(&[obs], &[mask], &[prev])?;
        self.root_states[idx] = outs.pop().flatten();
        Ok(())
    }

    pub fn evaluate_batch_recurrent(
        &mut self,
        obs: &[&[f32]],
        masks: &[&[f32]],
        states_in: &[Option<RecurrentState>],
    ) -> Result<(Vec<EvalResult>, Vec<Option<RecurrentState>>), InferenceError> {
        let _profile = crate::search::profiling::span("inference_total");
        if obs.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        if obs.len() != masks.len() || obs.len() != states_in.len() {
            return Err(InferenceError::ShapeMismatch(format!(
                "obs/masks/states batch mismatch: {}/{}/{}",
                obs.len(),
                masks.len(),
                states_in.len()
            )));
        }

        if !self.supports_dynamic_batch && obs.len() > 1 {
            let mut results = Vec::with_capacity(obs.len());
            let mut states = Vec::with_capacity(obs.len());
            for i in 0..obs.len() {
                let (mut r, mut s) =
                    self.run_chunk(&obs[i..=i], &masks[i..=i], &states_in[i..=i])?;
                results.push(r.pop().unwrap());
                states.push(s.pop().unwrap());
            }
            return Ok((results, states));
        }

        self.run_chunk(obs, masks, states_in)
    }

    fn run_chunk(
        &mut self,
        obs: &[&[f32]],
        masks: &[&[f32]],
        states_in: &[Option<RecurrentState>],
    ) -> Result<(Vec<EvalResult>, Vec<Option<RecurrentState>>), InferenceError> {
        let batch = obs.len();
        let width = obs[0].len();
        if width != self.obs_size {
            return Err(InferenceError::ShapeMismatch(format!(
                "obs width {width} != selected profile width {}",
                self.obs_size
            )));
        }

        let mut flat_obs = Vec::with_capacity(batch * width);
        for row in obs {
            if row.len() != width {
                return Err(InferenceError::ShapeMismatch(format!(
                    "ragged obs batch: {} vs {width}",
                    row.len()
                )));
            }
            flat_obs.extend_from_slice(row);
        }

        let h_dim = self.hidden_size;
        let mut flat_h_pi = vec![0.0f32; batch * h_dim];
        let mut flat_c_pi = vec![0.0f32; batch * h_dim];
        let mut flat_h_vf = vec![0.0f32; batch * h_dim];
        let mut flat_c_vf = vec![0.0f32; batch * h_dim];
        for (i, st) in states_in.iter().enumerate() {
            if let Some(s) = st {
                if s.h_pi.len() != h_dim
                    || s.c_pi.len() != h_dim
                    || s.h_vf.len() != h_dim
                    || s.c_vf.len() != h_dim
                {
                    return Err(InferenceError::ShapeMismatch(format!(
                        "recurrent state width mismatch (expected {h_dim})"
                    )));
                }
                let off = i * h_dim;
                flat_h_pi[off..off + h_dim].copy_from_slice(&s.h_pi);
                flat_c_pi[off..off + h_dim].copy_from_slice(&s.c_pi);
                flat_h_vf[off..off + h_dim].copy_from_slice(&s.h_vf);
                flat_c_vf[off..off + h_dim].copy_from_slice(&s.c_vf);
            }
        }

        let obs_arr_pi = Array2::from_shape_vec((batch, width), flat_obs.clone())
            .map_err(|e| InferenceError::ShapeMismatch(e.to_string()))?;
        let h_pi_arr = Array3::from_shape_vec((1, batch, h_dim), flat_h_pi)
            .map_err(|e| InferenceError::ShapeMismatch(e.to_string()))?;
        let c_pi_arr = Array3::from_shape_vec((1, batch, h_dim), flat_c_pi)
            .map_err(|e| InferenceError::ShapeMismatch(e.to_string()))?;

        let obs_arr_vf = Array2::from_shape_vec((batch, width), flat_obs)
            .map_err(|e| InferenceError::ShapeMismatch(e.to_string()))?;
        let h_vf_arr = Array3::from_shape_vec((1, batch, h_dim), flat_h_vf)
            .map_err(|e| InferenceError::ShapeMismatch(e.to_string()))?;
        let c_vf_arr = Array3::from_shape_vec((1, batch, h_dim), flat_c_vf)
            .map_err(|e| InferenceError::ShapeMismatch(e.to_string()))?;

        let onnx_span = crate::search::profiling::span("onnx_run");
        let actor_out = self
            .actor_session
            .run(ort::inputs![
                "obs" => Value::from_array(obs_arr_pi).map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?,
                "h_in" => Value::from_array(h_pi_arr).map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?,
                "c_in" => Value::from_array(c_pi_arr).map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?,
            ])
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;

        let critic_out = self
            .critic_session
            .run(ort::inputs![
                "obs" => Value::from_array(obs_arr_vf).map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?,
                "h_in" => Value::from_array(h_vf_arr).map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?,
                "c_in" => Value::from_array(c_vf_arr).map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?,
            ])
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;
        drop(onnx_span);

        let (logits_shape, logits) = actor_out
            .get("logits")
            .ok_or_else(|| InferenceError::UnsupportedOutputs(vec!["logits".into()]))?
            .try_extract_tensor::<f32>()
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;
        let (_, h_pi_out) = actor_out
            .get("h_out")
            .ok_or_else(|| InferenceError::UnsupportedOutputs(vec!["h_out".into()]))?
            .try_extract_tensor::<f32>()
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;
        let (_, c_pi_out) = actor_out
            .get("c_out")
            .ok_or_else(|| InferenceError::UnsupportedOutputs(vec!["c_out".into()]))?
            .try_extract_tensor::<f32>()
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;

        let (_, values) = critic_out
            .get("value")
            .ok_or_else(|| InferenceError::UnsupportedOutputs(vec!["value".into()]))?
            .try_extract_tensor::<f32>()
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;
        let (_, h_vf_out) = critic_out
            .get("h_out")
            .ok_or_else(|| InferenceError::UnsupportedOutputs(vec!["h_out".into()]))?
            .try_extract_tensor::<f32>()
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;
        let (_, c_vf_out) = critic_out
            .get("c_out")
            .ok_or_else(|| InferenceError::UnsupportedOutputs(vec!["c_out".into()]))?
            .try_extract_tensor::<f32>()
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;

        let action_size = *logits_shape
            .last()
            .ok_or_else(|| InferenceError::ShapeMismatch("logits output has no shape".into()))?
            as usize;
        if logits.len() != batch * action_size {
            return Err(InferenceError::ShapeMismatch(format!(
                "logits len {} != batch {} * action_size {}",
                logits.len(),
                batch,
                action_size
            )));
        }
        if values.len() != batch {
            return Err(InferenceError::ShapeMismatch(format!(
                "values len {} != batch {}",
                values.len(),
                batch
            )));
        }
        if h_pi_out.len() != batch * h_dim
            || c_pi_out.len() != batch * h_dim
            || h_vf_out.len() != batch * h_dim
            || c_vf_out.len() != batch * h_dim
        {
            return Err(InferenceError::ShapeMismatch(format!(
                "recurrent output len mismatch for batch {batch}, hidden {h_dim}"
            )));
        }

        let mut results = Vec::with_capacity(batch);
        let mut states_out = Vec::with_capacity(batch);
        for i in 0..batch {
            let row_logits = &logits[i * action_size..(i + 1) * action_size];
            let policy = masked_softmax(row_logits, masks[i])?;
            let value = values[i];
            if !value.is_finite() {
                return Err(InferenceError::RuntimeFailed(
                    "non-finite LSTM value output".into(),
                ));
            }
            results.push(EvalResult { policy, value });
            let off = i * h_dim;
            states_out.push(Some(RecurrentState {
                h_pi: h_pi_out[off..off + h_dim].to_vec(),
                c_pi: c_pi_out[off..off + h_dim].to_vec(),
                h_vf: h_vf_out[off..off + h_dim].to_vec(),
                c_vf: c_vf_out[off..off + h_dim].to_vec(),
            }));
        }
        Ok((results, states_out))
    }
}

fn resolve_companion_value_path(actor_path: &Path) -> Result<PathBuf, InferenceError> {
    let default_path = actor_path.with_extension("value.onnx");
    if default_path.exists() {
        return Ok(default_path);
    }
    let meta_path = PathBuf::from(format!("{}.meta.json", actor_path.display()));
    if meta_path.exists() {
        if let Ok(raw) = std::fs::read_to_string(&meta_path) {
            if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(name) = meta.get("value_head").and_then(|v| v.as_str()) {
                    if name != "inline" {
                        let candidate = actor_path
                            .parent()
                            .unwrap_or_else(|| Path::new("."))
                            .join(name);
                        if candidate.exists() {
                            return Ok(candidate);
                        }
                    }
                }
            }
        }
    }
    Err(InferenceError::LoadFailed(format!(
        "LSTM search requires companion value graph (expected {})",
        default_path.display()
    )))
}

fn inspect_h_in(session: &Session) -> Result<(usize, bool), InferenceError> {
    let h_in = session
        .inputs()
        .iter()
        .find(|i| i.name() == "h_in")
        .ok_or_else(|| InferenceError::ShapeMismatch("LSTM model missing 'h_in' input".into()))?;
    let shape = h_in
        .dtype()
        .tensor_shape()
        .ok_or_else(|| InferenceError::ShapeMismatch("h_in has no tensor shape".into()))?;
    if shape.len() != 3 {
        return Err(InferenceError::ShapeMismatch(format!(
            "h_in expected rank 3, got {shape:?}"
        )));
    }
    let hidden = shape[2];
    if hidden <= 0 {
        return Err(InferenceError::ShapeMismatch(format!(
            "h_in hidden dim must be positive, got {hidden}"
        )));
    }
    let dynamic_batch = shape[1] <= 0;
    Ok((hidden as usize, dynamic_batch))
}

/// Inspect an ONNX file to determine whether it is a recurrent LSTM graph
/// (carries `h_in` + `c_in` inputs) or a stateless MLP graph.
pub fn is_lstm_onnx(path: &Path) -> Result<bool, InferenceError> {
    let session = Session::builder()
        .map_err(|e| InferenceError::LoadFailed(e.to_string()))?
        .with_intra_threads(1)
        .map_err(|e| InferenceError::LoadFailed(e.to_string()))?
        .commit_from_file(path)
        .map_err(|e| InferenceError::LoadFailed(e.to_string()))?;
    let has_h = session.inputs().iter().any(|i| i.name() == "h_in");
    let has_c = session.inputs().iter().any(|i| i.name() == "c_in");
    Ok(has_h && has_c)
}
