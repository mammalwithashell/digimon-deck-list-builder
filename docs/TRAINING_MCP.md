# Training Status MCP

Read-only MCP stdio server for inspecting RL training runs. Companion to the engine debug MCP — the latter owns per-game forensics (`LiveGame` over a single recording or live game); this one owns cross-game training inspection (filesystem artifacts under `runs/` and `models/`).

Shipped by the [`add-training-status-mcp`](../openspec/changes/add-training-status-mcp/) change. Python package at [`code/digimon-training-mcp/`](../code/digimon-training-mcp/).

```
┌──────────────────────────────────────────────────────────────┐
│  pilot_training run                                          │
│  ├── runs/<name>/console.log                                 │
│  ├── runs/<name>/evals.jsonl       (structured eval sidecar) │
│  ├── runs/<name>/MaskablePPO_1/    (TB scalar events)        │
│  └── models/<name>/<run_id>/                                 │
│      ├── recordings/  ├── checkpoints/  ├── deck_pool.json   │
└────────────┬──────────────────────────────┬──────────────────┘
             │                              │
             ▼                              ▼
   ┌───────────────────┐         ┌──────────────────────────┐
   │ digimon-training- │  path   │ digimon-engine-mcp       │
   │ mcp (this server) │ ──────▶ │ (load_recording, etc.)   │
   └───────────────────┘         └──────────────────────────┘
   cross-game / time-series        per-game forensics
```

## Install

```bash
pip install -r requirements-mcp.txt
pip install -e code/digimon-training-mcp
```

The MCP dependencies (`mcp`, `tensorboard`) are intentionally kept separate from `requirements-training.txt` so the training CLI stays lean.

> **Dependency conflict note.** The `mcp` SDK pulls in `starlette >= 0.42`, which conflicts with the hosted API's FastAPI 0.115 pin (`starlette < 0.42`). Installing both into the same environment will break FastAPI's `Router.__init__`. If you also run the hosted API locally, install the training MCP into a **separate virtualenv**:
> ```bash
> python -m venv .venv-mcp
> .venv-mcp\Scripts\activate     # PowerShell / cmd  (use 'source .venv-mcp/bin/activate' on Unix)
> pip install -r requirements-mcp.txt
> pip install -e code/digimon-training-mcp
> ```
> The MCP runs out-of-process over stdio, so the client doesn't care which venv it lives in. The `.mcp.json` `command` can point at the venv's `python` if needed.

## Run

```bash
python -m digimon_training_mcp --runs-dir ./runs --models-dir ./models
```

Both flags are optional. If omitted, the server walks up to six ancestor directories from the current working directory looking for `./runs` and `./models` (mirrors the engine MCP's `data/cards.json` discovery).

The server writes a single banner line to stderr on startup:

```
digimon-training-mcp: ready (runs_dir=/path/to/runs, models_dir=/path/to/models, repo_root=/path/to/repo)
```

## Registering with a client

`.mcp.json` is strict JSON in this repo (no comments allowed), so the activation entry is **not** pre-added. After installing, append:

```json
{
  "mcpServers": {
    "digimon-engine-mcp": { /* existing — leave as-is */ },
    "digimon-training-mcp": {
      "type": "stdio",
      "command": "python",
      "args": ["-m", "digimon_training_mcp", "--runs-dir", "./runs", "--models-dir", "./models"]
    }
  }
}
```

## Tools

All read-only. No start/stop/checkpoint controls in v1.

### `list_runs()`

List every subdirectory of `--runs-dir` as one logical run.

```json
{
  "ok": true,
  "runs_dir": "/abs/path/runs",
  "runs": [
    {
      "name": "generalist_v2",
      "started_at": "2026-05-23T10:03:55+00:00",
      "last_modified": "2026-05-23T16:48:12+00:00",
      "last_modified_epoch": 1716482892.0,
      "active": true,
      "latest_step": 387000,
      "latest_win_rate": 0.71
    },
    ...
  ]
}
```

`active` is `true` when `console.log` *or* the most recent `events.out.tfevents.*` was written within the last 60 seconds. `latest_step` and `latest_win_rate` come from the last row of `evals.jsonl`; both are `null` for runs that pre-date the sidecar emission landing.

### `run_summary(name, tail_evals=10)`

Compose the four headline pieces.

```json
{
  "ok": true,
  "header": {
    "algorithm": "MaskablePPO",
    "opponent": "greedy",
    "total_steps": "1,000,000",
    "tensorboard": "runs/generalist_v2",
    "tensor_profile": "standard_lite_v2",
    "layout_hash": "abc123def456",
    "gauntlet": "4 archetypes, 8 decks",
    ...
  },
  "evals": [ /* last 10 rows from evals.jsonl */ ],
  "evals_source": "sidecar",  // or "console" if evals.jsonl absent
  "panics": {
    "total": 7,
    "by_family": {
      "G-DSL-OUTER-TAIL-NESTED-PARK": 4,
      "G-OPTION-PLAY-REENTRANT": 2,
      "other": 1
    }
  },
  "recent_console_tail": [ /* last 50 lines of console.log */ ]
}
```

Header keys are normalized: `"Algorithm"` → `"algorithm"`, `"Total steps"` → `"total_steps"`, `"Layout hash"` → `"layout_hash"`. Panic families are sourced from [`qa/archetype-qa/panic-families.json`](../qa/archetype-qa/panic-families.json); unmatched panics roll up under `"other"`. If the eval sidecar is absent (older runs), `evals` falls back to regex-parsing `[Eval @ N steps] ...` lines from the console — the row shape is a subset of the sidecar shape (no `draw_rate`, no `by_archetype`).

### `run_metric(name, tag, since_step?)`

TensorBoard scalar time-series. `tag` may be a string or an array.

```bash
# Single-tag — returns a list
run_metric("generalist_v2", "pilot/win_rate")
→ { "ok": true, "tag": "pilot/win_rate", "values": [
      {"step": 10000, "wall_time": 1716000000.0, "value": 0.45}, ...
    ]}

# Multi-tag — returns a dict
run_metric("generalist_v2", ["pilot/win_rate", "train/loss"])
→ { "ok": true, "series": {
      "pilot/win_rate": [...],
      "train/loss":     [...]
    }}

# since_step filters server-side
run_metric("generalist_v2", "pilot/win_rate", since_step=100_000)
```

Active runs work too — the server caches one `EventAccumulator` per run name and calls `Reload()` on every tool invocation to pick up new events.

### `run_tags(name)`

Discover scalar tags for a run.

```json
{
  "ok": true,
  "tags": [
    "pilot/win_rate", "pilot/draw_rate", "pilot/games_played",
    "pilot/mean_eval_reward", "pilot/mean_eval_terminal_score",
    "pilot/mean_eval_dense_reward", "pilot/mean_eval_episode_length",
    "rollout/ep_rew_mean", "rollout/ep_len_mean",
    "train/loss", "train/value_loss", "train/policy_gradient_loss",
    "time/fps"
  ]
}
```

### `run_recordings(name, filter?="all", limit?)`

Inventory the recordings produced by the crash-resilient training wrapper.

- `filter: "crash"` → recordings with `reason == "crash"` (engine panics caught by the wrapper).
- `filter: "draw"` → `result == "draw"` AND `reason != "crash"` (drawn games that weren't crashes — typically `step_limit`).
- `filter: "all"` → no filter.
- `limit` truncates post-filter, most-recently-modified first.

```json
{
  "ok": true,
  "model_run_id": "pilot_ppo_20260523_100355",
  "filter": "crash",
  "count": 12,
  "recordings": [
    {
      "path": "/abs/path/models/generalist_v2/pilot_ppo_20260523_100355/recordings/train_env_003_game_000128_draw_crash.json",
      "source": "train",
      "env": 3,
      "game": 128,
      "result": "draw",
      "reason": "crash",
      "mtime": 1716482892.0,
      "mtime_iso": "2026-05-23T16:48:12+00:00",
      "size_bytes": 84321
    },
    ...
  ]
}
```

Pass any `path` to `digimon-engine-mcp`'s `load_recording` to drill into the per-step state. The `model_run_id` reflects which timestamped subdirectory under `models/<name>/` was resolved (the most recently modified one when multiple exist).

### `run_checkpoints(name)`

Model checkpoint inventory, sorted by step ascending.

```json
{
  "ok": true,
  "model_run_id": "pilot_ppo_20260523_100355",
  "count": 5,
  "checkpoints": [
    {"step": 100000, "path": "...", "mtime_iso": "...", "size_mb": 25.4},
    {"step": 200000, "path": "...", "mtime_iso": "...", "size_mb": 25.4},
    ...
  ]
}
```

### `run_deck_pool(name)`

Read `deck_pool_snapshot.json` verbatim, plus derived `deck_count`.

```json
{
  "ok": true,
  "model_run_id": "pilot_ppo_20260523_100355",
  "archetypes": ["Medusamon", "BlackWargreymon", "Tyrant", "Imperialdramon"],
  "deck_count": 8,
  "decks": [{"name": "deck_0", "cards": [...]}, ...]
}
```

Returns `{ok: false}` for non-gauntlet runs (no snapshot file).

## Path resolution

`<runs-dir>` and `<models-dir>` are independent — the operator typically names them similarly (`runs/generalist_v2` paired with `models/generalist_v2`) but the MCP doesn't enforce that. For model-side tools (`run_recordings` / `run_checkpoints` / `run_deck_pool`), `<models-dir>/<name>/` may either contain `recordings/` / `checkpoints/` / `deck_pool_snapshot.json` directly (flat layout) or wrap a timestamped subdirectory (e.g. `pilot_ppo_<timestamp>/`) that holds them (nested layout — the default). The MCP picks the most-recently-modified marker-bearing subdirectory when multiple exist and surfaces the choice in the `model_run_id` field of every response.

## Recipes

**"Which run is the most recent, and how is it doing?"**
```
list_runs() → take runs[0] (sorted desc by last_modified)
run_summary(name=runs[0].name, tail_evals=3)
```

**"What's the panic mix in the active run?"**
```
run_summary(name) → response.panics.by_family
```

**"Plot win-rate over time."**
```
run_metric(name, "pilot/win_rate") → [{step, wall_time, value}, ...]
```

**"Find a crash to investigate."**
```
run_recordings(name, filter="crash", limit=1)
→ pass returned `path` to digimon-engine-mcp:load_recording
→ step through with engine MCP's `step` / `state` / `pending_selection`
```

**"Which checkpoint should I export for ONNX?"**
```
run_checkpoints(name) → pick the latest step with confirmed good metrics from run_metric()
```

## Caveats and limits

- **Active-run detection is mtime-based.** A 60-second idle period during a long rollout can briefly mark a live run as `active=false`. The TB event-file mtime is unioned with the `console.log` mtime to keep this rare. v2 may add a lockfile signal.
- **`run_summary` evals fall back to regex.** Runs started before the sidecar emission landed have no `evals.jsonl`; the fallback `[Eval @ N steps]` parser populates a subset of the row shape (no `draw_rate`, no `mean_terminal_score`, no `by_archetype`).
- **No DB access.** Hosted-API training-worker state lives in Postgres; this MCP doesn't speak to it. It's for local + dev-host inspection only.
- **No mutations.** v1 has no `delete_recording`, `promote_checkpoint`, `start_run`, `stop_run`, or anything else that writes to disk.

## See also

- [DEBUG_MCP.md](DEBUG_MCP.md) — engine MCP companion (per-game forensics).
- [TRAINING_RUNBOOK.md](TRAINING_RUNBOOK.md) — training-run lifecycle context.
- [qa/archetype-qa/panic-families.json](../qa/archetype-qa/panic-families.json) — machine-readable index of engine panic families.
- [qa/archetype-qa/engine-gaps.md](../qa/archetype-qa/engine-gaps.md) — prose source-of-truth for those families.
- [openspec/changes/add-training-status-mcp/](../openspec/changes/add-training-status-mcp/) — design / spec / tasks for this change.
