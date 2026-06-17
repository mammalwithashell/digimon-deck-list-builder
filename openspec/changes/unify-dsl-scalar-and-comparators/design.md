# Design — unify scalar + comparator types

## Guiding principle

Every item here is "collapse N representations of one concept into one parametrized type." The win is not lines saved; it is that the *next* card automatically inherits formula-capability and operator-completeness it would otherwise have to wait for a gap to add. Risk is concentrated in two places — serde back-compat and the RL action/tensor contract — so both are explicit gates.

## D1 — Backward compatibility is mandatory, via deserialize aliases

No card may be forced to change. `FormulaSpec` already deserializes a bare int to `Literal(i32)`, so `gain_memory: 1` keeps working after the field is retyped. For predicate threshold keys (`dp_lte: 5000`), keep the legacy key names as deserialize aliases that lower into the new `Comparator`, so `dp_lte: 5000` and a future `dp: { op: lte, value: 5000 }` both parse to the same compiled form. `PredicateSpec` already omits `deny_unknown_fields` and `DpConstraint` already has a custom untagged `Deserialize` — that is the precedent for the alias layer. A mechanical migration of YAML to canonical forms is a *separate, optional* sweep, never a prerequisite.

## D2 — Stage the comparator factoring (highest-YAML-surface item last)

The identity comparators (`dp_lte`, `level_eq`, …) are the highest-traffic predicate keys in the corpus, so a mistake there is the most expensive. Stage in three steps, each independently shippable:
1. **Event leaves first.** Factor `event_target_*`/`event_card_*` into `Comparator`. These are low-traffic and 4 of 8 dp/level leaves are 0-use — validates the pattern and *widens* every event metric to all three operators while deleting dead leaves. Low blast radius.
2. **`DpConstraint` fold.** Collapse the bespoke `DpConstraint` union into the shared `Comparator`/`FormulaSpec` path (it already has custom deser, so this is mostly deleting a parallel implementation).
3. **Identity metrics.** Convert `dp/level/play_cost/stack_size/materials_count/security_count` to `Comparator` behind the legacy-key alias deser. This is where operator-completeness gaps (`_eq` missing on `play_cost`/`materials_count`/`count`) get fixed for free.

Each stage keeps the test suite green before the next begins.

## D3 — `FormulaSpec` retype order

Do the leaf scalars before the unions:
1. Memory verbs (`gain_memory`/`lose_memory`/`set_memory`) → `FormulaSpec`; delete `gain_memory_fn`/`lose_memory_fn`.
2. Struct fields (`de_digivolve.amount`, `aura.dp_modifier`, `aura.security_attack`, `cost_reduction.amount`) → `FormulaSpec`; delete the `_fn` twins.
3. `CostDelta::ReduceFn` → fold into `Reduce { reduce: FormulaSpec }`.
4. Delete `ModifierValueSpec` (its two arms are literal + formula → `FormulaSpec`).

`set_memory` keeps literal-only *semantics* in practice but accepting a formula costs nothing and removes the "which fields allow formulas?" cliff.

## D4 — RL contract is a hard gate (budget merge)

The budget-verb merge collapses two `SelectionKind` variants (`DpBudget`, `PlayCostBudget`) into one `BudgetSelect { metric, remaining, picked }`. The action-space encoder and observation tensor MUST be byte-identical before/after: same selection slots, same legal-action mask, same tensor fields — only the internal code path unifies. Add a parity assertion (encode a DP-budget and a play-cost-budget selection through both old and new and diff the mask + tensor) and check `ACTION_SPEC.md`/`TENSOR_SPEC.md` need no edits. If the encoder *can't* be made identical, stop and treat it as an action-contract change (out of scope here).

## D5 — Sequencing vs the other two changes

- Independent of `fix-dsl-substrate-rot-and-bugs` except both delete `lose_memory_fn` — whichever lands first removes it; the other drops that task.
- Should land **before** `collapse-dsl-step-idioms`'s `reveal_search`/budget-adjacent work only if convenient; no hard dependency. The shared `FormulaSpec` canonicalization (this change) is reused by any new verb the idiom change adds, so doing this first slightly de-risks that one.

## Risks

- **Serde alias correctness** (D1): mitigated by an exhaustive parser round-trip test over a sample of every legacy key form before deleting any old type.
- **RL drift** (D4): mitigated by the parity assertion; the budget merge is the only RL-touching item — if it's too risky, ship the FormulaSpec + Comparator parts and defer the budget merge.
- **Comparator value typed as `FormulaSpec`** means a predicate threshold can now reference a runtime formula everywhere; confirm the eval context for predicate evaluation can resolve formulas in every position they'll appear (some predicates evaluate in read-only mask contexts — the formula must be read-safe there).
