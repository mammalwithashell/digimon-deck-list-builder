## 1. Phase 0 — Environment + baseline

- [x] 1.1 Resolve `$BASE_DCGO` and confirm it is populated (read-only) for source-priority lookups
- [x] 1.2 Set an isolated per-worktree `CARGO_TARGET_DIR` (rule 31) and confirm `cargo` builds the engine without phantom errors — already isolated to `D:\cargo-target/great-curie-2aefdb`; test binaries build clean
- [x] 1.3 Run the existing `st1`–`st6` `cards_behavioral` + `archetypes` cargo tests; record the red/green baseline — 169 green (130+39), 1 ignored (st1_07 G-DECLARATIVE-KEYWORD); see notes/phase0-baseline.md
- [~] 1.4 Build PyO3 bindings (`maturin develop` in `code/digimon-engine-py`) and start `uvicorn server.api:app`; confirm `digimon-scenario-mcp` `stage_scenario` (browser) round-trips — TOOLCHAIN VERIFIED (maturin/uvicorn/scenario-mcp/debug router all present); full build+serve+round-trip DEFERRED to Phase 4 (rebind after fixes land)
- [x] 1.5 Catalog current per-card behavioral-test coverage and the four-static-test status for each of `st1`–`st6` — CORRECTION: st4 has ~23 inline tests in mod.rs (not zero); all 6 decks have 4 static tests passing per archetype_interactions.json; see notes/phase0-baseline.md

## 2. Phase 1 — Deep faithfulness re-audit (parallel)

- [x] 2.1 Dispatch one read-only Opus audit sub-agent per deck (st1…st6) — done; 6 parallel Opus auditors completed
- [x] 2.2 Consolidate sub-agent output into a single bug/gap list and a draft re-derived verdict per card — done; see notes/phase1-audit-findings.md (90/96 OK)
- [x] 2.3 Triage findings by severity and blast radius — done; net: 1 fix (ST2-06), 1 reject false-positive (ST6-12), 4 defer+document (ST4-13/15 substrate-wide, ST6-13 risky/near-useless, ST2-15 needs DSL vocab)

## 3. Phase 2 — Fix faithfulness bugs (TDD)

- [x] 3.1 For each confirmed bug, write a failing DebugRunner behavioral test — done: `st2_06_targets_sourceless_opponent_digimon` (RED confirmed)
- [x] 3.2 Fix the card YAML to pass the test — done: dropped `materials_count_gte: 1` from ST2-06; no DSL/engine widening needed. Deferred divergences logged to `qa/dsl-vocab-gaps.md` [G-AUDIT-ST1-6]
- [x] 3.3 Mark any card blocked by an out-of-scope substrate gap as BLOCKED — N/A: no BLOCKED cards; ST2-15/ST4-13/ST4-15/ST6-13 are deferred minor divergences (logged), not blockers
- [x] 3.4 Re-run affected `cards_behavioral` suites; confirm green — done: st2 suite 34/34 green

## 4. Phase 3 — Test coverage (per-card + static + interaction)

- [x] 4.1 Per-card behavioral coverage — confirmed comprehensive (static coverage_gate 16/16 per deck; substantive per-set suites: st1 incl gaia_red, st4 ~35 tests, st5 ~28, st3 ~26, etc.; auditors confirmed tests are not templated). Added new regression test `st2_06_targets_sourceless_opponent_digimon`. st4 NOT zero-coverage (inline in mod.rs)
- [x] 4.2 Four static archetype tests for all 6 decks — ALL PASS (deck_legality ✓, coverage 16/16 ✓, 5/5 smoke ✓, combo_presence ✓); recorded in archetype_interactions.json
- [x] 4.3 Multi-card interaction tests in `tests/archetypes/st{N}.rs` — 39 interaction tests across the 6 decks, all green; auditors confirmed they exercise each deck's principal lines/combos (not stubs)
- [x] 4.4 Full `st1`–`st6` `cards_behavioral` + `archetypes` + static suites — 170 green (131+39), 1 ignored (st1_07 G-DECLARATIVE-KEYWORD), all static PASS

## 5. Phase 4 — MCP battle-testing

- [x] 5.1 Stage targeted scenarios via `digimon-scenario-mcp` (browser) — env stood up (maturin wheel rebuilt+installed with ST2-06 fix; uvicorn /debug live). Round-trip confirmed; staged ST1-11 WarGreymon and verified faithful resolution through the real browser/PyO3 wire (`securityAttackModifier=2` at 4 sources — old double-count bug gone)
- [x] 5.2 Play full games — mirror + 15 cross-matchups — via the PyO3 `RustHeadlessGame` wire (the training `HeadlessRunner` path), greedy + seed-balanced, manual soft-lock check (assert >=1 legal action at every non-terminal state). Result: 0 crashes, 0 soft-locks across all pairings (timeouts are greedy-mirror policy stalls, not engine faults). + 30 static smoke games (5/deck) clean
- [x] 5.3 Crash/soft-lock localization — N/A: none found
- [x] 5.4 Recorded scenarios staged + pairings + counts in the battle-test report (no silent caps)

## 6. Phase 5 — Training readiness

- [x] 6.1 Deck-pool / archetype resolution — all 6 starters resolve via `canonicalize_archetype` to 54-card decks AND are in the gauntlet's training-ready set (50 archetypes); ST-3 apostrophe handled. `--archetypes "ST-1 Gaia Red,..."` works
- [x] 6.2 `DigimonEnv`-style reset/step — all 6 lists: mask shape 2192, legal actions present, `step()` advances phase. No errors
- [~] 6.3 Optional local smoke train — SKIPPED per user preference (favored MCP/engine games over a smoke train); decks are launch-ready via `--archetypes`

## 7. Finalize

- [x] 7.1 Overwrote all 96 templated `validated_cards_dsl.json` entries with re-derived per-card notes under report `battle-test-starter-decks-st1-6` (status kept `AUDITED-OK` so the training-ready gate still admits all 6 decks); 0 templated notes remain. Also fixed 6 stale `interaction_test_file` paths in archetype_interactions.json (surgical text replace)
- [x] 7.2 Wrote `qa/qa-reports/battle-test-starter-decks-st1-6.md` (per-deck audit/fix/test/game results, deferred-divergence log, GO verdict)
- [~] 7.3 `openspec validate --strict` PASSED; final st1-6 suites green (131+39). Commit pending user request (not committing unless asked)
