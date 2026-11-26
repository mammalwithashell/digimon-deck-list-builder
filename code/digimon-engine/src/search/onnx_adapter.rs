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

use std::path::Path;

use crate::enums::PlayerId;
use crate::inference::{
    is_lstm_onnx, BatchedLstmPolicyValueEvaluator, BatchedPolicyValueEvaluator, InferenceError,
};
use crate::observation::ObservationProfileId;

use super::evaluator::{EvalRequest, Evaluation, PolicyValueFn, RecurrentState};
use super::MemoizedEvaluator;

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

impl PolicyValueFn for BatchedLstmPolicyValueEvaluator {
    fn evaluate_batch(&mut self, requests: &[EvalRequest]) -> Vec<Evaluation> {
        let states_in = vec![None; requests.len()];
        <Self as PolicyValueFn>::evaluate_batch_recurrent(self, requests, &states_in).0
    }

    fn evaluate_batch_recurrent(
        &mut self,
        requests: &[EvalRequest],
        states_in: &[Option<RecurrentState>],
    ) -> (Vec<Evaluation>, Vec<Option<RecurrentState>>) {
        let obs: Vec<&[f32]> = requests.iter().map(|r| r.obs.as_slice()).collect();
        let masks: Vec<&[f32]> = requests.iter().map(|r| r.mask.as_slice()).collect();
        let (rows, states_out) =
            BatchedLstmPolicyValueEvaluator::evaluate_batch_recurrent(self, &obs, &masks, states_in)
                .expect("ONNX LSTM policy+value evaluation failed during search");
        let evals = rows
            .into_iter()
            .map(|row| Evaluation {
                policy: row.policy,
                value: row.value,
            })
            .collect();
        (evals, states_out)
    }

    fn is_recurrent(&self) -> bool {
        true
    }

    fn root_recurrent_state(&self, player: PlayerId) -> Option<RecurrentState> {
        self.root_state(player)
    }

    fn reset_episode(&mut self) {
        BatchedLstmPolicyValueEvaluator::reset_episode(self);
    }

    fn observe_committed_step(&mut self, player: PlayerId, obs: &[f32], mask: &[f32]) {
        BatchedLstmPolicyValueEvaluator::observe_committed_step(self, player, obs, mask)
            .expect("ONNX LSTM committed-step state update failed during self-play");
    }
}

/// Load either an MLP (`logits` + `value`) or recurrent LSTM (`logits, h_out, c_out`
/// + companion `.value.onnx`) policy+value evaluator for search.
///
/// Memoization (`cache_entries > 0`) is applied only to stateless MLP evaluators,
/// since recurrent outputs depend on `(obs, h_in, c_in)`.
pub fn load_search_evaluator_for_profile(
    path: &Path,
    profile: ObservationProfileId,
    require_terminal_value: bool,
    cache_entries: usize,
) -> Result<Box<dyn PolicyValueFn>, InferenceError> {
    if is_lstm_onnx(path)? {
        let eval = BatchedLstmPolicyValueEvaluator::load_for_profile(path, profile)?;
        if require_terminal_value {
            eval.require_terminal_value()?;
        }
        return Ok(Box::new(eval));
    }
    let eval = BatchedPolicyValueEvaluator::load_for_profile(path, profile)?;
    if require_terminal_value {
        eval.require_terminal_value()?;
    }
    if cache_entries == 0 {
        Ok(Box::new(eval))
    } else {
        Ok(Box::new(MemoizedEvaluator::new(eval, cache_entries)))
    }
}
