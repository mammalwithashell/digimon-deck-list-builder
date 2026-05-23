## Why

Running and triaging RL training runs is currently a manual ritual: `tail` the console log, eyeball the eval lines, open TensorBoard for the metric curves, `ls` the recordings directory, then pivot into the engine MCP one recording at a time. Each surface is a separate tool with no shared agent affordance, so investigations across runs (e.g. "which step did win-rate plateau on?", "what's the panic mix today?", "how many crash-verdict recordings are queued?") require ad-hoc shell work that an agent cannot reproduce.

The engine debug MCP (PR #519) solved this for per-game forensics by exposing `LiveGame` over JSON-RPC, but its domain is intentionally narrow — one Rust game at a time. Cross-game, time-series, and ML-shaped questions about training runs don't fit there. A parallel Python MCP — `digimon-training-mcp` — gives agents a read-only inspection surface over `runs/` and `models/` artifacts (console log, TensorBoard event files, recordings, checkpoints, deck pool snapshot) and cooperates with the engine MCP via a path-handoff bridge: training MCP returns a recording path; agent passes it to the engine MCP's existing `load_recording`. No duplication, clean domain split.

## What Changes

- Add a new Python MCP stdio server `digimon-training-mcp` at `code/digimon-training-mcp/`, runnable as `python -m digimon_training_mcp`.
- Scope: **read-only**. No start/stop/checkpoint controls in v1. No engine instantiation. No DB access.
- Filesystem-scoped to `./runs` and `./models` relative to working directory, with `--runs-dir` / `--models-dir` overrides, mirroring the engine MCP's `--cards-json` pattern.
- Expose seven tools, all read-only:
  - `list_runs()` → `[{name, started_at, last_modified, active, latest_step, latest_win_rate}]` with `active = console.log mtime within last 60s`.
  - `run_summary(name, tail_evals=10)` → header block, recent eval lines, panic totals grouped by family, recent console tail.
  - `run_metric(name, tag, since_step?)` → time-series of `[{step, wall_time, value}]` from the TensorBoard event file.
  - `run_tags(name)` → list of available TB tag strings.
  - `run_recordings(name, filter?: "crash"|"draw"|"all", limit?)` → recordings inventory with parsed metadata from filename + JSON header.
  - `run_checkpoints(name)` → checkpoint inventory with `{step, path, mtime, size_mb}`.
  - `run_deck_pool(name)` → contents of `deck_pool_snapshot.json` (archetypes, deck count, decks).
- Add a (commented-out, disabled) `.mcp.json` entry for `digimon-training-mcp` so the activation surface exists but is opt-in until v1 ships.
- Add a small structured-eval sidecar emitter to `pilot_training.py` (one append-only `runs/<name>/evals.jsonl` write per eval) so `run_summary` doesn't depend on regex-parsing console output. This is the only **outside-the-new-package** code change in the change set.
- Add `requirements-mcp.txt` for the new server's dependencies (official `mcp` Python SDK + `tensorboard` for the event-file reader); keep `requirements-training.txt` lean.

## Capabilities

### New Capabilities
- `training-status-mcp`: Read-only MCP stdio server exposing per-run inspection tools over `runs/` and `models/` filesystem artifacts. Tools cover run listing, summary (header + evals + panic mix), TensorBoard metrics, recordings inventory, checkpoint inventory, and deck pool. Path handoff to the engine MCP for individual recording inspection — no duplication of engine-side tools.

### Modified Capabilities

None. The eval sidecar (`evals.jsonl`) is an additive write from `pilot_training.py` — no existing capability behaviour changes.

## Impact

- **New code**: `code/digimon-training-mcp/` Python package (server entrypoint, tool dispatchers, console-log parser, TB event-file reader, recordings indexer, checkpoint indexer, deck-pool reader). Installable as a workspace member via `pyproject.toml`.
- **Touched code (one-line additive write)**: `code/digimon_gym/agents/pilot_training.py` — after each eval, append a structured row to `runs/<run>/evals.jsonl`. No change to existing console print or TB tag writes.
- **New dep file**: `requirements-mcp.txt` listing the official `mcp` Python SDK plus `tensorboard`. Not added to `requirements-training.txt` (training CLI stays lean) or `requirements.txt` (hosted API doesn't need the MCP).
- **`.mcp.json`**: new server entry, commented/disabled until v1 ships.
- **Docs**: new `docs/TRAINING_MCP.md` companion to the engine MCP's `docs/DEBUG_MCP.md`; one cross-reference paragraph added to `CLAUDE.md` under Commands. `docs/INDEX.md` index entry.
- **Service boundaries (CLAUDE.md §"Service Boundaries")**: the new package is a fourth deployable surface — read-only operator tool for local + dev use. Does not import `server.*`, `digimon_gym.db.*`, or any Rust binding crate. Pure stdlib + `mcp` + `tensorboard`.
- **No** changes to the Rust engine, the engine MCP, the hosted API, the training engine, the desktop app, or the frontend.
- **No** new top-level source directories (CLAUDE.md Working Rule #24 — extends `code/`).
