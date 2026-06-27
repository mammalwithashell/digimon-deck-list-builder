# Model Evaluation

How to tell whether an RL model is actually good — across training modes, and
without being fooled by the in-run win rate.

> **TL;DR.** The training eval reuses the *training* opponent, so its win rate
> means a different thing in every mode and is **degenerate under self-play**
> (a mirror pinned near 50%, or a meaningless 100% if the opponent seat goes
> passive). **Never claim a model improved from the in-run / mirror / self-play
> eval.** Judge a model with **anchored evaluation** against *fixed* references
> (greedy + frozen champions), seat-balanced, on one Elo scale; estimate
> robustness with an **exploiter**. (CLAUDE.md rule 30.)

This came out of a real incident (2026-05-31): a self-play run reported a flat
**100%** in-run win rate while it had actually **regressed to 22.5% vs greedy**
(its v22 starting point was 77.5%). Anchored evaluation caught it in minutes.

## 1. Why the in-run win rate lies

The eval opponent is whatever the run trained against
(`eval_env_fn = make_env(opponent=opponent, ...)`):

| Training mode | Eval opponent | What win rate measures | Failure |
|---|---|---|---|
| vs greedy | greedy | real skill vs a fixed bar | saturates once greedy is crushed |
| vs random | random | almost nothing | floor only |
| gauntlet | greedy on sampled meta decks | skill, **deck-confounded** | deck variance dominates |
| generalist | greedy, both decks sampled | skill, **deck-confounded** | high variance |
| **self-play** | **a mirror of itself** | **≈50% + first-player edge** | **no learning signal** |
| pool (frozen) | frozen snapshot(s) | best-response to that snapshot | overfits to the anchor |
| **league** | coupled (policy,deck) from the round pool | best-response to the pool — **mirror-pinned ≈50%** at round 1, where the pool ≈ the generalist the specialist was seeded from | no absolute signal |

Two structural problems: (1) self-play has no native progress signal, and
(2) no two modes are comparable — you can't rank a self-play model against a
vs-greedy model, or against last week's run.

A subtle trap: a raw `DigimonEnv` driven without `OpponentWrapper` leaves player
2 **passive**, so the first player always wins. That reads as a fake ~50%
mirror (or 100%). Any evaluation MUST drive the opponent through
`OpponentWrapper`.

A second trap, specific to the in-training **anchored panel** (the trustworthy
in-run signal, §3): it plays vs greedy + every layout-compatible champion in
`models/champions/registry.json`. With an **empty registry** it silently
degrades to greedy-only — one noisy anchor, not the absolute frame you wanted.
For a warm-started run (e.g. a deck-specialist league), **register the seed as a
champion before launch** (`champion_admin.py promote --candidate <seed>.zip
--name <name> --force`, layout-compatible) so the panel reads `vs ['greedy',
'<seed>']` and answers "is this beating what it was seeded from?". The panel
builds its anchor set lazily at the first panel, so register before the run, not
during it. See the runbook §6.5 / §14.

## 2. The fix: a fixed reference frame

Evaluate against opponents that do **not** move with the learner.

```
   Tier 0  random          floor / sanity
   Tier 1  greedy          skill bar; profile-agnostic (reads game state, not the tensor)
   Tier 2  frozen champions permanent benchmarks (registry); v022/v020-generalist-v1
   Tier 3  held-out scenarios pinned decks + seeds, seat-balanced
```

- **Greedy pins the scale.** Because greedy is identical across observation
  profiles, it bridges profile cohorts and anchors Elo so ratings compare
  across modes and runs.
- **Seat-balance.** Alternate the first player via `seed % 2` so first-player
  advantage cancels. A model-vs-itself mirror then lands at ~50% (verified:
  v22 vs itself = 12-12 at n=24) instead of being decided by who moves first.
- **Deck variance.** Small samples are noisy (same policy, different sampled
  decks can swing 25–75% at n≤8). Use an adequate `--n` and the exact training
  decks via a run's `deck_pool_snapshot.json`.

## 3. The layered eval stack

```
  cheap/noisy  L0  PPO diagnostics: value-loss, entropy, KL      every update
       │       L1  behavioral: game length, digivolves/game      every eval (already logged)
       │       L2  anchored win rate vs greedy + champions        cheap to run
       │       L3  Elo ladder over checkpoints + champions        end of run / periodic
  costly/sharp L4  exploitability (exploiter best-response)        on demand
```

L0/L1 are already emitted (`runs/<name>/evals.jsonl`,
`models/<run>/eval_game_log.jsonl`). L2–L4 are the tools below.

## 4. Tools

- **Anchored evaluation** — `code/digimon_gym/agents/anchored_eval.py`,
  CLI `code/tools/anchored_eval_cli.py`. Plays a candidate vs greedy + every
  layout-compatible champion, seat-balanced. Use
  `--deck-pool-snapshot <run>/deck_pool_snapshot.json` for the exact training
  decks and an adequate `--n`.
- **Champion registry** — `code/digimon_gym/agents/champion_registry.py`,
  `models/champions/registry.json`. Frozen, versioned model snapshots with
  provenance + layout-hash compatibility. Promote with
  `code/tools/champion_admin.py` (gated ≥55% vs the current panel, or manual).
- **Elo ladder** — `code/digimon_gym/agents/elo_ladder.py`,
  CLI `code/tools/elo_ladder_cli.py`. Seat-balanced round-robin over
  `{checkpoints + champions + greedy}`; Bradley-Terry/Elo fit (greedy = fixed
  anchor), Fisher-information standard errors, **cohort gating** by layout hash
  (model-vs-model only within a shared profile), and **forgetting/cycling
  detection** (a later checkpoint losing to an earlier one is flagged).
- **Exploiter** — `code/digimon_gym/agents/exploiter.py`,
  CLI `code/tools/exploiter_cli.py`. Freezes the target as a pool opponent and
  trains a fresh best-response; its peak win rate is the approximate
  exploitability (a **lower bound** — never a robustness certificate).

> **Env note.** Run the CLIs with `PYTHONPATH=code` (or re-run
> `pip install -e .`) — the editable install may point at a stale worktree.
> pytest uses `pythonpath=["code"]` and is unaffected.

## 5. Gated self-play (evaluation-as-training-control)

The AlphaGo-Zero idea: keep one frozen **best player**, train against it, and
**promote** the candidate only when it beats the best by ≥55%. Because the best
can never regress, its rating climbs monotonically — evaluation is *built into*
the training loop, and the best-player Elo curve is the progress signal. This
is the principled cure for self-play's non-stationarity; the champion registry
+ gated promotion implement its evaluation half.

> Distinction: AlphaGo Zero = single frozen best + gating. The win-rate-weighted
> "sample the opponents you lose to" idea is **PFSP** (AlphaStar), a different
> mechanism. The engine has a PFSP `sample(mode="pfsp")` but it is **dormant** —
> `record_match` is never called during training, so it degenerates to uniform.

## 6. Robustness & equilibrium methods (horizon)

The exploiter gives a robustness *number* cheaply. The principled treatments of
hidden-information play live one tier up and are **not yet feasible here**:

| Method | Needs | Offers |
|---|---|---|
| **Deep CFR** | forkable model + infoset structure | offline near-Nash policy (no test-time search) |
| **ReBeL** | public-belief-state structure + forkable model | test-time search, low exploitability (poker-shaped) |
| **Player of Games** | growing-tree CFR + forkable model | both info regimes, test-time search (most general) |

All three require **cheap forking of game state** to traverse the tree, plus
explicit infoset / public-belief structure, plus a tractable belief space (a
50-card deck dwarfs poker's hidden info).

> **Forking precondition PARTIALLY met (`make-engine-cloneable`, 2026-06-23).**
> `Game: Clone` is live and faithful at the **DSL card-effect selection-step
> surface** (the flipped installers — hand/trash/permanent/reveal/material/union +
> the multi-pick trampolines), via a resumable data VM (`src/resume.rs`). So a
> `Game` paused at those select kinds forks cleanly. **It is NOT yet faithful at
> the broader engine surface**: combat/keyword interrupts, digivolve cost-choice,
> BO3 play-order, Overclock, TriggerOrder, replacement-accept, `select_effect_choice`,
> and several other selections are still closure-based and would panic on a forked
> resolve. So the *forkability* requirement of Deep CFR / ReBeL / Player of Games
> is met **only on the flipped subset** — full-game search must wait for the rest
> of the selection surface to flip + the closure executor to be deleted (remaining
> `make-engine-cloneable` work). What each method ALSO needs is the **infoset /
> public-belief structure + the determinization seam** — the **`add-determinized-search`**
> change (determinized / IS-MCTS over `RevealSource`-sampled worlds, reusing the
> 2192-action mask + the observation tensor). The forward-only exploiter remains
> the cheap robustness number meanwhile.

## 7. Recommended workflow

1. Train against a stationary opponent: `opponent="pool"` over frozen champions
   (`champion_admin.py emit-pool` derives the manifest from the registry).
   `opponent="self-play"` is **retired** and fails at startup — the mode was
   structurally unsound (P1-perspective observations with no OpponentWrapper;
   see `harden-training-pipeline`).
2. During/after the run, point `anchored_eval_cli` at the run's checkpoints
   (greedy + champions, `--deck-pool-snapshot`, adequate `--n`).
3. Build the learning curve with `elo_ladder_cli --run <dir>`; watch for upset
   flags (forgetting).
4. For a robustness read, run `exploiter_cli --target <final.zip>` with a fixed
   budget.
5. Promote the result to a champion (`champion_admin`) if it gates ≥55% vs the
   panel — it becomes a permanent benchmark for the next run.

See also: `docs/TRAINING_RUNBOOK.md`, `docs/TRAINING_MCP.md`, and the
`add-model-evaluation-harness` OpenSpec change.
