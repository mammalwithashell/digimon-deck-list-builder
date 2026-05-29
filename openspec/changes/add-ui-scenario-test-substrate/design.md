## Context

The Playwright e2e suite (`code/frontend/e2e/`) was written against the pre-Rust architecture. Its `fixtures/debug-game.ts` calls a `/debug` router implemented in `code/server/routers/debug_games.py` on top of `engine_py_legacy.engine.runners.InteractiveGame`, gated behind `DEBUG_MODE=1`. The Rust rewrite re-homed the live `/games` router onto `RustHeadlessGame` (PyO3) but left `/debug` stranded on the sunset engine, and removed `GET /games/{id}/actions` (now `/action-mask`, a bare array). The suite therefore cannot stage scenarios against today's engine.

Meanwhile the Rust engine's `DebugRunner` (`code/digimon-engine/src/debug_runner.rs`) already implements the staging primitives:
- Construction-time (via `DebugRunnerBuilder`): per-player `hands`, `decks` (top = end of vec), `securities` (top = end), `digitamas`, and `initial_memory`.
- Post-`start()` mutation: `place_on_field(stack, suspended, turn_played)`, `place_stack`, `place_in_breeding`, `set_memory` (via `game_mut`), `force_base_dp`.
- Inspection: `memory`, `effective_dp`, `top_card`, zone sizes, `events_since`, `pending_selection_view`, `current_phase`, `turn_player`.

`DebugRunner`'s richer card behavior (real BT/EX/AD cards, not test stubs) is available only with the `dsl-yaml-loader` feature and the production card DB. None of this is exposed through PyO3 — `RustHeadlessGame` only accepts `(deck1, deck2, seed)` then `step()`.

This change builds the bridge: PyO3 → Rust-backed `/debug` router → revived fixture → a shared scenario fixture format → the fan-quiz corpus.

## Goals / Non-Goals

**Goals:**
- A debug-only PyO3 surface (`RustDebugGame`) that stages an arbitrary mid-game board against the real card pool and presents the same `step`/`mask`/`state`/`events` API as `RustHeadlessGame`.
- A Rust-backed `/debug` router that replaces the legacy one and serves the staging/inspection endpoints the e2e fixture needs.
- A declarative scenario fixture format consumable by both a Rust headless runner and the Playwright UI fixture from one file.
- The three orphaned specs back to green, plus the fan-quiz corpus encoded and runnable through the UI.

**Non-Goals:**
- Driving the real Tauri binary. Playwright cannot drive WebView2/WKWebView; browser-mode (same React bundle, same Rust engine over HTTP) is the tested proxy. This is stated so reviewers don't expect packaged-app coverage.
- Full card-implementation conformance. The substrate lets us *write* all 30 scenarios; whether each *passes* depends on per-card implementation, tracked separately via the blocked-on-card-impl tag.
- Scenario *export* (`to_scenario()` capture from a live game). Import-first; export is a follow-up.

## Decisions

### D1. Separate `RustDebugGame` pyclass, not debug methods on `RustHeadlessGame`
Add a distinct `#[pyclass]` wrapping `DebugRunner`. Rationale: keeps every staging/mutation method off the production game class; mirrors the engine's own `DebugRunner` vs `HeadlessRunner` split; the Python-free desktop bundle never links it. **Alternative considered**: gate debug methods on `RustHeadlessGame` behind a flag — rejected because it muddies the production surface and risks debug methods being reachable where they shouldn't be.

### D2. `RustDebugGame` loads the real card pool, not test stubs
The binding must wire `DebugRunner` to the production card DB and the embedded DSL registry the same way `RustHeadlessGame` does, so staged scenarios exercise real card behavior (Paildramon, Medusamon, etc.) rather than `make_test_card` stubs. This requires the `digimon-engine-py` crate to enable the `dsl-yaml-loader` feature. **Alternative considered**: reuse the test-card path — rejected; the quiz corpus needs real cards.

### D3. Staging split: builder-time vs post-start mutation
Hand/deck/security/egg/memory are set through `DebugRunnerBuilder` at construction. Field stacks and breeding are placed after `start()` via the existing `place_on_field`/`place_in_breeding`. Phase, turn count, and first-player have no builder hooks today; rather than expose raw `game_mut()` across the FFI boundary, add minimal invariant-preserving setters on `DebugRunner` (`set_phase`, `set_turn`, `set_first_player`) that maintain derived flags (turn_player/active_player), matching what the legacy router's manual first-player swap did. The PyO3 constructor orchestrates: build → start → place fields/breeding → apply phase/turn/first-player → set memory.

### D4. `active_games` stays duck-typed; relax the `isinstance` gate
`active_games` is already `dict[str, Any]`. Both `RustHeadlessGame` and `RustDebugGame` implement the same `step`/`get_action_mask`/`to_ui_json`/`get_events`/`current_player_id`/`is_game_over` surface, so the live `/games/{id}/...` routes operate on either. The current `_require_game` `isinstance(runner, RustHeadlessGame)` check in `games.py` must be relaxed to accept either type (a tuple isinstance or a shared structural check). This is what lets a test **stage via `/debug` then play via `/games`**.

### D5. Two-bucket assertion model — engine assertions vs UI assertions
A fixture declares assertions in two buckets:
- **engine assertions** (memory value, stack-top, effective DP, zone contents, effect-triggered, action-legal, legal-selection-options): evaluated against engine state. To avoid a Rust-evaluator-vs-TS-evaluator drift problem, these are evaluated **server-side** — the Rust-backed `/debug` router exposes an evaluate-assertions endpoint, and both the Rust headless runner and the Playwright fixture get the same verdicts from one implementation.
- **ui assertions** (a DNA button is present, legal field targets are highlighted, the selection panel shows N options): Playwright-only, evaluated against the rendered DOM.

Rationale: engine-correctness has exactly one source of truth (the engine), so it should be evaluated once; UI-wiring is inherently DOM-level and lives only in the Playwright layer. This cleanly separates "is the rule right" from "can a human reach it" — the two altitudes that the fan quiz and the recent bugs respectively probe. **Alternative considered**: parallel Rust + TS evaluators implementing the same vocabulary — rejected; guaranteed drift, double the maintenance.

### D6. Fixture location: `qa/scenarios/`
Fixtures live in a shared, language-agnostic `qa/scenarios/` tree alongside the existing archetype QA, not under `code/frontend/e2e/fixtures/`. Rationale: the corpus is cross-cutting (Rust runner + Playwright both read it); it is conformance data, not frontend-owned test config. **Alternative considered**: `code/frontend/e2e/fixtures/scenarios/` — rejected; implies frontend ownership of an engine-conformance corpus.

### D7. Action selection via `/action-mask`
The revived fixture drops the removed `GET /games/{id}/actions` and reads `GET /games/{id}/action-mask` (bare legal-id array), selecting action ranges (digivolve 400+, DNA 63–92, etc.) by id arithmetic, matching how the live UI's `useActionMask` already works.

### D8. `/games` seed + action_script + undo are the lightweight precursor — keep, don't duplicate
Earlier in this session the live `/games` router already gained three staging/replay primitives that overlap this change and are now folded in as the foundation:
- **`seed`** on `CreateGameRequest` — explicit RNG seed (else a cryptographic one is generated), stored in `GameMeta`. This is what makes any staged or replayed game reproducible; `RustDebugGame` reuses the same seed discipline.
- **`action_script`** on `CreateGameRequest` — an ordered list of human action ids replayed after opening autoplay to fast-forward into a mid-game state. It reaches a state by *replaying decisions*; `/debug` direct-staging reaches one by *injecting board state*. Both are valid scenario entry points and share the seed + history machinery. Scripted actions are **validated against the live mask before stepping** (the engine no-ops illegal actions, so unchecked scripts would silently diverge) and rejected with 400 on an illegal id — the same fail-loud discipline `RustDebugGame` construction applies.
- **`/games/{id}/undo`** — pops the last human action from `GameMeta.human_action_history` and replays the remainder from `(decks, seed)`. Already implemented and tested; the scenario substrate inherits it for stepping a staged game backward during debugging.

Decision: these stay on `/games` (they are general-purpose and already landed); the `/debug` surface adds *direct* state injection on top, not a replacement. The fixture loader chooses per scenario: `action_script` for sequences naturally reached by play, `/debug` staging for arbitrary boards (the common case for the quiz corpus). **Alternative considered**: rip out `action_script`/seed from `/games` and route everything through `/debug` — rejected; seed+undo are useful to the live play path independent of testing, and action_script is a near-free reproducible entry point that the corpus can use where it fits.

## Risks / Trade-offs

- **Staging can produce rule-illegal boards** (e.g., a phase/turn/zone combination the engine never reaches naturally) → Validate-on-stage: `RustDebugGame` construction runs the engine's consistency checks and fails loudly with a diagnostic rather than returning an undefined state (see `scenario-fixture-format` spec).
- **Phase/first-player setters poking internals desync derived state** → Encapsulate in `DebugRunner` setters that maintain invariants (turn_player/active_player/mulligan order), not raw field writes across FFI.
- **`dsl-yaml-loader` feature not enabled in the PyO3 build** → staged games would silently use stub behavior or fail to find cards. Mitigation: enable the feature in `digimon-engine-py/Cargo.toml` and add a smoke assertion that a known real card (e.g. `AD1-011`) loads with its real effect.
- **Relaxed `_require_game` lets a non-game object through** → Use an explicit `isinstance(runner, (RustHeadlessGame, RustDebugGame))` tuple check, not `Any`.
- **Blocked fixtures masking real regressions** → Pending fixtures are reported every run (not silently skipped); a fixture flips from blocked→expected-pass only by an explicit tag edit, so a newly-passing blocked scenario surfaces as actionable.
- **PyO3 rebuild friction** → The debug routes are dead until `digimon_engine` is rebuilt (`maturin`/pip). Document as the first task; the FastAPI process must restart after rebuild.

## Migration Plan

1. Rebuild path: enable `dsl-yaml-loader` in `digimon-engine-py`, add `RustDebugGame`, `maturin`/pip rebuild, restart FastAPI.
2. Add `DebugRunner` setters (`set_phase`/`set_turn`/`set_first_player`) if not already expressible.
3. Rewrite `debug_games.py` onto `RustDebugGame`; remove `engine_py_legacy` import; register before DB routers.
4. Relax `_require_game` in `games.py` to accept either runner type.
5. Add the scenario fixture schema + the server-side assertion evaluator.
6. Revive `debug-game.ts` / page objects / helpers against the new routes; green the three orphaned specs.
7. Encode the implementable fan-quiz subset under `qa/scenarios/`, Q16 first; tag each expected-pass vs blocked.

Rollback: the legacy `/debug` router is already non-functional against the Rust engine, so there is nothing to preserve; if the new router regresses, the live `/games` play path (unchanged) still works and e2e simply stays red until fixed.

## Open Questions

- Should the server-side assertion evaluator live in the `/debug` router (Python, reading `to_ui_json` + internal-state) or inside `RustDebugGame` (Rust, richer access to effective DP / event log)? Leaning Rust for fidelity on DP/event assertions, with the router as a thin pass-through — to be settled in the first implementation task.
- Minimum viable corpus size for the first cut: all implementable quiz scenarios, or a curated ~8 that exercise each mechanic category (timing, immunity, on-deletion, DNA, DigiXros, cost, tokens, security-count) plus Q16? Leaning curated-first to prove the substrate, then backfill.
