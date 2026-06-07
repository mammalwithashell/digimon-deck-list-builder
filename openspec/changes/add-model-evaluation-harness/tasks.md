# Tasks

Phased so the cheapest, highest-value work (docs + guardrail) lands first; each phase is independently useful.

> **Increment 1 landed (2026-05-31):** the eval *core* — anchored evaluation +
> champion registry + CLI + the CLAUDE.md Working Rule. This is the "lets us
> see" capability, verified by catching that the self-play run regressed (75%
> vs greedy for v22 vs the run's degenerate 100%). **Deviation:** rather than
> extending `eval_suite.py` (task 2.1), a focused new module
> `code/digimon_gym/agents/anchored_eval.py` was built (the `HeldOutEvalSuite`
> is deck-fixed-matchup shaped; the generalist models needed pool-sampled,
> `OpponentWrapper`-driven, seat-balanced play). Still deferred: docs
> (1.1–1.4), default in-run anchored suite (2.3–2.4), gated promotion (3.2),
> exploiter (Phase 4), MCP surface (Phase 5).
>
> **Increment 2 landed (2026-05-31):** the Elo ladder (3.3–3.4) — `code/digimon_gym/agents/elo_ladder.py` (Bradley-Terry/Elo fit, greedy-anchored, Fisher-information SEs, cohort gating by layout hash, forgetting/cycling detection) + CLI `code/tools/elo_ladder_cli.py`. Verified: v020=1172, v022=1123, greedy=1000(anchor).
>
> **Increment 3 landed (2026-05-31):** exploiter (`exploiter.py` + `exploiter_cli.py`), gated promotion (`champion_admin.py`), docs (`docs/MODEL_EVALUATION.md` + INDEX + CLAUDE rule 30 & key-ref), and the training-MCP read-only surface (`champion_standings`, `run_elo_ladder`, `run_exploitability`). Only 2.3/2.4 (in-training anchored callback) deferred — see note. 33 eval tests + 81 MCP tests green.

## 1. Phase 1 — Documentation & guardrail (no code)

- [x] 1.1 Author `docs/MODEL_EVALUATION.md`: per-mode metric taxonomy (greedy/random/gauntlet/generalist/self-play/pool/FSP/PFSP) with failure modes; anchored reference-frame tiers; layered eval stack L0–L4; gated self-play as evaluation-as-training-control (monotone best-player Elo).
- [x] 1.2 Add the "Robustness & equilibrium methods" horizon section (Deep CFR / ReBeL / Player of Games — requirements, payoffs, and the cloneable-engine blocker), forward-referencing `make-engine-cloneable`.
- [x] 1.3 Add a `docs/INDEX.md` entry for `MODEL_EVALUATION.md`.
- [x] 1.4 Add a CLAUDE.md key-reference pointer to `docs/MODEL_EVALUATION.md`.
- [x] 1.5 Add the CLAUDE.md Working Rule (in-run win rate is not a cross-mode signal and is degenerate under self-play; rank on the anchored benchmark, seat-balanced; use exploitability for robustness).

## 2. Phase 2 — Anchored held-out suite

- [x] 2.1 Extend `code/digimon_gym/agents/eval_suite.py` with a frozen-model opponent kind (load SB3 zip; verify observation profile + tensor-layout hash; fail fast on mismatch).
- [x] 2.2 Make every matchup seat-balanced (play both seats, average) and cover it with tests.
- [ ] 2.3 Wire a default anchored suite (greedy + current champion panel) selectable via `TrainingConfig`/job-config; write results alongside `evals.jsonl`. _(DEFERRED: the corrected run uses `opponent="pool"` over frozen champions, whose native in-run eval is already meaningful — not the degenerate self-play mirror — so the in-training callback is a convenience, not a blocker. Anchored eval is available on a run's checkpoints via `anchored_eval_cli`/`elo_ladder_cli` during/after the run. Kept out of the training hot path since it can't be integration-tested without a long run.)_
- [ ] 2.4 Verify on a short self-play run that the anchored win rate is emitted and is independent of the mirror win rate. _(DEFERRED with 2.3; already demonstrated manually — v22 anchored 75% vs the self-play run's degenerate 100%.)_

## 3. Phase 3 — Champion registry & Elo ladder

- [x] 3.1 Define the champion registry artifact under `models/champions/` (name, source run/checkpoint, observation profile, tensor-layout hash, creation date) with read/write helpers.
- [x] 3.2 Implement gated promotion (default ≥55% seat-balanced vs the current panel) plus manual promotion; register v22-final as the first champion. _(`should_promote` + `code/tools/champion_admin.py promote --gate/--force`; v022/v020-generalist-v1 registered.)_
- [x] 3.3 Implement an offline round-robin Elo/TrueSkill evaluator (`code/tools/`) over `{checkpoints + champions + greedy-anchor}`; enforce profile-cohort keys (refuse mismatched layout hashes); emit ratings + uncertainty + full matchup matrix.
- [x] 3.4 Tests: greedy-anchored comparability across two runs; cohort separation; a forgetting/cycling case shows in the matrix; minimum-games + uncertainty reporting.

## 4. Phase 4 — Exploitability (exploiter)

- [x] 4.1 Implement an exploiter driver: freeze a target policy as a pool opponent and train a fresh best-response PPO against it (forward-only; reuse the `opponent="pool"` path; fixed compute budget).
- [x] 4.2 Record the exploiter's peak seat-balanced win rate as approximate exploitability, with the budget, labeled as a lower bound; persist into the run dir.
- [x] 4.3 Tests: exploiter requires no cloning; metric + budget persisted and surfaced as a floor.

## 5. Phase 5 — Training-MCP surface

- [x] 5.1 Add `run_elo_ladder` (ratings + matchup matrix + cohort key) to `code/digimon-training-mcp/` (read-only; no `server.*`/binding imports).
- [x] 5.2 Add `champion_standings` (registry + ratings) and an exploitability query tool.
- [x] 5.3 Update `docs/TRAINING_MCP.md` with the new tools; cross-link from `MODEL_EVALUATION.md` and `docs/TRAINING_RUNBOOK.md`.

## 6. Validation

- [x] 6.1 `openspec validate add-model-evaluation-harness --strict` passes.
- [x] 6.2 Each new tool/evaluator has targeted tests; `python -m pytest code/tests/rl -v` green for touched areas.
- [x] 6.3 Confirm no engine changes were required and the equilibrium-methods section correctly defers to `make-engine-cloneable`.
