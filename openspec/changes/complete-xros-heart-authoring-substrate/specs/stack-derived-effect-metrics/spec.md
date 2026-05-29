## ADDED Requirements

### Requirement: Effects can target by source-stack metrics

The engine SHALL provide reusable selectors that can filter or choose permanents
by source-stack metrics, including no-source targets and targets with the fewest
source cards among legal candidates.

#### Scenario: Select an opponent Digimon with no sources

- **WHEN** an effect can affect one opponent Digimon with no digivolution cards
- **THEN** only opponent Digimon whose source stacks are empty are legal targets

#### Scenario: Select the opponent Digimon with the fewest sources

- **WHEN** an effect deletes or affects the opponent Digimon with the fewest
  digivolution cards
- **THEN** the pending selection includes every tied legal opponent Digimon with
  the minimum source count and excludes opponent Digimon with more sources

### Requirement: Effects can count colors in source stacks

The engine SHALL expose formulas that count colors represented in a permanent's
source stack for use in DP changes, cost changes, and similar effect math.

#### Scenario: Apply a per-color DP modifier from own sources

- **WHEN** an effect gives a modifier for each color represented in the acting
  Digimon's digivolution cards
- **THEN** the modifier amount is computed from the represented colors beneath
  the acting Digimon's top card at the time the effect resolves, counting each
  represented color once

#### Scenario: Source stack has no represented colors

- **WHEN** the queried source stack is empty or has no color-bearing cards
- **THEN** the formula returns zero and downstream effect math uses zero

### Requirement: Effects can count matching source cards

The engine SHALL expose formulas that count source cards under a target
permanent, optionally filtered by card predicates such as level.

#### Scenario: Count level 6 source cards

- **WHEN** an effect declares a formula counting cards in the acting Digimon's
  source stack with `level_eq: 6`
- **THEN** the formula returns only matching source cards beneath the top card
  and can be used as a count-capped selection bound or memory/DP amount

### Requirement: Effects can compare against the acting Digimon's current DP

The engine SHALL support predicates that compare a target Digimon's current DP
against the acting Digimon's current DP after active modifiers are applied.

#### Scenario: Target has DP less than or equal to acting Digimon

- **WHEN** an effect can delete or return an opponent Digimon with DP less than
  or equal to the acting Digimon's DP
- **THEN** target legality is evaluated against both Digimon's current DP values
  at selection time

#### Scenario: Acting Digimon's DP changes before effect resolution

- **WHEN** a prior effect step changes the acting Digimon's DP before the
  comparison effect resolves
- **THEN** the comparison uses the updated current DP value
