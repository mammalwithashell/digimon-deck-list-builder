## Why

We can't write UI end-to-end tests for gameplay-rules scenarios because there is no way to stage an arbitrary mid-game board against the real (Rust) engine over HTTP. The Playwright e2e suite that exists (`code/frontend/e2e/`) is orphaned: its `debug-game.ts` fixture targets a `/debug` router built on the sunset `engine_py_legacy` engine (gated behind `DEBUG_MODE=1`), and it calls `GET /games/{id}/actions`, a route the Rust rewrite removed. The Rust engine already has every staging primitive we need (`DebugRunner`), but none are exposed beyond `cargo test`. Without this bridge, every rules interaction — DNA digivolve, selection effects, timing windows — can only be verified by playing through the game by hand, which is exactly the toil that let recent UI-wiring bugs (DNA digivolve misfiring, field-selection targets not highlighted) ship unnoticed.

## What Changes

- Expose the Rust engine's `DebugRunner` staging primitives through a new debug-only PyO3 class, `RustDebugGame`, that stages per-player zones (hand, deck order, field stacks, breeding, security, trash) plus initial memory / phase / first-player / skip-shuffle, while still presenting the normal `step` / `get_action_mask` / `to_ui_json` / `get_events` surface so existing HTTP routes work unchanged.
- Replace the legacy `code/server/routers/debug_games.py` with a Rust-backed `/debug` router. **BREAKING** for the legacy debug router: it drops the `engine_py_legacy` import (per CLAUDE.md rule #22) and the `DEBUG_MODE`-gated `InteractiveGame`-based implementation. The route surface the existing e2e fixture depends on (create-with-staging, set-memory, inject-card, internal-state) is preserved, plus richer place-on-field / bulk-zone-setup endpoints.
- Introduce a declarative **scenario fixture format** (JSON): decks + per-player zones + initial state + a list of expected assertions (memory value, stack-top card, effective DP, zone contents, did-effect-trigger, is-action-legal, legal-selection-options). One fixture file is consumable by both a Rust headless runner and the Playwright UI fixture.
- Revive `code/frontend/e2e/debug-game.ts` and its page objects/helpers against the Rust-backed routes; bring the three orphaned specs (`digivolution`, `timing-regression`, `memory-accounting`) back to green.
- Seed a conformance corpus from the implementable subset of the 30-question fan rules quiz, each fixture tagged expected-pass vs blocked-on-card-impl, with Q16 (Paildramon over ExVeemon + Stingmon — the actual reported bug) as a priority fixture.

Non-goals (called out explicitly): driving the real Tauri binary (Playwright cannot; browser-mode is the tested proxy); full card-implementation conformance (the substrate lets us *write* all 30 scenarios — whether each *passes* depends on per-card implementation, which is separate work); scenario *export* / capture-from-live-game (`to_scenario()`) is a deliberate follow-up — this change is import-first.

## Capabilities

### New Capabilities
- `scenario-staging-engine`: The `RustDebugGame` PyO3 surface — constructing a game with fully staged zones and mid-game state, and mutating that state (set-memory, inject-card, place-on-field, bulk-setup) for deterministic test setup.
- `scenario-fixture-format`: The declarative JSON schema for a staged scenario plus its expected assertions, and the contract that the same fixture is consumable by both the Rust headless runner and the Playwright UI fixture.
- `debug-game-http-surface`: The Rust-backed `/debug` HTTP router that exposes scenario staging and inspection to browser-mode clients, replacing the legacy `engine_py_legacy` debug router.

### Modified Capabilities
<!-- None. The live `/games` play surface, RustHeadlessGame, and existing specs are unchanged; this change is additive plus a one-for-one replacement of the orphaned legacy debug router. -->

## Impact

- **Rust engine bindings**: `code/digimon-engine-py/src/lib.rs` gains a `RustDebugGame` pyclass (new `#[pyclass]`, registered in the module). May require small `pub` visibility additions on `DebugRunner` / `DebugRunnerBuilder` if any needed primitive isn't already public.
- **Rust engine**: `code/digimon-engine/src/debug_runner.rs` — confirm/extend the public staging API (security/trash injection, phase/first-player setters) if gaps surface; no rules-logic changes.
- **Hosted API**: `code/server/routers/debug_games.py` rewritten onto `RustDebugGame`; `code/server/api.py` router registration reordered (debug router before DB routers, mirroring `desktop_decks`); `engine_py_legacy` import removed. New `/debug` request/response schemas in `code/server/routers/schemas.py`.
- **Frontend e2e**: `code/frontend/e2e/fixtures/debug-game.ts`, `page-objects/game-page.ts`, `helpers/assertions.ts` updated; `digivolution.spec.ts`, `timing-regression.spec.ts`, `memory-accounting.spec.ts` revived; new scenario-driven specs added.
- **Scenario corpus**: new fixture files (location decided in design — `qa/scenarios/` shared vs `code/frontend/e2e/fixtures/scenarios/`).
- **PyO3 rebuild**: requires `maturin`/pip rebuild of `digimon_engine` before the FastAPI debug routes work; document in tasks.
- **No desktop impact**: the Python-free Tauri bundle never imports `RustDebugGame`; debug staging is browser-mode/test-only.
