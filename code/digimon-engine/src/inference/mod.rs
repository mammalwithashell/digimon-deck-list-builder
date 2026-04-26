use std::path::Path;

use ort::session::Session;

pub mod lstm;
pub mod mlp;

pub use lstm::OnnxLstmPolicy;
pub use mlp::OnnxMlpPolicy;

#[derive(Debug)]
pub enum InferenceError {
    LoadFailed(String),
    UnsupportedOutputs(Vec<String>),
    ShapeMismatch(String),
    RuntimeFailed(String),
}

impl std::fmt::Display for InferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LoadFailed(msg) => write!(f, "failed to load ONNX model: {msg}"),
            Self::UnsupportedOutputs(names) => write!(
                f,
                "unrecognized ONNX outputs {names:?}; expected [logits] (MLP) or [logits, h_out, c_out] (LSTM)"
            ),
            Self::ShapeMismatch(msg) => write!(f, "shape mismatch: {msg}"),
            Self::RuntimeFailed(msg) => write!(f, "ONNX runtime error: {msg}"),
        }
    }
}

impl std::error::Error for InferenceError {}

pub trait OnnxPolicy: Send {
    fn predict(&mut self, obs: &[f32], mask: &[f32]) -> Result<usize, InferenceError>;
    fn reset(&mut self);
}

pub fn load_policy(path: &Path) -> Result<Box<dyn OnnxPolicy>, InferenceError> {
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
    let has_h_out = output_names.iter().any(|n| n == "h_out");
    let has_c_out = output_names.iter().any(|n| n == "c_out");
    if has_logits && has_h_out && has_c_out {
        return Ok(Box::new(OnnxLstmPolicy::from_session(session)?));
    }
    if has_logits {
        return Ok(Box::new(OnnxMlpPolicy::from_session(session)));
    }
    Err(InferenceError::UnsupportedOutputs(output_names))
}

/// Mask invalid actions with `-1e9` and return `argmax(logits)`.
///
/// Identical semantics to Python's `_masked_argmax` in `onnx_policy.py` so
/// Rust-side inference and the Python training stack agree on action choice
/// for the same (logits, mask).
pub(crate) fn masked_argmax(logits: &[f32], mask: &[f32]) -> usize {
    assert_eq!(
        logits.len(),
        mask.len(),
        "logits/mask length mismatch: {} vs {}",
        logits.len(),
        mask.len()
    );
    let mut best_idx = 0usize;
    let mut best_val = f32::NEG_INFINITY;
    for (i, (&l, &m)) in logits.iter().zip(mask.iter()).enumerate() {
        let v = if m < 0.5 { -1e9 } else { l };
        if v > best_val {
            best_val = v;
            best_idx = i;
        }
    }
    best_idx
}
