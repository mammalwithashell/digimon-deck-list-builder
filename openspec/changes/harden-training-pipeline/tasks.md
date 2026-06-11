## 1. Retire self-play + fail-fast reward YAML (D1, D4)

- [x] 1.1 In `pilot_training.make_env`, replace the `opponent == "self-play"` branch with a `ValueError` explaining the P1-perspective limitation and the `opponent="pool"` replacement recipe; keep `--self-play` parseable but failing with the same message; verify the job-runner path (`tools/run_training_job.py`) surfaces it at startup
- [x] 1.2 Add tests: `make_env(opponent="self-play")` raises with the migration message; every accepted opponent value constructs a chain containing an `OpponentWrapper` with non-None `opponent_fn`; a full episode driven only by agent-side actions reaches terminal with both players acting (regression guard)
- [x] 1.3 In the reward-profile factory setup (`pilot_training._make_reward_profile_factory` and `TrainingConfig` validation), raise `FileNotFoundError` naming the path when a non-null `reward_profiles_path`/`reward_gameplay_path` does not exist; support `reward_profiles_path: null` as the explicit legacy opt-out; add tests for typo'd path, missing default, and null opt-out
- [x] 1.4 Update `docs/TRAINING_RUNBOOK.md` §1/§12 and `docs/MODEL_EVALUATION.md` to remove `--self-play` examples and document the retirement + replacement; note the reward-YAML breaking behavior

## 2. Provenance hardening (D5)

- [x] 2.1 Add `git_sha` (best-effort `git rev-parse HEAD`, `"unknown"` fallback), `bounty_threshold`, `bounty_bonus` to `TrainingRunMetadata` in `training_metrics.py`; populate from `train()`; test both the git and non-git paths
- [x] 2.2 Write `action_space_structure` `(SOURCE_SELECT_END, BREEDING_SOURCE_SELECT_START, BREEDING_SOURCE_SELECT_END)` into checkpoint `.meta.json`; extend `resume_from`/`init_from` contract validation to compare it (error on mismatch, warn-only when absent); tests for mismatch-rejected and legacy-warns
- [x] 2.3 Upstream the `init_from` job-runner patch: add `init_from: Optional[str] = None` parameter to `train()`, wire it into the config-override block, add `"init_from"` to `_TRAIN_KWARGS` in `tools/run_training_job.py`; test a job config with `init_from` and the `init_from`+`resume_from` mutual-exclusion error

## 3. In-training anchored eval (D2)

- [x] 3.1 Add `anchored_eval_freq` (default 100000) and `anchored_eval_games` (default 24) to `TrainingConfig` + CLI flags
- [x] 3.2 Implement `AnchoredEvalCallback` (new module or in `pilot_training.py`): every `anchored_eval_freq` steps, build the panel (greedy + layout-compatible champions from `models/champions/registry.json`, logging excluded champions once at run start), run seat-balanced games via `anchored_eval.evaluate_against_anchors` with the run's deck-pool snapshot (fallback ladder: snapshot → run decks → 6 starters), wrapping each game so any exception marks the anchor failed and never aborts training
- [x] 3.3 Emit `pilot/anchored/greedy/win_rate`, `pilot/anchored/<champion>/win_rate`, `pilot/anchored/panel_mean` scalars and append one row per panel to `<run_dir>/anchored_evals.jsonl` (step, wall_time, per-anchor W/L/D + win rate, failed anchors, panel wall-clock seconds); assert `evals.jsonl` is untouched
- [x] 3.4 Tests: panel cadence (runs exactly N times for given freq/timesteps), seat balance (equal first-player split), `anchored_eval_freq=0` writes nothing, panel-crash containment (injected exception → run continues, anchor marked failed), incompatible champion excluded
- [x] 3.5 Training MCP: add `run_anchored_evals(name, limit?)` reader for `anchored_evals.jsonl` and surface the latest panel row in `run_summary`; tests in the MCP suite; document in `docs/TRAINING_MCP.md`
- [x] 3.6 Smoke-verify end-to-end on a short local run (release-build bindings, `--timesteps 50000 --anchored-eval-freq 25000`) and confirm scalars in TensorBoard + sidecar rows + MCP reader output

## 4. Champion-pool curriculum (D3)

- [x] 4.1 Implement `OpponentPool.from_champion_registry(registry_path, layout_hash)` (uniform weights; explicit error when no compatible champion) and `champion_admin.py emit-pool --out <manifest.json>`; tests for the two-compatible/one-incompatible case and the empty-cohort error
- [x] 4.2 Record the derived manifest path in `TrainingRunMetadata` when `opponent="pool"` uses a registry-derived manifest
- [x] 4.3 Run the recorded promotion decision for `starter1_6_flat_control_v1` (from `cloud_downloads/starter1_6_flat_control_v1/`): gate panel vs compatible champions via `champion_admin promote`; register on pass, or `--force` with anchored-vs-greedy evidence in `source`, or document the failing verdict — outcome visible in `models/champions/registry.json` or the runbook
- [x] 4.4 Write the standing-cadence section in `docs/TRAINING_RUNBOOK.md`: train vs registry-derived pool → `anchored_eval_cli` + `elo_ladder_cli` → gated promotion → emit-pool for the next run; state that mirror/in-run metrics never drive promotion (rule 30 cross-reference)

## 5. Validation

- [x] 5.1 Full RL test suite green: `python -m pytest code/tests/rl -v` plus the training-MCP suite
- [x] 5.2 Verify the breaking behaviors end-to-end: a config with `opponent="self-play"` and a config with a typo'd reward path both fail at startup with actionable messages
- [x] 5.3 Update `CLAUDE.md` rule 30 (or add a sibling note) referencing the in-training anchored suite and self-play retirement; sync `AGENTS.md` §3 opponent-mode list
