## Why

PR #538 added a digivolve reward shaping signal and two top-level TensorBoard scalars (`pilot/mean_eval_digivolves_per_game` / `..._dna_...`), but those scalars only aggregate the agent side, are scoped to a single eval window, and are not persisted to `evals.jsonl` at all. The `by_archetype` sidecar block exposes per-archetype `wins/draws/games/win_rate` but no digivolve activity. As a result, basic questions about training history — "did the agent ever DNA-digivolve during the 1M-step run?" or "is the digivolve reward dead code for DNA Omnimon specifically?" — can only be answered by loading the final checkpoint and replaying fresh eval games, which probe the *current* policy and not training history.

A 120-game ad-hoc eval at step-350k of the v2 run found 0 DNA digivolves and 0.95 regular digivolves/game for DNA Omnimon (lowest of any archetype), with shaping contributing only ~1–3% of terminal reward magnitude. We need persistent per-archetype digivolve telemetry to confirm whether this was *always* the case, whether it varies across archetypes, and whether the agent unlearned digivolving over training.

## What Changes

- Extend `WinRateCallback` per-archetype tally state (`_archetype_*` and `_agent_archetype_*` dicts) with cumulative `digivolves` and `dna_digivolves` counters, keyed identically to the existing wins/games/draws dicts.
- Read all four engine-exposed counters (`p1_digivolutions`, `p1_dna_digivolutions`, `p2_digivolutions`, `p2_dna_digivolutions`) at each terminal step and credit them to the correct archetype bucket.
- Emit four new families of TensorBoard scalars: `pilot/agent_archetype/<X>/digivolves_per_game`, `pilot/agent_archetype/<X>/dna_digivolves_per_game`, `pilot/archetype/<X>/opponent_digivolves_per_game`, `pilot/archetype/<X>/opponent_dna_digivolves_per_game` (where the `archetype/` axis is the opponent's archetype, matching the existing `pilot/archetype/<X>/win_rate` convention).
- Extend the `evals.jsonl` `by_archetype` value shape from `{wins, draws, games, win_rate}` to `{wins, draws, games, win_rate, digivolves, dna_digivolves, opponent_digivolves, opponent_dna_digivolves}` — cumulative across the run, parallel to wins.
- Add top-level sidecar fields `mean_eval_digivolves_per_game`, `mean_eval_dna_digivolves_per_game`, `mean_eval_opponent_digivolves_per_game`, `mean_eval_opponent_dna_digivolves_per_game` to mirror the per-eval TB scalars on disk.
- Telemetry fires unconditionally — counters are observational, not gated on `digivolve_shaping`. When shaping is off, fields are emitted with their true (often zero) values so the sidecar schema is stable across runs.
- No engine, PyO3, env, reward, or CLI changes — all four counters already round-trip through `get_rl_state()`.

## Capabilities

### New Capabilities

- `per-archetype-digivolve-telemetry`: Persistent per-archetype digivolve activity telemetry on the pilot training eval surface — extends the existing per-archetype win-rate channel (TB scalars + `evals.jsonl` sidecar) with cumulative digivolve and DNA-digivolve counts on both the agent and opponent axes.

### Modified Capabilities

- None. The existing `pilot/mean_eval_digivolves_per_game` / `..._dna_...` top-level TB scalars (added in PR #538) remain unchanged in semantics; this change adds parallel per-archetype scalars and adds persistence to the sidecar.

## Impact

- Affected Python modules: `code/digimon_gym/agents/pilot_training.py` (`WinRateCallback` state + `_on_step` tally + sidecar emission + TB scalar emission). No changes to `training_metrics.py`, `training_config.py`, `digimon_gym.py`, or any engine/binding code.
- Affected artifacts: `<run_root>/evals.jsonl` — per-row `by_archetype` value shape and top-level mean fields gain four new keys each. Older sidecar readers that ignore unknown keys continue to round-trip; readers that whitelist fields will need to widen their allowlist.
- Affected docs: `docs/TRAINING_RUNBOOK.md` (sidecar schema section, if any) and `docs/TRAINING_MCP.md` (the training MCP's `run_summary` tool surfaces sidecar rows verbatim, so the new fields will appear automatically — worth a one-line note).
- New tests: one Python unit test (`code/tests/rl/test_training_metrics_digivolve_sidecar.py`) covering archetype tally accumulation, sidecar shape, zero-default when shaping is off, and stable round-trip with older readers.
