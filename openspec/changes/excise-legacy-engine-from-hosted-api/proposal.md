> **STATUS: ACTIVE — PvP cutover landed (2026-06-15).** The hosted API now runs all gameplay on the Rust engine and `code/server/` imports zero `engine_py_legacy` (guardrail-enforced). Phases 1–3 + 5 were delivered by `shrink-legacy-engine-surface`; phase 4 (live PvP) was a full cutover to the `RustInteractiveGame` adapter. **Remaining:** GameEvent wiring for animation parity (task 4.4 — also fixes desktop) and the final `code/engine_py_legacy/` deletion (tasks 6.2/6.3). The "differential-test against the Python path" gate in the sketch below was dropped — the Python engine is bit-rotted and cannot serve as an oracle (see the design findings update, 2026-06-14); the Rust faithfulness suite + new interactive-wire tests are the gate instead.

## Why

The hosted API (`code/server/`) is the last production surface that still **runs games on the Python `engine_py_legacy` engine** rather than the Rust engine. Unlike the training stack (which is Rust-backed and only *imported* legacy on dead paths) and the desktop app (Python-free by construction), the server genuinely executes PvP matches, replays, deck validation, and admin tooling through Python-engine code. As long as that is true, the project ships two divergent rules engines in production, `docs/RUST_PYTHON_PARITY.md` cannot be retired, and rule 22 ("production code must not import `engine_py_legacy.*`") is structurally violated by `code/server/`.

This change documents the end-state — the hosted API runs entirely on the Rust engine via PyO3 — and the path to get there. It is deferred because it is a real migration with live-traffic risk (PvP WebSocket sessions, persisted recordings, deck-legality contracts), not a refactor.

## Current legacy coupling (the surface to migrate)

Seven clusters, by `engine_py_legacy` symbol:

1. **Live game runtime** — `InteractiveGame` in `ws_manager.py`, `ws_games.py`, `matchmaking.py`, `lobby.py`. The PvP turn loop, selection prompts, and per-player views run on the Python engine.
2. **Replay** — `ReplayRunner` in `state.py`, `replays.py`, `recordings.py`. Recorded games are stepped through the Python engine.
3. **Recordings** — `HeadlessGame` + `ReplayRunner` in `recordings.py`.
4. **Deck rules** — `parse_deck` (`simulations.py`, `lobby.py`), `validate_deck` / `RESTRICTED_LIST` / `CardRestriction` (`db/routers/decks.py`), `summarize_deck` (`db/routers/training.py`). The legality + restricted-list contract.
5. **State filtering** — `state_filter` / `filter_state_for_player` / `filter_state_for_spectator` (`ws_manager.py`, `ws_games.py`). Opponent/spectator redaction (rules 9 & 14).
6. **Enums** — `PlayerType` (`lobby.py`).
7. **Script promotion** — `script_promotion` (`db/routers/admin_ai.py`). The admin AI pipeline's frozen-script flow — Python-engine-specific and may be retired rather than migrated as the engine becomes Rust-only.

## What Changes (target end-state)

- **BREAKING** PvP games run on a Rust interactive runner exposed via PyO3 (extend `code/digimon-engine-py/` with an interactive surface analogous to `RustHeadlessGame` but driving selection prompts and per-player observation). `ws_*`/`matchmaking`/`lobby` consume it; no `InteractiveGame` import remains.
- **BREAKING** Replays + recordings step through the Rust replay core (`code/digimon-engine/src/runners/replay.rs`) via PyO3, replacing `ReplayRunner`/`HeadlessGame` on the server. Recording format compatibility is preserved or a migration is provided.
- **BREAKING** Deck parsing/validation/summary route through the Rust deck tools (`code/digimon-engine/src/deck_tools.rs`, already used by the Tauri desktop layer) via PyO3, replacing `parse_deck`/`validate_deck`/`summarize_deck`/`RESTRICTED_LIST`/`CardRestriction`. The restricted-list source of truth moves to (or is verified against) the Rust side.
- **BREAKING** State redaction for network clients is provided over Rust game state (a Rust-native filter or a thin Python filter over the PyO3 state), preserving the `state_filter` contract (never leak opponent `handIds`/`handCards`).
- `PlayerType` and other small enums are re-homed to a non-legacy location (PyO3 export or a shared constants module).
- The admin AI `script_promotion` flow is evaluated for **retirement** (it is Python-card-script machinery) vs migration; decision recorded in design before any code moves.
- `code/server/` imports zero `engine_py_legacy`; `docs/RUST_PYTHON_PARITY.md` becomes eligible for retirement; rule 22 holds for the server.

## Capabilities

### New Capabilities
- `legacy-free-hosted-api`: defines that the hosted API executes all gameplay, replay, recording, deck-legality, and state-redaction logic on the Rust engine (directly or via PyO3) and imports zero `engine_py_legacy` symbols, while preserving the existing network contracts (per-player state redaction, deck legality + restricted list, recording/replay compatibility).

## Impact

- **Affected code (server)** — `routers/ws_manager.py`, `routers/ws_games.py`, `routers/matchmaking.py`, `routers/lobby.py`, `routers/simulations.py`, `routers/state.py`, `routers/replays.py`, `routers/recordings.py`, `db/routers/decks.py`, `db/routers/training.py`, `db/routers/admin_ai.py`
- **Affected code (bindings)** — `code/digimon-engine-py/src/lib.rs` (new interactive + deck-tools + replay + state-redaction surfaces); possibly new helpers in `code/digimon-engine/src/runners/` and `deck_tools.rs`
- **Affected docs** — `docs/RUST_PYTHON_PARITY.md` (retirement), `docs/ARCHITECTURE.md`, `docs/DEPLOYMENT.md`, deck-legality docs
- **Risk surface** — live PvP traffic, persisted recordings/replays, deck-legality contracts consumed by clients and the deck builder, the admin AI pipeline
- **No changes to** — the training build (covered by `make-training-build-legacy-free`), the desktop app (already Python-free)
