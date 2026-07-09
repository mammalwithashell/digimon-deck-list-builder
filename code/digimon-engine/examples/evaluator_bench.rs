//! Batched-evaluator benchmark (add-determinized-search task 0.3).
//!
//! Measures evals/sec of `BatchedPolicyValueEvaluator` at several batch
//! sizes on synthetic observations. Feed it a REAL-SHAPE value-head export
//! (task 0.2) so the numbers price actual MCTS leaf evaluation:
//!   cargo run --release --example evaluator_bench -- <model.onnx> <obs_size>
use digimon_engine::inference::BatchedPolicyValueEvaluator;
use std::path::Path;
use std::time::Instant;

const ACTION_SIZE: usize = 2192;

fn main() {
    let mut args = std::env::args().skip(1);
    let model = args
        .next()
        .expect("usage: evaluator_bench <model.onnx> <obs_size>");
    let obs_size: usize = args
        .next()
        .and_then(|s| s.parse().ok())
        .expect("usage: evaluator_bench <model.onnx> <obs_size>");

    let mut evaluator =
        BatchedPolicyValueEvaluator::load(Path::new(&model)).expect("load value-head model");

    // Deterministic pseudo-random observations + a mask with ~40 legal
    // actions (a realistic mask density for a mid-game decision).
    let mut state = 0x9E3779B97F4A7C15u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 33) as f32 / (1u64 << 31) as f32) - 1.0
    };
    let mut mask = vec![0.0f32; ACTION_SIZE];
    for i in 0..ACTION_SIZE {
        if i % 55 == 0 {
            mask[i] = 1.0;
        }
    }

    for &batch in &[1usize, 8, 32, 128, 512] {
        let obs: Vec<Vec<f32>> = (0..batch)
            .map(|_| (0..obs_size).map(|_| next()).collect())
            .collect();
        let obs_refs: Vec<&[f32]> = obs.iter().map(|r| r.as_slice()).collect();
        let mask_refs: Vec<&[f32]> = (0..batch).map(|_| mask.as_slice()).collect();

        // Warmup, then timed runs.
        for _ in 0..3 {
            evaluator
                .evaluate_batch(&obs_refs, &mask_refs)
                .expect("warmup");
        }
        let iters = (2000 / batch).max(5);
        let t = Instant::now();
        for _ in 0..iters {
            let r = evaluator
                .evaluate_batch(&obs_refs, &mask_refs)
                .expect("eval");
            std::hint::black_box(&r);
        }
        let secs = t.elapsed().as_secs_f64();
        let evals = (iters * batch) as f64;
        let per_eval_us = secs * 1e6 / evals;
        let per_call_us = secs * 1e6 / iters as f64;
        eprintln!(
            "batch {:4}: {:8.1} us/call  {:7.1} us/eval  => {:8.0} evals/sec",
            batch,
            per_call_us,
            per_eval_us,
            1e6 / per_eval_us
        );
    }
}
