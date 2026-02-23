# Admin AI Batch Runbook

## Modes

- `pr`: Applies approved AI fixes into an isolated worktree branch and opens a draft PR with `gh`.
- `main`: Applies and pushes directly to `origin/main`. Disabled by default unless `AI_APPLY_MAIN_ALLOWED=1`.

## Batch Source

- Only issues with `status=approved_for_ai` are eligible.
- The selected `set_id` filters cards by `CARD_ID` prefix (`BT24-*`, `EX11-*`, etc.).

## UI Workflow

1. Open `Admin Issues`.
2. In `Run Set Batch`, set:
   - `set_id`
   - `run_mode`
   - `scope_profile`
   - `model`, `concurrency`, `max_total_cost_usd`, `failure_rate_stop`, `max_tasks`, `dry_run`
3. Click `Preview Eligible Cards`.
4. Run with `dry_run=false` to start.
5. Track progress in `Batch Status`.

## Scope Profiles

- `script`: Only the target generated card script path.
- `script_engine`: Script path + `digimon_gym/engine/**/*.py`.
- `script_engine_transpiler`: Script + engine + `tools/transpiler/**/*.py`.

Global deny list blocks `.github`, migrations, lockfiles, secrets, and out-of-repo paths.

## Required Host Tooling

- `git`
- `gh` (for `pr` mode)
- Authenticated GitHub CLI session (`gh auth status`)

## Environment Flags

- `AI_APPLY_MAIN_ALLOWED=0` by default.
- Set `AI_APPLY_MAIN_ALLOWED=1` to enable `run_mode=main`.
- `AI_BATCH_FAILURE_MIN_SAMPLE` controls minimum samples before failure-rate stop.

## Guard Stops

- Stops when cumulative `cost_actual_usd` exceeds `max_total_cost_usd`.
- Stops when failure rate exceeds `failure_rate_stop` after minimum sample size.

## Traceability

- `ai_tasks` store `batch_id`, `run_mode`, `scope_profile`.
- `ai_fix_batch_items` track per-card status, task IDs, and commit SHAs.
- `ai_fix_apply_audits` store applied files and check outputs.
- Promotions include `ai_task_id` and derived `batch_id`.
