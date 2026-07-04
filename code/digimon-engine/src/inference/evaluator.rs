//! Batched policy+value evaluator for search (add-determinized-search 0.3).
//!
//! Loads a task-0.2 value-head export (an MLP graph with `logits` AND
//! `value` outputs and a dynamic batch axis) and evaluates a whole batch
//! of observations in one ONNX run — the shape MCTS leaf evaluation wants
//! (collect pending leaves, evaluate together, back up). The per-row
//! output is a masked softmax policy (zero mass on illegal actions) plus
//! the scalar value for the player to move.

use std::path::Path;

use ndarray::Array2;
use ort::session::Session;
use ort::value::Value;

use super::InferenceError;

/// One row of a batched evaluation: a masked policy distribution over the
/// full action space and the value-head scalar.
#[derive(Debug, Clone)]
pub struct EvalResult {
    /// Softmax over legal actions only; illegal entries are exactly 0.0 and
    /// the legal entries sum to ~1.0.
    pub policy: Vec<f32>,
    pub value: f32,
}

/// Masked softmax: probability mass only on entries whose mask is set.
///
/// Max-subtracted over the LEGAL entries for numerical stability. Returns
/// `ShapeMismatch` if logits/mask lengths differ (library/CLI-facing code —
/// a stale model or a bad mask must not abort the process) or if no action
/// is legal (an empty mask is unreachable in a well-formed game).
pub fn masked_softmax(logits: &[f32], mask: &[f32]) -> Result<Vec<f32>, InferenceError> {
    if logits.len() != mask.len() {
        return Err(InferenceError::ShapeMismatch(format!(
            "masked_softmax: logits len {} != mask len {}",
            logits.len(),
            mask.len()
        )));
    }
    let mut max_legal = f32::NEG_INFINITY;
    for (&l, &m) in logits.iter().zip(mask.iter()) {
        if m >= 0.5 && l > max_legal {
            max_legal = l;
        }
    }
    if max_legal == f32::NEG_INFINITY {
        return Err(InferenceError::ShapeMismatch(
            "masked_softmax: no legal actions in mask".into(),
        ));
    }
    let mut out = vec![0.0f32; logits.len()];
    let mut denom = 0.0f32;
    for (i, (&l, &m)) in logits.iter().zip(mask.iter()).enumerate() {
        if m >= 0.5 {
            let e = (l - max_legal).exp();
            out[i] = e;
            denom += e;
        }
    }
    for v in &mut out {
        *v /= denom;
    }
    Ok(out)
}

/// Batched policy+value evaluator over an inline-value MLP export.
///
/// LSTM policies are intentionally unsupported here: threading per-node
/// recurrent state through a search tree is a different design problem
/// (and the LSTM value head lives in a companion graph — see export_onnx).
pub struct BatchedPolicyValueEvaluator {
    session: Session,
}

impl BatchedPolicyValueEvaluator {
    pub fn load(path: &Path) -> Result<Self, InferenceError> {
        let session = Session::builder()
            .map_err(|e| InferenceError::LoadFailed(e.to_string()))?
            .commit_from_file(path)
            .map_err(|e| InferenceError::LoadFailed(e.to_string()))?;
        let output_names: Vec<String> = session
            .outputs()
            .iter()
            .map(|o| o.name().to_string())
            .collect();
        let has_logits = output_names.iter().any(|n| n == "logits");
        let has_value = output_names.iter().any(|n| n == "value");
        if !(has_logits && has_value) {
            return Err(InferenceError::UnsupportedOutputs(output_names));
        }
        Ok(Self { session })
    }

    /// Evaluate a batch. `obs` rows must all have the same length; `masks`
    /// must be parallel to `obs` (one mask per observation, over the full
    /// action space).
    pub fn evaluate_batch(
        &mut self,
        obs: &[&[f32]],
        masks: &[&[f32]],
    ) -> Result<Vec<EvalResult>, InferenceError> {
        if obs.is_empty() {
            return Ok(Vec::new());
        }
        if obs.len() != masks.len() {
            return Err(InferenceError::ShapeMismatch(format!(
                "obs batch {} vs masks batch {}",
                obs.len(),
                masks.len()
            )));
        }
        let width = obs[0].len();
        let mut flat = Vec::with_capacity(obs.len() * width);
        for row in obs {
            if row.len() != width {
                return Err(InferenceError::ShapeMismatch(format!(
                    "ragged obs batch: {} vs {}",
                    row.len(),
                    width
                )));
            }
            flat.extend_from_slice(row);
        }
        let obs_arr = Array2::from_shape_vec((obs.len(), width), flat)
            .map_err(|e| InferenceError::ShapeMismatch(e.to_string()))?;
        let obs_value =
            Value::from_array(obs_arr).map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;
        let outputs = self
            .session
            .run(ort::inputs!["obs" => obs_value])
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;

        let (logits_shape, logits) = outputs
            .get("logits")
            .ok_or_else(|| InferenceError::UnsupportedOutputs(vec!["logits".into()]))?
            .try_extract_tensor::<f32>()
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;
        let (_value_shape, values) = outputs
            .get("value")
            .ok_or_else(|| InferenceError::UnsupportedOutputs(vec!["value".into()]))?
            .try_extract_tensor::<f32>()
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;

        let action_size = *logits_shape.last().ok_or_else(|| {
            InferenceError::ShapeMismatch("logits output has no shape".into())
        })? as usize;
        if logits.len() != obs.len() * action_size {
            return Err(InferenceError::ShapeMismatch(format!(
                "logits len {} != batch {} x actions {}",
                logits.len(),
                obs.len(),
                action_size
            )));
        }
        if values.len() != obs.len() {
            return Err(InferenceError::ShapeMismatch(format!(
                "value len {} != batch {}",
                values.len(),
                obs.len()
            )));
        }

        let mut results = Vec::with_capacity(obs.len());
        for (i, mask) in masks.iter().enumerate() {
            let row = &logits[i * action_size..(i + 1) * action_size];
            results.push(EvalResult {
                policy: masked_softmax(row, mask)?,
                value: values[i],
            });
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masked_softmax_zeroes_illegal_and_normalizes() {
        let logits = [1.0, 2.0, 3.0, 4.0];
        let mask = [1.0, 0.0, 1.0, 0.0];
        let p = masked_softmax(&logits, &mask).unwrap();
        assert_eq!(p[1], 0.0);
        assert_eq!(p[3], 0.0);
        let sum: f32 = p.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
        // Legal relative ordering preserved: logit 3.0 beats 1.0.
        assert!(p[2] > p[0]);
    }

    #[test]
    fn masked_softmax_single_legal_action_gets_all_mass() {
        let logits = [-5.0, 100.0, 3.0];
        let mask = [0.0, 0.0, 1.0];
        let p = masked_softmax(&logits, &mask).unwrap();
        assert_eq!(p, vec![0.0, 0.0, 1.0]);
    }

    #[test]
    fn masked_softmax_empty_mask_errors() {
        let logits = [1.0, 2.0];
        let mask = [0.0, 0.0];
        assert!(masked_softmax(&logits, &mask).is_err());
    }

    #[test]
    fn masked_softmax_length_mismatch_errors_not_panics() {
        // Library/CLI-facing: a stale model's logits width vs a current mask
        // must surface as ShapeMismatch, never abort the process.
        let logits = [1.0, 2.0, 3.0];
        let mask = [1.0, 1.0];
        match masked_softmax(&logits, &mask) {
            Err(InferenceError::ShapeMismatch(msg)) => {
                assert!(msg.contains("3") && msg.contains("2"), "unhelpful msg: {msg}");
            }
            other => panic!("expected ShapeMismatch, got {other:?}"),
        }
    }

    #[test]
    fn masked_softmax_is_stable_for_large_logits() {
        let logits = [1e4, 1e4 - 1.0];
        let mask = [1.0, 1.0];
        let p = masked_softmax(&logits, &mask).unwrap();
        assert!(p.iter().all(|v| v.is_finite()));
        assert!(p[0] > p[1]);
    }
}
