# Admin AI Batch Runbook

Operational reference for `/admin/*` AI task and batch workflows.

## Modes

- `pr`
  - Applies approved fixes in isolated worktree/branch flow.
  - Uses GitHub CLI for draft PR creation when configured.
- `main`
  - Applies and pushes to `origin/main`.
  - Guarded by `AI_APPLY_MAIN_ALLOWED=1`.

## Eligibility and Source

- Batch input is issues with `status=approved_for_ai`.
- `set_id` filtering uses card ID prefix (example `BT24-*`, `EX11-*`).

## Scope Profiles

- `script`: target generated script only.
- `script_engine`: script + engine python files.
- `script_engine_transpiler`: script + engine + transpiler python files.

A deny list blocks out-of-scope and sensitive paths.

## Environment and Guards

Important controls:

- `AI_APPLY_MAIN_ALLOWED` (default disabled)
- `AI_BATCH_FAILURE_MIN_SAMPLE`
- worker enable/disable via `AI_WORKER_DISABLED`

Batch stop conditions include:

- Actual cost exceeds `max_total_cost_usd`
- Failure rate exceeds `failure_rate_stop` after minimum sample threshold

Task-level hard cap is also enforced by worker cost policy.

## Lifecycle

1. Triage issues to `approved_for_ai`.
2. Preview eligible cards for a set.
3. Create batch (`dry_run` optional).
4. Worker executes queued tasks.
5. Batch orchestrator updates counters and applies guard stops.
6. Successful tasks can be applied/committed per configured mode.
7. Promotions and backlog actions can be recorded from task outputs.

## Endpoint Quick Reference

Admin routes are mounted with prefix `/admin`.

### Issues -> Queue

- `POST /admin/issues/{issue_id}/queue-fix`

### AI Batches

- `POST /admin/ai-batches`
- `GET /admin/ai-batches/preview`
- `GET /admin/ai-batches`
- `GET /admin/ai-batches/{batch_id}`
- `POST /admin/ai-batches/{batch_id}/cancel`

### AI Tasks

- `POST /admin/ai-tasks`
- `GET /admin/ai-tasks`
- `GET /admin/ai-tasks/{task_id}`
- `POST /admin/ai-tasks/{task_id}/retry`
- `POST /admin/ai-tasks/{task_id}/apply-fix`
- `POST /admin/ai-tasks/{task_id}/promote`

### Promotions and Backlog

- `POST /admin/promotions`
- `GET /admin/promotions`
- `POST /admin/engine-backlog`
- `GET /admin/engine-backlog`

## Audit and Traceability

Database tables track full lineage:

- `ai_tasks`: task payload, status, costs, attempts, batch linkage
- `ai_fix_batches`: batch config/status/counters
- `ai_fix_batch_items`: per-card item state/task/commit linkage
- `ai_fix_apply_audits`: applied files and check outputs
- `script_promotion_audits`: promotion records with optional task linkage
- `engine_backlog_items`: deferred engineering follow-ups

## Operational Notes

- Validate `run_mode` and `scope_profile` before running large batches.
- Prefer preview before creating real batches.
- Use smaller batches when tuning guard thresholds.
- Keep run metadata (`model`, `concurrency`, limits) explicit for reproducibility.
