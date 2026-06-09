## 0. Preconditions (verify, don't rebuild)

- [x] 0.1 Confirm `add-ui-scenario-test-substrate` is landed: `/debug` router on `RustDebugGame`, `qa/scenarios/` fixtures present (`q16-*`, `dna-paildramon-hand`), e2e page objects/helpers revived (`code/frontend/e2e/`). VERIFIED.
- [x] 0.2 Confirm the staging primitives `Game::to_scenario()` will invert are public on the engine: per-zone read access (hand/deck/field stacks + suspended/turn-played/breeding/security/trash) and deck order. VERIFIED — `Player` fields are `pub`; staging lives in `game/staging.rs`; `card_sources` is bottom-to-top.
- [x] 0.3 Confirm the documented `RustEngineState` lock order (`game` before `session`) and how `engine_commands.rs` holds the game, so the bridge can share state without deadlock. VERIFIED — `RustEngineState { game: Mutex<Option<Game>>, session: Mutex<GameSession> }`, lock order game→session.

## 1. Engine capture primitive

- [x] 1.1 Add `Game::to_scenario()` in `code/digimon-engine/` (new `game/snapshot.rs`) emitting the existing scenario-fixture schema with `assertions: {engine:[],ui:[]}`, using importer-inverse zone ordering (deck idx 0 = vec tail; security/deck bottom-first). Also added `Game::apply_scenario()` (Rust-side fixture applier, reused by the Tauri bridge).
- [x] 1.2 Round-trip test in `code/digimon-engine/tests/scenario_capture.rs`: non-trivial board (stacks w/ suspend+turn_played, breeding, trash, off-default memory/phase/turn/first-player), `to_scenario()`, re-stage via `apply_scenario()`, assert zone + scalar + deck-multiset equality. PASSES.
- [x] 1.3 Schema-shape test (`captured_fixture_has_the_documented_schema_shape`): captured fixture carries every required top-level + per-zone key and empty assertion buckets. PASSES. (Python-loader deserialization covered by the group-2 API test.)

## 2. Browser export wire

- [x] 2.1 Exposed `to_scenario()` on `RustDebugGame` and `RustHeadlessGame` in `code/digimon-engine-py/src/lib.rs`; rebuilt bindings (maturin build wheel + force-reinstall — no venv, so `develop` unavailable). Both classes now expose `to_scenario`.
- [x] 2.2 Added `POST /games/{id}/export-scenario` to `code/server/routers/games.py` (engine-only; no DB import). Returns the fixture dict directly — no new schema class needed (response is the captured fixture).
- [x] 2.3 API test (`code/tests/api/test_export_scenario.py`): stage via `/debug`, export via the route, re-stage from the captured fixture, assert `internal-state` scalar + per-player zone equality. PASSES.

## 3. Desktop debug bridge

- [x] 3.1 Made `RustEngineState` shareable: `game`/`session` are now `Arc<Mutex<...>>` (derive `Clone`); `.lock()` derefs through `Arc` so existing command code is unchanged. The bridge clones both Arcs at setup. Made `game_state_dto` / `action_mask_bytes` / `current_decision_player` `pub` for reuse.
- [x] 3.2 Added `code/src-tauri/src/debug_bridge.rs`. Staging/inspection verbs are realized as the bridge's HTTP endpoints (the MCP is out-of-process and cannot call Tauri `invoke`, so Tauri `#[command]` fns would be unreachable to it — design refinement). Each delegates to the shared engine API (`Game::apply_scenario`/`stage_inject_card`/`set_memory`/`to_scenario`/`decode_action`); `/state` & `/ui` return the desktop `game_state_dto` so the desktop DTO wire is exercised. No staging logic duplicated.
- [x] 3.3 Added `debug-bridge` cargo feature + optional `axum` dep (`debug-bridge = ["dep:axum", "tokio/net"]`) in `code/src-tauri/Cargo.toml`; the whole `debug_bridge` module + its `mod` decl are `#[cfg(feature = "debug-bridge")]`.
- [x] 3.4 Localhost HTTP server (axum on Tauri's async runtime), gated by `DIGIMON_DEBUG_BRIDGE=1`, bound to `127.0.0.1`, port via `DIGIMON_DEBUG_BRIDGE_PORT` (default `5174`); endpoints stage/apply/inject-card/set-memory/step/internal-state/state/mask/export-scenario. Writes a discovery file `dirs::data_dir()/digimon-tcg/debug_bridge.json` ({port, base_url}) the MCP auto-reads.
- [x] 3.5 Emit `debug:state-changed` after each external mutation (`notify()`); GamePage gains a Tauri-runtime-guarded listener that lazily imports the event API and refetches `getState`/`getMask` → store. Frontend typecheck clean.
- [x] 3.6 `cargo test --manifest-path code/src-tauri/Cargo.toml --lib --features debug-bridge`: 44 pass (incl. `stage_into_installs_a_board_that_round_trips`, `..._rejects_an_illegal_board_without_installing`, `..._rejects_a_fixture_without_decks`). Default build (no feature) compiles with NO bridge linked — proving release/prod excludes it. (A dedicated CI workflow asserting the same is a group-6 follow-up.)

## 4. Scenario MCP

- [x] 4.1 Scaffolded `code/digimon-scenario-mcp/` (package + `pyproject.toml` + `__main__.py` + `py.typed`), `mcp` SDK + `httpx` deps; installed editable. (`.mcp.json` registration in task 6.)
- [x] 4.2 Transport layer (`transport.py`): `BrowserClient` (FastAPI base URL, game-id addressed) + `DesktopClient` (bridge base URL via `resolve_desktop_base_url` — explicit > env > discovery dotfile > default); server normalizes both to one `{ok, ...}` shape. Pure `build_debug_create_body` + discovery resolution unit-tested.
- [x] 4.3 14 tools in `server.py`: `stage_scenario`, `read_state`, `get_mask`, `get_pending_selection`, `step`, `set_memory`, `inject_card`, `capture_snapshot`, `evaluate`, `list_fixtures`, `load_fixture`, `save_fixture`, `add_assertion`, `emit_playwright_spec` — each state-touching tool takes `target: browser|desktop`.
- [x] 4.4 `capture_snapshot(target=desktop)` goes through the live bridge `/export-scenario` endpoint. The no-bridge file-drop fallback is unnecessary in this design: the MCP cannot reach the Tauri process at all without the bridge, so capture (like all desktop interaction) requires the bridge up — documented in the design.
- [x] 4.5 Tests (18, all green): fixtures file-ops + `add_assertion` immutability, `build_debug_create_body`, desktop-URL precedence, server/handler dispatch, evaluate-rejects-desktop. Cross-target contract test (`test_contract_live.py`) is opt-in (`SCENARIO_MCP_LIVE=1` + both surfaces up) — skipped headless, as the real Tauri process can't be driven in CI.

## 5. Test emission

- [x] 5.1 Generator at `code/digimon-scenario-mcp/src/digimon_scenario_mcp/spec_gen.py` (testable + `python -m ... spec_gen <name>` CLI for CI): fixture → write to `qa/scenarios/` + scaffold `<name>.scenario.spec.ts` under `code/frontend/e2e/` reusing the existing `./fixtures/debug-game` helpers; idempotent; `@generated` banner. (Lives in the MCP package, not `code/tools/`, to avoid a cross-package import — design deviation noted; CLI preserves standalone/CI use.)
- [x] 5.2 Wrapped as the MCP `emit_playwright_spec` tool (`_h_emit_spec`).
- [x] 5.3 Proof (`code/tests/api/test_capture_emit_proof.py`): stage Q16 board → export via the route → `add_assertion` (stack_top AD1-011 + effective_dp 8000) → `generate_spec` (fixture + `.spec.ts`) → re-stage the captured fixture → `/debug evaluate` asserts `all_passed`. PASSES — the emitted fixture re-stages identically and its assertions hold. (Running the generated `.spec.ts` itself needs Playwright/browsers — out of scope for the unit-level proof.)

## 6. Docs

- [x] 6.1 `docs/SCENARIO_MCP.md` (+ package `README.md`): architecture diagram, browser vs desktop transports, the `debug-bridge` feature + env gate, the tool surface, and the capture→test loop.
- [x] 6.2 Updated CLAUDE.md: added a "Write-capable dev/test MCP" block recording `digimon-scenario-mcp` as the documented exception to the read-only rule (bounds: dev/test-only, never bundled, never imports `server.*`/`digimon_gym.*`, dev-gated surfaces only).
- [x] 6.3 Registered in `.mcp.json`; added the `docs/INDEX.md` row; appended an "agent-driven staging via the debug bridge" section to the run-desktop skill (the `--features debug-bridge` + `DIGIMON_DEBUG_BRIDGE=1` launch). (A dedicated CI workflow asserting release-bundle bridge-absence is the noted follow-up from 3.6.)
