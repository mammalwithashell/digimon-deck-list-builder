## Why

In-run "win rate" (`runs/<name>/evals.jsonl`, `WinRateCallback`) is the only progress signal we report, but it means a **different thing in every training mode** because eval reuses the training opponent (`eval_env_fn = make_env(opponent=opponent, ...)`):

- **vs greedy** → a real skill signal.
- **gauntlet / generalist** → skill, but confounded by which decks got sampled.
- **self-play** → a mirror match, pinned at **≈50% + first-player edge** — no learning signal at all.
- **pool (frozen snapshot)** → best-response to that snapshot; overfits to it.

Two structural failures follow: (1) **no two modes are comparable** — you cannot rank a self-play model against a vs-greedy model, or against last week's run; and (2) **there is no robustness/exploitability signal**, so PPO self-play can produce a cycling or exploitable policy with no warning. We need a mode-independent way to put every model on one comparable scale, and the lesson written down so it isn't relitigated.

## What Changes

- **Anchored evaluation against a fixed reference frame** that does not move with the learner: Tier 0 random (floor), Tier 1 greedy (skill bar, profile-agnostic), Tier 2 frozen **champion** model snapshots, Tier 3 held-out scenarios (pinned decks + seeds, **seat-balanced** to remove deck-luck and first-player confounds). Extends the existing-but-unused `HeldOutEvalSuite` (`code/digimon_gym/agents/eval_suite.py`, today greedy/random only) to frozen-model anchors, and enables an anchored suite **by default** for future runs.
- **A champion registry** of frozen, versioned, named model snapshots as permanent benchmarks (v22-final is the first), with an AlphaGo-Zero-style **gating/promotion rule** (e.g. ≥55% to promote) defining what earns champion status.
- **Cross-model Elo/TrueSkill ranking** via an offline round-robin over `{checkpoints + champions + greedy-anchor}`, with the greedy anchor pinning the scale so numbers compare across modes and runs, and **cycling/forgetting detection** (a later checkpoint that loses to an earlier one). Ratings are valid within a shared observation-profile cohort; greedy/random bridge cohorts approximately.
- **Exploitability via a periodic exploiter**: freeze the current policy, train a fresh PPO best-response against it on the **real engine** (forward-only — **no engine cloning required**), and read its peak win rate as approximate exploitability. This is the near-term, highest-leverage borrow from the CFR/equilibrium family.
- **`docs/MODEL_EVALUATION.md`** documenting the per-mode metric taxonomy, the anchored reference-frame tiers, the layered eval stack (L0 PPO diagnostics → L1 behavioral → L2 anchored win rate → L3 Elo ladder → L4 exploitability), gated self-play as evaluation-as-training-control, and a **"Robustness & equilibrium methods" horizon section** (Deep CFR / ReBeL / Player of Games — requirements, payoffs, and their shared dependency on a cloneable engine, forward-referencing `make-engine-cloneable`).
- **CLAUDE.md**: a key-reference pointer to the new doc and a **Working Rule** encoding the lesson.
- **Training-MCP surface**: new query tools (`run_elo_ladder`, `champion_standings`, exploitability) on the existing `digimon-training-mcp`.

## Capabilities

### New Capabilities
- `anchored-model-evaluation`: fixed-reference evaluation tiers (random / greedy / frozen champions / held-out scenarios), seat-balanced, frozen-model anchors, default-on for runs; plus the champion registry and its gating/promotion rule.
- `checkpoint-elo-ladder`: offline round-robin Elo/TrueSkill over checkpoints + champions + greedy-anchor; profile-cohort-aware; cross-mode/cross-run comparability; cycling/forgetting detection.
- `policy-exploitability-eval`: periodic exploiter best-response producing an approximate-exploitability robustness metric, engine-clone-free.
- `model-evaluation-guide`: `docs/MODEL_EVALUATION.md` content contract + `docs/INDEX.md` entry + CLAUDE.md reference and Working Rule.

### Modified Capabilities
- `training-status-mcp`: add read-only `run_elo_ladder`, `champion_standings`, and exploitability query tools over `runs/` + `models/` artifacts.

## Impact

- **Code**: `code/digimon_gym/agents/eval_suite.py` (frozen-model anchors, seat balance), `code/digimon_gym/agents/pilot_training.py` + `code/tools/run_training_job.py` (anchored-suite default wiring), new `code/tools/` evaluators (Elo ladder, exploiter driver), `code/digimon-training-mcp/` (new tools), a champion-registry artifact under `models/` (e.g. `models/champions/`).
- **Docs**: new `docs/MODEL_EVALUATION.md`, `docs/INDEX.md`, `CLAUDE.md` (reference + Working Rule). Cross-links to `docs/TRAINING_RUNBOOK.md` and `docs/TRAINING_MCP.md`.
- **Artifacts consumed**: `runs/<name>/evals.jsonl`, `models/<run>/eval_game_log.jsonl`, `models/<run>/checkpoints/` (saved every 100k).
- **No engine changes** and no breaking changes; the equilibrium-methods horizon depends on the separate `make-engine-cloneable` change.
