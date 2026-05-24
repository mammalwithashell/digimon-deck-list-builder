## ADDED Requirements

### Requirement: DSL supports material-count aggregate predicates

The DSL SHALL provide a permanent predicate that evaluates whether a candidate permanent's material count is tied for an aggregate material count among a referenced player's battle-area Digimon. Material count means digivolution stack size minus the top card. The predicate SHALL support at least `fewest_materials`, SHALL compose with existing filters such as `kind: digimon`, and SHALL include all tied candidates.

#### Scenario: All Digimon tied for fewest materials match

- **WHEN** a filter uses `materials_count_matches_aggregate: { selector: fewest_materials, of: opponent }`
- **AND** the opponent has Digimon with 0, 0, 1, and 2 materials
- **THEN** both 0-material Digimon satisfy the predicate
- **AND** the 1-material and 2-material Digimon do not satisfy the predicate

#### Scenario: Non-Digimon candidates are excluded by composed filter

- **WHEN** the aggregate predicate is composed with `kind: digimon`
- **THEN** opponent Tamers and other non-Digimon permanents do not satisfy the composed filter

### Requirement: DSL supports formula-valued De-Digivolve amounts

The `de_digivolve` step SHALL accept a formula-valued amount in addition to the existing literal amount. The formula SHALL evaluate at effect resolution time using the resolving effect context, and the resulting amount SHALL be passed through the normal De-Digivolve caps and immunity checks.

#### Scenario: De-Digivolve amount equals own Digimon count

- **WHEN** a `de_digivolve` step uses `amount_fn` based on the controller's Digimon count
- **AND** the controller has three Digimon when the effect resolves
- **THEN** the engine attempts to De-Digivolve the selected target by 3
- **AND** normal stop-at-level and available-source caps still apply

#### Scenario: Literal De-Digivolve remains supported

- **WHEN** a `de_digivolve` step uses the existing literal `amount` field
- **THEN** it compiles and resolves with the same behavior as before this change

### Requirement: DSL supports predicate-scoped timing suppression

The DSL SHALL allow card authors to suppress activation of specific effect timings for permanents matched by a predicate-scoped modifier. The suppression SHALL support `[When Attacking]` and `[When Digivolving]` timings and SHALL apply through the shared timing-dispatch path so face-up, inherited, and granted effects from affected permanents are blocked consistently.

#### Scenario: Affected permanent cannot activate When Attacking

- **WHEN** a permanent is affected by a modifier that suppresses `[When Attacking]`
- **AND** that permanent attacks
- **THEN** its `[When Attacking]` effects are not enqueued or activated
- **AND** unaffected permanents still activate their legal `[When Attacking]` effects

#### Scenario: Affected permanent cannot activate When Digivolving

- **WHEN** a permanent is affected by a modifier that suppresses `[When Digivolving]`
- **AND** that permanent digivolves
- **THEN** its `[When Digivolving]` effects are not enqueued or activated
- **AND** global observer effects from other unaffected sources are not suppressed unless their own source permanent is affected
