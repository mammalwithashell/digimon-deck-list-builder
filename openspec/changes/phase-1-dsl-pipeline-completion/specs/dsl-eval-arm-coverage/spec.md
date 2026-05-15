## ADDED Requirements

### Requirement: Exhaustive match on `CompiledPredicate` in evaluator

The `eval_predicate` function and any sibling top-level predicate-evaluation function in `code/digimon-engine/src/dsl_cards/predicate.rs` SHALL match every `CompiledPredicate` variant with an explicit arm. The match MUST NOT include a wildcard catch-all (`_ => …`).

#### Scenario: Adding a new CompiledPredicate variant forces compilation error

- **WHEN** a developer adds a new variant to `CompiledPredicate` in `code/digimon-dsl/src/compiled.rs` without updating `eval_predicate`
- **THEN** `cargo build --manifest-path code/digimon-engine/Cargo.toml` fails with a non-exhaustive-match error at the line of the `match` over `CompiledPredicate`

#### Scenario: Existing CompiledPredicate variants all evaluate without a silent default

- **WHEN** the test suite runs `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage --test-name predicate_variants_have_eval_arms`
- **THEN** every variant name returned by `<CompiledPredicate as IntoEnumIterator>::iter()` is found textually in the source of `code/digimon-engine/src/dsl_cards/predicate.rs` and the test passes

### Requirement: Exhaustive match on `CompiledFormula` in evaluator

The `eval_formula` function and any sibling top-level formula-evaluation function in `code/digimon-engine/src/dsl_cards/formula_eval.rs` SHALL match every `CompiledFormula` variant with an explicit arm. The match MUST NOT include a wildcard catch-all.

#### Scenario: Adding a new CompiledFormula variant forces compilation error

- **WHEN** a developer adds a new variant to `CompiledFormula` without updating `eval_formula`
- **THEN** `cargo build` fails with a non-exhaustive-match error

#### Scenario: All existing CompiledFormula variants resolve to an explicit eval arm

- **WHEN** the lint test runs the formula-coverage assertion
- **THEN** every variant name from `<CompiledFormula as IntoEnumIterator>::iter()` is found textually in `formula_eval.rs` source

### Requirement: Exhaustive match on `CompiledStep` in executors

Every step-executor function in `code/digimon-engine/src/dsl_cards/step/*.rs` that matches over `CompiledStep` SHALL match every variant the function dispatches with an explicit arm. The top-level step-dispatch function MUST NOT include a wildcard catch-all over `CompiledStep`.

#### Scenario: Adding a new CompiledStep variant forces compilation error in dispatcher

- **WHEN** a developer adds a new variant to `CompiledStep` without updating the step dispatcher
- **THEN** `cargo build` fails with a non-exhaustive-match error at the dispatcher's `match`

#### Scenario: All CompiledStep variants have at least one match arm somewhere in `dsl_cards/step/`

- **WHEN** the lint test runs the step-coverage assertion
- **THEN** every variant name from `<CompiledStep as IntoEnumIterator>::iter()` is found textually in the concatenated source of all files under `code/digimon-engine/src/dsl_cards/step/`

### Requirement: Exhaustive match on `CompiledTiming` in timing map

The `compiled_timing_to_engine` function in `code/digimon-engine/src/dsl_cards/timing_map.rs` SHALL return `Some(EffectTiming::…)` for every `CompiledTiming` variant whose corresponding `EffectTiming` exists in `code/digimon-engine/src/enums.rs`. The function MAY return `None` only for `CompiledTiming` variants whose corresponding engine timing intentionally does not exist yet, and each such case MUST include a comment explaining why.

#### Scenario: All CompiledTiming variants with a corresponding EffectTiming map non-None

- **WHEN** the lint test enumerates `<CompiledTiming as IntoEnumIterator>::iter()` and checks each variant's mapping
- **THEN** every variant that has a matching engine-side `EffectTiming` returns `Some(…)` from `compiled_timing_to_engine`

#### Scenario: Adding a new CompiledTiming variant forces compilation error

- **WHEN** a developer adds a new variant to `CompiledTiming` without updating `compiled_timing_to_engine`
- **THEN** `cargo build` fails with a non-exhaustive-match error

### Requirement: Variant-coverage lint test exists and runs in CI

A test file `code/digimon-engine/tests/dsl_eval_arm_coverage.rs` SHALL exist and contain at minimum these `#[test]` functions:

- `predicate_variants_have_eval_arms`
- `formula_variants_have_eval_arms`
- `step_variants_have_exec_arms`
- `timing_variants_have_map_arms`
- `no_wildcard_catchalls_in_eval_predicate`
- `no_wildcard_catchalls_in_eval_formula`
- `no_wildcard_catchalls_in_step_dispatcher`

The test file SHALL be discoverable by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage` and SHALL run on every CI invocation that runs the engine test suite.

#### Scenario: Lint test is part of the default test run

- **WHEN** a contributor runs `cargo test --manifest-path code/digimon-engine/Cargo.toml`
- **THEN** the `dsl_eval_arm_coverage` test binary executes and all its `#[test]` functions pass

#### Scenario: Removing the lint test file fails CI

- **WHEN** the file `code/digimon-engine/tests/dsl_eval_arm_coverage.rs` is deleted or renamed
- **THEN** the CI pipeline fails because the variant-coverage invariant is no longer enforced (enforced by a meta-check or by the absence of the binary causing a downstream guard to trip)

### Requirement: No wildcard catch-alls in top-level eval functions

The top-level functions `eval_predicate` (in `predicate.rs`), `eval_formula` (in `formula_eval.rs`), the step dispatcher entry function (in `step/mod.rs` or equivalent), and `compiled_timing_to_engine` (in `timing_map.rs`) SHALL NOT contain wildcard match arms of the form `_ => false`, `_ => None`, `_ => Ok(())`, `_ => unreachable!()`, or any equivalent. Inner helper functions MAY use narrow defaults when those defaults are documented with a comment explaining the intent.

#### Scenario: Lint test rejects forbidden wildcards in eval bodies

- **WHEN** a top-level eval function in scope contains `_ => false`, `_ => None`, `_ => Ok(())`, or `_ => unreachable!()`
- **THEN** the corresponding `no_wildcard_catchalls_in_*` test fails with a descriptive message naming the file and the offending wildcard

#### Scenario: Inner helper with documented narrow default passes the lint

- **WHEN** an inner helper function uses a wildcard with an inline `// Intentional narrow default because …` comment
- **THEN** the lint test does not flag the helper (it scopes to top-level eval functions only)

### Requirement: `EnumIter` derive on `Compiled*` enums

The enums `CompiledPredicate`, `CompiledFormula`, `CompiledStep`, and `CompiledTiming` in `code/digimon-dsl/src/compiled.rs` SHALL derive `strum_macros::EnumIter` so the lint test can iterate variants programmatically.

#### Scenario: Variant iteration works

- **WHEN** the lint test calls `<CompiledPredicate as strum::IntoEnumIterator>::iter()`
- **THEN** it yields every variant currently defined in `CompiledPredicate` without requiring a hardcoded variant list

### Requirement: Closed gap-tag entries relocate to `qa/resolved-gaps.md` per batch

Each batch's PR SHALL move closed gap-tag entries from `qa/dsl-vocab-gaps.md` and `qa/archetype-qa/engine-gaps.md` to `qa/resolved-gaps.md` under a header `## Phase 1 Batch N closure — 2026-05-XX (PR #YYY)`. Each relocated entry MUST preserve its body verbatim and append an "Audit closure note (2026-05-XX)" paragraph citing the PR.

#### Scenario: Batch 1 closure rollup exists in resolved-gaps

- **WHEN** Batch 1 lands and includes G-PRED-DP-LTE in its closed-tag list
- **THEN** `qa/resolved-gaps.md` contains a `## Phase 1 Batch 1 closure — 2026-05-XX (PR #YYY)` header followed by the G-PRED-DP-LTE entry text, and `qa/dsl-vocab-gaps.md` no longer contains an active G-PRED-DP-LTE entry

#### Scenario: Tag is referenced by an ignored test before the batch lands

- **WHEN** a `#[ignore = "BLOCKED: G-PRED-DP-LTE …"]` annotation exists on a test before Batch 1
- **THEN** after Batch 1 lands, the annotation is removed and the test passes without body changes

### Requirement: Ignored-test count drops to ~150 by end of Phase 1

After all four batches land, the total count of `#[ignore]` annotations across `code/digimon-engine/tests/` SHALL be approximately 150 ± 20. Remaining `#[ignore]` annotations MUST reference either Phase 2 substrate edges (G-OPT-TRIGGERED, G-INHERITED-DISPATCH, the 12 named OPEN substrate items) or be tagged as `card-local authoring` follow-ups.

#### Scenario: End-of-phase ignore-count verification

- **WHEN** all four batches have landed and `git grep -c '#\[ignore' code/digimon-engine/tests | awk -F: '{s+=$2} END {print s}'` is run
- **THEN** the total is between 130 and 170

#### Scenario: Every remaining ignored test cites a Phase 2 or authoring tag

- **WHEN** a reviewer inspects each remaining `#[ignore]` annotation
- **THEN** every annotation's `BLOCKED:` or `pending:` reason refers to a tag listed in the Phase 2 backlog or marked as `card-local authoring`
