## Why

The 2026-05-14 substrate audit found that ~58% of active gap tags (~430 of ~740 test-reference counts) are not engine substrate gaps — they are DSL pipeline residue of a single anti-pattern: the DSL crate parses a field, the engine has a matching `CompiledPredicate` / `CompiledFormula` / `CompiledStep` / `CompiledTiming` variant, but the corresponding evaluator/executor has no match arm for it. Authors get a silent default (`false`, `None`, `Ok(())`) instead of correct behavior; parse tests pass, only behavioral tests catch it. 74 wildcard catch-alls across 24 files in `code/digimon-engine/src/dsl_cards/` are where these silent defaults hide today. Closing them now retires ~80% of remaining gap tags at low risk; adding a coverage invariant prevents the next 150 silent gaps from accumulating.

## What Changes

- Add a **variant-coverage CI test** (`code/digimon-engine/tests/dsl_eval_arm_coverage.rs`) that asserts every `CompiledPredicate` / `CompiledFormula` / `CompiledStep` / `CompiledTiming` variant name appears textually in its evaluator/executor body. Forbids `_ => false` / `_ => None` / `_ => Ok(())` / `_ => unreachable!()` wildcards in the top-level eval functions.
- **Batch 1 — Predicate evaluator coverage.** Wire missing arms in `code/digimon-engine/src/dsl_cards/predicate.rs` (1,803 lines) for ~152 tag references: G-PRED-DP-LTE (65), G-EVENT-TARGET-OWNER (39), G-PLAY-COST-LTE (27), G-DSL-SOURCE-NAME-CONTAINS (21), G-COUNT-GTE-NOT-EVALUATED (13), plus long-tail predicates.
- **Batch 2 — Formula evaluator coverage.** Wire missing arms in `code/digimon-engine/src/dsl_cards/formula_eval.rs` (1,132 lines) for ~50 tag references: G-FORMULA-SOURCE-DP (21), G-DSL-DISTINCT-TAMER-COLORS-FORMULA (19), G-BINDING-DP-FORMULA (11), plus long-tail formulas.
- **Batch 3 — Step executor coverage.** Wire missing arms across `code/digimon-engine/src/dsl_cards/step/*.rs` (24 step modules) and `lower_*.rs` bridges for ~241 tag references: G-DECLARATIVE-KEYWORD (81), G-PLACE-SELF-AS-OPTION-PERMANENT (38), G-ALT-PATH-CONDITION (37), G-IGNORE-COLOR-MASK (23), G-DSL-UNION-PLAY-FREE (17), G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM (15), G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH (15), G-COST-REDUCE-ALLY-DIGIVOLVE (15), plus tail.
- **Batch 4 — Timing-map and modifier-map coverage.** Wire `None`-returning branches in `code/digimon-engine/src/dsl_cards/timing_map.rs` (60 lines) and `modifier_map.rs` (396 lines) for the long-tail timing/modifier gaps.
- Add `#[derive(strum::EnumIter)]` to `Compiled*` enums in `digimon-dsl` so the coverage test can iterate variants programmatically. Adds `strum` and `strum_macros` as dev-dependencies on the engine crate.
- **Remove wildcards.** Each batch's PR removes the wildcard arm from every touched evaluator, letting the Rust compiler enforce exhaustiveness going forward.
- **Tracker hygiene.** Each batch moves its closed gap-tag entries from `qa/dsl-vocab-gaps.md` / `qa/archetype-qa/engine-gaps.md` to `qa/resolved-gaps.md` under a "Phase 1 Batch N closure — 2026-05-XX" header.
- **Unignore tests.** Every `#[ignore = "BLOCKED: G-…"]` whose tag is closed by a batch gets its annotation removed; tests must pass without body changes.

## Capabilities

### New Capabilities

- `dsl-eval-arm-coverage`: The variant-coverage substrate invariant. Owns the lint test, the requirement that every `Compiled*` enum variant be covered by an explicit match arm in its evaluator/executor, the rule against wildcard catch-alls in top-level eval functions, and the four batched eval-arm completions (predicate / formula / step / timing+modifier) that bring the existing evaluator surfaces into compliance.

### Modified Capabilities

<!-- None — no pre-existing OpenSpec specs in this repo (OpenSpec was initialized 2026-05-15). The DSL eval surfaces are existing engine code but not yet captured as OpenSpec capabilities. -->

## Impact

- **Affected code:**
  - `code/digimon-engine/src/dsl_cards/predicate.rs` (1,803 lines)
  - `code/digimon-engine/src/dsl_cards/formula_eval.rs` (1,132 lines)
  - `code/digimon-engine/src/dsl_cards/step/*.rs` (24 modules)
  - `code/digimon-engine/src/dsl_cards/lower_*.rs` (lowering bridges)
  - `code/digimon-engine/src/dsl_cards/timing_map.rs` (60 lines)
  - `code/digimon-engine/src/dsl_cards/modifier_map.rs` (396 lines)
  - `code/digimon-engine/src/dsl_cards/bindings.rs` (11 wildcards)
  - `code/digimon-dsl/src/compiled.rs` (add `EnumIter` derive)
  - `code/digimon-engine/tests/dsl_eval_arm_coverage.rs` (NEW)
- **Affected tests:** ~445 currently-`#[ignore]`'d tests across `code/digimon-engine/tests/cards_behavioral/` and `code/digimon-engine/tests/dsl/` should pass without body changes once their tag is closed; baseline ignore count drops from 595 to ~150.
- **Affected trackers:**
  - `qa/dsl-vocab-gaps.md` — most entries relocate to resolved.
  - `qa/archetype-qa/engine-gaps.md` — cross-reference updates.
  - `qa/resolved-gaps.md` — gains four "Phase 1 Batch N closure" rollup sections.
  - `docs/RUST_ENGINE_GAPS.md` — sweep note added; severity badges may adjust.
- **New dependencies:** `strum = "0.26"` and `strum_macros = "0.26"` (or current versions) as dev-dependencies on `code/digimon-engine` (test-only consumer) and `code/digimon-dsl` (derive consumer). No runtime impact.
- **No contract changes:** `ACTION_SPACE_SIZE`, observation tensor shape, PyO3 exports, frontend constants, and RL wrapper assumptions are not touched. The Python parity test must still pass under `DIGIMON_BACKEND=rust`.
- **No card-author work in scope:** Card YAML migration and the `raw_rust:` retirement are Phase 3 work, not Phase 1.
- **No substrate edge fixes in scope:** G-OPT-TRIGGERED (139 refs), G-INHERITED-DISPATCH residue (107), G-OPT-RESET-VIA-ATTACK-CYCLE, and the 12 truly-OPEN substrate items from the audit are Phase 2 work.
- **Parent context:** Phase 1 of the four-phase plan in `docs/superpowers/specs/2026-05-14-substrate-reality-check-plan.md`. Detailed batch breakdown lives in `docs/superpowers/plans/2026-05-15-phase-1-dsl-pipeline-completion.md`.
