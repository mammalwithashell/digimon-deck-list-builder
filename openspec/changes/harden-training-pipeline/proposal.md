# Harden Training Pipeline

## Why

Despite a mature training stack and a fully implemented evaluation harness, the project has produced no competitive models: the one self-play run collapsed catastrophically (22.5% vs greedy at 500k steps, down from the v22 baseline's 77.5%) while its in-run eval reported a flat 100% — evidence the opponent seat goes passive under `opponent="self-play"` — and the champion-promotion loop (train → anchored eval → promote → grow opponent pool) has never completed a full cycle. The path beyond greedy-level play requires a trustworthy opponent curriculum and trustworthy in-run signal; both are currently broken or unwired.

## What Changes

- **Diagnose and fix or retire `opponent="self-play"`**: verify whether the opponent seat actually acts during self-play training; either fix the wiring or remove the mode with a hard error pointing at the pool-based alternative. Add a runtime guard so a degenerate eval configuration (in-run eval against the training mirror) can never again report a meaningless 100%.
- **Wire an in-training anchored eval suite** (the deferred task 2.3 of `add-model-evaluation-harness`): a small seat-balanced panel (greedy + layout-compatible champions, ~20–40 games) runs inside `WinRateCallback` every N steps, logging trustworthy `pilot/anchored/*` scalars and sidecar rows so a collapsing run is visible within ~1–2 eval windows instead of after the run ends.
- **Codify the champion-promotion loop**: register the validated `starter1_6_flat_control_v1` model as a champion; add an `OpponentPool` manifest generator that derives the training opponent pool from the champion registry, so each new run trains against all promoted champions; document the standing cadence (run → anchored eval + Elo ladder → gated promote → next run trains vs grown pool) in the training runbook.
- **Run-provenance hygiene**: capture git SHA in `TrainingRunMetadata`; persist bounty parameters to the sidecar; validate action-space *structure* (sub-range boundaries), not just the size integer, in the checkpoint contract; upstream the `init_from` forwarding gap in `tools/run_training_job.py`.
- **BREAKING (fail-fast)**: a configured-but-missing reward-profile YAML path now raises at training start instead of silently falling back to legacy rewards.

## Capabilities

### New Capabilities
- `self-play-opponent-integrity`: the self-play training mode (or its replacement guidance) guarantees an active opponent seat; degenerate mirror-eval configurations are guarded at runtime.
- `in-training-anchored-eval`: an anchored reference panel (greedy + compatible frozen champions) evaluated periodically inside the training loop, with TensorBoard scalars and `evals.jsonl` sidecar rows.
- `champion-pool-curriculum`: the training opponent pool is derivable from the champion registry, closing the train → promote → grow-pool loop; promotion cadence is documented and exercised.
- `training-run-provenance`: run metadata captures git SHA and all reward/bounty/curriculum knobs; checkpoint contract validation covers action-space structure.

### Modified Capabilities
- `reward-profiles`: loading behavior changes from silent legacy fallback to fail-fast when a configured reward YAML path does not exist.

## Impact

- **Code**: `code/digimon_gym/agents/pilot_training.py` (OpponentWrapper/self-play wiring, WinRateCallback, run metadata), `code/digimon_gym/agents/anchored_eval.py` (reuse from in-training context), `code/digimon_gym/agents/opponent_pool.py` (registry-derived manifest), `code/digimon_gym/agents/training_metrics.py` (sidecar fields), `code/digimon_gym/agents/reward/` loader path validation, `code/tools/run_training_job.py` (`init_from` forwarding), `code/tools/champion_admin.py` (registration of the control model).
- **Docs**: `docs/TRAINING_RUNBOOK.md` (cadence section, self-play guidance), `docs/MODEL_EVALUATION.md` (in-training anchored suite), CLAUDE.md rule 30 cross-reference.
- **Artifacts**: `models/champions/registry.json` gains the starter-flat control champion; `evals.jsonl` schema gains anchored-panel rows (lenient readers unaffected).
- **Compat**: existing checkpoints and run layouts unchanged; the only breaking behavior is the reward-YAML fail-fast, which converts a silent misconfiguration into an explicit error.
