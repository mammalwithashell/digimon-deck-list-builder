## ADDED Requirements

### Requirement: DSL supports player Digimon attack-history predicates

The DSL SHALL provide a predicate or condition that evaluates whether a referenced player attacked with at least one Digimon during the current turn. The predicate SHALL be usable in triggered effect conditions, including inherited end-of-opponent-turn clauses, and SHALL support normal DSL negation so card authors can express "the opponent did not attack with a Digimon this turn."

#### Scenario: Predicate is true after referenced player attacks with a Digimon

- **WHEN** a player attacks with one of their Digimon during the current turn
- **THEN** evaluating the attack-history predicate for that player returns true

#### Scenario: Predicate is false when referenced player has not attacked with a Digimon

- **WHEN** a player reaches an end-of-turn timing without attacking with any Digimon during that turn
- **THEN** evaluating the attack-history predicate for that player returns false
- **AND** a negated form of the predicate can be used to authorize effects that require no Digimon attack

#### Scenario: Predicate resets across turn boundaries

- **WHEN** a player attacked with a Digimon on a previous turn
- **AND** a later turn begins
- **THEN** evaluating the attack-history predicate for that player reflects only the later turn's attack history

#### Scenario: Predicate can be referenced from inherited end-of-opponent-turn effects

- **WHEN** a card author writes an inherited `end_of_opponents_turn` clause conditioned on the opponent not having attacked with a Digimon this turn
- **THEN** the DSL compiles the clause
- **AND** the engine evaluates the condition at trigger resolution time using authoritative game attack history
