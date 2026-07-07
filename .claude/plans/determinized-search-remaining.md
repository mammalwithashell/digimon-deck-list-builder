# Determinized search — remaining-work execution plan (2026-07-05)

Change: `openspec/changes/add-determinized-search/` (proposal, design, tasks, baselines, spike-throughput).
State at planning time: Phases 0–2 fully DONE; Phase 3 done except **3.3**; Phase 4 open (**4.0**, **4.1**; 4.2 deprioritized by the real-workload profile); Phases **5** (review annotations), **6** (PvP deck prior), **7** (validation) open.

Ordering principle: every hour of search compute downstream is multiplied by Milestone A, so A goes first; B is the experiment the whole change exists for; C is product/QA payoff that consumes A; D is non-blocking.

---

## Milestone A — Make self-play cheap (Phase 4) — ~3–5 days

### A1. Task 4.0 — `intra_op_num_threads=1` A/B (one day, one-line change)
- Where: the `ort` session builder in `code/digimon-engine/src/inference/evaluator.rs` (`BatchedPolicyValueEvaluator`). Make it a config knob, default per A/B outcome.
- Why: every MCTS leaf is currently a batch-of-1 call; the profile in `spike-throughput.md` ("Real self-play profile") shows ~7–9% of self-time burned in ORT `SpinPause`/thread-dispatch for work too small to parallelize.
- Verify: `examples/evaluator_bench.rs` single-row latency, then a 10-game driver run mirroring the 3.4 config (`--sims 320 --worlds 4`) and compare games/sec. Record numbers in `spike-throughput.md`.

### A2. Task 4.1 — leaf batching + virtual loss (the big lever, 2–4 days)
- Evidence: NN-eval path is 55–60% of self-time; ORT GEMM alone 51.7%. Batched rows cost 30–43 µs vs 128 µs single → ~3–4× on the NN share ≈ **~1.8–2.3× end-to-end** games/hour.
- Shape: `PolicyValueFn` is already batch-shaped (task 0.3) — the change is in the search loop, not the evaluator:
  - In `src/search/mcts.rs`: select until B leaves are pending (apply **virtual loss** along each selected path so successive selections diverge), evaluate the frontier as one batch, then back up all leaves and remove virtual losses.
  - PIMC gives a second, coupling-free batching axis: the K worlds each contribute pending leaves to a shared batch per iteration (`src/search/determinized.rs` drives the worlds; interleave instead of sequential per-world budgets).
  - Start B = 8–16; sweep on the bench.
- Correctness watchpoints:
  - Virtual loss must respect the identity-keyed sign backup (consecutive same-player decisions were explicitly tested in 2.1) — apply/remove symmetrically.
  - **Determinism per seed** is a tested contract (`tests/search/`): batch assembly order must be seed-stable. Fixed world-iteration order + deterministic frontier fill preserves it.
  - Re-run the 2.5 observational-equivalence (no-X-ray) tests — batching must not change what state the evaluator sees.
- Close: re-profile on cloud hardware (repeat 3.4 protocol) and update `spike-throughput.md`; only then size Milestone B's generations.

### A3. Task 4.2 — COW clone: leave deprioritized
Profile shows `Game::clone` + the whole determinize/materialize layer ≤0.1% self-time. Re-visit only if a post-4.1 profile promotes it.

---

## Milestone B — Close the loop with promotion gates + run the real experiment (task 3.3 + 7.x) — ~1 week elapsed (mostly compute)

### B1. Task 3.3 — full per-generation evaluation in the orchestrator (1–2 days)
- Extend `code/tools/run_selfplay_generation.py` from "optionally call `anchored_eval_cli.py`" to the full anchored frame per generation:
  1. **Anchored eval** — seat-balanced vs greedy + frozen champions, n per the `baselines.md` protocol (n=300-scale; small n is deck-luck noise).
  2. **Elo ladder** — `elo_ladder_cli.py` over generations + champions + greedy (forgetting/cycling detection across generations).
  3. **Exploiter** — `exploiter_cli.py` at the pre-registered fixed 1M budget (comparability with the 0.70–0.75 baselines).
- Write all three into `gen<G>/summary.json`; keep `state.json` resume semantics.
- **Promotion rule (rule 30):** gen g+1 becomes the next driver/eval reference only if its anchored number ≥ the last promoted generation's. Self-play win rate is never a gate. Exploiter is reported, not gating (budget-noisy), but a large regression is a stop-and-look signal.

### B2. The experiment — multi-generation run on cloud (compute-bound)
- Host: Hetzner ccx33-class (the 3.4-profiled box). Pre-4.1: ~1,000 games/hour → a 2–5k-game generation is 2–5 h; post-4.1 expect ~1–2 h/gen. Budget ~10–20 generations.
- Warm start from `starter_pool_single_v1` (the loop was e2e-verified with it); starter-deck pool = exact `DeckPrior` regime (1.6).
- **Ops gotchas (from memory/baselines):** the published training image is stale/broken — `exploiter_cli.py` postdates `training-v0.46` and had to be hand-patched; cut a fresh image first and verify CI conclusion + GHCR manifest 200. Mirror `runs/` + shards before terminating boxes. RunPod was unschedulable during 0.4 — default to Hetzner.
- Success = task **7.4**: a generation beats the 61% anchored PPO baseline and shows ≤ baseline exploitability at equal exploiter budget (documented, not asserted as Nash).
- If it plateaus with skill-but-not-robustness: the design's D6 follow-up levers are IS-MCTS mode (already behind the config knob) and policy-weighted world sampling (ReBeL-lite) — A/B on the exploiter number.

### B3. Task 7.1–7.3 housekeeping
`openspec validate add-determinized-search --strict`; confirm determinization round-trip (18/18) and unit-game optimal-line tests still green; tick the boxes.

---

## Milestone C — Game-review annotations (Phase 5 + 7.5) — parallelizable with B2

Consumes A (annotation cost = one search per recorded decision). Product payoff (coach mode) + QA payoff (blunder-flagged decisions are where engine bugs hide).

1. **5.1 schema**: extend the 0.5 `winprob-annotations-v1` sidecar → v2 with per-decision `grade`, `played_value`, `best_value`, `best_action`, `prior`. Keep 0.5's hard-won gotcha: mulligan rows are not replayable — reuse `replayable_action_indices`.
2. **5.2 annotator**: new `digimon-engine-cli review <recording> --model <pv.onnx>` subcommand beside `winprob.rs`; at each replayable decision, extract the **mover's infoset** and run `determinized_search`; delta = value(played) − value(best).
3. **5.3 grading**: two grade tracks — decision-time (belief-aware over K sampled worlds) and hindsight (true recorded world). Map delta thresholds → brilliant/great/best/inaccuracy/mistake/blunder; brilliant = best + low prior + only-winning-line. Calibration caveat: the value head is a discounted shaped return, **not** calibrated P(win) (documented in 0.5) — either calibrate value→P(win) on eval games first, or define thresholds in value-space and say so in the schema.
4. **5.4 viewer**: surface grades in the replay viewer; also fix the known 0.5 gap — the `replay` subcommand doesn't unwrap eval-sidecar envelopes (winprob does).
5. **7.5 validation**: grades reproduce (same seeds/config) on a held-out set of recorded games.

---

## Milestone D — Opponent deck prior (Phase 6, PvP) — defer until PvP review demand

Non-blocking by design (hindsight grades need no prior; self-play uses exact priors). When picked up:
- **6.1 recommendation**: archetype posterior over `data/deck_library.json` (robust, cheap), *materialized as* per-card inclusion probabilities derived from the posterior mix — gets the rich representation without full Bayesian inference; point-estimate decklists are too fragile (one wrong card corrupts every world).
- 6.2 classifier from revealed cards + color/turn signals; labels from the recording corpus's endgame decklists. 6.3 wire into the materializer (`DeckPrior` enum is already shaped for an inferred variant). 6.4 surface uncertainty in the review UI.
- Belief-staleness input documented in 1.2 (past peeks/reveals lost at extraction) lands here too if it ever matters empirically.

---

## Sequencing summary

```
A1 (1d) → A2 (2–4d) → re-profile
                     → B1 (1–2d) → B2 (compute, ~1wk elapsed) → 7.4 verdict
A2 done → C1..C5 (parallel with B2)
D: on demand (PvP review feature)
```

Top risks: (1) 4.1 breaking seed-determinism or the sign backup — covered by existing `tests/search/` + 2.5 equivalence tests, run them per iteration; (2) B2 plateau from PIMC strategy fusion — pre-planned A/B levers (IS-MCTS, policy-weighted sampling); (3) stale training image blocking B2 ops — cut image first, verify before renting boxes.
