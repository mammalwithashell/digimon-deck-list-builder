---
name: run-desktop
description: Start the Digimon TCG simulator UI in DESKTOP mode (the Tauri app window) for local play / manual testing. Use whenever the user wants to run, start, launch, open, relaunch, or "see" the desktop app, the Tauri app, the game window, or "the UI in desktop mode" — and after editing frontend or src-tauri/engine code that you want to verify in the real desktop app. This is the verified launch recipe; prefer it over rediscovering `cargo tauri dev` flags, because the repo's configured `beforeDevCommand` is broken in this environment and a naive `cargo tauri dev` fails.
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

4. **Launch the Tauri app, skipping the broken `beforeDevCommand`** (background), from `code/src-tauri`:
   ```bash
   cd code/src-tauri && cargo tauri dev --config '{"build":{"beforeDevCommand":""}}'
   ```
   First build compiles the engine + shell (~1–3 min); incremental relaunches are ~15–60s. When it prints `Running .../target/debug/digimon-tcg.exe`, the window is open. Confirm with `tasklist | grep -i digimon-tcg.exe` (Windows).

## After it's running

- **Frontend edits (`code/frontend/`) hot-reload live** via Vite HMR — no Tauri rebuild needed. An already-rendered result (e.g. a finished game's Victory/Defeat) won't retroactively change; start a fresh game/state to exercise the new code.
- **Rust edits (`code/src-tauri/` or `code/digimon-engine/`)** trip the `cargo tauri dev` file-watcher, which recompiles and **relaunches the window automatically**. Just wait for the next `Finished`/`Running` line.
- **Stop it:** stop the `cargo tauri dev` background task (or close the window) and the dev-server task. Closing the window exits the process with code 0 — that's normal, not a crash (a crash is a non-zero exit).

## Gotchas (learned the hard way)

- **Do NOT run plain `cargo test` on `src-tauri`** — `tauri::generate_context!()` in `main.rs` panics with `frontendDist ... doesn't exist` because we run via the dev server, not a built `../frontend/dist`. Use `cargo test --manifest-path code/src-tauri/Cargo.toml --lib` (the commands/DTOs live in the `digimon_tcg` lib).
- **Stop `cargo tauri dev` before running any `src-tauri` `cargo` build/test** — two concurrent builds of the crate corrupt the build and surface as `error: proc macro panicked`.
- **Web (browser) mode is different**: `npm run dev` (not `dev:desktop`) + the hosted API `python -m uvicorn server.api:app` from the repo root (its SQLite path `./data/app.db` is cwd-relative, so launch from the root). Desktop mode needs neither the browser build nor uvicorn for local bot games.
- The app downloads trained AI models at runtime from the hosted API's manifest; local greedy-bot games work fully offline.
