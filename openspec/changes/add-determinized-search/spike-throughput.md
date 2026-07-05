# Throughput go/no-go spike (task 0.1)

**Date:** 2026-07-01 · **Hardware:** local dev box (Windows 11, single core measured; release build) · **Harness:** `code/digimon-engine/examples/search_throughput_spike.rs` (`cargo run --release --example search_throughput_spike --features dsl-yaml-loader -- 40000`) · **Decks:** ST-1 vs ST-5 (real card pool, DSL effects)

## Raw numbers

| Metric | Value |
|---|---|
| Engine `step` (greedy playout, step only) | **283 µs** → 3,534 steps/sec/core |
| `step` + mask build (random playout — the per-node search shape) | **721 µs** → 1,387 steps/sec/core |
| `Game::clone`, early game (<50 steps) | **115 µs** (n=5,881) |
| `Game::clone`, mid game (50–150 steps) | **184 µs** (n=793) |
| Reset-and-replay to same depth, early | 8.4 ms → clone is **73×** cheaper |
| Reset-and-replay to same depth, mid | 44.6 ms → clone is **243×** cheaper |
| Derived sim = clone + 1 step (AlphaZero-style, tree reuse, NN eval excluded) | **~904 µs → ~1,100 sims/sec/core** |
| Derived sim = clone + 10-step rollout | ~7.4 ms → ~135 sims/sec/core |
| Game length | greedy 33 steps, random ~75 steps |

## Evaluator numbers (task 0.3, same box, CPU `ort`)

Real-shape untrained MLP (standard_lite_v2: 8410 obs → 2192 logits + value), `code/digimon-engine/examples/evaluator_bench.rs`:

| Batch | µs/eval | evals/sec |
|---|---|---|
| 1 | 127.9 | 7,819 |
| 8 | 43.3 | 23,110 |
| 32 | 51.8 | 19,305 |
| 128 | 41.3 | 24,194 |
| 512 | 29.5 | 33,942 |

**Complete sim price (mid-game): clone 184 µs + step-with-mask 721 µs + batched eval ~43 µs ≈ 950 µs → ~1,050 sims/sec/core.** The engine is ~95% of the cost; NN evaluation is a rounding error once batched (and only 13% even unbatched, on CPU with no GPU involved). Leaf batching pays 3–4× on the eval slice but the design's throughput risk lives almost entirely in `step` + mask build.

## Verdict: GO (AlphaZero-style), with levers identified

- **Clone decisively beats reset-replay as the fork primitive** (73–243×, growing with depth). The `make-engine-cloneable` bet is validated empirically.
- **AlphaZero-style search (no rollouts, value net at leaves) is viable on CPU cores:** at 200 sims/decision, engine-side cost ≈ 0.18 s/decision/core ≈ 13.5 s per 75-decision game/core → a 16-core box generates ~100k self-play games/day engine-side (NN eval excluded; GPU-batched eval overlaps rather than adds if the evaluator pipeline is async).
- **PIMC multiplies by K worlds at the root:** K=8 × 200 sims ≈ 1.4 s/decision/core → ~13k games/day on 16 cores. Feasible for 5–10k-game generations; tight for more. Use small K during self-play, larger K at evaluation/review time.
- **Rollout-style MCTS (10+ steps per sim) is NOT viable** at scale (135 sims/sec/core) — confirms the design's choice of value-net leaf evaluation.

## Optimization levers (in observed-impact order)

1. **Mask build dominates the per-node cost** (~440 µs of the 721 µs random-path step — ~60% of a sim). The search expands a node once but needs the mask for every expansion; memoizing or incrementalizing mask construction is the single biggest engine-side lever for search throughput. (Not previously on the perf radar — `effects_for_card` memoization was.)
2. **`effects_for_card` memoization** (~2.5–3× on step cost per prior profiling; not yet implemented, so not A/B-able in this spike — deferred to the perf change).
3. **COW/structural-sharing clone** (make-engine-cloneable task 4.3): clone is already cheap (115–184 µs); at ~13–20% of sim cost this is a second-order win. Deprioritize vs (1).

## Caveats

- Single-core, local Windows box; cloud training hardware will differ (re-run there before sizing a generation).
- Greedy/random policies bound the step-cost range; a real search policy's action mix sits between.
- NN eval excluded by design — combine with task 0.3's batched-evaluator evals/sec for the full sim price.
- The clone-cost numbers are for games paused at *safe* points (the playout never parks mid-closure); once the non-DSL cutover lands, clone cost at interrupt prompts should be re-sampled.

## Real self-play profile (post-Phase-3, supersedes the mask-build lever above)

**Date:** 2026-07-05 · **Hardware:** Hetzner cpx62 (16 shared vCPU, idle/uncontended), debug-symbol release build (`CARGO_PROFILE_RELEASE_STRIP=false CARGO_PROFILE_RELEASE_DEBUG=1`) · **Harness:** `perf record -F 999 -g --call-graph=dwarf` over a single-process `digimon-engine-cli selfplay` run (8 games, `--sims 320 --worlds 4`, PIMC, the real trained gen-14 model from the `add-determinized-search` self-play run) · **290K samples, 383 decisions**

The 0.1 spike above measured raw `step`+mask cost with **no search and no NN** and concluded mask-build was the #1 engine-side lever (~60% of a sim). That conclusion does **not** carry over to the real production workload (determinized PIMC search + a real trained-model evaluator per leaf). Flat self-time by symbol:

| Component | Self-time | |
|---|---|---|
| ONNX Runtime — GEMM math (`MlasGemmFloatKernelAvx512F`) | **51.7%** | dominant |
| ONNX Runtime — thread-pool spin-wait (`SpinPause` + `WorkerLoop`/`EndParallelSection`/etc.) | ~7–9% | overhead, not math |
| malloc/cfree | ~2.8% | mostly per-call I/O tensor (re)allocation |
| Rust engine — `effects_for_card` | 1.4% | known lever, still unimplemented |
| Rust engine — MCTS tree (`make_node` + `Node` alloc/drop chain) | ~4–6% | node-arena churn |
| Rust engine — `build_action_mask` | **0.05% self / 1.32% children** | negligible here |
| Rust engine — `determinized_search`/`pimc_search`/`materialize`/`canonicalize_hidden`/`sample_world` | **≤0.1% each** | negligible |

**Why the flip:** every MCTS leaf currently triggers its own **batch-of-1** ONNX inference call (`mcts.rs` doc: "the evaluator is called with batch = 1 for now"). Once a real trained-model NN evaluation sits on top of the engine, it swamps engine-side costs by an order of magnitude — mask-build and the entire Phase-1 determinization/materialization layer (the design's own flagged "novel correctness risk") turn out to be essentially free relative to the NN. The `SpinPause`/thread-pool share is a symptom of running ORT's default multi-threaded intra-op executor on calls too small to amortize the dispatch cost.

### Updated optimization priority (supersedes the Section-above ranking for search workloads)

1. **Leaf batching** (already tracked as task 4.1 — "embedded ONNX evaluator... with leaf batching + virtual loss"). Now has hard evidence behind it: batching leaves amortizes both the GEMM cost and the thread-dispatch tax, attacking the ~55–60% dominant slice directly.
2. **Try `intra_op_num_threads=1`** on the ONNX session as a cheap pre-batching experiment — for batch=1 calls, ORT's internal parallelism is plausibly pure overhead. One-line config change, worth A/B-testing before investing in (1). More relevant than this single-process measurement suggests: production runs 6 of these processes concurrently on 16 shared cores, so 6 independently-spinning thread pools contending for physical cores likely costs more than shown here.
3. `effects_for_card` memoization — small (~1.4%) but cheap, already known from prior profiling.
4. Node-arena reuse across searches — modest, lower priority than 1–2.

**Not worth touching right now:** mask-build, the determinization/materialization layer, MCTS dispatch code — all individually ≤2% in the real workload.
