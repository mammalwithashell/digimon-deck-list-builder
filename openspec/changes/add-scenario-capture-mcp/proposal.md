## Why

The landed `add-ui-scenario-test-substrate` change gave us a way to *stage* an arbitrary mid-game board against the real Rust engine over HTTP (`/debug` router, backed by `RustDebugGame`) and to encode expected outcomes as a declarative `qa/scenarios/` fixture. But two things stop that substrate from removing the manual UI-testing toil it was built to remove:

1. **No agent driver.** The staging surface is raw HTTP for Playwright. An assistant helping test a card or interaction still has to hand-author HTTP calls or fixtures; there is no tool surface that lets it stage "Kaiser Nail with X underneath at 3 memory," drive to the decision point, read the result, and assert — the loop the substrate was meant to enable.
2. **No capture-from-live.** The original proposal explicitly deferred scenario *export* (`to_scenario()`): "Import-first; export is a follow-up." So you cannot turn the game you are actually playing into a reusable fixture. You can build a board by hand-writing JSON, but you cannot snapshot a board you stumbled into.

And the desktop app — the build people actually play — is doubly unreachable. It is a Python-free Tauri process with no HTTP server, holding one game in `RustEngineState { game: Mutex<Option<Game>> }`. Browser-dev mode is the project's tested UI proxy, but it renders a *different* serialization wire (`serialization.rs`) than the desktop's hand-maintained `engine_commands.rs` DTOs — exactly the wire whose drift produced this development cycle's run of desktop-only bugs (missing selection `kind`, permanent `sources`, `trash` cards, the raw-vs-UI winner id). Browser-mode testing structurally cannot catch those.

This change closes both gaps: a capture primitive on the engine, a live debug bridge into the Tauri process, and an MCP that drives all of it.

## What Changes

- **Engine capture primitive.** Add `Game::to_scenario()` to `code/digimon-engine/`: serialize the current full-information game into the **existing** `qa/scenarios/` fixture schema (per-player decks, zones with suspended/turn-played, scalar state), with an empty assertion list. It is the exact inverse of the existing import path and MUST round-trip — re-applying a captured fixture reproduces the identical board.
- **Browser export route.** Expose `to_scenario()` over PyO3 on both `RustDebugGame` and `RustHeadlessGame`, and add `POST /games/{id}/export-scenario` (engine-only router) so a *live* browser-dev game — not just a debug-staged one — is capturable.
- **Desktop debug bridge** (NEW). Add `rust_debug_*` Tauri commands (stage / inject-card / place-on-field / bulk-setup / set-memory / step / read-internal-state / export-scenario) that operate on the existing single `RustEngineState` game by wrapping the *same* engine staging primitives the browser path uses. Behind a `debug-bridge` cargo feature **and** a runtime env gate, spin up a localhost-only HTTP server inside the Tauri process exposing those commands, so the MCP can drive the actual window being played. The bridge emits a `debug:state-changed` window event so the webview refreshes after external staging. **Never compiled into release/prod bundles.**
- **`digimon-scenario-mcp`** (NEW). A Python MCP at `code/digimon-scenario-mcp/`, sibling to `digimon-training-mcp`, exposing a unified tool surface with a `target: browser | desktop` selector that routes to the FastAPI `/debug` surface or the Tauri bridge: stage, read-state, get-mask, get-pending-selection, step, capture-snapshot, fixture-file ops over `qa/scenarios/`, add-assertion / evaluate, and `emit_playwright_spec`. Unlike the two existing read-only operator MCPs, it is **write-capable** — a deliberate, documented departure, justified by its dev/test-only scope.
- **Test emission.** `emit_playwright_spec(fixture)` scaffolds a `.spec.ts` under `code/frontend/e2e/` that loads the fixture, stages it via `/debug`, drives the UI, and asserts — reusing the page objects/helpers the prior change revived. The fixture JSON remains the single source of truth.

Non-goals (explicit): no new *staging* primitives (the `/debug` surface and `DebugRunner` already cover hand/deck/field/breeding/security/trash + memory/phase/turn/first-player); no rules-logic changes; no change to the fixture schema itself (capture targets the existing format); the bridge and MCP are never shipped in any production build; the MCP does not introspect the Tauri process's memory directly — desktop interaction goes through the gated bridge server only.

## Capabilities

### New Capabilities
- `scenario-capture`: `Game::to_scenario()` and its both-wire exposure — turning any live or staged game into a round-trip-faithful `qa/scenarios/` fixture.
- `desktop-debug-bridge`: the dev-only Tauri staging commands plus the gated localhost bridge server that makes the actual desktop game stageable / inspectable / capturable from outside the process.
- `scenario-mcp`: the write-capable `digimon-scenario-mcp` tool surface and its `browser | desktop` transport routing.
- `scenario-test-emission`: generating a Playwright `.spec.ts` from a captured or staged fixture.

### Modified Capabilities
<!-- None. The existing scenario-staging-engine, scenario-fixture-format, and debug-game-http-surface capabilities are reused unchanged; this change is additive (capture is the named follow-up to that change's import-first scope). -->

## Impact

- **Rust engine**: `code/digimon-engine/src/game.rs` (or a `snapshot.rs` module) gains `Game::to_scenario()`; may need small `pub` additions to read deck order / per-zone contents. No rules changes.
- **PyO3 bindings**: `code/digimon-engine-py/src/lib.rs` — `to_scenario()` on `RustDebugGame` and `RustHeadlessGame`.
- **Hosted API**: `code/server/routers/games.py` gains `POST /games/{id}/export-scenario`; new response schema in `schemas.py`. Engine-only router rules preserved (no DB import).
- **Desktop (Tauri)**: `code/src-tauri/src/` — new `debug_bridge.rs` (commands + gated axum server), wired in `main.rs`/`lib.rs` behind a `debug-bridge` feature; `Cargo.toml` feature + optional `axum` dep. `RustEngineState` made shareable with the server thread (e.g. inner `Arc`). Frontend listens for `debug:state-changed` to refresh.
- **New MCP**: `code/digimon-scenario-mcp/` (package, `pyproject.toml`, `__main__.py`, tool modules). `requirements-mcp.txt` or a sibling adds the `mcp` SDK + `httpx`. Registered in `.mcp.json` for local dev.
- **Frontend e2e**: `emit_playwright_spec` output lands in `code/frontend/e2e/`; reuses existing page objects/helpers.
- **Docs**: new `docs/SCENARIO_MCP.md` (tool surface + browser/desktop setup); CLAUDE.md "Read-only operator MCPs" note updated to record the write-capable dev MCP exception.
- **No production impact**: the bridge is feature-gated out of release builds; the MCP is dev-only and never imported by `server.*`, `digimon_gym.*`, or any prod path.
