# Batched search profiling and portable scheduling, 2026-10-23

The user authorized profiling, dynamic scheduling and worker scaling while
keeping the English-meta training run stopped. All runs here are isolated
diagnostics. No generation training, held-out evaluation or promotion ran.

## Implementation boundary

- Native `selfplay --game-start` preserves the original serial deck RNG stream
  and derives every game/search/action-sampling seed from the global game ID.
  Changing worker assignment or count cannot change the requested games.
- The Python generation runner defaults to a dynamic queue with 16 games per
  chunk. Each worker writes a private attempt; successful output is hashed and
  atomically published. Explicit resume validates inputs, settings and outputs,
  preserving completed chunks and rerunning only uncommitted ranges. Worker
  count may change; chunk partition may not. Legacy static mode is retained
  explicitly, with its historical per-worker seed scheme.
- Kernel locks protect both queue and full-generation ownership. Heartbeat and
  event logs, bounded retries, whole-chunk deadlines and owned-child cleanup
  support unattended operation. Failed attempts remain available for diagnosis.
- Opt-in native profiling records nested inclusive/exclusive wall-clock spans.
  It never changes masks, tensors, seeds, budgets, model outputs or backup order.
- Added Windows/Linux scheduler CI, including a POSIX SIGTERM cleanup test.
  Linux execution is not verified locally: no WSL distribution or running Linux
  Docker engine is available. No cloud host was provisioned or contacted.

## Paired observation profile

Normal Cargo release CLI; frozen v3 English pool and seed model; seed 21261025;
320 simulations, four worlds, batch four, cache disabled, default depth/noise/
temperature. Two diagnostic games each stop at 40 actual decisions. Both cases
produce 74 search positions over 80 actual decisions, with two intentional
truncations. These rows are not terminal-outcome training data.

| Case | Process wall seconds | Generated positions/sec |
|---|---:|---:|
| Profiling disabled | 91.595 | 0.808 |
| Profiling enabled | 91.433 | 0.809 |

All eight target arrays are byte-identical. The first game's 38 rows also match
the saved pre-change batched reference bit-for-bit across all eight arrays
(`profile-batched/prechange-parity.json`). The 0.18% timing difference is noise,
not a profiling speedup. This single paired sample shows no measurable profiling
overhead; it does not establish an overhead bound for all workloads.

Exclusive registered region times total 90.547 seconds:

| Region | Self seconds | Share of registered self time |
|---|---:|---:|
| Leaf observation request construction | 75.672 | 83.57% |
| Engine decode/step | 4.981 | 5.50% |
| ONNX session run | 4.692 | 5.18% |
| Explicit search mask builds | 1.980 | 2.19% |
| Explicit game clones | 1.913 | 2.11% |
| Batched scheduler/tree bookkeeping | 0.959 | 1.06% |
| Inference packing/postprocessing, excluding session run | 0.259 | 0.29% |

The observation-request region includes the observation builder and mask copy.
The observation builder itself rebuilds masks, so 2.19% is **not** total engine
mask cost. Spans do not attribute every helper or allocation individually.
`node_request` ran 23,976 times and inference ran 5,994 four-row batches: each of
the 74 searched positions evaluated four roots plus 320 simulation leaves.
Do not sum overlapping inclusive spans or extrapolate this two-game mix to all
English matchups. It supersedes inference-dominant assumptions for this sample.

Code inspection identifies follow-up profiling targets inside observations:
`write_permanent_table` and `write_own_hand` each rebuild the same observer mask;
permanent/source metadata calls `opt_total`, `opt_used`, `source_opt_state` and
`source_dp_contribution`, with repeated effect construction and state-dependent
queries. These are candidates, not individually timed conclusions. A future
optimization must retain exact tensors and masks, reuse the existing mask where
valid, and avoid stale caches of dynamic effects. No tensor or action contract
changed in this work. Additional inference batching and COW clones address much
smaller measured regions here.

Raw report: `runs/2026-10-23-search-performance/profile-batched/benchmark.json`.
Tool: `PYTHONPATH=code python -m tools.benchmark_search_workers profile ...`.

## Worker scaling

The fixed diagnostic game set uses the same master
seed and eight global games, one game per chunk, 40 actual decisions per game,
and unchanged search settings. Counts 1/2/4/6 run sequentially to avoid mutual
contention. All arrays are checked against the one-worker reference. Process
startup, admission and chunk publication are included in wall throughput.

| Workers | Wall seconds | Generated positions/sec | Relative to one worker | Native-work utilization |
|---|---:|---:|---:|---:|
| 1 | 232.589 | 1.230 | 1.00 | 97.0% |
| 2 | 128.041 | 2.234 | 1.82 | 92.1% |
| 4 | 79.307 | 3.606 | 2.93 | 81.4% |
| 6 | 82.290 | 3.476 | 2.83 | 58.3% |

Each case produces the same 286 searched positions over eight deliberately
truncated games. All 64 array files match byte-for-byte across worker counts.
The first two independently dispatched English games also match the monolithic
two-game reference (`worker-scaling/monolithic-parity.json`). Per-game process
time in the single-worker case ranges from 12.4 to 70.9 seconds. The tiny set
has a substantial slow-game tail; native-work utilization is the sum of native
chunk durations divided by workers times wall duration, **not CPU utilization**.

Four workers are a sensible development setting from this sample; six had
3.6% lower throughput, without implying a statistically established optimum. This is one
pass on an occupied desktop. It does not measure steady-state generation speed,
isolate dynamic scheduling's gain against a matched static scheduler, or show
that the same worker count is best on cloud Linux. One-game chunks deliberately
exercise dispatch/recovery and expose startup; tune larger chunks on the cloud
host. Do not use these capped games to estimate a full-generation ETA.

Raw report: `runs/2026-10-23-search-performance/worker-scaling/benchmark.json`.
Reproduce with `PYTHONPATH=code python -m tools.benchmark_search_workers scale ...`.

## Validation and recovery evidence

Ordinary Cargo release verification passed 15 search, eight ONNX and one full
self-play integration test, plus the CLI build. The full self-play test also
replays global game 1 independently and compares all eight terminal target
arrays byte-for-byte. The final Python checks passed 23 tests; the POSIX signal
test is skipped on Windows. The new CI matrix covers Linux and Windows but has
not been executed remotely in this session.

A copy of the completed native eight-game diagnostic was explicitly resumed
with three workers instead of one. All 64 array hashes remained unchanged, the
queue completed, and a process-launch guard confirmed **zero workers launched**.
Recovery tests additionally cover partial failure, preserving already committed
chunks, bounded retries, timeouts, exclusive ownership, corrupt output and
changed-input rejection. Evidence: `native-resume-validation.json` and
`scheduling-final-python-tests.log` under the diagnostic root. Strict array
presence/hash verification passed again for all four native scaling cases.

## Cloud Linux operation

This is an occupied Windows development host (Ryzen 7 3700X, eight physical /
16 logical cores), not the production Linux target. Worker-count results are
development diagnostics, not production sizing or a full-generation ETA. Build
the Linux executable in a frozen image, then repeat profiling/scaling there.
Compare chunk sizes and representative complete games, set deadlines above the
slow-chunk tail, and choose concurrency against CPU quotas and memory pressure.

Keep the full run on one persistent local/POSIX filesystem, with off-host
backups and process-group supervision. Atomic publication handles process
interruption; native array writes are not guaranteed durable through sudden
power loss, and hash failures reject resume. Object-store mounts and unverified
network locks are unsupported. A hard-killed controller requires its supervisor
to stop orphan workers too. See `docs/TRAINING_RUNBOOK.md` for commands, resume
limits, supervisor settings and monitoring expectations.

The existing v3 `pipeline_state.json` remains `stopped_by_user` with
`auto_resume: false`. The older untracked, interrupted generation is preserved;
the new scheduler does not adopt or silently resume it.
