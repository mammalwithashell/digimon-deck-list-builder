# digimon-training-mcp

Read-only MCP stdio server for inspecting Digimon RL training runs. Parallel to `digimon-engine-mcp` — the latter owns per-game engine forensics (Rust, `LiveGame`); this one owns cross-game training inspection (Python, filesystem artifacts under `runs/` and `models/`).

## Install

```bash
pip install -e code/digimon-training-mcp
```

Or via the bundled requirements file:

```bash
pip install -r requirements-mcp.txt
```

## Run

```bash
python -m digimon_training_mcp --runs-dir ./runs --models-dir ./models
```

`--runs-dir` / `--models-dir` are optional. If omitted, the server walks up from the working directory looking for `./runs` and `./models` (same ancestor-walk pattern as `digimon-engine-mcp`'s `data/cards.json` discovery).

## Activating in `.mcp.json`

Once v1 ships and dependencies are installed, add the following to `.mcp.json` alongside the existing `digimon-engine-mcp` entry:

```json
"digimon-training-mcp": {
  "type": "stdio",
  "command": "python",
  "args": ["-m", "digimon_training_mcp", "--runs-dir", "./runs", "--models-dir", "./models"]
}
```

`.mcp.json` is strict JSON in this repo (no comments), so the entry is intentionally NOT pre-added — the operator chooses when to enable it.

## Tools (v1)

| Tool                | Purpose                                                                    |
|---------------------|----------------------------------------------------------------------------|
| `list_runs`         | List all runs under `--runs-dir` with active-status + latest step/win-rate |
| `run_summary`       | Header block, recent eval rows, panic counts by family, console tail       |
| `run_metric`        | TensorBoard time-series for one or more scalar tags                        |
| `run_tags`          | Discover scalar tags available for a run                                   |
| `run_recordings`    | Inventory recordings (with crash / draw / all filter)                      |
| `run_checkpoints`   | Inventory `step_*.zip` model checkpoints                                   |
| `run_deck_pool`     | Read the `deck_pool_snapshot.json` for a generalist run                    |

All read-only. No start/stop/checkpoint controls. To inspect a specific recording's per-step engine state, pass `run_recordings`' returned `path` to `digimon-engine-mcp`'s `load_recording`.

## See also

- [docs/TRAINING_MCP.md](../../docs/TRAINING_MCP.md) — user-facing reference (forthcoming)
- [docs/DEBUG_MCP.md](../../docs/DEBUG_MCP.md) — the engine MCP
- [openspec/changes/add-training-status-mcp/](../../openspec/changes/add-training-status-mcp/) — design / spec / tasks
