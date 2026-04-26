use ndarray::{Array2, Array3};
use ort::session::Session;
use ort::value::Value;

use super::{masked_argmax, InferenceError, OnnxPolicy};

pub struct OnnxLstmPolicy {
    session: Session,
    hidden_size: usize,
    h: Array3<f32>,
    c: Array3<f32>,
}

impl OnnxLstmPolicy {
    pub(crate) fn from_session(session: Session) -> Result<Self, InferenceError> {
        let hidden_size = detect_hidden_size(&session)?;
        Ok(Self {
            session,
            hidden_size,
            h: Array3::<f32>::zeros((1, 1, hidden_size)),
            c: Array3::<f32>::zeros((1, 1, hidden_size)),
        })
    }

    pub fn hidden_size(&self) -> usize {
        self.hidden_size
    }
}

/// Read the hidden size off the `h_in` input's declared shape — mirrors the
/// Python-side default of 256 but lets us load models with any hidden size
/// without hardcoding it at the call site.
fn detect_hidden_size(session: &Session) -> Result<usize, InferenceError> {
    let h_in = session
        .inputs()
        .iter()
        .find(|i| i.name() == "h_in")
        .ok_or_else(|| InferenceError::ShapeMismatch("LSTM model missing 'h_in' input".into()))?;
    let shape = h_in
        .dtype()
        .tensor_shape()
        .ok_or_else(|| InferenceError::ShapeMismatch("h_in has no tensor shape".into()))?;
    let last = shape
        .last()
        .copied()
        .ok_or_else(|| InferenceError::ShapeMismatch("h_in shape is empty".into()))?;
    if last <= 0 {
        return Err(InferenceError::ShapeMismatch(format!(
            "h_in last dim must be positive, got {last}"
        )));
    }
    Ok(last as usize)
}

impl OnnxPolicy for OnnxLstmPolicy {
    fn predict(&mut self, obs: &[f32], mask: &[f32]) -> Result<usize, InferenceError> {
        let obs_arr = Array2::from_shape_vec((1, obs.len()), obs.to_vec())
            .map_err(|e| InferenceError::ShapeMismatch(e.to_string()))?;
        let obs_value = Value::from_array(obs_arr)
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;
        let h_value = Value::from_array(self.h.clone())
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;
        let c_value = Value::from_array(self.c.clone())
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;

        let outputs = self
            .session
            .run(ort::inputs![
                "obs" => obs_value,
                "h_in" => h_value,
                "c_in" => c_value,
            ])
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;

        let logits_value = outputs
            .get("logits")
            .ok_or_else(|| InferenceError::UnsupportedOutputs(vec!["logits".into()]))?;
        let (_shape, logits) = logits_value
            .try_extract_tensor::<f32>()
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;
        let action = masked_argmax(logits, mask);

        let h_out_value = outputs
            .get("h_out")
            .ok_or_else(|| InferenceError::UnsupportedOutputs(vec!["h_out".into()]))?;
        let (h_shape, h_data) = h_out_value
            .try_extract_tensor::<f32>()
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;
        self.h = reshape_state(h_data, &*h_shape, self.hidden_size, "h_out")?;

        let c_out_value = outputs
            .get("c_out")
            .ok_or_else(|| InferenceError::UnsupportedOutputs(vec!["c_out".into()]))?;
        let (c_shape, c_data) = c_out_value
            .try_extract_tensor::<f32>()
            .map_err(|e| InferenceError::RuntimeFailed(e.to_string()))?;
        self.c = reshape_state(c_data, &*c_shape, self.hidden_size, "c_out")?;

        Ok(action)
    }

    fn reset(&mut self) {
        self.h.fill(0.0);
        self.c.fill(0.0);
    }
}

fn reshape_state(
    data: &[f32],
    shape: &[i64],
    hidden_size: usize,
    label: &str,
) -> Result<Array3<f32>, InferenceError> {
    let expected = hidden_size;
    let total: i64 = shape.iter().product();
    if total as usize != data.len() {
        return Err(InferenceError::ShapeMismatch(format!(
            "{label} numel mismatch: shape {shape:?} vs data len {}",
            data.len()
        )));
    }
    Array3::from_shape_vec((1, 1, expected), data.to_vec())
        .map_err(|e| InferenceError::ShapeMismatch(format!("{label}: {e}")))
}
