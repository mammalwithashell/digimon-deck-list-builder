//! Policy+value evaluator abstraction for the search core (task 2.2).
//!
//! The search is decoupled from any concrete network runtime: it only needs
//! a function from (observation tensor, action mask) to (masked policy over
//! the 2192-action space, scalar value in `[-1, 1]` for the player to
//! move). The serial reference uses batch one. Independent PIMC world trees
//! cooperate through `Mcts::search_batch`, evaluating one pending leaf per
//! tree together before each tree resumes its own sequential selection.
//!
//! Concrete implementations:
//!   - [`UniformEvaluator`] — the stub: uniform priors over legal actions,
//!     value 0.0. Enough for perfect-information validation (task 2.4).
//!   - `inference::BatchedPolicyValueEvaluator` — the ONNX evaluator from
//!     task 0.3, adapted in `search::onnx_adapter` (kept in its own file so
//!     the core and its tests never touch `ort`).

/// One leaf evaluation request: the mover's observation tensor and the full
/// action mask (1.0 = legal) over the action space.
#[derive(Debug, Clone)]
pub struct EvalRequest {
    pub obs: Vec<f32>,
    pub mask: Vec<f32>,
}

/// One row of a batched evaluation.
#[derive(Debug, Clone)]
pub struct Evaluation {
    /// Masked policy over the full action space: zero mass on illegal
    /// actions, legal entries sum to ~1.0.
    pub policy: Vec<f32>,
    /// Scalar value in `[-1, 1]` from the perspective of the player to move
    /// in the evaluated state.
    pub value: f32,
}

/// Recurrent hidden states (actor + critic `(h, c)`) threaded per player
/// along a search path when using a recurrent evaluator such as
/// `BatchedLstmPolicyValueEvaluator`.
#[derive(Debug, Clone, PartialEq)]
pub struct RecurrentState {
    pub h_pi: Vec<f32>,
    pub c_pi: Vec<f32>,
    pub h_vf: Vec<f32>,
    pub c_vf: Vec<f32>,
}

/// A batched policy+value function guiding selection and leaf evaluation.
pub trait PolicyValueFn {
    /// Evaluate a batch of states. Must return exactly one [`Evaluation`]
    /// per request, in order, each policy the same length as the request's
    /// mask with zero mass on masked-out actions.
    fn evaluate_batch(&mut self, requests: &[EvalRequest]) -> Vec<Evaluation>;

    /// Evaluate a batch with explicit per-request recurrent input states.
    /// Stateless evaluators ignore `states_in` and return `None` states.
    fn evaluate_batch_recurrent(
        &mut self,
        requests: &[EvalRequest],
        _states_in: &[Option<RecurrentState>],
    ) -> (Vec<Evaluation>, Vec<Option<RecurrentState>>) {
        let evals = self.evaluate_batch(requests);
        let states_out = vec![None; evals.len()];
        (evals, states_out)
    }

    /// True if this evaluator threads recurrent hidden state across decisions.
    fn is_recurrent(&self) -> bool {
        false
    }

    /// Live root recurrent state for `player` at the start of a search call.
    fn root_recurrent_state(&self, _player: crate::enums::PlayerId) -> Option<RecurrentState> {
        None
    }

    /// Reset live per-episode recurrent state at the start of a new game.
    fn reset_episode(&mut self) {}

    /// Advance the live per-episode recurrent state for `player` on a real
    /// non-forced game decision committed in self-play.
    fn observe_committed_step(
        &mut self,
        _player: crate::enums::PlayerId,
        _obs: &[f32],
        _mask: &[f32],
    ) {
    }
}

/// Stub evaluator: uniform priors over legal actions, value 0.0.
///
/// With this evaluator the search degrades to plain (unguided) PUCT-MCTS,
/// which is exactly what the perfect-information validation wants — any
/// forced win it finds is found by search alone.
#[derive(Debug, Default, Clone, Copy)]
pub struct UniformEvaluator;

impl PolicyValueFn for UniformEvaluator {
    fn evaluate_batch(&mut self, requests: &[EvalRequest]) -> Vec<Evaluation> {
        requests
            .iter()
            .map(|req| {
                let legal = req.mask.iter().filter(|&&m| m > 0.5).count();
                let p = if legal > 0 { 1.0 / legal as f32 } else { 0.0 };
                let policy = req
                    .mask
                    .iter()
                    .map(|&m| if m > 0.5 { p } else { 0.0 })
                    .collect();
                Evaluation { policy, value: 0.0 }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_evaluator_is_uniform_over_legal_only() {
        let mut e = UniformEvaluator;
        let req = EvalRequest {
            obs: vec![0.0; 4],
            mask: vec![1.0, 0.0, 1.0, 0.0, 1.0],
        };
        let out = e.evaluate_batch(&[req]);
        assert_eq!(out.len(), 1);
        let p = &out[0].policy;
        assert_eq!(p.len(), 5);
        assert_eq!(p[1], 0.0);
        assert_eq!(p[3], 0.0);
        for &i in &[0usize, 2, 4] {
            assert!((p[i] - 1.0 / 3.0).abs() < 1e-6);
        }
        let sum: f32 = p.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
        assert_eq!(out[0].value, 0.0);
    }

    #[test]
    fn uniform_evaluator_handles_batches_and_empty_masks() {
        let mut e = UniformEvaluator;
        let reqs = vec![
            EvalRequest {
                obs: vec![],
                mask: vec![0.0, 0.0],
            },
            EvalRequest {
                obs: vec![],
                mask: vec![1.0],
            },
        ];
        let out = e.evaluate_batch(&reqs);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].policy, vec![0.0, 0.0]);
        assert_eq!(out[1].policy, vec![1.0]);
    }
}
