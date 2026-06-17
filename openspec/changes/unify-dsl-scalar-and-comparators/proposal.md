## Why

The DSL was built one field/verb per card-need. The audit found the single most-repeated structural defect is **"one concept encoded many ways"** — specifically two type-discipline failures that account for most of the substrate's sprawl:

**1. "A scalar that may be a literal or a runtime formula" is encoded 6+ ways.** Verified in source:
- twin step verbs with `_fn` suffix: `gain_memory`/`gain_memory_fn`, `lose_memory`/`lose_memory_fn` (`step.rs:68,76,81`);
- twin struct fields with `_fn` suffix: `de_digivolve.amount`/`amount_fn`, `aura.dp_modifier`/`dp_modifier_fn`, `aura.security_attack`/`security_attack_fn`, **and** `cost_reduction.amount`/`amount_fn` (`clause.rs:472-474`);
- `CostDelta::Reduce { reduce: i32 }` vs `CostDelta::ReduceFn { reduce_fn: FormulaSpec }` (`step.rs:1840,1849`);
- the bespoke `ModifierValueSpec` union enum (`step.rs:2094`);
- the `DpConstraint` union enum (predicate side);
- bare `FormulaSpec`, which **already round-trips bare ints** (`Literal(i32)`).

So `FormulaSpec` is already the canonical type — every other encoding is redundant. The `_fn` convention is also incomplete (only some magnitudes have it), so authors must know per-field whether a formula is allowed.

**2. `PredicateSpec` is a 145-field flat struct whose growth is dominated by per-metric comparator triples.** `event_target_*_{eq,gte,lte}`, `event_card_level_{eq,gte}`, and identity `dp/level/play_cost/stack_size/materials_count/security_count _{eq,gte,lte}` are the same `(op, value)` shape stamped out per metric. The families are **inconsistently complete** (only `dp`/`level` have `_eq`; `play_cost`/`materials_count`/`count` lack it; there is no `event_card_level_lte`), they have a `u8`-vs-`DpConstraint` type inconsistency, and the event leaves are mostly dead (4 of 8 event dp/level leaves are 0-use). One unified `Comparator { op, value: FormulaSpec }` collapses the family, fixes the completeness gaps, and makes every metric uniformly formula-capable.

**3. The two budget-select verbs are a smaller instance of the same disease.** `select_opponent_dp_budget` and `select_opponent_play_cost_budget` are structurally identical except the axis and a type inconsistency (one budget is `FormulaSpec`, one is bare `i32`), duplicated down into two `SelectionKind` variants + mask/tensor wiring.

All three are *type* problems: collapsing them shrinks the inventory authors and sub-agents reason over, deletes dead/duplicate code, and makes magnitude/threshold fields uniformly formula-capable forever — directly serving the "widen the substrate, not route around it" flywheel.

## What Changes

- **Adopt `FormulaSpec` as the one canonical scalar-or-formula type.** Retype `gain_memory`/`lose_memory`/`set_memory` args and the `amount`/`dp_modifier`/`security_attack`/`cost_reduction.amount` fields from `i32` to `FormulaSpec` (deserializes bare ints unchanged → **no card rewrites forced**). Then delete the `_fn` twin verbs/fields, `ModifierValueSpec`, and fold `CostDelta::ReduceFn` into `Reduce` and `DpConstraint` into the shared canonicalization.
- **Introduce a reusable `Comparator { op: eq|gte|lte, value: FormulaSpec }`** for numeric predicate comparisons, sharing the FormulaSpec canonicalization. Stage it: factor the low-traffic/near-dead `event_target_*`/`event_card_*` comparators first (validates the pattern, kills the dead leaves while *widening* every event metric to all three ops), then the high-traffic identity metrics behind a back-compat alias deserializer so existing `dp_lte: N` YAML keeps parsing.
- **Merge the two budget-select verbs** into one `select_opponent_budget { axis: dp|play_cost, budget: FormulaSpec, ... }` and collapse the paired `SelectionKind` variants into one parameterized `BudgetSelect { metric, remaining, picked }`; normalize both budgets to `FormulaSpec`.

## Capabilities

### Modified Capabilities
- `dsl-card-scripting-vocabulary`: magnitude fields accept literals or formulas uniformly through one `FormulaSpec` type (the `_fn` twin convention and `ModifierValueSpec` are removed); numeric predicate thresholds use a uniform `Comparator { op, value: FormulaSpec }` that is complete across all three operators for every metric and formula-capable everywhere; a single metric-parameterized budget-selection verb replaces the per-axis pair.

## Impact

- **DSL crate:** `step.rs` (retype memory verbs; delete `_fn` verbs, `ModifierValueSpec`, fold `CostDelta`), `clause.rs` (retype aura/cost_reduction fields, drop `_fn` twins), `predicate.rs` (introduce `Comparator`, alias deser for legacy keys, drop the per-metric triples + `DpConstraint`), `formula.rs` (canonical `FormulaSpec` deser of bare ints — already present), `compile.rs`/`compiled.rs` (lowering), `validator.rs`.
- **Engine lowering:** `dsl_cards/*` arms for the retyped fields + the merged budget `SelectionKind`; mask/tensor wiring for `BudgetSelect` collapses from two variants to one (verify the action-space encoder is unchanged — same selection slots, just one code path).
- **Cards:** no forced rewrites (bare-int back-compat), but the `_fn` and `dp_*`/`event_*` legacy keys can be mechanically migrated to the canonical forms in a follow-up sweep; the two budget cards (EX4-073 + any others) move to `select_opponent_budget`.
- **Docs:** regenerate the vocab block (verbs/predicates removed); update `RUST_DSL_AGENT_GUIDE.md` §6 formula/predicate prose.
- **Tests:** parser round-trip for legacy + canonical forms; behavioral parity for the retyped magnitude cards and the merged budget verb.
- **RL contract:** must be a no-op. The merged `BudgetSelect` SelectionKind must encode to the same action slots/tensor as the two it replaces; verify against `ACTION_SPEC.md`/`TENSOR_SPEC.md` before landing.

## Non-Goals

- The two card bugs, loader guard, dead-vocab retirement, and doc-rot sweep (`fix-dsl-substrate-rot-and-bugs`). Note: `lose_memory_fn` is deleted by whichever of the two changes lands first.
- Step-idiom collapse — `then:` tails, `reveal_search`, security-placement consolidation, `link_card_to_self` migration (`collapse-dsl-step-idioms`).
- Adding net-new predicates/metrics beyond completing the comparator operators for existing metrics.
