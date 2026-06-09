## Context

`add-ui-scenario-test-substrate` landed the *import* half of scenario testing: a `/debug` FastAPI router backed by a `RustDebugGame` PyO3 class stages arbitrary per-player zones + scalar state against the real engine, and a declarative `qa/scenarios/` fixture format (decks + zones + initial state + assertions) is consumed by both a Rust headless runner and a revived Playwright e2e suite. Three fixtures already exist (`q16-*`, `dna-paildramon-hand`). That change deferred two things by name: scenario *export* (`to_scenario()`) and any reach into the real Tauri binary (Playwright drives browser-mode as a proxy).

This change delivers both deferrals plus the missing agent driver (an MCP), so the stage→drive→capture→assert→emit loop is usable without hand-playing a game, against either browser-dev or the actual desktop app.

## Goals / Non-Goals

**Goals**
- Capture any live or staged game into the existing fixture format, round-trip-faithfully.
- Let an assistant stage, drive, inspect, capture, and assert via MCP tools — against browser-dev *and* the real desktop window.
- Make the desktop-only `engine_commands.rs` DTO wire testable end-to-end (the one surface browser-mode cannot cover).
- Emit a durable fixture + Playwright spec per scenario.

**Non-Goals**
- New staging primitives — `DebugRunner` + `/debug` already cover every zone and scalar.
- Rules/engine-logic changes; fixture-schema changes.
- Shipping the bridge or MCP in any production build.
- MCP reading Tauri process memory directly (only via the gated bridge server).
- Multi-game on desktop — the Tauri process holds one game; that is accepted.

## Decisions

### D1. `to_scenario()` lives on the engine `Game`, not per-binding
A single capture implementation on `code/digimon-engine` is the inverse of the importer and the only way both wires stay faithful. `RustDebugGame` and `RustHeadlessGame` (PyO3) and the Tauri command all call it. This mirrors the project's hard-won lesson that *two hand-maintained serialization wires drift* (`serialization.rs` vs `engine_commands.rs`); capture gets exactly one implementation.

### D2. Capture targets the existing fixture schema verbatim
`to_scenario()` emits the `scenario-fixture-format` schema with an empty `assertions: []`. Field semantics match the importer (deck index 0 = top = end of engine vec; security/deck injected bottom-first). The fixture-format spec's existing round-trip requirement ("a fixture round-trips into a staged game") becomes the acceptance test for capture: `to_scenario()` → `/debug` apply → `internal_state()` equals the source board.

### D3. Browser capture works on live `/games`, not only `/debug`
A captured snapshot is most valuable when you stumble into a board during a *real* game. So export is exposed as `POST /games/{id}/export-scenario` (works for any `RustHeadlessGame`), in addition to riding on `RustDebugGame`. Engine-only router discipline preserved (no DB import).

### D4. Desktop reach = a feature-gated localhost bridge, not a permanent server
The Tauri app must never carry a network surface in production. The bridge is gated by **both** a `debug-bridge` cargo feature (compiled out of release) **and** a runtime env var (`DIGIMON_DEBUG_BRIDGE=1`) so even a debug build is inert unless explicitly opted in. It binds `127.0.0.1` only. The endpoints mirror the `/debug` verbs minus the game id (desktop is single-game). They wrap the *same* `Game::stage_*` primitives as the PyO3 path — no duplicated staging logic.

### D5. Shared state + refresh event
The bridge server thread and the Tauri `invoke` commands mutate the *same* game. `RustEngineState` is made shareable (inner `Arc` around the existing `Mutex<Option<Game>>`, handed to both `app.manage(...)` and the server thread) preserving the documented lock order (`game` before `session`). After any external mutation the bridge emits a `debug:state-changed` Tauri event; the frontend's existing refresh path re-`invoke`s `get_rust_game_state`. Without this the webview would render a stale board after the MCP stages a state.

### D6. One MCP, two transports, a `target` selector
`digimon-scenario-mcp` exposes verb tools (`stage_scenario`, `read_state`, `get_mask`, `get_pending_selection`, `step`, `capture_snapshot`, `evaluate`, fixture-file ops, `emit_playwright_spec`). Each state-touching tool takes `target: "browser" | "desktop"`. `browser` → FastAPI base URL (multi-game, returns/accepts `game_id`); `desktop` → the Tauri bridge base URL (single implicit game, no id). The MCP normalizes the two response shapes so the caller sees one schema. Default `target` is configurable per session.

### D7. Write-capable MCP — a documented exception
`digimon-engine-mcp` and `digimon-training-mcp` are read-only by policy (CLAUDE.md "Read-only operator MCPs"). This one mutates game state and writes files (`qa/scenarios/`, e2e specs). That is the whole point — it is the staging driver. It is justified and bounded: dev/test-only, never bundled, never imported by `server.*` / `digimon_gym.*`, talks only to dev-gated surfaces. The CLAUDE.md note is updated to record the exception rather than silently violating the convention.

### D8. Test emission is fixture-first; the spec is a thin driver
`emit_playwright_spec` writes the fixture to `qa/scenarios/` (source of truth) and scaffolds a `.spec.ts` that loads it, POSTs to `/debug`, drives the UI via the existing page objects, and asserts via the existing `evaluate` surface. Regenerating the spec from the fixture is idempotent; hand-edits go in the fixture, not the generated TS.

## Risks / Trade-offs

- **A networked surface inside the desktop app is a footgun.** Mitigated by the double gate (feature + env) and `127.0.0.1` bind; CI asserts the release bundle exports no bridge symbol.
- **Two transports, divergent shapes.** Mitigated by the MCP normalization layer + a contract test that the same fixture staged via `browser` and `desktop` yields equal `internal_state()`.
- **Single-game desktop** limits parallel scenarios on the real app; acceptable — browser-dev covers multi-game, desktop covers the DTO-wire check.
- **Generated specs rot** if hand-edited. Mitigated by D8 (fixture is canonical; spec is regenerable) and a header banner marking the file generated.

## Migration Plan

Purely additive. No existing route, fixture, or binding changes behavior. Order: (1) `to_scenario()` + PyO3 + round-trip test; (2) browser export route; (3) Tauri commands + gated bridge + refresh event; (4) MCP over both transports; (5) `emit_playwright_spec` + one end-to-end captured fixture (re-capture an existing `q16` board as the proof). Each stage is independently shippable.

## Open Questions

- Bridge port: fixed default (e.g. `5174`) vs env-configurable vs handshake file the MCP reads. Lean: env-configurable with a sane default, written to a dev dotfile the MCP auto-discovers.
- Should `capture_snapshot(target=desktop)` go through the bridge HTTP `export-scenario` endpoint, or read a file the `rust_export_scenario` command drops? Lean: bridge endpoint when the bridge is up (live), file-drop as the no-bridge fallback so plain capture works without opting into the server.
- Does `emit_playwright_spec` belong in the MCP or as a standalone `code/tools/` script the MCP shells out to? Lean: a `code/tools/scenario_spec_gen/` script (testable, reusable in CI) that the MCP wraps.
