use ndarray::Array2;
use ort::session::Session;
use ort::value::Value;

use super::{masked_argmax, InferenceError, OnnxPolicy};

pub struct OnnxMlpPolicy {
    session: Session,
}

impl OnnxMlpPolicy {
    pub(crate) fn from_session(session: Session) -> Self {
        Self { session }
    }
}

impl OnnxPolicy for OnnxMlpPolicy {
    fn predict(&mut self, obs: &[f32], mask: &[f32]) -> Result<usize, InferenceError> {
        let obs_arr = Array2::from_shape_vec((1, obs.len()), obs.to_vec())
            .map_err(|e| InferenceError::ShapeMismatch(e.to_string()))?;
        let obs_value =
            Value::from_array(obs_arr).map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;
        let outputs = self
            .session
            .run(ort::inputs!["obs" => obs_value])
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;
        let logits_value = outputs
            .get("logits")
            .ok_or_else(|| InferenceError::UnsupportedOutputs(vec!["logits".into()]))?;
        let (_shape, logits) = logits_value
            .try_extract_tensor::<f32>()
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;
        Ok(masked_argmax(logits, mask))
    }

    fn reset(&mut self) {}
}
