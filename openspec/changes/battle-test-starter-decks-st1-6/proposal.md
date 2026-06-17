## Why

Starter decks ST-1 … ST-6 (`st1`–`st6`, 96 cards) are all authored in the Rust DSL and were marked `AUDITED-OK` on 2026-05-29 — but with identical templated notes ("Faithful to printed text + DCGO"), which is a strong smell that the audit was shallow. Test coverage is uneven: the existing `st4-giga-green-starter-deck` spec *requires* behavioral coverage for every effect, yet `st4` has **zero** per-card behavioral test files, and other sets are sparse. Before these six lists drive RL training runs, we need real confidence that every card is faithfully implemented and that each deck plays full games without panics, soft-locks, or mask violations.

## What Changes

- **Re-audit faithfulness** of all 96 ST-1…ST-6 card YAML DSL specs against authoritative sources (card image → DCGO C# → `general_rule.pdf` → fandom), treating the templated 2026-05-29 verdicts as untrusted and re-deriving each verdict.
- **Fix** every faithfulness bug found — correcting YAML, or widening DSL vocabulary / engine primitives per CLAUDE.md rule 28 (logging to `qa/dsl-vocab-gaps.md` / `docs/RUST_ENGINE_GAPS.md`), TDD-first.
- **Fill test gaps**: add per-card DebugRunner behavioral tests for every non-vanilla effect lacking one (especially all of `st4`), and ensure each `st1`–`st6` archetype test carries the four static archetype tests (deck-legality, coverage gate, smoke games, combo-presence) plus the key multi-card interaction tests.
- **MCP battle-testing**: stage targeted scenarios with the real DSL cards via `digimon-scenario-mcp` (browser target) and play full games (mirror + cross-matchups) per deck, confirming faithful resolution, full choice exposure (no auto-selection, per the no-approximations policy), and the absence of panics / soft-locks / illegal-mask states. Durable fixtures saved under `qa/scenarios/`.
- **Training-readiness check**: verify the six lists resolve through the deck-pool / archetype wiring and that `DigimonEnv` resets and steps on each.
- Refresh the audit report (replacing the templated one) and update `qa/qa-reports/validated_cards_dsl.json` with re-derived per-card verdicts.

No new gameplay features are introduced; the change re-verifies, corrects, and hardens existing card implementations and their test coverage.

## Capabilities

### New Capabilities
- `starter-deck-battle-readiness`: Cross-cutting guarantees that the six ST-1…ST-6 starter lists are battle-tested and training-ready — every card re-verified faithful against authoritative sources, every non-vanilla effect covered by behavioral tests, each deck carrying the four static archetype tests plus interaction tests, each deck playing full games without panics / soft-locks / mask violations and exposing every player choice, and each list resolving through deck-pool/archetype wiring with a working `DigimonEnv` reset/step.

### Modified Capabilities
<!-- Card-behavior corrections discovered during the Phase 1 audit will be folded into the relevant existing per-deck coverage spec (st1-gaia-red-starter-deck-coverage, st2-cocytus-blue-coverage, st3-heavens-yellow-starter-coverage, st4-giga-green-starter-deck, st5-machine-black-starter-coverage, st6-venomous-violet-coverage) as MODIFIED deltas at fix time, since the specific corrected requirements are not known until the audit runs. No requirement changes are enumerable up front. -->

## Impact

- **Card specs**: `code/digimon-engine/cards/st{1..6}/*.yaml` (corrections only where the audit finds bugs).
- **Tests**: `code/digimon-engine/tests/cards_behavioral/st{1..6}/` (new per-card tests), `code/digimon-engine/tests/archetypes/st{1..6}.rs` (static + interaction tests), `archetype-static-tests` verdicts.
- **DSL / engine** (only if a card needs it): `code/digimon-dsl/`, `code/digimon-engine/src/`, with gaps logged to `qa/dsl-vocab-gaps.md` / `docs/RUST_ENGINE_GAPS.md`.
- **QA artifacts**: `qa/qa-reports/validated_cards_dsl.json`, a new audit report under `qa/`, fixtures under `qa/scenarios/`.
- **Tooling used (not modified)**: `digimon-scenario-mcp` (browser target) → requires `maturin develop` for `digimon-engine-py` + `uvicorn server.api:app`; `digimon-engine-mcp` for crash-recording forensics.
- **Training**: deck-pool / archetype resolution for `starter_st{1..6}_*` in `deck_library.json`; no training code changes expected beyond confirming wiring.
