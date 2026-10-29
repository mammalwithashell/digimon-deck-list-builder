# Fair belief self-play: final local evidence

Implementation and the preregistered comparison are complete. **Promotion failed; no champion was promoted.** The closed-list models were weaker in policy-only play and more vulnerable to the fresh exploiters in this cohort. Search-assisted point estimates improved, which does not establish stronger policy-only deployment.

## Population and controls

- Twenty distinct admitted ST1-derived variants: 16 training lists, two validation lists, two held-out test lists; 15 distinct source card IDs in one named family. This is neither the full starter deck nor a cross-archetype evaluation.
- Stock ST1-09 was excluded by the existing card-admission gate. Legal counts were refilled using admitted cards. No admission or mechanics restriction was weakened.
- Frozen standard/ENG repository restriction policy; 1,066 globally admitted fallback IDs. Candidate lists use only the training split, equal distinct-list weights, 10% unseen mass and 128 bounded fallback attempts. Held-out lists never enter candidate priors or training.
- Identical compatible untrained MLP initializer and matched training budgets: seeds 11, 22, 33; two 32-game generations per seed/regime; 32 simulations, four worlds; two training epochs per generation. The initializer itself was evaluated separately.
- All 384 training games completed without truncation. Independent comparison: 6,144 games, plus 768 separate initializer games, for **6,912 evaluation games**. Every panel/seed/model received 256 games, 128 opening and 128 playing second. No evaluation game failed or truncated; none was dropped.
- Six fresh fair-information masked-PPO exploiters, one per frozen target/seed, each trained for exactly 2,048 learner steps. Their held-out results are a finite-budget best-response estimate, not certified exploitability.
- Native search-assisted evaluation uses the frozen CLI configuration: 32 simulations/four worlds, root Dirichlet epsilon 0.25 and temperature for the first 16 decisions. Both sides use the same settings. Policy-only panels use deterministic masked actors.

## Descriptive aggregates

Each entry pools 768 games across three separately trained seeds. These aggregates are descriptive; the registered gates use per-seed conservative intervals. Exploiter rows report the **attacker** win rate, where lower is better for the target.

| Panel | Exact-prior-trained baseline | Closed-prior-trained candidate | Candidate minus baseline |
|---|---:|---:|---:|
| Policy only vs fixed champion | 420/768 (54.7%) | 354/768 (46.1%) | -8.6 pp |
| Policy only vs greedy | 274/768 (35.7%) | 202/768 (26.3%) | -9.4 pp |
| Fresh exploiter win rate | 346/768 (45.1%) | 472/768 (61.5%) | +16.4 pp |
| Equal-budget closed search vs fixed champion | 403/768 (52.5%) | 454/768 (59.1%) | +6.6 pp |

The unchanged initializer won 2/768 (0.26%) against greedy on its separately registered opening seeds. Both trained regimes learned useful behavior relative to that initializer; the closed regime did not beat the trained baseline.

## Per-seed results and registered uncertainty

Each cell shows baseline wins, candidate wins, and the candidate-minus-baseline conservative 95% interval in percentage points. All denominators are 256. Intervals subtract 97.5% Wilson component intervals; no simultaneous multi-panel coverage is claimed.

| Panel | Seed | Baseline wins | Candidate wins | Delta interval (pp) | Registered rule | Passed |
|---|---:|---:|---:|---:|---|---|
| heldout_policy | 11 | 156 | 158 | [-12.8, +14.3] | lower bound >= +2 pp | no |
| heldout_policy | 22 | 87 | 24 | [-34.8, -13.4] | lower bound >= +2 pp | no |
| heldout_policy | 33 | 177 | 172 | [-14.9, +11.0] | lower bound >= +2 pp | no |
| anchors | 11 | 113 | 78 | [-26.7, -0.1] | lower bound >= -3 pp | no |
| anchors | 22 | 42 | 11 | [-20.0, -3.8] | lower bound >= -3 pp | no |
| anchors | 33 | 119 | 113 | [-16.1, +11.5] | lower bound >= -3 pp | no |
| exploiter | 11 | 123 | 135 | [-9.3, +18.5] | upper bound <= +3 pp | no |
| exploiter | 22 | 125 | 246 | [+36.6, +56.1] | upper bound <= +3 pp | no |
| exploiter | 33 | 98 | 91 | [-16.1, +10.7] | upper bound <= +3 pp | no |
| heldout_search | 11 | 135 | 161 | [-3.7, +23.6] | lower bound >= -3 pp | no |
| heldout_search | 22 | 139 | 148 | [-10.3, +17.2] | lower bound >= -3 pp | no |
| heldout_search | 33 | 129 | 145 | [-7.7, +19.9] | lower bound >= -3 pp | no |

Seed 22 shows a clear regression: candidate policy-only wins fell to 24/256 from 87/256; its fresh attacker won 246/256 versus 125/256 against the baseline. The other seeds do not establish the required uplift/non-regression. Every required panel failed or remained inconclusive, so the gate blocks promotion. No held-out result was used to tune or rerun model training.

## Counting, belief and runtime evidence

- Composition-changing paired games produce identical closed worlds and PIMC/IS-MCTS outputs under fixed entitled facts/seeds. A hypothetical card absent from the true list can legally resolve. Closed world conservation is checked against the sampled list.
- Per-seat recovery/inspection memory, recycled-copy counting, shuffle invalidation, unique instances, legal masks and explicit unsupported continuation errors are covered. Private security-to-hand transfers preserve entitled identities. Hidden transformations, opaque games and unaudited continuation forms fail explicitly.
- Full trash accounting sees the 46th trash identity even when the policy tensor is identical. The existing 45-row policy identity window remains a deployment limitation; no observation or action contract was changed.
- Training scoring: 11,211 completed decisions, 45,640 truth cards, 44,844 sampled worlds, 4,463 fallback worlds, 71.09% empirical card support, log loss 11.724, Brier 0.9781. No incomplete decisions were included as completed evidence.
- Held-out search scoring: baseline 41,537 decisions / 179,222 cards, 72.20% empirical support, log loss 11.391, Brier 0.9769; candidate 40,345 decisions / 171,347 cards, 71.54% support, log loss 11.598, Brier 0.9780. Different played trajectories make these descriptive, not a paired policy-strength test.
- Scores are finite-world per-card marginal estimates, not exact joint-hand posteriors. A possible true card can have zero empirical mass in four samples; log loss uses a 1e-15 floor. Calibration bins and all sample counts are retained in JSON. Truth is written only after games end and never read by actors, priors or trainers.
- No played-cohort scoring decision eliminated all training candidates. The independent unfamiliar-card probe does: all 1,000 fallback requests succeeded with zero failures, validating this path without claiming it occurred in these played games.
- Prior benchmark (16 candidates / 1,066 IDs / 1,000 requests per case): initial conditioning 0.129 s, mixture sampling 1.685 s; unfamiliar conditioning 0.175 s, all-fallback sampling 14.039 s. These are fallback-request successes, not per-proposal rejection acceptance. Larger populations were not benchmarked.
- Local CPU training job wall times were 82.0-118.1 s per two-generation model; observed peak process-tree RSS was 4.13-4.28 GiB at 0.5-second intervals. Timing includes training/export and concurrent host load. Phase/search and per-chunk timing denominators are preserved separately; no scaling guarantee is inferred.

## Validation and artifacts

- Rust: 46 determinization, 19 search and six self-play tests passed. Tier 1: 2,077 tests passed, five existing ignores. Extra security tests: two passed, one recorded-prefix failure at game268 decision79 before reaching search; the installed pre-change binding reproduces the rejection. This aggregate failure remains documented.
- Repository tier 0: all 13 checks passed, including 183 tooling tests. Python focused suites: 54 passed, one existing skip. Strict OpenSpec validation covers this change and the reconciled parent search change.
- Closed end-to-end smoke completed generations 0/1, then cold-resumed generation 2: six complete games, compatible SB3/ONNX exports, no truncations. Changed regimes/priors, mixed replay buffers and mutated scoring sidecars are rejected.
- Models, per-game outputs, raw logs, failed development preflights and immutable executable remain under `D:/cargo-target-wt/closed-deck-belief-training/`. No cloud resource was provisioned. The protocol comparison directory is `full-20261028/comparison-release`.
- Study controller/algorithm source: commit `c8f3e8c7d`; frozen hashes are in `evidence/identity.json`. The later metadata-only controller hardening also fingerprints the actual loaded PyO3 extension, rather than only its package shim. The study extension was separately hashed during the run and verified after completion; its last write preceded run identity creation. See `runtime-integrity.json`. Future controller identities require a new comparison directory; completed study data was not altered.
- Six final candidate checkpoints and all six exploiter target/model hashes are in `evidence/models.json`. Exact prior, population, splits, runtime versions, outcomes, intervals, scoring and resource measurements are committed next to this report.

This experiment establishes working fair-information training and rejection of an unsuccessful candidate. It does not establish stronger closed-list policies or low exploitability. The reason for seed22 instability is not isolated by this study; any further tuning needs training/validation evidence and a new independent qualification protocol.
