## 0. Already landed this session (verify, don't rebuild)

- [x] 0.1 `seed` on `CreateGameRequest` + `GameMeta`; `create_game` honors an explicit seed or generates a cryptographic one and stores it.
- [x] 0.2 `action_script` on `CreateGameRequest`; `create_game` replays scripted human actions after opening autoplay, validating each against the live mask before stepping and rejecting illegal ids with 400.
- [x] 0.3 `/games/{id}/undo` endpoint: pops last human action from `human_action_history`, rebuilds from `(decks, seed)`, replays the remainder.
- [x] 0.4 `gameApi.undoGame` HTTP fallback + the in-game `← Undo` debug button.
- [x] 0.5 Reconcile: keep these on `/games` per design D8; ensure the `/debug` work (groups 2–4) does not duplicate seed/undo and that `RustDebugGame` reuses the same seed discipline.
- [x] 0.6 Backfill a regression test for the `action_script` mask-validation (legal script advances; illegal id → 400) so the fold-in is covered.

## 1. Rust engine staging setters

- [x] 1.1 Audit `DebugRunner` for missing staging hooks: phase, turn count, first-player. Confirm hand/deck/security/egg/memory builder hooks and `place_on_field`/`place_in_breeding` cover the rest.
- [x] 1.2 Add invariant-preserving `set_phase(GamePhase)` to `DebugRunner` (updates derived phase state without leaving the turn machine inconsistent).
- [x] 1.3 Add `set_turn(count)` and `set_first_player(PlayerId)` to `DebugRunner`, maintaining `turn_player`/`active_player`/mulligan-order invariants (mirror the legacy first-player swap logic).
- [x] 1.4 Add a trash-injection helper if not already expressible (place a card directly into a player's trash).
- [x] 1.5 Add a `validate()` / consistency check that `RustDebugGame` construction can call to reject rule-illegal staged boards with a diagnostic.
- [x] 1.6 Unit-test the new setters in `code/digimon-engine/tests/` (phase set, first-player swap, trash inject, validate-rejects-bad-board).

## 2. PyO3 `RustDebugGame` binding

- [x] 2.1 Enable the `dsl-yaml-loader` feature for `digimon-engine` in `code/digimon-engine-py/Cargo.toml`. (Already enabled.)
- [x] 2.2 Add a `RustDebugGame` `#[pyclass]` in `code/digimon-engine-py/src/lib.rs`. Wraps `HeadlessRunner` (real card DB + embedded DSL registry + full RL surface) rather than `DebugRunner` — staging delegates to new `Game::stage_*` methods so there is one implementation.
- [x] 2.3 Staging via mutators the router orchestrates (build → skip_mulligan → clear/place zones → memory/phase/turn/first-player → validate). Field stacks carry suspended/turn_played; breeding/hand/deck/security/trash supported.
- [x] 2.4 Mutation methods: `clear_zone`, `inject_card`, `place_on_field`, `place_in_breeding`, `set_memory`, `set_phase`, `set_turn`, `set_first_player`, `skip_mulligan`, `validate`.
- [x] 2.5 Mirror the `RustHeadlessGame` play surface: `step`, `get_action_mask`, `greedy_action`, `to_ui_json`, `get_events_since_last_step`, `get_last_log`, `current_player_id`, `is_game_over`, `get_pending_selection`, `concede`.
- [x] 2.6 Implement `internal_state()` returning structured per-player zones + scalar state for test assertions.
- [x] 2.7 Register `RustDebugGame` in the module init; rebuilt bindings and smoke-checked AD1-011 (Paildramon) staged over BT12-022/BT12-050 with real card data.

## 3. Server-side assertion evaluator

- [x] 3.1 Evaluator home: Python in the `/debug` router as a thin pass-through over the binding's introspection (`internal_state` + `to_ui_json` + mask + pending-selection + accumulated events). One source of truth for both layers.
- [x] 3.2 Implemented the engine-assertion vocabulary: memory_equals, stack_top, effective_dp, zone_count, zone_contains, effect_triggered (event scan), action_legal (mask), legal_selection_options.
- [x] 3.3 Each assertion returns `{kind, passed, message}` with expected-vs-actual detail.
- [x] 3.4 Unit-tested the evaluator (`code/tests/api/test_debug_scenario_evaluator.py`): one assertion of each kind through `/debug/.../evaluate`, both pass and fail paths, plus the `all_passed` flag. 2 passed.

## 4. Rust-backed `/debug` HTTP router

- [x] 4.1 Rewrote `code/server/routers/debug_games.py` onto `RustDebugGame`; removed all `engine_py_legacy` imports.
- [x] 4.2 `POST /debug/games` (create-with-staging) — legacy fields + rich `zones`/`phase`/`turn`; registers in `active_games` + a `GameMeta` so staged games play via `/games`.
- [x] 4.3 `POST /debug/games/{id}/set-memory`, `/inject-card`, `/place-on-field`, `/bulk-setup` (+ `/step` for debug-driven event accumulation).
- [x] 4.4 `GET /debug/games/{id}/internal-state`.
- [x] 4.5 `POST /debug/games/{id}/evaluate` — per-assertion verdicts.
- [x] 4.6 Added debug request schemas (`DebugZoneSpec`, `DebugFieldStack`, `DebugInjectCardRequest`, `DebugPlaceOnFieldRequest`, `DebugBulkSetupRequest`, `DebugAssertion`, `DebugEvaluateRequest`) with forward-ref rebuilds.
- [x] 4.7 Registered the debug router with the engine routers (before DB routers); dropped the `DEBUG_MODE` gate — `/debug` prefix, in-memory only.
- [x] 4.8 Relaxed `_require_game` to accept `(RustHeadlessGame, RustDebugGame)`.
- [x] 4.9 Round-trip verified: stage via `/debug` → evaluate engine assertions → play via `POST /games/{id}/actions` (200) → delete cleans up.

## 5. Scenario fixture format

- [x] 5.1 Defined the JSON schema (schema_version, decks, zones, scalar state, optional action_script, engine + ui assertion lists, readiness tag) — see `qa/scenarios/README.md`.
- [x] 5.2 Wrote the schema doc with the full assertion vocabulary under `qa/scenarios/README.md`.
- [x] 5.3 HTTP fixture loader proven: a fixture maps to a `/debug/games` create body; `_apply_zone` clears-then-populates each zone; unknown card ids / illegal boards fail loud via `stage_inject_card` / `validate()`. (The Rust-side loader is group 6; the Playwright loader is 7.2.)
- [x] 5.4 Created `qa/scenarios/` with the schema doc and the first fixture (`q16-paildramon-staging.json`).

## 6. Rust headless fixture runner

- [x] 6.1 Added `code/digimon-engine/tests/scenario_corpus.rs` — loads `data/cards.json` (real pool), reads `qa/scenarios/*.json`, stages a `DebugRunner`, evaluates engine assertions (memory/stack_top/effective_dp/zone_count/action_legal).
- [x] 6.2 Readiness tagging: `blocked_on_card_impl` → printed PENDING (never fails); `expected_pass` → assertions must pass.
- [x] 6.3 `cargo test --test scenario_corpus` green — Q16 fixture PASS in the Rust layer (same fixture also passes via the HTTP/server layer).

## 7. Playwright fixture revival

- [x] 7.1 Rewrote `code/frontend/e2e/fixtures/debug-game.ts` against the Rust-backed `/debug` routes; `getActions()` now derives legal ids from `GET /games/{id}/action-mask`; added `placeOnField`/`evaluate`/`delete` + normalized `internal-state` (`hand_p1`/`hand_p2`).
- [x] 7.2 Added `loadScenario()` + `stageScenario()` — read a `qa/scenarios/*.json` fixture and stage it via `/debug/games`. (Design D8's `/games` seed+action_script entry point remains available for replay-reached states.)
- [x] 7.3 Engine-assertion vs ui-assertion split: engine assertions evaluated server-side via `/debug/.../evaluate` (exposed on the fixture handle); DOM ui-assertions belong to the navigating game-flow specs.
- [~] 7.4 New `scenario-conformance.spec.ts` (revived e2e against the substrate) is GREEN. The 3 legacy DOM specs (`digivolution`/`timing-regression`/`memory-accounting`) are de-orphaned (fixture no longer hits dead routes; `digivolution` mask usage fixed) but their full green run needs the Vite frontend + a seeded `mammal` DB user — environmental, not substrate. Documented.
- [x] 7.5 Full-stack wiring confirmed: Rust engine built, PyO3 rebuilt, FastAPI up, `npx playwright test scenario-conformance` GREEN (2 tests) — same fixtures the Rust `cargo test --test scenario_corpus` runs.

## 8. Fan-quiz seed corpus

- [x] 8.1 Implementability audit written: `qa/scenarios/CORPUS.md` maps all 30 quiz scenarios to mechanic categories + readiness (staging-proof = expected_pass; resolution-dependent = blocked_on_card_impl).
- [x] 8.2 Q16 board encoded (`q16-paildramon-staging.json`) AND the live DNA flow shipped: `dna-paildramon-hand.json` + `code/frontend/e2e/dna-digivolve.spec.ts` drive the rendered browser — click Paildramon → assert the **DNA Digivolve** option appears (the original bug) → click it → assert the material-selection prompt. Green in both layers (Rust corpus + Playwright DOM).

## 10. DNA digivolve root-cause fix (surfaced by the e2e)

The "forced into regular digivolve" bug was an engine/data gap, not UI:

- [x] 10.1 Diagnosed: DNA digivolve requirements live in the free-text `xros_req` field and are never parsed into the structured `dna_costs` the engine matches — so `dna_costs` is empty across all 4085 cards and DNA digivolve is unoffered engine-wide.
- [x] 10.2 Fixed the latent color-deserialization bug: `DnaRequirement.card_colors` (`deserialize_card_colors`) only accepted color-name strings, mismatching cards.json's int convention. Now accepts ints (via `parse_card_color`) and strings. `cargo test --lib` 168 green.
- [x] 10.3 Populated Paildramon's DNA cost faithfully from `xros_req` ("Blue Lv.4 + Green Lv.4 : Cost 0") in `card_overrides.json` + a surgical `data/cards.json` insert (7-line diff). Engine now offers DNA action 63 alongside regular digivolve 400 with ExVeemon (Blue Lv.4) + Stingmon (Green Lv.4) on field.
- [x] 10.4 SYSTEMIC FIX: `ingest_cards.py` already had the `xros_req → dna_costs`/`digixros_costs` parser (`--backfill` mode / `_parse_xros_costs`) — it had simply never been run against the shipped `cards.json` (stale data). Ran `python code/tools/ingest_cards.py --backfill`: **62 cards gained `dna_costs`, 60 gained `digixros_costs`** (+ 4 pending overrides applied). The parser faithfully distinguishes color-based reqs (Paildramon: Blue Lv.4 + Green Lv.4) from name-based ones (Omnimon: [Greymon] + [Garurumon]) — the misinterpretation the field is prone to. Removed the now-redundant AD1-011 hand-patch from `card_overrides.json` + `cards.json` (the parser supersedes it). Verified all 62 `dna_costs` deserialize in the engine; DNA digivolve now offered engine-wide.
- [~] 8.3 Encoded the staging-proof fixture (Q16 board, expected_pass) + a blocked exemplar (`q16-partition-self-deletion`, the full rules outcome). The remaining categories are resolution-dependent (blocked_on_card_impl) and catalogued in `CORPUS.md`; each becomes a fixture when its cards are implemented.
- [x] 8.4 Ran the corpus through both layers: Rust runner (1 pass, 1 pending) and Playwright (2 passed — expected_pass asserts, blocked is pending). Verdicts agree across layers.
- [~] 8.5 Remaining quiz scenarios documented as blocked in `CORPUS.md` with per-category card dependencies; they are added as fixtures as cards land (flip the readiness tag).

## 9. Docs & verification

- [x] 9.1 Wrote `docs/SCENARIO_TESTING.md` (authoring + both runners) and linked it from `docs/INDEX.md`.
- [x] 9.2 Added the `RustDebugGame` test/browser-only note to `docs/RUST_PYTHON_PARITY.md`.
- [x] 9.3 Verified: desktop links only the `digimon-engine` crate (not the `digimon-engine-py` PyO3 binding); `RustDebugGame` pyclass absent from the bundle; no debug staging `invoke` command registered.
- [x] 9.4 Final pass — all green:
  - `cargo test -p digimon-engine --lib` → 168 passed (no regression from the additive `Game::stage_*` methods).
  - `cargo test --test debug_staging_setters` → 7 passed.
  - `cargo test --test scenario_corpus` → 1 expected-pass, 1 pending.
  - `pytest` (action_script + rust bindings surface + formats) → 57 passed.
  - `npx playwright test scenario-conformance` → 2 passed (expected_pass asserts, blocked pending).
  - Both layers agree on the same fixtures.
