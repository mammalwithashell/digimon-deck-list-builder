# English-meta search throughput, 2026-10-23

The first quality-preserving candidates improved measured native decision-prefix
throughput by roughly 10–20%. This is a small diagnostic, not a full-game or
2,000-game speed claim. The ongoing v3 generation was not stopped or changed.

## Controlled native comparisons

Ryzen 7 3700X, Windows, the frozen v3 seed model and admitted English deck
pool. Every comparison retains 320 total simulations, four PIMC worlds,
the same search depth, root noise, temperature, profile and seed. Each
diagnostic stops after 40 actual decisions; these deliberately truncated
outputs live under `runs/2026-10-23-search-performance/` and are not training
data or an additional health check. Other applications and four production
workers remained active, so these are indicative timings.

| Seed | Candidate | Seconds | Throughput relative to uncached system allocator |
|---|---|---:|---:|
| 21261025 | Reference, two repeats | 45.09 / 45.91 | 1.00 |
| 21261025 | System allocator, 4096 cache entries | 60.96 | 0.74 |
| 21261025 | System allocator, 128 cache entries | 42.93 | 1.05 |
| 21261025 | mimalloc, no cache | 40.55 | 1.11 |
| 21261025 | mimalloc, 128 cache entries | 38.27 | 1.18 |
| 21261023 | Reference | 43.58 | 1.00 |
| 21261023 | mimalloc, 128 cache entries | 39.59 | 1.10 |

The actual candidate CLI subsequently completed the first prefix in 37.7
seconds, with its admission record and cache capacity stamped into its
manifest. All eight shard arrays match the reference exactly: observations,
players, game IDs, sparse action IDs, offsets, visits, termination and labels.
Both combined diagnostic prefixes match their respective references exactly.
The all-zero outcome labels here only reflect the deliberate cap; these
prefix checks are evidence about search behavior, not completed-game outcomes.

Network evaluation consumed 11.4–11.8 seconds of the approximately 45-second
first reference prefix. Inference is about one quarter of this sample's
runtime, unlike the older starter-deck profile. The large cache reduced model
calls but lost overall; it is not enabled. A 128-entry cache saved 14–23% of
model calls across the two prefixes. Larger is not automatically faster.

## Implementation and validation

- The Windows CLI uses mimalloc, already used by the engine's test targets.
- `search::MemoizedEvaluator` keys the complete observation and mask bits,
  with equality checking as well as hashing. It stores only outputs of one
  frozen, deterministic, batch-independent model; it never caches legality,
  world state or tree statistics. Eviction only causes recomputation.
- The cache is opt-in: `DIGIMON_SEARCH_EVAL_CACHE_ENTRIES=128`. The default
  remains zero. Native manifests record the active capacity. Uniform runs
  bypass it. Model/profile changes require a fresh wrapper.
- Seven evaluator/cache unit tests pass. The search/cache integration harness
  passes all 16 tests, including unchanged MCTS and PIMC results across three
  seeds. Eight native admission/generation-contract Python tests pass.
- Formatting and scoped diff whitespace checks pass.

To avoid continuing an expensive whole-engine rebuild during generation,
the native probe, test harness and candidate CLI were relinked against the
existing release engine `libdigimon_engine-a74fde63aa3ee862.rlib`, compiling
the new cache and CLI sources separately. The source change does not alter
engine rules. `comparison.json` records the model and engine-library SHA256.
A fresh ordinary Cargo build was not completed in this experiment; interrupted
build logs are retained. The candidate executable is isolated under the
diagnostic directory, and did not replace the running executable or wheel.

Normal subsequent builds use `cargo build --release -p digimon-engine-cli`.
The reusable probe is `code/digimon-engine/examples/search_performance.rs`:

```text
search_performance <model> <decks> <new-output> <seed> <games> <decision-cap> <cache-entries>
```

## Larger next opportunity

Current MCTS submits batch-of-one leaf evaluations. A separate Python ONNX
Runtime 1.24.4 CPU microbenchmark of the same frozen model, one intra-op
thread, measured 1,734 evaluations/sec at batch one, 4,508 at batch four,
and 6,539 at batch sixteen. All logits and values were bit-identical across
64 real bootstrap observations at batches 4/8/16/32. This is an inference
microbenchmark, not the native runtime or a measured complete-search gain.

Batch across independent games or independent PIMC world trees, retaining each
tree's sequential selection/evaluation/backup order and fixed seeds. Batching
multiple pending leaves within one tree using virtual loss changes selection
behavior and is not an exact-equivalence optimization. GPU evaluation also
needs output and search-level parity checks before adoption.

Other candidates are dynamically queued game chunks to reduce idle worker
tails and controlled worker-count scaling. Neither lowers the per-decision
budget. Increasing concurrency without measurement on this occupied machine
could reduce throughput. The ongoing run uses fixed 500-game allocations;
the new executable and cache cannot be applied to its live searches in place.

## Independent-world batching follow-up

The user requested stopping the current job and adding batching. All eight v3
controller/worker processes were stopped; `interruption.json` records 111
reported completed games and zero truncations. Files and checkpoints were
preserved, but unflushed in-memory rows cannot be counted as saved training
data. `pipeline_state.json` is `stopped_by_user`, with no automatic resume.

Implemented cooperative batching across independent PIMC world trees. Each
tree pauses at its next leaf and receives its evaluation and backup before
selecting another simulation. World seeds, root noise, total/per-world budgets,
forced chains, depth caps and world-order aggregation are unchanged. There is
no virtual loss and no speculative same-tree selection. `--eval-batch-size 4`
is the CLI/generation default; `1` retains the serial reference. IS-MCTS remains
serial. Manifests record requested/effective batching, and the generation CLI
forwards the setting to every native worker.

Unlike the initial allocator/cache experiment above, ordinary Cargo release
verification completed: 15 search tests, 8 ONNX tests (including real ONNX
serial/batched equality), one self-play shard test, and a release CLI build.
Thirteen Python admission/generation/pipeline tests also pass against the normal
release binary. Known forced-win and same-player-sign cases additionally
compare batched results directly with serial results. OpenSpec strict validation,
formatting and scoped whitespace checks pass.

Two complete English-meta games at 320 simulations/four worlds, using the frozen
v3 seed model and seed 20261023, finished with zero truncations. All 136 rows
match the first two games of the saved serial `check-320/shards/proc0` reference
bit-for-bit across all eight arrays, including terminal flags and outcome
labels. Evidence: `batched-full-games/serial-parity.json` under the diagnostic
directory. These were validation games, not a restarted training generation.

Matched normal-release CLI benchmarks (same allocator, model, 320 simulations,
four worlds, cache disabled, 40-decision diagnostic cap) measured:

| Seed | Batch 1 positions/sec | Batch 4 positions/sec | Throughput ratio |
|---|---:|---:|---:|
| 21261025 | 1.293 | 1.548 | 1.197 |
| 21261023 | 1.268 | 1.425 | 1.124 |

All eight arrays match exactly in each pair. Thus batching alone improved these
two prefixes by **12–20%**, not the inference-only multiplier. This small,
occupied-desktop experiment is not a measured full-generation ETA or a strength
evaluation. The training job remains stopped.

Reproduce with `code/tools/benchmark_search_batching.py`; its output directory
must be new. The paired raw report is
`runs/2026-10-23-search-performance/batching-paired-release/benchmark.json`.

The subsequent profiling, dynamic scheduling, recovery and worker-scaling work
is documented in [search scheduling](2026-10-23-search-scheduling.md). That
two-game profile identifies observation construction as the dominant region;
the Windows development measurements do not size future Linux cloud runs.
