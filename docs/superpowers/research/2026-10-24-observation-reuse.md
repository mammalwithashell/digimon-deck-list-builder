# Exact observation reuse in search

This change removes duplicate work while preserving the existing observation,
action and search contracts. The running cloud generation is not modified.

Search passes its already prepared mask into the observation builder for the
same immutable state and observer. Lite, deck-aware lite and full observations
share that mask across hand/battlefield affordances (and full-profile action
rows). The compact compatibility profile retains its existing builder.

The lite projection constructs each source's effects once and shares the result
between aggregate once-per-turn counts, per-source availability and DP features.
Succession membership is computed once per stack. Aggregate counts still cover
the entire stack, even beyond visible source rows. Face-down-source redaction,
the historical OPT inherited/top filter, and DP adoption/condition semantics are
preserved. Nothing is cached across state changes, players or sampled worlds.

## Reproduction

Build `observation_performance` in release mode at the baseline and candidate
revisions and preserve the two executables separately. Use the same compiler,
model, card data, deck pool and machine for both. The probe always uses 320 total
simulations, four PIMC worlds, batch four and no evaluator cache.

```text
python code/tools/benchmark_observation_reuse.py --baseline BASELINE --candidate CANDIDATE --model SEED.onnx --decks DECKS.json --out NEW_TIMING_DIRECTORY
python code/tools/benchmark_observation_reuse.py --baseline BASELINE --candidate CANDIDATE --model SEED.onnx --decks DECKS.json --out NEW_PARITY_DIRECTORY --repeats 1 --decision-cap 1200 --hash-leaves
```

The paired runner reverses order on alternate repetitions and compares every
saved array bit-for-bit. `--hash-leaves` additionally compares a SHA-256 digest of
every evaluated observation, legal mask and batch boundary, including leaves
that are not saved as training positions. Keep hashing disabled for the primary
timing comparison. Diagnostic outputs never enter the training pipeline.

## Results

Baseline commit: `eaedce7ce01c5441f627b8510f369031b74eafb7` (already includes the
Lamiamon repair). Both executables were ordinary Cargo release builds using the
same probe, frozen cloud seed model and 24-slot admitted meta/pet pool. Machine:
AMD Ryzen 7 3700X, Windows, single native worker, inference threads set to one.

Four seeded 40-decision prefixes, two reversed-order repetitions each:

| Seed | Baseline mean seconds | Optimized mean seconds | Throughput ratio |
|---|---:|---:|---:|
| 24261024 | 30.10 | 15.35 | 1.96x |
| 24261025 | 35.61 | 16.29 | 2.19x |
| 26261024 | 59.12 | 24.68 | 2.40x |
| 26261025 | 32.45 | 14.85 | 2.19x |

Each implementation generated **294 saved search positions** across the eight
prefix runs. Total native time fell from **314.56s to
142.33s**: **2.21x throughput**
(0.935 to 2.066 positions/sec),
or 54.8% less elapsed native time. Including process startup, the ratio is
2.20x. Observation-request self time fell from
223.12s to 63.12s (**71.7% reduction**).
All eight paired comparisons match bit-for-bit across all eight saved arrays.

A separate full-game parity run completed **two games, 108 decisions, 98 saved
positions and 29,864 evaluated leaves**, with no truncations. The SHA-256 digest
of every evaluated observation, mask and batch boundary matches, as do all eight
saved arrays (including visits, players, termination and outcome labels).
Digest: `b0bafe1be4cf1c58525f2547ac08aed10103fcb1ead10a5912a50a591270af53`.
These full-game runs overlapped compilation and are excluded from timing claims.

Validation:

- **224 Rust tests passed:** 182 mask/tensor (including five new reuse tests),
  18 determinization, 15 search, eight ONNX and one self-play test.
- New tests cover supplied-mask legality reuse, both players and all profiles,
  pending selection, inherited/adopted source effects, dynamic DP conditions,
  shared OPT counters, hidden-source redaction, deep-stack aggregate counts,
  and effect-construction counts.
- **All 13 tier-zero checks passed**, including 174 verification-tool tests.
  The admission manifest's stale Lamiamon-ledger source hash was refreshed;
  every card verdict and aggregate admission count is unchanged.
- Compile check and scoped whitespace checks passed.

These are local development-host measurements, not a Linux cloud benchmark or
an estimate of full-generation games/hour. Repeats reuse seeds and are not
independent playing-strength trials. No search budget, world count, batching,
noise, temperature, model or tensor/action contract changed. The existing cloud
run continues on its original pinned image; adoption requires a later image.

[Machine-readable evidence](2026-10-24-observation-reuse-benchmark.json) includes
input/executable hashes, per-case profiles, exact array checks and full-game
parity. Raw local artifacts live in `runs/2026-10-24-observation-performance/`.
