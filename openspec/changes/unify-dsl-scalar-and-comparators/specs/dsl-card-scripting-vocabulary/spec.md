## ADDED Requirements

### Requirement: Magnitude fields accept literals or formulas through one canonical type

Every DSL field expressing a numeric magnitude — memory deltas (`gain_memory`/`lose_memory`/`set_memory`), De-Digivolve amount, aura `dp_modifier` and `security_attack`, cost-reduction amount, and cost deltas — SHALL accept either a literal integer or a `FormulaSpec`, parsed through one canonical type. The previous parallel encodings (the `_fn` twin verbs/fields, `ModifierValueSpec`, and the `CostDelta` literal-vs-formula split) SHALL be removed. A bare integer in any of these positions SHALL continue to parse unchanged.

#### Scenario: Bare integer still parses after retype

- **WHEN** a card uses `gain_memory: 2` (or `dp_modifier: 3000`, `de_digivolve` amount `1`, etc.)
- **THEN** it parses and behaves exactly as before the unification

#### Scenario: Formula accepted in a position that previously required a `_fn` twin

- **WHEN** a card uses `gain_memory: { base: 0, per: ally_count, delta: 1 }` (a formula directly in the magnitude field)
- **THEN** it parses and resolves the formula at effect resolution — without a separate `gain_memory_fn` verb

#### Scenario: Retired `_fn` twins no longer exist

- **WHEN** the vocabulary is enumerated
- **THEN** `gain_memory_fn`, `lose_memory_fn`, `dp_modifier_fn`, `security_attack_fn`, the `cost_reduction.amount_fn` twin, `ModifierValueSpec`, and `CostDelta::ReduceFn` are absent (their function is subsumed by the canonical magnitude type)

### Requirement: Numeric predicate thresholds use a uniform, complete comparator

Numeric predicate comparisons SHALL be expressed through a uniform `Comparator { op: eq | gte | lte, value: FormulaSpec }` shape that is available for every numeric metric (DP, level, play cost, stack size, materials count, security count, and the event-payload metrics) and supports all three operators for each. Legacy key spellings (e.g. `dp_lte: N`, `level_eq: N`) SHALL continue to parse via deserialize aliases that lower to the same compiled comparator.

#### Scenario: Legacy threshold key still parses

- **WHEN** a card uses `filter: { dp_lte: 5000 }`
- **THEN** it parses and filters identically to before, lowering to the canonical comparator

#### Scenario: Operator completed for a metric that previously lacked it

- **WHEN** a card needs "play cost equal to N" (an `_eq` the legacy surface lacked for `play_cost`)
- **THEN** it is expressible through the uniform comparator without a new bespoke predicate field

#### Scenario: Threshold value may be a formula for any metric

- **WHEN** a card needs "DP ≤ (a runtime formula)" on any metric position
- **THEN** the comparator's `value` accepts a `FormulaSpec` and resolves it read-safely in the evaluation context where the predicate is checked

### Requirement: A single metric-parameterized budget-selection verb

Player-visible "delete/target up to a budget of total <metric>" selections SHALL be expressed by one verb parameterized by metric axis (DP or play cost), replacing the per-axis verb pair, with the budget value typed as `FormulaSpec`. The merged selection SHALL present the identical action mask and observation-tensor encoding as the per-axis verbs it replaces.

#### Scenario: DP budget via the merged verb

- **WHEN** a card uses the merged budget verb with `axis: dp` and a budget
- **THEN** it offers the same legal targets and consumes budget identically to the former `select_opponent_dp_budget`

#### Scenario: RL encoding is unchanged

- **WHEN** a budget selection is active under the merged verb
- **THEN** the action mask and observation tensor are byte-identical to the encoding produced by the former per-axis verbs (no action-space or tensor contract change)
