# EX13 fresh run with value-head fixes and CP8 as an opponent

Status: **planned, not launched** (2026-11-07). Background: [EX13_TRAINING_FINDINGS.md](../../EX13_TRAINING_FINDINGS.md).

## Why

The fresh-history EX13 lineage improved steadily against fixed opponents without search, but with
search on both sides CP8 beat CP12, CP16 and CP20 (39–44%). An October 31 audit (in
`D:/wt/p3-constructs/runs/ex13-value-offset-20261031` and that day's position-audit script) found the
value head barely beats guessing on unseen games: held-out MSE 0.88 (CP8) to 0.97 (CP16), against
0.98 for always predicting the average. Meanwhile, its training loss fell from 0.43 to 0.25. Adding
a constant +0.25 to CP16's value raised it from 0.38–0.39 to 0.46 against CP8. So the value head
memorises its training games, and its overall level alone moves search strength.

This run starts from fresh weights with the value fixes in place from generation 0, a broader
seed, and CP8 phased in as a fixed training opponent. It is judged with search on both sides.

## What changed in code (this branch)

| Change | Where | Flag |
|---|---|---|
| Hold out whole games from both losses; save the best held-out epoch | `azero_trainer.train` | `--value-holdout-fraction` (driver), `--holdout-fraction` (trainer) |
| Log held-out value MSE next to the always-guess-the-mean MSE each epoch | `azero_trainer.heldout_metrics` | (in `summary.json` → `train.epochs[].heldout`, `train.selection`) |
| `tanh` on the value output, baked into the ONNX graph | `value_activation.py`, `export_onnx.py`, `sb3_from_onnx.py` | `--value-activation tanh` |
| Critic-only refit from the CLI, keeping the best held-out epoch | `azero_trainer --calibrate-value-only` | |
| Seed arm `mixed_v2`: greedy/development pairing, Tamer-first variants on chosen archetypes, uniform-random seats whose rows are value-only (`policy_weight` 0) | `cloud_search/mixed_bootstrap.py` | `--arm mixed_v2 --tamer-first-archetype "Light Hybrid" --random-fraction 0.175` |

Native search is unchanged: a `tanh` export is already in [-1, 1], so the engine's clamp is a no-op.
Unvisited moves are still scored at Q=0 (`search/mcts.rs`). That interaction with value level is
**not** addressed here and is a separate A/B test.

## Pre-checks (before spending collection compute)

1. **Value error by decision type on CP8.** Rerun the October 31 held-out audit, split into
   main-phase decisions vs effect choices made mid-resolution, and check the turn-handoff sign flip
   `V(t) + V(t+1) ≈ 0`. If mid-resolution error is far worse, the board encoding lacks
   pending-effect information. Fix that before training: a fresh run would hit the same limit.
2. **CP8 value-only refit.** `--calibrate-value-only --value-activation tanh` on CP8's own
   generations 6–8 games (the policy is unchanged bit for bit), then about 240 games against plain
   CP8 with search on both sides (320 sims, 4 worlds, held-out closed lists). Above 50% is direct
   evidence that the value fixes are worth building the run around.
3. **Tamer-first sanity.** About 200 Light Hybrid games, Tamer-first teacher vs greedy. If it
   clearly loses, keep it for coverage but give it a smaller share.

### Pre-check 2 result (2026-11-08): the refit lost — do not launch yet

CP8 with a critic-only `tanh` refit (generations 6–8, whole-game holdout, best epoch 1 of 8)
against unmodified CP8, 520 games on the October 31 value-offset protocol (same seed, decks,
engine build, 320 sims / 4 worlds both sides, known decklists): **196–324, 37.7%
[33.6%, 41.9%]**. Every archetype except TS Mervamon (65%) and TS Jupitermon (50%) scored below 50%.
Evidence: `D:/ex13-data/cp8-tanh-refit-vs-cp8/` (summary, plan, receipts) and
`D:/ex13-data/cp8-value-refit/` (refit, `heldout-comparison.json`).

On the refit's held-out games the refit was slightly more accurate than original CP8 (MSE 0.233
vs 0.264; always-guess-the-average 0.978) and never left [−1, 1] (original: 34% of raw outputs did).
But it lowered the average value from +0.29 to +0.13. Those held-out games come from CP8's own
training window, so neither number is a true out-of-sample measure.

Reading: more accurate terminal-outcome values made search **worse**, about as much as the
October −0.25 constant shift (0.399). The value level relative to search's Q=0 for unvisited
moves looks like the main lever, which this plan explicitly left out. Until that is separated,
the plan's core assumption (better value fitting → better search) is unsupported. Next test:
the same refit with its average shifted back to CP8's level. If that recovers to about 50%, change
search's unvisited-move value before training anything.

### Follow-up panels on the current-main engine (2026-11-08, Hetzner)

Engine built from `c9832170b` (main as of #760) with main's card data; same decks, seed and search
settings. Evidence: `/opt/digimon-search/ex13-main-panels-20261108` (copied to
`D:/ex13-data/main-panels-20261108`).

| CP8 variant vs plain CP8 | Games | Score | 95% interval |
|---|---:|---:|---:|
| tanh critic refit | 520 | 37.9% (197–323) | 33.8–42.1% |
| tanh critic refit + 0.1602 (average value matched to CP8) | 520 | 43.1% (224–296) | 38.9–47.4% |

The refit loses on the current engine too, and matching the level recovers only about 5 of the 12
points. A critic refitted onto CP8's frozen body from three generations of its own games is worse
for search than CP8's jointly trained critic, beyond its level. This does not test a value head
trained jointly from scratch with holdout and tanh, which is what the campaign tests; the
CP8 stop rule guards it. **Decision: launch as prepared.**

Pre-checks 1 and 2 need the generation 6–8 game data, which is still on the Hetzner host under
`/opt/digimon-search/` (never archived locally).

## Run recipe

Kept identical to the old lineage except for the deliberate changes, so its generations remain a
rough comparison point.

- **Seed:** `mixed_bootstrap collect --arm mixed_v2 --tamer-first-archetype "Light Hybrid"
  --random-fraction 0.175 --rounds 32` (416 games, same training snapshot), then `fit`.
  The seed fit is imitation followed by a held-out value calibration. For `mixed_v2` that
  calibration uses `tanh` and keeps the best held-out epoch.
- **Collection (unchanged):** 1,300 games per generation, 8 workers, `--sims 320 --worlds 4
  --eval-batch-size 4 --reduced-sims 64 --full-search-fraction 0.25 --cheap-sims 64`,
  `--window 3 --replay-games-per-generation 1300`, same `decks.json`.
- **Fitting (changed):** `--epochs 4 --lr 1e-4 --value-holdout-fraction 0.2 --value-activation
  tanh`. Four epochs is now the ceiling, not the choice: the best held-out epoch is saved.
- **Opponents (as launched, 2026-11-08).** Non-self-play games are split across pool entries by
  weight; history generations weigh 1. The old fresh campaign played the page-search CP2 at
  self-play 0.5 from generation 1, so that is kept, and CP8 is added with weight 2:

  | Generations | Pool | Self-play | CP8 share |
  |---|---|---|---|
  | 1–2 | CP2 (w1) | 0.5 | 0 |
  | 3–4 | CP2 (w1), CP8 (w2), history window 2 | 0.6 | 0.4 × 2/5 = 16% (20% at gen 3) |
  | 5–12 | same | 0.5 | 0.5 × 2/5 = 20% |

  Self-play cannot go below 0.5: the driver requires at least 150 expected learner appearances per
  archetype, 1,300 × (1 + self-play) / 13. One 12-generation campaign carries all three phases via
  `historical_opponents_by_generation` (new, `run_small_generations.py`); the plan is written by
  `code/tools/cloud_search/prepare_fresh_value_campaign.py`. CP8 ONNX: `D:/wt/p3-constructs/runs/ex13-cp8-search-lift-20261029/inputs/checkpoint8.onnx`,
  sha256 `4a026e27b7af5e88be35ace34819b847c035a87fe80e55589c33ef8dc99d5963` (the same file the old run
  used as `ex13-lineage-checkpoint8`).
- **Length:** 12 generations at most. The old lineage stopped improving around generation 14.

Not in scope: training on CP8's own rows. The collection pipeline refuses opponent-seat rows by
design (`search_opponents.summarize_opponents`), so this would need its own change.

## Checks and stop rules

- **Every generation:** held-out value MSE vs the constant baseline (from `summary.json`). Stop if
  it fails to beat the baseline for two generations in a row.
- **Every generation:** also the held-out average value (`train.selection.selected.mean_value`).
  Search scores unvisited moves at Q=0, so drift in value level changes exploration even when
  accuracy is fine; that drift is what the pre-checks implicated.
- **Every 2 generations (from generation 4):** the 520-game `local_checkpoint_search` panel against
  frozen CP8 (known decklists, the October 31 protocol and seed), 320 sims and 4 worlds on both sides,
  run on this PC with a Windows build of the same engine source so it does not take CPU from
  training. (Closed held-out lists remain the stronger test; that harness is heavier and was not
  used for the per-generation gate.)
- **Stop:** two of those checks in a row below 47% → stop and investigate. Do not keep training to
  see if it recovers.
- **Success:** above 50% against CP8 by generation 12, confirmed by a full 480-game panel before
  any promotion. Policy-only anchor results are reported alongside, never instead.

## Open risks

- The closed-list RunTail continuation rejection (October 30) is unexplained; scored closed-list
  panels can still stop on it.
- The Virus Busters repeated-action loop (CP16 search-lift panel) was filed as an engine fault.
- Hetzner's 6.3 GB container limit caused an out-of-memory failure in closed-list evaluation.
  Keep collection and scored checks on separate hosts or reserve memory.
- `code/tests/tools/test_payment_recovery.py::test_native_generation_driver_starts_two_from_cp1_without_old_shards`
  fails on unmodified HEAD (`legacy run has no information identity`). It is unrelated to this
  change, but it covers the generation driver, so fix it before relying on the driver's resume path.
