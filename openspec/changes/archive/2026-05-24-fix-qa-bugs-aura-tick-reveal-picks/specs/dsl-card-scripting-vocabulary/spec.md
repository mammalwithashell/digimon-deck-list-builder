## ADDED Requirements

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
