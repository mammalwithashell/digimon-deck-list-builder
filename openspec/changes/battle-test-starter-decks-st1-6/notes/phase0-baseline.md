# Phase 0 — Baseline (2026-06-14)

## Environment
- Worktree root confirmed: `.claude/worktrees/great-curie-2aefdb` (NOT base repo).
- `$BASE_DCGO` = `C:/Users/james/Documents/digimon-deck-list-builder-1/DCGO` — populated (read-only).
- `CARGO_TARGET_DIR` = `D:\cargo-target/great-curie-2aefdb` — isolated per-worktree (rule 31 active; post-restart session).
- Engine test binaries build clean (4m24s, warnings only).
- MCP toolchain viable: maturin 1.13.1, `digimon_engine` importable, uvicorn 0.34.0 + fastapi, `digimon_scenario_mcp` installed, `/debug` router = `code/server/routers/debug_games.py`.
  - **Deferred to Phase 4**: `maturin develop` rebuild + `uvicorn` start + `stage_scenario` round-trip — done after Phase 2 fixes so the browser surface reflects corrected cards.

## Test baseline (st1–st6 only)
- `cards_behavioral` (filtered st1::…st6::): **130 passed, 0 failed, 1 ignored**.
- `archetypes` (filtered st1::…st6::): **39 passed, 0 failed**.
- Total: **169 green**, 1 ignored.

### Ignored test
- `code/digimon-engine/tests/cards_behavioral/st1/st1_07.rs:251` — `#[ignore]` for engine gap **G-DECLARATIVE-KEYWORD** (`EffectTiming::Declarative` not fired by engine): ST1-07 inherited `SecurityAttackPlus(1)` runtime installation.
  - NOTE: ST-1 WarGreymon `<Security A.>` formula issues were later RESOLVED (2026-05-30) via "tick-fresh strike" (`archetype_interactions.json` ST-1 findings). Audit must determine whether this ignore is now superseded or still a real gap.

## Coverage catalog (correcting the pre-flight assumption)
- **st4 is NOT zero-coverage.** Its ~23 behavioral tests live inline in `tests/cards_behavioral/st4/mod.rs` (incl. mandatory-selection faithfulness checks), not in separate `st4_NN.rs` files. The earlier "st4 = 0 tests" read was a file-listing artifact.
- Per-card behavioral test files by set (plus inline mod.rs tests):
  - st1: gaia_red.rs, st1_07.rs, st1_15.rs, wargreymon_security_attack.rs
  - st2: st2_13.rs, st2_cards.rs
  - st3: st3_starter.rs
  - st4: mod.rs (inline, substantial)
  - st5: st5_04, st5_06, st5_14, st5_effects, st5_static, st5_options
  - st6: st6_cards.rs
- Archetype interaction test files exist: `tests/archetypes/st{1..6}.rs` (6–10 fns each, 39 tests total).
- `archetype_interactions.json` has entries for all 6 decks with the **four static tests recorded passing** (verified ST-1: coverage 16/16, deck-legal 4 egg/50 main, 10/10 smoke, combo_presence) and combos_tested PASS. Model docs exist: `qa/archetype-qa/st{1,2,3,6}-*-model.md` (confirm st4/st5).

## Data-quality issues to fix at finalize
- `archetype_interactions.json` `interaction_test_file` paths look **stale**: e.g. `st1_gaia_red.rs`, `st2_cocytus_blue.rs`, `st3_heavens_yellow.rs`, `st6_venomous_violet.rs` — actual files are `st{1..6}.rs`. Verify and correct.
- `validated_cards_dsl.json` st1–6 notes are the identical templated string (the reason for this change) — to be overwritten with re-derived per-card verdicts at finalize.

## Conclusion
Baseline is GREEN and coverage is materially better than the pre-flight read. The high-value work is the Phase 1 deep faithfulness re-audit (does each YAML match printed text), then targeted gap-filling + MCP battle-testing.
