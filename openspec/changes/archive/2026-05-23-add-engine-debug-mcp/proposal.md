## Why

Agents implementing or debugging Rust card effects today have only one feedback channel: write a Rust test, run `cargo test`, read the assertion output. Mid-game state is invisible unless you author Rust code to dump it; pending selections (where most card bugs live) are a black box; alternative-line debugging ("what if I had chosen differently?") requires re-running from scratch.

Three workflows share this pain point:

1. **Card debugging during implementation** — turn-by-turn introspection while writing or fixing a `CardEffect`.
2. **Smoke-test forensics** — reconstruct a game from `(decks, seed)` to investigate flaky failures.
3. **Training-run forensics** — replay a `GameRecorder` recording to find the engine panic or anomalous play that broke a training run.

All three converge on one missing capability: **a live, inspectable game an agent (or human) can poke at, however it got there.** The Python engine has a `ReplayRunner`; the Rust engine has only the recording half, so even the existing forensic pattern can't run against the source-of-truth engine.

## What Changes

- **NEW** `LiveGame` wrapper in `digimon-engine` that exposes a stable, agent-friendly surface over `Game` (state views, decoded actions, action submission, recording load).
- **NEW** Rust `ReplayRunner` (port of `engine_py_legacy/engine/runners/replay_runner.py`) so the source-of-truth engine can replay any recording — closes a Python/Rust parity gap.
- **NEW** View serialization layer (`StateView`, `HandView`, `FieldView`, `PendingSelectionView`, `EffectQueueView`, `ModifierView`) — token-efficient JSON shapes that are *not* `to_ui_json` (which is lossy and frontend-shaped).
- **NEW** Action decoding helpers — translate action IDs into human-readable labels (`"play hand[2]: Agumon"`) so agents can read `legal_actions` without re-implementing the action decoder.
- **NEW** `digimon-engine-cli` binary — REPL for interactive humans, scenario runner (Rust port of `tools/run_scenario.py`), and replay viewer. Sibling consumer of the engine crate.
- **NEW** `digimon-engine-mcp` binary — stdio MCP server exposing lifecycle, inspection, action, and replay tools. State lives in server memory (`HashMap<GameId, LiveGame>`) so views are cheap.
- Card pool defaults to `load_implemented_card_ids()` (same filter `pilot_training`, `gauntlet`, and the architect agents use). An `--all-cards` flag exists for replays that reference unimplemented cards.
- **DEFERRED to v1.5** — snapshot/restore (branching) and the `Arc`-wrap engine refactor it requires. v1 ships without `Game::Clone`; agents work with the linear replay-and-seek model.

## Capabilities

### New Capabilities

- `live-game-surface`: A stable, view-oriented abstraction over `Game` that lets external callers (CLI, MCP, future tools) construct a game from any of {decks+seed, hand-specified setup, recording, recording-at-step}, submit actions, and inspect state through compact serializable views without depending on internal `Game` field shapes.
- `recording-replay`: Deterministic reconstruction of a `Game` from a `GameRecorder` recording (Rust). Step-forward, seek-to-step, and run-to-completion operations against the in-memory game.
- `engine-cli`: A `digimon-engine-cli` binary providing REPL, scenario execution, and replay viewing against the live-game surface. Replaces the Python-side `tools/run_scenario.py` for the Rust engine and adds interactive use.
- `engine-debug-mcp`: A stdio MCP server (`digimon-engine-mcp`) exposing live-game lifecycle, state inspection, action submission, and replay tools so AI agents can debug cards, investigate smoke-test failures, and forensically analyze training-run recordings.

### Modified Capabilities

None. All work is additive; no existing spec's requirements change.

## Impact

**Affected code**
- `code/digimon-engine/` — new modules: `live_game`, `view`, `replay_runner`, `action_decode`. Existing modules untouched except for exposing types via `lib.rs`.
- `code/digimon-engine/Cargo.toml` — adds two workspace binaries (`digimon-engine-cli`, `digimon-engine-mcp`) or, alternatively, new sibling crates in the workspace.
- `Cargo.toml` (workspace) — new members.
- `.mcp.json` — register the new MCP server.

**No impact on**
- `digimon-engine-py` (PyO3 bindings) — untouched.
- `digimon_gym` (RL training) — untouched.
- `code/server/` (hosted API) — untouched.
- `code/src-tauri/` — untouched.
- `code/engine_py_legacy/` — untouched (the existing Python `ReplayRunner` remains as parity reference until Python sunset completes).

**Dependencies**
- New: an MCP server crate (e.g., `rmcp` or the Anthropic-published Rust MCP SDK if available). Selection deferred to design phase.
- New: `clap` for the CLI (already a transitive dep via other tools).

**Documentation**
- `docs/RUST_ENGINE_API.md` — add a "Debugging" section pointing at the CLI and MCP.
- `docs/RUST_PYTHON_PARITY.md` — mark `ReplayRunner` parity gap as resolved.
- New `docs/DEBUG_MCP.md` (or similar) — tool surface reference and recipe cookbook.

**Out of scope (v1.5+)**
- `Game::Clone` and `Arc`-wrap refactor of `card_data` / registries.
- `snapshot`/`restore`/`list_snapshots` MCP tools.
- Training-worker integration that flushes recordings on engine panic (separate proposal).
- Skill wrappers (`/debug-card`, `/investigate-crash`) that use the MCP — separate skill-authoring work, not blocked on this proposal.
