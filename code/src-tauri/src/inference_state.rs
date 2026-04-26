//! Tauri-managed policy cache — keyed by `model_id`, shared across
//! inference calls. Loading is a one-shot operation (expensive: creates an
//! `ort::Session` + reads weights off disk); prediction mutates each
//! policy's internal state (LSTM h/c) so every call takes `&mut`.
//!
//! Locking: a single `Mutex` guards the whole map. Desktop play is single-
//! game at a time, so contention is trivial and this keeps the API
//! tractable for Tauri command handlers.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use digimon_engine::inference::{load_policy, OnnxPolicy};

#[derive(Default)]
pub struct InferenceState {
    policies: Mutex<HashMap<String, Box<dyn OnnxPolicy>>>,
}

impl InferenceState {
    pub fn load(&self, model_id: impl Into<String>, path: &Path) -> Result<(), String> {
        let policy = load_policy(path).map_err(|e| e.to_string())?;
        self.policies
            .lock()
            .map_err(|e| e.to_string())?
            .insert(model_id.into(), policy);
        Ok(())
    }

    pub fn unload(&self, model_id: &str) -> Result<bool, String> {
        Ok(self
            .policies
            .lock()
            .map_err(|e| e.to_string())?
            .remove(model_id)
            .is_some())
    }

    /// Reset a loaded policy back to a fresh state (zero LSTM h/c). Called at
    /// episode boundaries so LSTM models don't carry state across games.
    /// No-op if the model isn't currently loaded.
    pub fn reset(&self, model_id: &str) -> Result<(), String> {
        let mut cache = self.policies.lock().map_err(|e| e.to_string())?;
        if let Some(p) = cache.get_mut(model_id) {
            p.reset();
        }
        Ok(())
    }

    pub fn predict(&self, model_id: &str, obs: &[f32], mask: &[f32]) -> Result<usize, String> {
        let mut cache = self.policies.lock().map_err(|e| e.to_string())?;
        let policy = cache
            .get_mut(model_id)
            .ok_or_else(|| format!("model '{model_id}' not loaded"))?;
        policy.predict(obs, mask).map_err(|e| e.to_string())
    }

    pub fn loaded_ids(&self) -> Result<Vec<String>, String> {
        Ok(self
            .policies
            .lock()
            .map_err(|e| e.to_string())?
            .keys()
            .cloned()
            .collect())
    }

    pub fn is_loaded(&self, model_id: &str) -> Result<bool, String> {
        Ok(self
            .policies
            .lock()
            .map_err(|e| e.to_string())?
            .contains_key(model_id))
    }

    /// Test-only seam: inject a pre-built policy (typically a mock) into the
    /// cache, bypassing ONNX load. Lets callers unit-test the agent loop
    /// without shipping an engine-shape `.onnx` fixture.
    #[cfg(test)]
    pub fn insert_for_test(&self, model_id: impl Into<String>, policy: Box<dyn OnnxPolicy>) {
        self.policies
            .lock()
            .expect("poisoned InferenceState mutex")
            .insert(model_id.into(), policy);
    }
}
