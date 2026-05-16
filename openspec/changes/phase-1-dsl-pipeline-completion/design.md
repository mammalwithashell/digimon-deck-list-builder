## Context

The Rust DSL pipeline (`code/digimon-dsl/` parsing, `code/digimon-engine/src/dsl_cards/` lowering + evaluation) currently has a silent-default anti-pattern:

1. `digimon-dsl/src/predicate.rs` exposes ~105 `pub` predicate fields.
2. `digimon-dsl/src/compiled.rs` defines matching `CompiledPredicate` / `CompiledFormula` / `CompiledStep` / `CompiledTiming` variants.
3. `code/digimon-engine/src/dsl_cards/predicate.rs` (1,803 lines), `formula_eval.rs` (1,132 lines), `step/*.rs` (24 modules), `timing_map.rs` (60 lines), and `modifier_map.rs` (396 lines) consume those variants — but ~150 of the variants have no explicit match arm. They fall through a `_ => false` / `_ => None` / `_ => Ok(())` wildcard and return a silent default.

Grep on 2026-05-15 found 74 such wildcards across 24 files inside `code/digimon-engine/src/dsl_cards/`. Behavioral tests have been catching individual cases for months — 595 `#[ignore]` annotations across 164 test files reference 216 distinct gap tags, and the 2026-05-14 audit confirmed ~58% of those tag references are this same shape, not substrate gaps.

The substrate audit also confirmed:

- All ~38 named substrate primitives in `docs/RUST_ENGINE_GAPS.md` are either CLOSED (8), NARROW with closed cores (16), OPEN (12 narrow items), or UNCLEAR (2).
- The 12 truly-OPEN items belong to Phase 2 of the four-phase plan, not Phase 1.
- DSL pipeline gaps dominate the remaining work and admit a single uniform fix.

This change captures the work needed to fully close the DSL pipeline and prevent the anti-pattern from recurring.

## Goals / Non-Goals

**Goals:**

- Make every `CompiledPredicate`, `CompiledFormula`, `CompiledStep`, and `CompiledTiming` variant evaluate correctly by adding the missing match arm.
- Remove all 74 wildcard catch-alls from top-level evaluators in `code/digimon-engine/src/dsl_cards/`, letting the Rust compiler enforce exhaustiveness on future variant additions.
- Add a CI lint test (`dsl_eval_arm_coverage`) that asserts variant-name coverage textually as a safety net for cases where the compiler-level approach is awkward (inner helpers, generic eval).
- Drop the ignored-test count from 595 to ~150 (the residual being Phase 2 substrate edges + card-local authoring).
- Migrate ~150 closed gap-tag entries from `qa/dsl-vocab-gaps.md` to `qa/resolved-gaps.md` in four batched rollups.

**Non-Goals:**

- Substrate edge fixes (G-OPT-TRIGGERED, G-INHERITED-DISPATCH residue, G-OPT-RESET-VIA-ATTACK-CYCLE, the 12 named OPEN items from the 2026-05-14 audit). These are Phase 2.
- Card YAML migration or `raw_rust:` retirement. These are Phase 3.
- Changes to `ACTION_SPACE_SIZE`, observation tensor shape, PyO3 exports, frontend constants, or RL wrapper assumptions.
- Refactoring the DSL crate's parser or the `Compiled*` enum representations themselves — only their evaluators are in scope.
- New DSL vocabulary or new card capabilities. Every closed tag corresponds to a field that already parses; this change only makes the field actually evaluate correctly.

## Decisions

### Decision 1: Compiler-level exhaustiveness over runtime-only lint

**Choice:** Remove the wildcard arm from each `match` over a `Compiled*` enum so the Rust compiler enforces exhaustiveness. Add a runtime CI lint test as a safety net.

**Why:** Compile-time enforcement is faster (catches drift in pre-merge builds, not in CI), more local (the error points at the exact `match` statement, not at a test file), and free (no runtime overhead). The runtime lint catches cases where the compiler-level approach is awkward — for example, nested matches inside generic eval helpers, where moving to exhaustive matching would require a substantial refactor.

**Alternatives considered:**

- *Runtime lint only* (no wildcard removal). Rejected: the lint would only fire in CI, not in `cargo build`. Authors could push variant additions and discover gaps only at merge time.
- *Procedural macro that auto-generates eval arms.* Rejected for scope: writing a proc-macro to derive eval skeletons from `Compiled*` variants is its own design problem, and the current evaluators have hand-tuned ergonomics that a derive would erase.

### Decision 2: `strum::IntoEnumIterator` for variant iteration in the lint test

**Choice:** Add `strum_macros::EnumIter` derive to the `Compiled*` enums in `code/digimon-dsl/src/compiled.rs` and pull `strum::IntoEnumIterator` into the lint test so it can programmatically iterate all variants.

**Why:** Hardcoding variant names in the lint test creates a second source of truth that goes stale every time a variant is added. With `EnumIter`, the lint test iterates whatever the enum currently defines and asserts each name appears in the evaluator source.

**Alternatives considered:**

- *Hardcoded variant list with a "MUST update when adding variants" comment.* Rejected: relies on author discipline, which is exactly what fails today.
- *AST-level analysis via `syn` in a build script.* Rejected as over-engineered for the scope.

**Trade-off:** Adds two dev-dependencies (`strum`, `strum_macros`). They're tiny, widely used, and tree-shake at build time; the cost is acceptable.

### Decision 3: Batched PRs (one per evaluator surface)

**Choice:** Land four sequential PRs — predicate, formula, step+lowering, timing+modifier — instead of one mega-PR.

**Why:** A single sweep PR would touch 3,391 lines of evaluator code across 28 files and unignore ~445 tests. That's too large to review meaningfully. Per-batch PRs are reviewable in isolation, the variant-coverage lint becomes incrementally stricter, and trackers stay consistent between PRs.

**Alternatives considered:**

- *One sweep PR* with a single tracker rollup. Rejected for reviewability.
- *Per-tag PRs.* Rejected as too granular — ~150 tag closures would mean ~150 PRs, most touching 5–20 lines each. Per-evaluator batching is the right grain.

### Decision 4: Lint test lands in Batch 1, then tightens through Batch 4

**Choice:** The `dsl_eval_arm_coverage` test file is created in Batch 1 with assertions only for `CompiledPredicate`. Each subsequent batch adds assertions for its evaluator (formula, step, timing/modifier). By end of Batch 4, the lint is fully enforcing.

**Why:** Trying to land the full lint up front would require all four evaluators to be exhaustive on day one — impossible without merging all four batches simultaneously. Incremental tightening lets each batch land independently with a passing test.

**Alternatives considered:**

- *Land a stub lint test that does nothing in Batch 1, then add assertions in a final PR.* Rejected: the stub provides no value during the batched rollout.
- *Per-batch separate lint test files.* Rejected: a single file with grouped `#[test]` functions is cleaner.

### Decision 5: Tracker hygiene per batch, not at the end

**Choice:** Every batch's PR moves its closed gap-tag entries to `qa/resolved-gaps.md` as part of the same commit.

**Why:** Doing tracker hygiene at the end of the phase creates a cliff where reviewers can't tell what each PR closed. Per-batch rollups make the closure narrative legible in `git log` and `qa/resolved-gaps.md` history.

## Risks / Trade-offs

[Risk] **Removing wildcards may break compilation for currently-evaluating-correctly variants whose match arm relied on the wildcard to skip.**
→ Mitigation: each batch's first commit removes wildcards in a single function, then runs `cargo build`. Compiler errors are caught immediately and addressed in the same PR. The test suite then proves correctness.

[Risk] **Unignoring 445 tests may reveal that some "DSL eval-arm" tags were misclassified by the audit and are actually substrate-edge issues.**
→ Mitigation: contributors re-classify tags on contact. The plan explicitly anticipates this in its open-questions section. Misclassified tags get a fresh `#[ignore = "BLOCKED: G-… (Phase 2)"]` annotation and a dsl-vocab-gaps entry, deferring them to Phase 2.

[Risk] **`strum` dependency conflict with existing engine crates.**
→ Mitigation: `strum` and `strum_macros` are dev-dependencies only (not in production builds) and are pinned to stable versions. The `EnumIter` derive on `digimon-dsl` types does flow into the production crate, but the iterator code is gated behind feature flags or only used in test code.

[Risk] **Behavioral test failures after unignoring may indicate a bug in the existing test, not a missing arm.**
→ Mitigation: each unignored test should fail-then-pass cleanly when its tag's arm lands. If a test continues failing after the arm is in place, that's a test bug — fix the test in the same PR.

[Risk] **Batch 3 (step+lowering) is much larger than Batches 1, 2, and 4 combined (~241 tag refs).**
→ Mitigation: Batch 3 sub-divides into 3a (schema additions), 3b (lowering bridges), 3c (step executors). Each sub-batch is its own commit; if any sub-batch grows too large, it can be promoted to its own PR.

[Risk] **The variant-coverage lint may be too strict for cases where a variant intentionally has no eval arm in a particular evaluator (e.g., a predicate that only makes sense during attack resolution and is undefined elsewhere).**
→ Mitigation: such variants get an explicit no-op arm with a comment explaining why, instead of relying on a wildcard. The lint passes; the intent is documented.

## Migration Plan

Sequential, four batches:

1. **Batch 1 — Predicate evaluator + initial lint test**
   - Add `strum` / `strum_macros` dev-deps.
   - Add `#[derive(EnumIter)]` on `CompiledPredicate` in `digimon-dsl/src/compiled.rs`.
   - Create `code/digimon-engine/tests/dsl_eval_arm_coverage.rs` with predicate-only assertions.
   - Wire missing arms in `code/digimon-engine/src/dsl_cards/predicate.rs`.
   - Remove wildcards in `eval_predicate` and its inner helpers.
   - Unignore ~155 tests.
   - Move closed tag entries from `qa/dsl-vocab-gaps.md` to `qa/resolved-gaps.md` under "Phase 1 Batch 1 closure".

2. **Batch 2 — Formula evaluator**
   - Add `#[derive(EnumIter)]` on `CompiledFormula`.
   - Extend `dsl_eval_arm_coverage.rs` with formula assertions.
   - Wire arms in `code/digimon-engine/src/dsl_cards/formula_eval.rs`.
   - Remove wildcards.
   - Unignore ~80 tests.
   - Tracker rollup.

3. **Batch 3 — Step executors + lowering bridges** (three sub-batches)
   - 3a: Schema additions in `digimon-dsl/src/spec.rs` and `step.rs` (e.g. `AltPathSpec.condition`).
   - 3b: Lowering bridges in `code/digimon-engine/src/dsl_cards/lower_*.rs`.
   - 3c: Step executors in `code/digimon-engine/src/dsl_cards/step/*.rs`.
   - Add `#[derive(EnumIter)]` on `CompiledStep`.
   - Extend lint test.
   - Unignore ~140 tests.
   - Tracker rollup.

4. **Batch 4 — Timing-map and modifier-map**
   - Add `#[derive(EnumIter)]` on `CompiledTiming` and any `CompiledModifier`-style enum.
   - Wire `None`-returning branches in `timing_map.rs` and `modifier_map.rs`.
   - Final lint-test extension.
   - Unignore ~70 tests.
   - Phase 1 closure rollup at the top of `qa/resolved-gaps.md`.

**Rollback strategy:** Each batch is a single PR, revertible with `git revert`. Lint-test additions are guarded by per-evaluator helpers so a partial revert (e.g. revert Batch 3 only) still leaves the lint test passing for Batches 1, 2, and 4.

## Open Questions

1. **`strum` derive on production types.** Adding `#[derive(EnumIter)]` to `Compiled*` enums in `code/digimon-dsl/src/compiled.rs` means the production crate technically depends on `strum_macros`. Confirm that `strum_macros` is acceptable as a non-dev dep (it's a build-time macro, no runtime cost), or alternatively gate the derive behind `#[cfg(test)]` and use a different iteration approach in the lint test (e.g. a single match-all helper that asserts exhaustiveness at compile time).

2. **Wildcard exemption policy.** Some inner helpers in `predicate.rs` and `step/*.rs` may legitimately need narrow defaults (e.g., a helper that only handles a subset of variants by design). The lint test currently forbids wildcards in top-level `eval_*` functions only — confirm this scoping is sufficient or whether the lint should walk into all functions defined in the same file.

3. **Re-classification policy for misclassified audit tags.** When a tag the audit called "DSL eval-arm" turns out to be a substrate edge, the contributor should defer it to Phase 2. Confirm the deferral protocol: open a new dsl-vocab-gaps entry with "Phase 2" prefix? Reopen the entry in `RUST_ENGINE_GAPS.md`? Document the convention in the first batch's PR description.

4. **Batch 3 PR sizing.** Sub-batches 3a/3b/3c may individually be large enough to warrant their own PRs. The plan permits this but doesn't mandate it — the executing contributor should split if any sub-batch exceeds ~600 lines of code change or ~50 unignored tests.

5. **Dependency on Phase 0 commits.** Phase 0's tracker hygiene sweep (already landed in this worktree, not yet committed/merged) renamed several open entries in `RUST_ENGINE_GAPS.md`. Phase 1 batches' tracker rollups should target the post-hygiene names, not the pre-hygiene names. Confirm the Phase 0 hygiene PR merges before Phase 1 Batch 1 lands.
