# Scenario Capture MCP (`digimon-scenario-mcp`)

A **WRITE-capable, dev/test-only** MCP that drives the scenario stage→test
loop, so a card or interaction can be tested in the UI without playing a
game to draw the right cards. It is the agent front-end for the
`add-ui-scenario-test-substrate` `/debug` staging surface plus the
capture-from-live primitive added by `add-scenario-capture-mcp`.

> **Why it's a documented exception.** The other operator MCPs
> (`digimon-engine-mcp`, `digimon-training-mcp`) are read-only by policy
> (CLAUDE.md "Read-only operator MCPs"). This one mutates game state and
> writes files (`qa/scenarios/`, `code/frontend/e2e/`) — that is the point.
> It is bounded: dev/test-only, never bundled into any production build,
> never imported by `server.*` / `digimon_gym.*`, and it talks only to
> dev-gated surfaces.

## Architecture

```
                       ┌─ Game::to_scenario()  (capture)
 engine (one impl) ────┤  Game::apply_scenario() (stage)   code/digimon-engine/src/game/snapshot.rs
                       └─ Game::stage_* (existing)          code/digimon-engine/src/game/staging.rs
        │                                   │
   PyO3 bindings                       Tauri shell
   RustDebugGame / RustHeadlessGame    debug_bridge.rs (feature `debug-bridge`)
        │                                   │
   FastAPI: /debug/* + /games/{id}/    localhost axum server (127.0.0.1, env-gated)
   export-scenario                          │
        │                                   │
        └──────────── digimon-scenario-mcp ─┘
                 target: browser | desktop
```

## Transports

| `target`  | Surface | Needs |
|-----------|---------|-------|
| `browser` | hosted API `/debug` + `/games` (multi-game, `game_id`) | `python -m uvicorn server.api:app` (from repo root) |
| `desktop` | Tauri debug bridge (single live game, no id) | desktop app built `--features debug-bridge`, run with `DIGIMON_DEBUG_BRIDGE=1` |

The `desktop` target is the **only** way to exercise the desktop-only
`engine_commands.rs` DTO wire end-to-end (browser-mode renders the same React
UI but a *different* serialization wire — the one whose drift caused the
desktop-only selection/sources/trash/winner bugs).

### Desktop bridge

Compiled only under the `debug-bridge` cargo feature (absent from
release/prod), and starts only when `DIGIMON_DEBUG_BRIDGE=1`. Binds
`127.0.0.1` (port via `DIGIMON_DEBUG_BRIDGE_PORT`, default `5174`). It drops a
discovery file at `<data_dir>/digimon-tcg/debug_bridge.json` that the MCP
auto-reads. After each external mutation it emits a `debug:state-changed`
window event; the frontend (`GamePage`) refetches so the board reflects the
staged state without a reload.

Build + run the desktop app with the bridge:

```bash
cd code/src-tauri
DIGIMON_DEBUG_BRIDGE=1 cargo tauri dev --features debug-bridge \
  --config '{"build":{"beforeDevCommand":""}}'
```

## Tools

| Tool | Purpose |
|------|---------|
| `stage_scenario` | Stage a board from an inline fixture or a saved name — no play needed |
| `read_state` / `get_mask` / `get_pending_selection` | Inspect the staged game |
| `step` | Drive to a decision point |
| `set_memory` / `inject_card` | Tweak the staged state |
| `capture_snapshot` | Capture the current board as a fixture (optionally `save_as`) |
| `evaluate` | Check engine assertions (browser only) |
| `list_fixtures` / `load_fixture` / `save_fixture` | Manage `qa/scenarios/` |
| `add_assertion` | Append an assertion to a fixture (pure) |
| `emit_playwright_spec` | Save fixture + scaffold a `.spec.ts` under `code/frontend/e2e/` |

Each state-touching tool takes `target: "browser" | "desktop"`.

## The loop

stage (browser **or** the real desktop window) → drive to the moment →
`capture_snapshot` → `add_assertion` → `save_fixture` / `emit_playwright_spec`
→ run. The fixture JSON stays the source of truth; the generated spec is a
thin, regenerable UI driver.

## Run

```bash
pip install -e code/digimon-scenario-mcp
python -m digimon_scenario_mcp --browser-url http://127.0.0.1:8000
```

Registered in `.mcp.json` as `digimon-scenario-mcp`.

## Tests

```bash
python -m pytest code/digimon-scenario-mcp/tests -q          # pure + dispatch
python -m pytest code/tests/api/test_export_scenario.py \
                 code/tests/api/test_capture_emit_proof.py -q # capture round-trip + emit proof
cargo test --manifest-path code/digimon-engine/Cargo.toml --test scenario_capture
cargo test --manifest-path code/src-tauri/Cargo.toml --lib --features debug-bridge
```

The cross-target contract test (`test_contract_live.py`) is opt-in
(`SCENARIO_MCP_LIVE=1` with both surfaces up) — the real Tauri process can't
be driven headlessly in CI.
