---
name: run-desktop
description: Start the Digimon TCG simulator UI in DESKTOP mode (the Tauri app window) for local play / manual testing, with the dev-only scenario debug bridge enabled so digimon-scenario-mcp can stage/snapshot/test the running game. Use whenever the user wants to run, start, launch, open, relaunch, or "see" the desktop app, the Tauri app, the game window, or "the UI in desktop mode" — and after editing frontend or src-tauri/engine code that you want to verify in the real desktop app. This is the verified launch recipe; prefer it over rediscovering `cargo tauri dev` flags, because the repo's configured `beforeDevCommand` is broken in this environment and a naive `cargo tauri dev` fails. To launch without the bridge (pure play / release-shaped), drop the bridge flags noted in step 4.
---

# Run the desktop app (Tauri)

The desktop app is a Tauri v2 shell (`code/src-tauri/`) that loads the React frontend from a Vite dev server and runs gameplay/inference in the embedded `digimon-engine` Rust crate. No Python is needed for local bot games.

**Why this skill exists:** `cargo tauri dev` normally runs the `beforeDevCommand` from `tauri.conf.json` (`cd ../frontend && npm run dev:desktop`) to start the dev server. In this environment that command fails with `The system cannot find the path specified` (Tauri runs it from the wrong working directory). So the reliable recipe is: **start the dev server yourself, then launch Tauri with `beforeDevCommand` overridden to empty.**

## Launch sequence

Run these from the repo root. Use your background-task tooling for the long-running ones (the dev server and `cargo tauri dev` stay running).

1. **Ensure frontend deps exist** (first run only):
   ```bash
   test -d code/frontend/node_modules || (cd code/frontend && npm install)
   ```

2. **Start the desktop Vite dev server** (background). `dev:desktop` = `vite --mode desktop`; it serves the desktop build (admin/training UI tree-shaken) on **http://localhost:5173**:
   ```bash
   cd code/frontend && npm run dev:desktop
   ```
   If port 5173 is already taken by a stale Vite, kill it first (`netstat -ano | grep :5173`, then `taskkill //F //PID <pid>` on Windows) — Tauri's `devUrl` is hard-coded to 5173.

3. **Wait for the dev server to be listening** before launching Tauri:
   ```bash
   until netstat -ano | grep -q ':5173.*LISTEN'; do sleep 1; done; echo "5173 READY"
   ```

4. **Launch the Tauri app with the debug bridge, skipping the broken `beforeDevCommand`** (background), from `code/src-tauri`:
   ```bash
   cd code/src-tauri && DIGIMON_DEBUG_BRIDGE=1 cargo tauri dev --features debug-bridge \
     --config '{"build":{"beforeDevCommand":""}}'
   ```
   First build compiles the engine + shell + `axum` (~1–3 min; the `debug-bridge` feature pulls `axum` the first time); incremental relaunches are ~15–60s. When it prints `Running .../target/debug/digimon-tcg.exe`, the window is open. Confirm with `tasklist | grep -i digimon-tcg.exe` (Windows).

   The `debug-bridge` feature + `DIGIMON_DEBUG_BRIDGE=1` start a localhost-only staging server (`127.0.0.1:5174`, override with `DIGIMON_DEBUG_BRIDGE_PORT`) so the `digimon-scenario-mcp` can stage boards / snapshot state / author tests against the *running* window — see "the debug bridge" below. To launch **without** the bridge (e.g. a pure play session, or to reproduce a release-shaped build), drop both `DIGIMON_DEBUG_BRIDGE=1` and `--features debug-bridge`; the bridge is then absent and inert.

## After it's running

- **Frontend edits (`code/frontend/`) hot-reload live** via Vite HMR — no Tauri rebuild needed. An already-rendered result (e.g. a finished game's Victory/Defeat) won't retroactively change; start a fresh game/state to exercise the new code.
- **Rust edits (`code/src-tauri/` or `code/digimon-engine/`)** trip the `cargo tauri dev` file-watcher, which recompiles and **relaunches the window automatically**. Just wait for the next `Finished`/`Running` line.
- **Stop it:** stop the `cargo tauri dev` background task (or close the window) and the dev-server task. Closing the window exits the process with code 0 — that's normal, not a crash (a crash is a non-zero exit).

## Gotchas (learned the hard way)

- **Do NOT run plain `cargo test` on `src-tauri`** — `tauri::generate_context!()` in `main.rs` panics with `frontendDist ... doesn't exist` because we run via the dev server, not a built `../frontend/dist`. Use `cargo test --manifest-path code/src-tauri/Cargo.toml --lib` (the commands/DTOs live in the `digimon_tcg` lib).
- **Stop `cargo tauri dev` before running any `src-tauri` `cargo` build/test** — two concurrent builds of the crate corrupt the build and surface as `error: proc macro panicked`.
- **Web (browser) mode is different**: `npm run dev` (not `dev:desktop`) + the hosted API `python -m uvicorn server.api:app` from the repo root (its SQLite path `./data/app.db` is cwd-relative, so launch from the root). Desktop mode needs neither the browser build nor uvicorn for local bot games.
- The app downloads trained AI models at runtime from the hosted API's manifest; local greedy-bot games work fully offline.

## The debug bridge (enabled by this skill)

Step 4 launches with the dev-only **debug bridge** on, so `digimon-scenario-mcp` can drive the *running desktop game* — stage arbitrary boards, snapshot state, author tests without playing. The bridge is a localhost-only (`127.0.0.1:5174`, override with `DIGIMON_DEBUG_BRIDGE_PORT`) HTTP server compiled **only** under the `debug-bridge` cargo feature and started **only** when `DIGIMON_DEBUG_BRIDGE=1` — it is absent from release/prod builds entirely. It drops a discovery file at `<data_dir>/digimon-tcg/debug_bridge.json` the MCP auto-reads, and emits a `debug:state-changed` window event after each external mutation so the board refreshes without a reload. See `docs/SCENARIO_MCP.md`.

To launch a **bridge-free** session (pure play, or a release-shaped build with no network surface), drop both `DIGIMON_DEBUG_BRIDGE=1` and `--features debug-bridge` from the step-4 command.
