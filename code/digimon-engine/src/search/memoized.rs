//! Exact-input memoization for a deterministic, frozen policy/value model.
//!
//! This caches network outputs, never game states, legal actions or tree
//! statistics. Both observation AND mask bits form the key. A new model or
//! observation profile must get a fresh wrapper. Capacity bounds FIFO entries.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use super::evaluator::{EvalRequest, Evaluation, PolicyValueFn};

#[derive(Hash, PartialEq, Eq)]
struct InputKey {
    obs: Vec<u32>,
    mask: Vec<u32>,
}

impl From<&EvalRequest> for InputKey {
    fn from(request: &EvalRequest) -> Self {
        Self {
            obs: request.obs.iter().map(|x| x.to_bits()).collect(),
            mask: request.mask.iter().map(|x| x.to_bits()).collect(),
        }
    }
}

/// Bounded exact memoization. Only wrap a deterministic, batch-independent
/// evaluator whose model and profile stay frozen throughout this lifetime.
/// Floating-point keys compare all bits; hash collisions cannot create hits.
pub struct MemoizedEvaluator<E> {
    inner: E,
    capacity: usize,
    cache: HashMap<Arc<InputKey>, Evaluation>,
    fifo: VecDeque<Arc<InputKey>>,
    hits: u64,
    misses: u64,
}

impl<E> MemoizedEvaluator<E> {
    pub fn new(inner: E, capacity: usize) -> Self {
        Self {
            inner,
            capacity,
            cache: HashMap::new(),
            fifo: VecDeque::new(),
            hits: 0,
            misses: 0,
        }
    }
    pub fn inner(&self) -> &E {
        &self.inner
    }
    pub fn hits(&self) -> u64 {
        self.hits
    }
    pub fn misses(&self) -> u64 {
        self.misses
    }
    pub fn len(&self) -> usize {
        self.cache.len()
    }
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
    fn insert(&mut self, key: Arc<InputKey>, result: Evaluation) {
        while self.cache.len() >= self.capacity {
            let oldest = self.fifo.pop_front().expect("cache FIFO invariant");
            self.cache.remove(&oldest);
        }
        self.fifo.push_back(Arc::clone(&key));
        self.cache.insert(key, result);
    }
}

impl<E: PolicyValueFn> PolicyValueFn for MemoizedEvaluator<E> {
    fn evaluate_batch(&mut self, requests: &[EvalRequest]) -> Vec<Evaluation> {
        if requests.is_empty() {
            return Vec::new();
        }
        if self.capacity == 0 {
            self.misses += requests.len() as u64;
            return self.inner.evaluate_batch(requests);
        }
        // Current MCTS uses singleton calls: avoid staging a fresh batch or
        // cloning the large input tensor on every miss in that hot path.
        if requests.len() == 1 {
            let key = Arc::new(InputKey::from(&requests[0]));
            if let Some(result) = self.cache.get(&key) {
                self.hits += 1;
                return vec![result.clone()];
            }
            self.misses += 1;
            let results = self.inner.evaluate_batch(requests);
            assert_eq!(results.len(), 1, "evaluator returned wrong batch size");
            self.insert(key, results[0].clone());
            return results;
        }
        let mut outputs: Vec<Option<Evaluation>> = vec![None; requests.len()];
        let mut pending: HashMap<Arc<InputKey>, usize> = HashMap::new();
        let mut keys = Vec::new();
        let mut missing_requests = Vec::new();
        let mut missing_rows = Vec::new();
        for (row, request) in requests.iter().enumerate() {
            let key = Arc::new(InputKey::from(request));
            if let Some(result) = self.cache.get(&key) {
                outputs[row] = Some(result.clone());
                self.hits += 1;
            } else if let Some(&index) = pending.get(&key) {
                missing_rows.push((row, index));
                self.hits += 1;
            } else {
                let index = missing_requests.len();
                pending.insert(Arc::clone(&key), index);
                keys.push(key);
                missing_requests.push(request.clone());
                missing_rows.push((row, index));
                self.misses += 1;
            }
        }
        if !missing_requests.is_empty() {
            let results = self.inner.evaluate_batch(&missing_requests);
            assert_eq!(
                results.len(),
                missing_requests.len(),
                "evaluator returned wrong batch size"
            );
            for (row, index) in missing_rows {
                outputs[row] = Some(results[index].clone());
            }
            for (key, result) in keys.into_iter().zip(results) {
                self.insert(key, result);
            }
        }
        outputs
            .into_iter()
            .map(|r| r.expect("every row was evaluated"))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::{EvalRequest, Evaluation, PolicyValueFn, UniformEvaluator};

    #[derive(Default)]
    struct Counting {
        rows: usize,
        batches: Vec<usize>,
    }
    impl PolicyValueFn for Counting {
        fn evaluate_batch(&mut self, requests: &[EvalRequest]) -> Vec<Evaluation> {
            self.rows += requests.len();
            self.batches.push(requests.len());
            let mut results = UniformEvaluator.evaluate_batch(requests);
            for (row, request) in results.iter_mut().zip(requests) {
                row.value = request.obs.first().copied().unwrap_or(0.).tanh();
            }
            results
        }
    }
    fn request(value: f32) -> EvalRequest {
        EvalRequest {
            obs: vec![value],
            mask: vec![1., 0., 1.],
        }
    }
    fn equal(a: &Evaluation, b: &Evaluation) {
        assert_eq!(a.value.to_bits(), b.value.to_bits());
        assert_eq!(
            a.policy.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            b.policy.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn repeated_inputs_and_batch_duplicates_preserve_outputs_and_order() {
        let mut cached = MemoizedEvaluator::new(Counting::default(), 4);
        let inputs = vec![request(1.), request(2.), request(1.)];
        let expected = Counting::default().evaluate_batch(&inputs);
        let first = cached.evaluate_batch(&inputs);
        let second = cached.evaluate_batch(&inputs);
        for ((a, b), want) in first.iter().zip(&second).zip(&expected) {
            equal(a, want);
            equal(b, want);
        }
        assert_eq!(cached.inner().rows, 2);
        assert_eq!(cached.inner().batches, vec![2]);
        assert_eq!(cached.hits(), 4);
        assert_eq!(cached.misses(), 2);
    }

    #[test]
    fn mask_and_float_bits_are_part_of_key() {
        let mut cached = MemoizedEvaluator::new(Counting::default(), 8);
        let mut changed_mask = request(0.);
        changed_mask.mask = vec![0., 1., 0.];
        let inputs = vec![request(0.), request(-0.), changed_mask];
        let actual = cached.evaluate_batch(&inputs);
        let expected = Counting::default().evaluate_batch(&inputs);
        for (a, b) in actual.iter().zip(&expected) {
            equal(a, b);
        }
        assert_eq!(cached.misses(), 3);
        assert_eq!(actual[2].policy, vec![0., 1., 0.]);
    }

    #[test]
    fn capacity_is_bounded_and_eviction_only_recomputes() {
        let mut cached = MemoizedEvaluator::new(Counting::default(), 2);
        for value in [1., 2., 3., 1.] {
            let inputs = vec![request(value)];
            equal(
                &cached.evaluate_batch(&inputs)[0],
                &Counting::default().evaluate_batch(&inputs)[0],
            );
            assert!(cached.len() <= 2);
        }
        assert_eq!(cached.inner().rows, 4);
    }

    #[test]
    fn zero_capacity_is_a_batch_preserving_bypass() {
        let mut cached = MemoizedEvaluator::new(Counting::default(), 0);
        let inputs = vec![request(1.), request(1.)];
        cached.evaluate_batch(&inputs);
        cached.evaluate_batch(&inputs);
        assert_eq!(cached.inner().batches, vec![2, 2]);
        assert_eq!(cached.len(), 0);
        assert_eq!(cached.misses(), 4);
        assert_eq!(cached.hits(), 0);
    }

    #[test]
    fn empty_batch_does_not_call_model() {
        let mut cached = MemoizedEvaluator::new(Counting::default(), 2);
        assert!(cached.evaluate_batch(&[]).is_empty());
        assert!(cached.inner().batches.is_empty());
    }
}
