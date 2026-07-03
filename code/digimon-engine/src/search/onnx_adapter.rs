//! Thin [`PolicyValueFn`] adapter for the ONNX evaluator (task 0.3).
//!
//! Kept in its own file so the search core (and its tests) never depend on
//! `ort` / a model file — `UniformEvaluator` covers the core's validation,
//! and this adapter is exercised end-to-end once Phase 3/4 wire a real
//! policy+value export into self-play.
//!
//! `BatchedPolicyValueEvaluator` already returns exactly the shape the
//! search wants (masked softmax policy with zero illegal mass + value for
//! the player to move), so the adapter is a borrow-and-rewrap.
//!
//! Error policy: the trait is infallible by design (an evaluator failure
//! mid-search leaves the tree unusable — there is no sensible partial
//! result), so ONNX runtime errors panic with context rather than being
//! silently swallowed.

use crate::inference::BatchedPolicyValueEvaluator;

use super::evaluator::{EvalRequest, Evaluation, PolicyValueFn};

impl PolicyValueFn for BatchedPolicyValueEvaluator {
    fn evaluate_batch(&mut self, requests: &[EvalRequest]) -> Vec<Evaluation> {
        let obs: Vec<&[f32]> = requests.iter().map(|r| r.obs.as_slice()).collect();
        let masks: Vec<&[f32]> = requests.iter().map(|r| r.mask.as_slice()).collect();
        BatchedPolicyValueEvaluator::evaluate_batch(self, &obs, &masks)
            .expect("ONNX policy+value evaluation failed during search")
            .into_iter()
            .map(|row| Evaluation {
                policy: row.policy,
                value: row.value,
            })
            .collect()
    }
}
