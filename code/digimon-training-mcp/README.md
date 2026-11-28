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
| `run_per_game_evals`| Filter per-game telemetry rows across evaluation windows                   |
| `run_anchored_evals`| Read seat-balanced anchored evaluation results for a run                   |
| `champion_standings`| Read current champion registry standings                                   |
| `run_elo_ladder`    | Read Elo ladder ratings for a run                                          |
| `run_exploitability`| Read exploitability audit metrics for a run                                |
| `card_embedding_neighbors` | *(Optional `[whitebox]`)* Inspect `nn.Embedding` L2 norms & top-k cosine neighbors in a `.zip` checkpoint |
| `position_saliency` | *(Optional `[whitebox]`)* Compute PyTorch `grad * obs` attribution mapped onto tensor layout sections & card slots |
| `plasticity_probe`  | *(Optional `[whitebox]`)* Run in-memory trial gradient steps on a puzzle to verify learnability vs. tensor blindness |
| `export_checkpoint_onnx` | *(Optional `[whitebox]`)* Export any `.zip` checkpoint into a content-addressed `.onnx` cache for `digimon-engine-mcp` |

All tools are strictly read-only over `runs/` and `models/` (white-box probes load `.zip` bytes in memory on CPU, lazy-import PyTorch so base installs only need `mcp` + `tensorboard`, and write `.onnx` exports only to a scratch cache directory).

## See also

- [docs/PUZZLE_LAB_AND_MCP.md](../../docs/PUZZLE_LAB_AND_MCP.md) — full ModelPilot + LLM Puzzle Lab & Dual-MCP operator guide
- [docs/DEBUG_MCP.md](../../docs/DEBUG_MCP.md) — the Rust engine MCP (`digimon-engine-mcp`)
- [openspec/changes/add-training-status-mcp/](../../openspec/changes/add-training-status-mcp/) — design / spec / tasks
