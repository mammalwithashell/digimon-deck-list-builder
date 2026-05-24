# dsl-card-scripting-vocabulary Specification

## Purpose
TBD - created by archiving change unblock-medusamon-partial-cards. Update Purpose after archive.
## Requirements
### Requirement: Declinable `trash_self` activation cost

The card-scripting DSL SHALL accept `trash_self: true` as an `activation_cost` for a triggered clause, alongside the existing `suspend_self` and `return_self_to_deck_bottom` costs. The three cost kinds MUST be mutually exclusive — a clause specifying more than one is a compile error. A `trash_self` activation cost SHALL be declinable: when the trigger fires, the controller is offered an accept/decline choice, declining skips the entire clause body, and accepting trashes the source card and then runs the body. This makes `<Delay>` "by trashing this card" a true declinable cost per Comprehensive Rules 16-16-2.

#### Scenario: Author declares a `trash_self` activation cost

- **WHEN** a card YAML declares a triggered clause whose first body step is `activation_cost: { trash_self: true }`
- **THEN** the clause compiles and the cost is lifted onto the clause's activation cost rather than treated as a mid-body step

#### Scenario: Controller declines the activation cost

- **WHEN** a clause with a `trash_self` activation cost triggers and the controller declines
- **THEN** the source card is not trashed and the clause body does not run

#### Scenario: Controller accepts the activation cost

- **WHEN** a clause with a `trash_self` activation cost triggers and the controller accepts
- **THEN** the source card is moved to its owner's trash and the clause body resolves

#### Scenario: Mutually exclusive cost kinds

- **WHEN** a clause declares `activation_cost` with `trash_self` set together with `suspend_self` or `return_self_to_deck_bottom`
- **THEN** compilation fails with an error stating the cost kinds are mutually exclusive

### Requirement: Alt-path digivolution requirements can gate on a source card's printed text

An alternate-digivolution-path `from:` source filter SHALL be able to gate on whether a candidate source card has a given keyword (such as `<Save>`) printed in its effect text, so a printed digivolution requirement of the form "Lv.N w/<Keyword> in text" can be expressed.

This capability is provided by the DSL's existing `effect_text_contains` predicate — a case-insensitive substring scan of the candidate card's printed text (effect, inherited, and security text). When an alt-path `from:` filter is matched, the engine evaluates the filter against the candidate source permanent via `eval_predicate`, so `effect_text_contains` works there with no new predicate. (Implementation note: the change's design first proposed a dedicated `keyword_in_text` predicate; implementation found `effect_text_contains` already covers the need — and "w/<Keyword> **in text**" is itself worded as a text-presence check — so no redundant verb was added.)

#### Scenario: Source card has the keyword in its text

- **WHEN** an alt-path `from:` filter uses `effect_text_contains` for a keyword marker and a candidate source permanent's top card has that marker printed in its effect text
- **THEN** the candidate satisfies the filter and the alt-path is offered for that source

#### Scenario: Source card lacks the keyword

- **WHEN** the same filter is evaluated against a candidate source permanent whose top card does not have the marker in its text
- **THEN** the candidate does not satisfy the filter and the alt-path is not offered for that source

#### Scenario: Combined with other `from:` predicates under OR

- **WHEN** an alt-path `from:` filter combines the text-presence check with another predicate (such as `trait_has`) under an `any_of`
- **THEN** a candidate satisfying either branch is offered the alt-path

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

The `de_digivolve` step SHALL accept a formula-valued amount in addition to the existing literal amount. The formula SHALL evaluate at effect resolution time using the resolving effect context, and the resulting amount SHALL be passed through the normal De-Digivolve caps, immunity checks, and configured stop-at-level floor. DSL-authored `de_digivolve` steps that omit `stop_at_level` SHALL default to the normal level 3 floor, so card YAML that represents standard printed `<De-Digivolve N>` text preserves the floor even when using `amount_fn`. Non-standard stack-clearing effects that intentionally ignore the level 3 floor SHALL use a raw Rust/helper path that explicitly calls the engine primitive with no floor.

#### Scenario: De-Digivolve amount equals own Digimon count

- **WHEN** a `de_digivolve` step uses `amount_fn` based on the controller's Digimon count
- **AND** the controller has three Digimon when the effect resolves
- **THEN** the engine attempts to De-Digivolve the selected target by 3
- **AND** normal stop-at-level and available-source caps still apply

#### Scenario: Formula-valued standard De-Digivolve preserves the level 3 floor

- **WHEN** a standard printed `<De-Digivolve>` effect is authored with `amount_fn`
- **AND** the target stack contains a Digi-Egg under a level 3 card
- **THEN** the YAML-authored step SHALL preserve the standard level 3 floor
- **AND** resolving the effect SHALL NOT trash the level 3 card or expose the Digi-Egg

#### Scenario: Literal De-Digivolve remains supported

- **WHEN** a `de_digivolve` step uses the existing literal `amount` field
- **THEN** it compiles and resolves with the same behavior as before this change

#### Scenario: Non-standard unbounded stack trash remains expressible outside default DSL lowering

- **WHEN** a card's printed text requires trashing digivolution cards without the standard De-Digivolve level 3 floor
- **THEN** a raw Rust/helper implementation MAY call the engine De-Digivolve primitive with no stop-at-level floor for that non-standard effect
- **AND** that usage SHALL remain distinct from standard DSL-authored printed `<De-Digivolve>` text

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

### Requirement: `choose_from_reveal { optional: true }` requires printed-text "may"

The DSL primitive `choose_from_reveal` accepts an `optional: bool` field that, when `true`, lets the player decline the pick via the standard PASS action even when eligible candidates exist in the revealed pool. Card authors SHALL set `optional: true` ONLY when the printed card text explicitly grants the player permission to decline at that specific pick (printed wording variants include "you may add", "you may place", "may choose to add/place", and similar "may" formulations applied to the pick itself).

When the printed card text states the pick as an unconditional add (e.g., "Add 1 card with the [X] trait..."), the pick is mandatory and the YAML SHALL either omit `optional` (the default is `false`) or set it explicitly to `false`. The "no eligible candidates" case SHALL be handled by the engine's natural fizzle path — the bucket auto-skips when zero candidates match the filter — and SHALL NOT be modeled as a player-driven optional decline.

This rule applies to every `choose_from_reveal` invocation in `code/digimon-engine/cards/**/*.yaml`. Authors faced with a mandatory two-pick "Add 1 X and 1 Y" reveal-search pattern SHOULD prefer the `select_reveal_buckets` primitive (see BT24-031 Elecmon as the canonical reference), which surfaces a single combined bucket prompt and forbids `optional` by design.

The cost-payment surrounding a `choose_from_reveal` is orthogonal to the pick's `optional` field — a top-level effect clause MAY be `optional: true` (modeling a "by paying X..." optional activation) while the inner `choose_from_reveal` that follows the cost payment is mandatory. The two flags express different player choices: whether to activate the effect at all, versus whether to decline a specific pick once the effect is already mid-resolution.

#### Scenario: Mandatory "Add 1 trait card" pick rejects PASS

- **WHEN** a DSL clause uses `choose_from_reveal` with `optional: false` (or omitted) and the revealed pool contains at least one card matching the filter
- **THEN** the engine SHALL surface a pending selection whose `options` list contains the eligible card slots and SHALL NOT accept a PASS action (action_id 62) as a decline path — submitting PASS leaves the selection in place or returns an `ok: false` selection rejection

#### Scenario: Mandatory pick with zero candidates fizzles silently

- **WHEN** a DSL clause uses `choose_from_reveal` with `optional: false` and the revealed pool contains zero cards matching the filter
- **THEN** the engine SHALL skip the pick step without raising a pending selection, and any subsequent process steps (e.g., `order_remainder`) SHALL execute against the unchanged revealed pool

#### Scenario: Optional pick honors PASS decline

- **WHEN** a DSL clause uses `choose_from_reveal` with `optional: true` reflecting a printed-text "may" pick, and the revealed pool contains eligible candidates
- **THEN** the engine SHALL surface a pending selection with the eligible candidates AND SHALL accept PASS as a valid decline, after which subsequent process steps execute as if the pick produced no card

#### Scenario: Optional cost wrapping a mandatory pick

- **WHEN** a top-level effect clause is `optional: true` (modeling a "by paying X..." optional activation) and its `process` includes a `choose_from_reveal` step with `optional: false` after the cost is paid
- **THEN** declining the top-level activation SHALL skip the entire clause (no cost, no pick), while accepting the activation SHALL pay the cost and then surface the mandatory pick — declining the inner pick via PASS SHALL NOT be accepted in this case

