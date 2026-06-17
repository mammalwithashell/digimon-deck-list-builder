# Tasks — unify DSL scalar + comparator types

## 1. Canonical scalar: `FormulaSpec` everywhere (D3)
- [ ] 1.1 Confirm `FormulaSpec` deserializes a bare int to `Literal(i32)` and add a round-trip test fixing that contract before any retype.
- [ ] 1.2 Retype `gain_memory`/`lose_memory`/`set_memory` args `i32 → FormulaSpec`; delete `gain_memory_fn`/`lose_memory_fn`. Update lowering + compile arms.
- [ ] 1.3 Retype `de_digivolve.amount`, `aura.dp_modifier`, `aura.security_attack`, `cost_reduction.amount` `i32 → FormulaSpec`; delete the four `_fn` twin fields.
- [ ] 1.4 Fold `CostDelta::ReduceFn { reduce_fn }` into `CostDelta::Reduce { reduce: FormulaSpec }`; update call sites.
- [ ] 1.5 Delete `ModifierValueSpec`; route its two arms through `FormulaSpec`.
- [ ] 1.6 Parser round-trip tests: every retyped field accepts both a bare int and a formula; behavioral parity test on a representative card per field.

## 2. Uniform comparator (D2 — staged)
- [ ] 2.1 Introduce `Comparator { op: eq|gte|lte, value: FormulaSpec }` + its compiled form + eval.
- [ ] 2.2 Stage A — factor `event_target_*`/`event_card_*` numeric leaves into `Comparator`; delete the dead leaves (4 of 8 event dp/level are 0-use); widen each event metric to all three ops. Tests green.
- [ ] 2.3 Stage B — fold `DpConstraint` into the shared `Comparator`/`FormulaSpec` path; delete the parallel implementation.
- [ ] 2.4 Stage C — convert identity metrics (`dp`/`level`/`play_cost`/`stack_size`/`materials_count`/`security_count`) to `Comparator` behind legacy-key alias deser; fix the missing `_eq` operators for free.
- [ ] 2.5 Confirm legacy keys (`dp_lte: N`, `level_eq: N`, …) still parse to the identical compiled comparator (alias-deser round-trip test).
- [ ] 2.6 Verify predicate-eval contexts (incl. read-only mask contexts) can resolve a `FormulaSpec` comparator value read-safely (D-risk).

## 3. Merge budget-select verbs (RL-gated, D4)
- [ ] 3.1 Add `select_opponent_budget { axis: dp|play_cost, budget: FormulaSpec, min_picks, filter, bind_as, prompt, then }`; normalize both budgets to `FormulaSpec`.
- [ ] 3.2 Collapse `SelectionKind::{DpBudget, PlayCostBudget}` into one `BudgetSelect { metric, remaining, picked }`.
- [ ] 3.3 RL parity gate: assert the merged `BudgetSelect` encodes the SAME action mask + observation tensor as the two prior variants (diff old vs new on a DP-budget and a play-cost-budget selection). Confirm `ACTION_SPEC.md`/`TENSOR_SPEC.md` need no edits.
- [ ] 3.4 Migrate `select_opponent_dp_budget`/`select_opponent_play_cost_budget` (EX4-073 + any others) to the merged verb; delete the old verbs.
- [ ] 3.5 If the encoder cannot be made byte-identical, STOP and re-scope (ship FormulaSpec + Comparator, defer the budget merge) — do not change the RL contract here.

## 4. Docs + verification
- [ ] 4.1 Regenerate the vocab block; confirm removed verbs/predicates drop out and `dsl-vocab-doc-drift --check` is green.
- [ ] 4.2 Update `RUST_DSL_AGENT_GUIDE.md` §6 formula/predicate prose to describe the canonical scalar + comparator (point at the generated tables).
- [ ] 4.3 Full DSL + behavioral + action-mask test suites green.
- [ ] 4.4 Optional mechanical sweep: migrate legacy `_fn`/`dp_*`/`event_*` YAML keys to canonical forms (NOT required for correctness — back-compat covers it).
