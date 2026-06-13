# digimon-scenario-mcp

A **WRITE-capable, dev/test-only** MCP stdio server for staging, snapshotting,
and authoring Digimon TCG game-state tests — the agent driver for the
`add-ui-scenario-test-substrate` `/debug` surface plus the new
capture-from-live primitive.

Unlike the read-only operator MCPs (`digimon-engine-mcp`,
`digimon-training-mcp`), this one **mutates game state and writes files**
(`qa/scenarios/`, `code/frontend/e2e/`). It is dev/test-only: never bundled
into any production build, and it talks only to dev-gated surfaces.

## Two transports (`target`)

- `browser` → the hosted API's `/debug` + `/games` routes (multi-game,
  addressed by `game_id`). Needs `uvicorn server.api:app` running.
- `desktop` → the feature-gated Tauri **debug bridge** (single live game, no
  id). Needs the desktop app built with `--features debug-bridge` and run
  with `DIGIMON_DEBUG_BRIDGE=1`. The bridge URL is auto-discovered from
  `<data_dir>/digimon-tcg/debug_bridge.json`.

The desktop target is the only way to exercise the desktop-only
`engine_commands.rs` DTO wire end-to-end (browser-mode can't reach it).

## Tools

`stage_scenario`, `read_state`, `get_mask`, `get_pending_selection`, `step`,
`set_memory`, `inject_card`, `capture_snapshot`, `evaluate` (browser only),
`list_fixtures`, `load_fixture`, `save_fixture`, `add_assertion`,
`emit_playwright_spec`.

## The loop

stage (browser **or** the real desktop window) → drive to the moment →
capture → add assertions → save fixture + emit `.spec.ts`. No hand-played
games.

## Run

```bash
pip install -e code/digimon-scenario-mcp
python -m digimon_scenario_mcp --browser-url http://127.0.0.1:8000
```

Generate a Playwright spec from a saved fixture, standalone:

```bash
python -m digimon_scenario_mcp.spec_gen <fixture-name>
```
