## Why

The hosted API (`code/server/`) is the last production surface importing `engine_py_legacy` (rule 22), but an investigation (2026-06-14) found the coupling is **not monolithic**: only the **live PvP/WebSocket runtime** (`ws_games.py`, `ws_manager.py`, and `lobby.py`'s `InteractiveGame` construction) genuinely needs the deferred, high-risk interactive Rust runner. **Every other legacy touch-point in the server and in `code/tools/` can go Rust-only today, independently, at low risk** — and several are dead or near-dead code:

- `state_filter.py` is a pure dict filter with **zero engine imports** that already runs over `RustHeadlessGame.to_ui_json()` output unchanged; it lives under `engine_py_legacy/` only by accident of history (the READMEs already (wrongly) reference `server/state_filter.py`).
- Deck parsing/validation already runs on the Rust binding for `deck_tools`/desktop/matchmaking; the remaining Python consumers (`simulations.py`, `db/routers/training.py`, `db/routers/decks.py`, `lobby.py`'s `parse_deck`) are drop-in swaps, and the restricted/EDEN lists are byte-identical across engines today.
- The `replays.py`/`recordings.py`/`state.py` replay routers are **currently dead** (they import bit-rotted legacy and the save gate cannot pass); the Rust `NativeAdapter` reads the persisted recording format exactly, so reviving them on the Rust replay core carries near-zero data-compatibility risk.
- `script_promotion` (admin AI) promotes *Python* card scripts (a sunset model); its feeder paths already point at a **nonexistent directory** — it is dead-but-wired and should be retired, not migrated.
- Of the `code/tools/` legacy importers, 4 are obsolete (delete), 2 are trivial re-homes (a regex, a dead fallback), and 1 (`ingest_cards`) needs a small parser port.

Doing this now clears multiple live rule-22 violations, deletes dead code, and shrinks the deferred `excise-legacy-engine-from-hosted-api` change down to its one genuinely hard piece (the PvP interactive wire). It deliberately does **not** touch the PvP runtime, the interactive Rust PyO3 runner, GameEvent wiring, or the final deletion of `code/engine_py_legacy/`.

## What Changes

- **State redaction module relocated** — move `state_filter.py` verbatim (no logic change) from `engine_py_legacy/engine/` to a production package (`code/server/`), and repoint its importers (`ws_manager.py`, `ws_games.py`) and `code/tests/api/test_state_filter_modifiers.py`. Clears a live rule-22 violation; the filter already consumes Rust `to_ui_json` output unchanged.
- **Deck rules → Rust binding** — repoint `simulations.py` (`parse_deck`), `db/routers/training.py` (`summarize_deck`), `lobby.py` (`parse_deck`), and `db/routers/decks.py` (`validate_deck` / restricted-list) onto `from digimon_engine import …`. Route the `no_restriction` game-mode through the existing `validate_deck_for_game_mode("no_restriction")` (removes the need for a custom-`CardRestriction` PyO3 overload); keep `titan`/`edh_commander` as a thin Python size/singleton wrapper over the Rust binding **or** add Rust `DeckRuleset` arms (design decision). Re-home `PlayerType` into a server-side enum module.
- **Replay + recordings → Rust replay core** — add a thin PyO3 `RustReplayRunner` over `ReplaySession`/`NativeAdapter` (new / seek / per-step state via `to_ui_json` / total_steps / current_step), reconcile 0-based Rust vs 1-based server step indexing, repoint `replays.py` / `recordings.py` / `state.py`, fix the save gate to accept `RustHeadlessGame`, and enable `record_actions` at creation. Existing persisted recordings (Postgres `game_recordings.recording_json`) remain replayable — same format, no DB migration.
- **`script_promotion` retired** — remove the `script_promotion` import and its two admin endpoints from `db/routers/admin_ai.py`. The rest of the admin AI pipeline (review / QA / issue surfaces) is out of scope and untouched.
- **Tools de-legacied** — delete `promote_script.py`, `archive/bootstrap_frozen_manifest.py`, `check_frozen_integrity.py`, `run_qa_batch.py` (obsolete Python-script-lane machinery); re-home `meta_loader.py` (inline `RE_CARD_ID`) and `resolve_deck.py` (drop the dead `except ImportError` legacy fallback); port `parse_xros_req`/`parse_digixros_req` for `ingest_cards.py`; delete `train_card_autoencoder.py` (its warm-start path is already dead) and `run_scenario.py` (after the `gameplay-qa` skill is repointed at the Rust `digimon-engine-cli` scenario runner). `build_tested_cards.py` is **not** a legacy importer — left untouched.
- **Guardrail** — extend the existing legacy-free import test to assert the non-PvP server support surfaces and the retained tools import cleanly with `engine_py_legacy` blocked.

## Capabilities

### New Capabilities
- `legacy-free-support-surfaces`: defines that every hosted-API surface **except the live PvP/WebSocket runtime** (deck legality, replay, recordings, state redaction) and the `code/tools/` CLI execute on the Rust engine (directly or via PyO3) and import zero `engine_py_legacy`, while preserving the deck-legality, redaction, and recording/replay contracts.

## Impact

- **Affected code (server)** — `routers/simulations.py`, `routers/replays.py`, `routers/recordings.py`, `routers/state.py`, `routers/lobby.py` (`parse_deck` + `PlayerType` only; `InteractiveGame` stays), `db/routers/training.py`, `db/routers/decks.py`, `db/routers/admin_ai.py`; new `code/server/state_filter.py` + a server-side `PlayerType` enum home.
- **Affected code (bindings)** — `code/digimon-engine-py/src/lib.rs` (new `RustReplayRunner`; the `ingest_cards` xros parsers if ported via PyO3); possibly `DeckRuleset` arms in `code/digimon-engine/src/deck_tools.rs`.
- **Affected code (tools)** — delete 4, re-home 2, port 1, delete 2 (see What Changes).
- **Affected tests** — repoint `code/tests/api/test_state_filter_modifiers.py`; repoint or remove `code/tests/ai_pipeline/test_ai_pipeline.py` (with the `script_promotion` retirement); extend the legacy-free guardrail.
- **Affected docs** — `docs/TOOLS.md`, `AGENTS.md`, `README.md` (scrub deleted tools); `code/server/README.md` / `code/frontend/README.md` (state_filter path); `docs/RUST_PYTHON_PARITY.md` (consumer table).
- **Out of scope** — the PvP/WebSocket interactive runtime (`InteractiveGame` in `ws_*` / `lobby`), the interactive Rust PyO3 runner, GameEvent wiring, and the final deletion of `code/engine_py_legacy/` — all remain with `excise-legacy-engine-from-hosted-api`. **This change shrinks that one to the PvP wire only.**
- **Coordination** — sequence the tools deletions with the in-flight `make-training-build-legacy-free` (which already stops copying those tools into `Dockerfile.training`).
- **No changes to** — the Rust engine's gameplay rules, the action space, the wire/state JSON schema (beyond relocating the redaction module), or the desktop app.
