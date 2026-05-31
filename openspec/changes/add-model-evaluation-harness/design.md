## Context

The RL stack is `MaskableRecurrentPPO` (LSTM) over the Rust engine via PyO3; training enters through `code/digimon_gym/agents/pilot_training.py` and `code/tools/run_training_job.py`. Today the only progress metric is `WinRateCallback`'s eval win rate, written to `runs/<name>/evals.jsonl` and decomposed per game in `models/<run>/eval_game_log.jsonl`. Checkpoints land every 100k under `models/<run>/checkpoints/`.

The eval opponent is whatever the run trained against (`eval_env_fn = make_env(opponent=opponent, ...)`). That makes the metric mode-dependent and, under `opponent="self-play"`, degenerate: both seats are the live policy, so win rate is ≈50% + first-player edge regardless of learning. There is no fixed yardstick, no cross-mode comparability, and no robustness signal. `HeldOutEvalSuite` (`code/digimon_gym/agents/eval_suite.py`) already implements fixed decks/seeds against greedy/random but is unused and has no frozen-model anchor.

This design emerged from an explore session covering the self-play eval problem, AlphaGo-Zero-style gating, and the imperfect-information equilibrium family (Deep CFR / ReBeL / Player of Games).

## Goals / Non-Goals

**Goals:**
- One comparable scale across all training modes and runs (Elo anchored on a stationary reference).
- A stable in-run progress signal for self-play and pool modes (anchored win rate, not the mirror).
- A robustness signal (approximate exploitability) obtainable on the engine as-is.
- The lesson documented (`docs/MODEL_EVALUATION.md`) and guard-railed (CLAUDE.md Working Rule).
- Reuse existing artifacts (`evals.jsonl`, `eval_game_log.jsonl`, checkpoints) and the existing `digimon-training-mcp` surface.

**Non-Goals:**
- Implementing Deep CFR / ReBeL / Player of Games (documented as horizon only; they depend on `make-engine-cloneable`).
- Any engine change or new tensor profile.
- Cross-observation-profile exactness in model-vs-model play (handled approximately via the greedy bridge; documented as a limitation).
- Changing reward shaping or the training loop's optimization.

## Decisions

**D1 — Separate "is this run learning?" from "is model A better than B?".** The in-run callback answers the first cheaply (anchored win rate vs a stationary reference); an offline Elo ladder answers the second. Conflating them is the root cause today. Alternative (one richer in-run metric) rejected: round-robin Elo is too expensive to run every eval window.

**D2 — Anchor on greedy as the universal, profile-agnostic reference.** Greedy reads game state, not the observation tensor, so it is identical across tensor profiles and never drifts. It pins the Elo scale so ratings compare across modes/runs/cohorts. Frozen champions are added as stronger anchors but are profile-bound. Alternative (anchor on a fixed model) rejected: any model anchor is profile-bound and itself drifts as champions are promoted.

**D3 — Champions are an explicit, versioned registry under `models/champions/`.** Each entry: name, source run/checkpoint, observation profile, tensor-layout hash, creation date. Promotion is gated (default ≥55% over a seat-balanced match vs the current champion panel) — the AlphaGo-Zero rule, repurposed as an *evaluation* registry rather than a training pool. Manual blessing is also allowed. Alternative (auto-promote every improved model) rejected: bloats the ladder and the eval cost.

**D4 — Extend `HeldOutEvalSuite` rather than build a new evaluator.** Add a frozen-model opponent kind (load an SB3 zip, profile-checked) alongside greedy/random, and require seat-balanced matchups (each matchup played from both seats, results averaged) to remove first-player bias. Enable a default anchored suite (greedy + current champions) for runs via config. Alternative (bolt anchored eval onto `WinRateCallback`) rejected: the suite already models fixed decks/seeds/opponents cleanly.

**D5 — Exploitability via a forward-only PPO exploiter, not CFR.** Freeze the target policy as a pool opponent; train a fresh best-response PPO against it on the real engine; its peak win rate is approximate exploitability. This needs only `step()` (no cloning) and reuses the existing `opponent="pool"` path. It delivers the single most valuable property of the equilibrium family (a robustness number) at the cost of one extra PPO run. Alternative (true exploitability via best-response search / CFR) rejected: requires a cloneable/forkable engine — out of scope, see `make-engine-cloneable`.

**D6 — Elo cohorts are keyed by observation-profile + tensor-layout hash.** Model-vs-model games are only run within a cohort; greedy/random results bridge cohorts approximately. The ladder tool refuses to play two models with mismatched layout hashes and records the cohort key in its output. Alternative (force one profile) rejected: profile bumps are a normal occurrence (see the lite-deck-v2 flip).

**D7 — Surface through `digimon-training-mcp`, not the hosted API.** The training MCP already owns read-only cross-game inspection of `runs/`+`models/`; Elo/champion/exploitability queries fit its mandate and stay out of the DB-backed server. Alternative (hosted API endpoints) rejected: these are local-analysis concerns, and the MCP boundary (no `server.*` imports) is the right home.

**D8 — Phase the work cheap-first.** Docs + the CLAUDE.md rule ship first (highest value/lowest cost and the explicit user ask), then the anchored suite, then the Elo ladder + champion registry, then the exploiter, then the MCP surface. Each phase is independently useful.

## Risks / Trade-offs

- **Anchored win rate vs greedy saturates** once the agent crushes greedy → Mitigation: champion anchors (Tier 2) extend the ceiling; the Elo ladder is the real long-run curve.
- **Exploiter under-trains and reports falsely-low exploitability** → Mitigation: fix the exploiter's compute budget, report it alongside the number, and treat exploitability as a lower bound (a *floor* on how exploitable you are), never a guarantee of robustness.
- **Elo instability with few games / intransitive (rock-paper-scissors) matchups** → Mitigation: minimum games per pair, report confidence intervals, and surface the full matchup matrix (not just the scalar) so cycling is visible rather than hidden.
- **Cross-profile comparisons mislead** → Mitigation: cohort keys enforced by layout hash; the greedy bridge is labeled approximate in both the tool output and the doc.
- **Champion-registry drift / disk growth** → Mitigation: versioned, curated, gated promotion; document a retention policy.
- **Seat imbalance via the `Game::new` `seed % 2` first-player rule** → Mitigation: the suite plays each matchup from both seats and averages (the existing BO3 seed-parity trick is the precedent).

## Migration Plan

Additive only. Phase 1 (docs + CLAUDE.md rule) requires no code and is immediately useful. Later phases add opt-in tooling and a default-on anchored suite for *new* runs; existing runs and artifacts are untouched. No rollback concerns beyond reverting additive files. The equilibrium-methods horizon section explicitly forward-references `make-engine-cloneable` as its precondition.

## Open Questions

- Champion promotion cadence: per run, or a periodic offline gate against the panel?
- Elo vs TrueSkill (TrueSkill gives variance/uncertainty natively and handles sparse matchups better) — default choice to confirm.
- Should the exploiter also seed a permanent "exploiter champion" used as a future anchor?
- Default size of the held-out anchored suite (games per cell vs eval wall-clock budget).
