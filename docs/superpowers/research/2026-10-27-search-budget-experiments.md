# Search budget experiments: phase budgets, playout-cap randomization, Gumbel root

Protocol written 2026-10-27 before collecting any candidate results. Diagnostic
only: no model fitting, no shard promotion, no change to any running job. The
branch is `feat/search-budget-experiments` (worktree `D:/wt/search-bench`); the
baseline binary is an ordinary release build of `origin/main` `a640a4aaa` in
`D:/wt/main-bench` (`D:/cargo-target/main-bench`).

## Why: two thirds of search time is spent on pending selections

The 128-game 640/4-vs-320/4 confirmation panel (`2026-10-23-search-budget-confirmation.md`,
raw `runs/2026-10-23-search-confirmation/results/confirm-*.json` under the
`p3-constructs` worktree) recorded phase, legal-action count and wall seconds for
every non-forced decision of both seats. Tally over all 11,154 searched decisions:

| Root phase | Decisions | Share | Search seconds share | Median legal actions |
|---|---:|---:|---:|---:|
| Main | 3,685 | 33.0% | 33.3% | 9 |
| EffectChoice | 2,613 | 23.4% | 30.2% | 2 |
| SelectTarget | 1,201 | 10.8% | 9.4% | 2 |
| SelectHand | 865 | 7.8% | 8.6% | 5 |
| SelectPermutation | 623 | 5.6% | 2.1% | 1 |
| SelectReveal | 590 | 5.3% | 4.8% | 2 |
| Breeding | 334 | 3.0% | 1.9% | 2 |
| Mulligan | 256 | 2.3% | 0.9% | 2 |
| BlockTiming / SelectUnion / SelectTrash / SelectBudgeted | 906 | 8.1% | 7.9% | 2–5 |
| Alliance / Counter / EndOfTurn / SelectSource | 81 | 0.7% | 0.7% | 2–3 |

**Non-Main roots are 67.0% of searched decisions and 66.7% of search seconds**,
with a median of two legal actions. The budget study's 16-root panel found 160/4
agreed with the 2560-simulation references on all 13 pending-selection roots and
disagreed only on Main roots. The budget is therefore misallocated: most of it
is spent where the choice is nearly forced.

Both seats in that panel searched (640 and 320), so the shares describe the mix
of decisions, not one budget's absolute cost. The mix is what matters here.

## Candidates

All candidates keep PIMC, four worlds, evaluation batch four, a frozen model,
the 24-slot admitted deck pool

**Model.** The 2026-10-23 `seed.onnx` is on tensor schema `standard_lite_v2.3`
(8,410 floats) and no longer loads on `main` (`standard_lite_v2.4`, 9,434
floats). The only local 2.4 export is the archetype-audit checkpoint
(`runs/archetype-audit-20261026/model/model.onnx`, inline terminal-outcome
value head, "checkpoint 0 of the new campaign" in
`2026-10-26-archetype-strength-audit.md`); every run below uses it, with its
SHA-256 in each manifest. Consequently the budget study's 2560-simulation
reference sweeps are regenerated with this model and engine rather than reused.
(`runs/cloud-search-20261024/inputs/decks.json`), decision cap 1,200,
`temperature_decisions` 16, and the existing observation/action contracts.

| Id | Description | Expected cost vs 320/4 |
|---|---|---:|
| B | Baseline: 320 total simulations everywhere | 1.00 |
| C1 | Phase budget: Main roots 320; every other root (pending selection, mulligan, breeding, timing windows) 64 | ≈ 0.33 + 0.67·0.20 ≈ 0.47 |
| C2 | Playout-cap randomization: each non-forced decision is a full search (320, root noise, policy target) with probability 0.25, else a cheap search (64, no root noise) whose row carries `policy_weight = 0` and trains only the value head | ≈ 0.25 + 0.75·0.20 ≈ 0.40 |
| C3 | C1 and C2 combined (cheap-search budget 64 on both axes; a non-Main root under C3 gets 64 either way) | ≈ 0.33·(0.25+0.75·0.2) + 0.67·0.2 ≈ 0.27 |
| G1 | Gumbel root: sequential halving over at most 16 Gumbel-sampled candidates, 128 total simulations (32 per world), PUCT below the root, policy target = completed-Q improved policy averaged over worlds | ≈ 0.40 |

Cheap-search rows still count toward the recorded decision sequence and the
value target (z); they are excluded from the policy loss, following KataGo's
playout-cap randomization. Forced decisions remain unsearched and unrecorded.

## Exactness guards (must hold before any benchmark is reported)

- C1 with the reduced budget set to 320, and C2 with full-search fraction 1.0,
  must reproduce the baseline shards bit-for-bit on two complete games (all
  eight arrays plus the new `policy_weight` array equal to ones).
- Existing search, ONNX, self-play and determinization test suites stay green.
- The trainer loads shards without `policy_weight.npy` exactly as before (all
  weights one) and refuses a `policy_weight` array of the wrong length.

## Throughput measurement

Single native worker, release build, inference threads one, cache disabled,
profiling off, the same eight global game IDs (`--game-start 0 --games 8`,
master seed 24261027) for every candidate so deck pairs and shuffles match.
Report native wall seconds, decisions, saved positions, positions/second and
decisions/second. Repeat baseline and the leading candidate twice in reversed
order. These are occupied-desktop (Ryzen 7 3700X, Windows) numbers: ratios are
the evidence, absolute games/hour are not a Linux sizing claim.

Because the candidates change which action is played after the first few
decisions, the eight games are the same openings but not the same games; the
per-decision cost ratio (seconds per searched decision, split Main vs non-Main)
is the primary throughput metric, total wall time is secondary.

## Quality measurement

1. **Fixed-root panel.** Replay the existing 16 frozen roots
   (`runs/2026-10-23-search-quality/corpus.json`) under each candidate's
   *effective* root budget and compare the preferred action with the stored
   2560/8 and 2560/32 reference results. For C1 this is 64/4 on the 13 pending
   roots and 320/4 on the 3 Main roots; for G1 it is the Gumbel root at 128.
2. **Seat-balanced games vs the baseline.** For each candidate, 16 initial-state
   pairs (32 games) against 320/4 using the `search_quality match` harness
   extended with the new per-seat settings; identical decks, shuffle seed and
   search-seed stream for both seats of a pair; no root noise; first
   most-visited (or Gumbel-chosen) action. Score win=1, loss=0, cap/draw=0.5;
   95% block-bootstrap interval over the eight matchup blocks widened by the
   pair-level Wilson interval, as in the budget study.
3. **Gate (same as the budget study).** A cheaper candidate qualifies for a
   pilot recommendation only if no game truncates and the lower interval of its
   head-to-head score is at least 0.45. Candidates below the gate are reported as
   observed, not recommended. 32 games cannot establish equivalence; a passing
   gate means "no large regression detected", nothing stronger.

## Implementation (branch `feat/search-budget-experiments`)

- `selfplay` driver (`code/digimon-engine/src/selfplay/driver.rs`): a root is
  "Main" when it has no pending selection, no mulligan and `GamePhase::Main`;
  `reduced_simulations` caps every other root. Playout-cap randomization draws
  from its own seeded stream (`derive_seed(seed, 2, game_id | 1<<63)`, consumed
  only when the fraction is below 1, so the reference run's randomness is
  untouched); cheap searches run with `dirichlet_epsilon = 0` and record
  `policy_weight.npy = 0`. The manifest gains `phase_stats` (per root phase:
  searches, seconds, simulations), `full/cheap/reduced` decision counts and
  the budget settings. CLI: `--reduced-sims`, `--full-search-fraction`,
  `--cheap-sims`, `--root puct|gumbel`, `--gumbel-candidates`.
- Trainer (`code/digimon_gym/agents/azero_trainer.py`): loads
  `policy_weight.npy` when present (ones otherwise), filters it with game
  exclusions, validates its shape/sign, weights the per-row policy
  cross-entropy by it and normalizes by the weight sum; refuses a buffer with
  no policy-target rows.
- Gumbel root (`code/digimon-engine/src/search/gumbel.rs`,
  `RootPolicy::Gumbel` in `mcts.rs`): Gumbel-top-k candidates, sequential
  halving over the root budget, completed-Q improved policy
  (`SearchResult::root_policy`) and the survivor as the action to play
  (`SearchResult::chosen_action`); PUCT below the root; Dirichlet noise off.
  Serial and `search_batch` share the scheduler, so batched results stay
  bit-identical to serial. PIMC gives every world the same Gumbel sample
  (derived from the decision seed), averages the improved policies and plays
  the plurality survivor. IS-MCTS ignores the Gumbel root.
- `search_quality` example: request settings accept `reduced_simulations`,
  `root` and `gumbel_candidates`; `choose` plays `chosen_action` when present.
- Drivers: `code/tools/benchmark_search_budget.py` (throughput + exactness
  guards) and `code/tools/search_budget_experiment.py` (frozen-root panel and
  seat-balanced matches, reusing the budget study's job runner, corpus and
  reference sweeps).

### Seat-balanced games vs 320/4 (16 pairs / 32 games per candidate)

`search_budget_experiment.py matches`, match seed 2026102700, 8 held-out EX13
lists, both seats per initial state, no root noise, deterministic play
(argmax improved policy for Gumbel, first-most-visited otherwise). Raw:
`D:/wt/bench-runs/2026-10-27/quality/match_summary_16pairs.json`.

| Candidate | Score vs 320/4 | Approx. 95% interval | Pairs won / lost / split | Truncations |
|---|---:|---|---:|---:|
| c1: Main 320, other 64 | 43.8% | 23.1%–66.8% | 3 / 5 / 8 | 0 |
| g5: Gumbel 128, c_scale 0.1 | 43.8% | 23.1%–66.8% | 2 / 4 / 10 | 0 |
| s64-w4 (PUCT control) | 34.4% | 15.6%–58.5% | 1 / 6 / 9 | 0 |
| g7: Gumbel 64, c_scale 0.1 | 25.0% | 10.2%–49.5% | 1 / 9 / 6 | 0 |

Readings:

- **The frozen-root panel is not predictive of playing strength here.** s64-w4
  matched 320/4 on every panel root yet scored 34% head-to-head; g7 matched
  on 13/16 yet lost 9 of 16 pairs with an interval entirely below 50%. A 16-root
  panel of mostly two-option pending selections cannot see the Main-phase
  planning depth that decides games.
- **Budget matters in play even at pending roots**: c1 (which only cuts the
  non-Main budget) is 43.8% at 32 games — its interval (0.23–0.67) does not
  clear the study's 0.45 lower-bound gate, and does not exclude parity either.
  Eight of its sixteen pairs split (both seats produced the same result), i.e.
  were decided by the deck pairing rather than the budget. The 64-pair
  extension below is the decisive test.
- Gumbel at `c_scale` 0.1 behaves like PUCT at the same budget (g5 ≈ c1-level
  at 128 sims, g7 ≈ below s64 at 64 sims); nothing here shows the Gumbel root
  extracting more strength per simulation than PUCT with this model.
- C2 (playout-cap randomization) is not evaluable this way: in play it is a
  full 320 search; its effect is on training data only, so its test is a
  training generation, not a match panel.

### 64-pair extension (128 games per candidate; raw `match_summary.json`)

| Candidate | Score vs 320/4 | Approx. 95% interval | Pairs won / lost / split |
|---|---:|---|---:|
| c1: Main 320, other 64 | 44.5% | 33.0%–56.7% | 7 / 14 / 43 |
| g5: Gumbel 128, c_scale 0.1 | 43.8% | 32.3%–55.9% | 9 / 17 / 38 |

Neither candidate clears the 0.45 lower-bound gate; neither interval excludes
parity. 43 of c1's 64 pairs were decided by the deck pairing (both seats gave
the same result), so the budget effect is small relative to deck luck, but the
direction is consistently negative: cutting the non-Main budget to 64 is a
modest, real-looking strength loss at the search-policy level, not a free 2.3×.

**Conclusion.** The phase budget is a knob with a known cost, not a default.
The playout-cap randomization is the lever whose cost cut (0.36×) leaves play
unchanged; its test is a same-seed generation comparison on the anchored frame.
The Gumbel root, with this model, does not extract more strength per
simulation than PUCT and is shelved.

## Non-goals

No training runs, no model promotion, no change to the cloud campaign, no
engine-rule changes, and no claim about Linux cloud throughput. Results are
appended below this section as they complete.

## Results

### Exactness guards

The branch binary with default settings reproduced the `main` binary's
8-game run (seed 24261027, 320/4, batch 4) byte-for-byte across all eight
arrays (775 rows). Raw: `D:/wt/bench-runs/2026-10-27/ref-main-B320-8games`
vs `budget-bench/B-320-r0`.

### Throughput (8 games from seed 24261027, single process, two reversed passes)

`code/tools/benchmark_search_budget.py`, raw `D:/wt/bench-runs/2026-10-27/budget-bench-2/benchmark.json`.
Both exactness guards passed (reduced budget 320 and full-search fraction 1.0
each reproduce the baseline's nine arrays). The quality panel ran concurrently,
so absolute seconds are inflated; the ratios are the evidence.

| Candidate | Rows | Searched decisions non-Main | s / searched decision (pass 1, pass 2) | Mean ratio to B-320 | Main s/search | Other s/search |
|---|---:|---:|---:|---:|---:|---:|
| B-320 | 775 | 71.1% | 0.270, 0.226 | 1.00 | 0.27 / 0.22 | 0.27 / 0.23 |
| C1: Main 320, other 64 | 907 | 72.5% | 0.124, 0.095 | **0.44** | 0.29 / 0.22 | 0.06 / 0.05 |
| C2: cap 0.25, cheap 64 | 777 | 70.1% | 0.096, 0.085 | **0.36** | 0.10 / 0.09 | 0.10 / 0.08 |
| C3: both | 694 | 68.0% | 0.063, 0.057 | **0.24** | 0.10 / 0.09 | 0.05 / 0.04 |
| G1: Gumbel 128, m16 | 1,119 | 75.8% | 0.110, 0.099 | 0.42 | 0.10 / 0.09 | 0.11 / 0.10 |
| G2: Gumbel 64, m8 | 802 | 70.8% | 0.057, 0.051 | 0.22 | 0.06 / 0.05 | 0.06 / 0.05 |

No game truncated. The per-decision cost scales almost linearly with the
simulation budget (64 simulations ≈ 0.22× of 320), so the budget policies
deliver close to their arithmetic expectation: C1 0.44× (expected ≈ 0.47),
C2 0.36× (≈ 0.40), C3 0.24× (≈ 0.27). Game lengths differ between candidates
(the play changes after the first few decisions), so rows are not comparable
across rows of this table.

### Fixed-root panel (16 roots: 3 Main, 13 pending; two search seeds)

References regenerated with the archetype-audit model (2560/8 and 2560/32).
"Agreement" is the share of roots whose deterministic preferred action matches
each reference; "stability" is the share where both seeds chose the same action.
Timings are single-process on an occupied desktop with a benchmark running
concurrently — indicative only.

| Candidate | Agreement (w8 / w32) | Main | Pending (w8 / w32) | Seed stability | Mean JS (bits) | s/root |
|---|---:|---:|---:|---:|---:|---:|
| s320-w4 (baseline) | 0.875 / 0.938 | 0.67 | 0.92 / 1.00 | 0.875 | 0.017 | 0.446 |
| c1: Main 320, other 64 | 0.875 / 0.938 | 0.67 | 0.92 / 1.00 | 0.938 | 0.036 | 0.166 |
| s128-w4 (PUCT control) | 0.875 / 0.938 | 0.67 | 0.92 / 1.00 | 1.000 | 0.043 | 0.261 |
| s64-w4 (PUCT control) | 0.875 / 0.938 | 0.67 | 0.92 / 1.00 | 1.000 | 0.067 | 0.095 |
| s8-w4 (≈ prior) | 0.750 / 0.812 | 0.33 | 0.85 / 0.92 | 1.000 | 0.239 | 0.018 |
| g1: Gumbel 128, m16, paper constants | 0.438 / 0.375 | 0.33 | 0.46 / 0.38 | 0.500 | 0.229 | 0.174 |
| g2: Gumbel 64, m8 | 0.375 / 0.312 | 0.33 | 0.38 / 0.31 | 0.500 | 0.255 | 0.090 |
| g3: Gumbel 128, 2 worlds, c_scale 0.25 | 0.562 / 0.625 | 1.00 | 0.46 / 0.54 | 0.500 | 0.147 | 0.216 |
| g4: Gumbel 256, m8 | 0.562 / 0.500 | 0.33 | 0.62 / 0.54 | 0.312 | 0.236 | 0.404 |
| g5: Gumbel 128, m16, c_scale 0.1 | 0.875 / 0.938 | 0.67 | 0.92 / 1.00 | 0.938 | 0.060 | 0.193 |
| g6: Gumbel 320, 1 world, c_scale 0.25 | 0.562 / 0.500 | 0.33 | 0.62 / 0.54 | 0.375 | 0.158 | 0.628 |
| g7: Gumbel 64, m16, c_scale 0.1 | 0.812 / 0.875 | 0.67 | 0.85 / 0.92 | 0.812 | 0.065 | 0.086 |

Readings:

- **c1 chose the same action as the baseline on every root** while cutting
  the pending-root cost from 0.435 s to 0.09 s (2.7× cheaper per root on this
  panel's mix).
- With the current model, **plain PUCT at 64 total simulations already matches
  320's agreement** on this panel (and is more seed-stable); 8 simulations —
  essentially the prior — drops to 0.75. The panel is therefore too coarse to
  separate 64 from 320; that separation needs the seat-balanced games.
- **Gumbel with the paper's constants (`c_visit` 50, `c_scale` 1) is worse
  than PUCT at every budget tried here.** Mechanism (from the sweep rows): per
  world the candidates get 10–30 visits, and
  `σ(q) = (50 + max_visits)·q ≈ 80·q` turns a few hundredths of shallow-Q
  difference into several logits, so the completed-Q policy locks onto whichever
  action drew the better short-rollout value (e.g. 55% on an 11-visit action at
  Q 0.93 where the 2,560-simulation reference prefers a 1,279-visit action at
  Q 0.52) and flips between seeds. Shrinking `c_scale` to 0.1 (g5, g7) restores
  PUCT-level agreement, but at that setting the improved policy is close to the
  prior tilted by Q — i.e. it recovers the PUCT result rather than beating it.
  The evaluation plays the argmax of the improved policy; the Gumbel survivor
  (which carries the exploration sample) is for self-play only.

## Generation comparison: playout-cap randomization (started 2026-10-27 14:26)

Play-time search is unchanged under C2, so the only test that can judge it is
a training generation. Driver: `code/tools/cap_randomization_generation.py`
(`D:/wt/bench-runs/2026-10-27/cap-generation/`), preregistered as follows.

- **Parent:** the archetype-audit 2.4 model. The trainer needs it as an SB3
  `.zip`; only the ONNX exists locally (the cloud host that held the `.zip` is
  unreachable), so `code/tools/sb3_from_onnx.py` rebuilt it from the ONNX
  initializers — verified to ≤1e-6 max logit/value difference against
  onnxruntime on 64 random and 72 real observations, argmax agreement 1.0.
- **Arms**, same master seed `27102026` (game `k` draws the same deck pair and
  shuffle in every arm), 320 sims / 4 worlds / eval batch 4, decision cap 1200,
  16 temperature decisions, 24-slot EX13 pool:
  - `full`: every non-forced decision searched in full, games 0–999.
  - `cap`: `--full-search-fraction 0.25 --cheap-sims 64`, the same games
    0–999 (equal games; ≈0.36× the search compute; ≈25% policy-target rows).
  - `cap_eq`: `cap` plus games 1000–2999 (equal search compute at the
    measured 0.36 ratio, rounded up to whole 250-game partitions).
- **Update:** one `azero_trainer` update per arm from the same parent — 2
  epochs, batch 512, lr 1e-4 (the cloud campaign's settings), cheap rows
  weighted 0 in the policy loss, value loss on every completed row.
- **Judgement:** `code/tools/policy_head_to_head.py`, deterministic masked
  policy without search, four-game blocks per ordered archetype pair (both
  deck assignments × both opening orders), interval over blocks. Panels: each
  child vs parent, each child vs greedy, parent vs greedy, `cap` vs `full`,
  `cap_eq` vs `full`.
- **Reading the result:** `cap` ≈ `full` on equal games means the policy
  targets lost to cheap rows did not matter at this scale; `cap_eq` > `full`
  is the KataGo claim (more games per unit compute buys more); `cap` < `full`
  with `cap_eq` ≈ `full` means the lever is neutral per compute and only
  changes the games/positions mix. One generation from a weak parent is a
  small-effect regime, so a null here does not refute the lever at scale.

### Result (generation finished 2026-10-27 19:00, panels 19:06–20:05)

All 16 partitions completed (250 games each, 0 truncated). Compute and rows
per arm (`report.json`; search seconds are the CLI's own timers, summed over
partitions run 6-wide on one box, so they are relative not absolute):

| Arm | Games | Search s | vs `full` | Rows (value) | Policy-target rows |
|---|---|---|---|---|---|
| `full` | 1000 | 45,018 | 1.00× | 89,487 | 89,487 |
| `cap` | 1000 | 17,281 | 0.38× | 83,652 | 20,732 (24.8%) |
| `cap_eq` | 3000 | 49,401 | 1.10× | 251,005 | 62,243 |

The in-run cost ratio was 0.384, not the 0.36 the plan assumed, so `cap_eq`
is an *equal-plus-10%* compute arm, not exactly equal. Trainer wall: 108 s /
48 s / 144 s.

Head-to-head (candidate score, 95% block interval, 2,112 games per panel =
4 blocks × 132 ordered pairs × 4 seat/order games; `n` is scored games):

| Candidate | Opponent | n | W–L | Score | 95% |
|---|---|---|---|---|---|
| `full` | parent | 2112 | 1104–1008 | 0.523 | 0.501–0.544 |
| `cap` | parent | 2112 | 1114–998 | 0.527 | 0.506–0.549 |
| `cap_eq` | parent | 2111 | 1157–954 | **0.548** | 0.528–0.568 |
| parent | greedy | 2112 | 2046–66 | 0.969 | 0.961–0.976 |
| `full` | greedy | 2112 | 2068–44 | 0.979 | 0.973–0.985 |
| `cap` | greedy | 2112 | 2049–63 | 0.970 | 0.963–0.977 |
| `cap_eq` | greedy | 2112 | 2065–47 | 0.978 | 0.971–0.984 |
| `cap` | `full` | 2112 | 1071–1041 | 0.507 | 0.486–0.527 |
| `cap_eq` | `full` | 2111 | 1092–1019 | 0.517 | 0.497–0.537 |

Readings:

- **Equal games: `cap` ≈ `full`.** Direct panel 0.507 (0.486–0.527); both
  gain the same ~2.5 points over the parent. Discarding 75% of the policy
  targets cost nothing measurable at this scale, for 0.38× the search
  compute. This is the solid result.
- **Equal(+10%) compute: `cap_eq` is the best child.** 0.548 vs parent
  against `full`'s 0.523 (intervals overlap by ~1.6 points), and 0.517
  (0.497–0.537) against `full` directly — suggestive of the KataGo claim,
  not conclusive at one generation from a weak parent, and bought with 10%
  more compute than `full`.
- vs greedy is saturated (all ≥0.97) and cannot separate the arms; it only
  shows no child regressed.
- One generation here moves a policy ~2–5 points vs its parent, so the
  per-update effect size is small; a multi-generation run is where 0.38×
  per generation compounds (≈2.6 generations per `full` generation).

**Decision:** adopt playout-cap randomization (`--full-search-fraction 0.25
--cheap-sims 64`) for self-play generation; keep 320/4 at the full-search
roots. Re-check after a few generations with the same seat-balanced panel.
The phase budget (C1) still needs its own generation test; its 32-game match
was inconclusive.

**Engine fault found by the panels** (1 game, seed 54207751, same in both
`cap_eq` panels; recorded under `eval/engine-faults/`): a `Kotenken Token`
sat in player 2's **hand**, the mask offered "Play Kotenken Token from hand 9"
and execution refused it — the no-op tripwire fired. Filed in
`docs/RUST_ENGINE_GAPS.md` as `G-ENGINE-TOKEN-RETURNED-TO-HAND`. The game is
recorded as `incomplete`, never scored (`--on-engine-error skip`).

## Phase-budget generation test (preregistered 2026-10-28, before generation)

Same protocol, parent, seed, deck pool and panel as the cap-randomization
generation test; the `full` shards, child and `full-vs-parent` panel are
reused unchanged (`--arms full,phase,phase_eq,both,both_eq`, same `--out`).

| Arm | Selfplay flags | Games | Search cost vs `full` |
|---|---|---|---|
| `phase` | `--reduced-sims 64` (C1: every root that is not an unconstrained Main-phase decision gets 64 total sims; Main roots keep 320/4) | 1000 (same indices) | 0.44× (measured per decision) |
| `phase_eq` | same | 2500 = ceil(1000/0.44/250)·250 | ≈1.1× |
| `both` | `--reduced-sims 64 --full-search-fraction 0.25 --cheap-sims 64` (C1 + C2) | 1000 | 0.24× |
| `both_eq` | same | 4250 | ≈1.0× |

Panels: each child vs parent and vs the `full` child, 2112 games, seat-balanced,
block-bootstrap interval, engine faults recorded as `incomplete`. The greedy
panel is dropped: it was saturated (≥0.97) for every arm last time.

Questions, in order:
1. Does cutting the search at pending/mulligan/breeding/timing-window roots
   degrade the **training targets**? `phase` vs `full` at equal games. The
   play-time match had c1 at 44% (inconclusive); a child at or above `full`
   here would say the targets at those roots were already cheap to get right.
2. Equal compute: `phase_eq` and `both_eq` vs `full` and vs parent, read
   against `cap_eq`'s 0.548 / 0.517.
3. Is C3 safe to stack — `both` vs `cap` at equal games (both children exist),
   and `both_eq` is the first arm with >4× the games of `full`.

Reading rules fixed in advance: an interval that excludes 0.5 against `full`
is a result; one that spans it at equal games is "no measurable loss for the
compute saved" (the same rule the cap test used); a child below `full` at
equal games with an interval excluding 0.5 rejects the arm for generation.

### Result (generation 2026-10-27 22:40 – 2026-10-28 ~05:00, panels 07:05 – 09:30)

Generation: 27 partitions, 6750 games, no truncation. The in-run search cost
came out above the per-decision estimates (the reduced roots are cheaper per
simulation than Main roots, so their share of the seconds is smaller than
their share of the decisions):

| Arm | Games | Rows | Policy rows | Search s | vs `full` |
|---|---|---|---|---|---|
| `full` | 1000 | 89,487 | 89,487 | 45,018 | 1.00 |
| `phase` | 1000 | 89,441 | 89,441 | 24,782 | **0.55** (est. 0.44) |
| `phase_eq` | 2500 | 221,798 | 221,798 | 58,458 | 1.30 |
| `both` | 1000 | 85,133 | 21,171 | 12,118 | **0.27** (est. 0.24) |
| `both_eq` | 4250 | 357,036 | 89,145 | 46,737 | 1.04 |

Head-to-head, 2112 seat-balanced games per panel, candidate score with 95%
block interval (the first pass was run at 1 block per pair by mistake — 528
games — and is kept under `eval/blocks1/`; the table is the 4-block rerun):

| Candidate | vs parent | vs `full` child |
|---|---|---|
| `full` | 0.523 [0.501, 0.544] | — |
| `cap` (0.38×) | 0.527 [0.506, 0.549] | 0.507 [0.486, 0.527] |
| `phase` (0.55×) | 0.529 [0.508, 0.550] | 0.492 [0.472, 0.513] |
| `both` (0.27×) | 0.531 [0.509, 0.552] | 0.492 [0.470, 0.514] |
| `cap_eq` (1.10×) | 0.548 [0.528, 0.568] | 0.517 [0.497, 0.537] |
| `phase_eq` (1.30×) | 0.527 [0.507, 0.547] | 0.502 [0.481, 0.523] |
| `both_eq` (1.04×) | 0.525 [0.504, 0.546] | 0.494 [0.473, 0.516] |

Readings, against the preregistered rules:

1. **Equal games: `phase` ≈ `full`** (0.492, interval spans 0.5). Cutting the
   search at pending / mulligan / breeding / timing-window roots to 64 sims
   did not measurably degrade the training targets — the play-time c1 deficit
   (44%) does not carry into generation. No measurable loss for 0.55× compute.
2. **Equal games: `both` ≈ `full`** (0.492) and ≈ `cap`: the two levers stack
   without a measurable cost. 0.27× compute. Every child beats the parent by
   the same 2–3 points (0.525–0.531); one update cannot separate them.
3. **Equal compute: no gain.** `phase_eq` (0.502 vs full) and `both_eq`
   (0.494 vs full, with 4.25× the games) did not improve on `full`, so the
   "spend the saving on more games" effect that `cap_eq` suggested (0.517 vs
   full, 0.548 vs parent) is **not reproduced** by the other two equal-compute
   arms. Read `cap_eq` as the top of the noise band rather than as a KataGo
   effect confirmed here; a multi-generation run is still the only place the
   per-generation saving can show as strength.

**Decision.** Generate with **both levers**:
`--reduced-sims 64 --full-search-fraction 0.25 --cheap-sims 64` (≈0.27× the
search seconds of 320/4 everywhere, ≈3.7 generations per `full` generation),
keeping 320/4 at the Main-phase roots that win the full-search draw. The
equal-compute arms say the saving should be banked as more generations, not
as more games per generation. Re-check after 3+ generations with the same
seat-balanced panel against the `full` lineage.

**Second engine fault** (1 game of 8448 in the 1-block pass, seed 54204079;
did not recur in the 4-block schedule): Medusamon (EX11-012)'s
leave-replacement offered a `CannotBeAffected` token and the pick resolved to
no state change — `G-ENGINE-REPLACEMENT-PICK-UNAFFECTABLE-TOKEN` in
`docs/RUST_ENGINE_GAPS.md`, repro under `qa/engine-faults/2026-10-28-attack-noop-pending-selection/`.

## Gumbel root removed (2026-10-28)

When PR #754 (plan-based `SearchResult::root_plans`, multi-page card choices
searched as complete physical commitments) was merged into this branch, the
Gumbel root (`search/gumbel.rs`, `RootPolicy::Gumbel`, `--root gumbel`,
`--gumbel-candidates`, `SearchResult::root_policy` / `chosen_action`) was
removed rather than ported: it had measured worse than PUCT at every budget
tried, is not part of the adopted generation default, and porting the
improved-policy target onto plan routes would have been untested new work.
The implementation and its tests are recoverable from commit `451ae4de1`
(this branch) if the idea is revisited with a smaller `c_scale`.
