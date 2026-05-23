## ADDED Requirements

### Requirement: Optional breeding permanent selection

The DSL and engine SHALL support selecting an own breeding-area permanent as an optional player choice, including filtered King Drasil targets, without forcing the player to accept a printed "you may" effect.

#### Scenario: Player declines optional breeding selection

- **WHEN** an effect opens an optional breeding-permanent selection and at least one matching breeding permanent exists
- **THEN** PASS SHALL be legal
- **AND** resolving PASS SHALL decline that selection without running the selection's success body

#### Scenario: Player accepts optional breeding selection

- **WHEN** an effect opens an optional breeding-permanent selection for a matching breeding-area King Drasil
- **THEN** the matching breeding permanent SHALL be selectable
- **AND** resolving that selection SHALL bind the breeding permanent for subsequent source-placement or source-play steps

#### Scenario: Mandatory breeding selection remains mandatory

- **WHEN** an effect opens a non-optional breeding-permanent selection and a matching breeding permanent exists
- **THEN** PASS SHALL NOT be legal

### Requirement: Breeding source selections preserve player-visible choices

The system SHALL support selecting one or more digivolution cards from a breeding-area carrier, including count limits and name-uniqueness constraints, through pending selections rather than hidden auto-selection.

#### Scenario: Select one of each different name from King Drasil sources

- **WHEN** a breeding-area King Drasil has multiple Royal Knight source cards, including duplicate names
- **THEN** the player SHALL be able to choose Royal Knight source cards up to the printed count
- **AND** after one source with a given name is chosen, other sources sharing that name SHALL no longer be legal for that same selection

#### Scenario: Played material sources can suppress On Play

- **WHEN** selected source cards are played by an effect whose printed text suppresses On Play effects
- **THEN** the played Digimon SHALL enter battle without enqueueing their On Play effects

### Requirement: Budgeted opponent target selections

The DSL and engine SHALL support selecting opponent permanents under a running aggregate budget so card text such as "any number whose total DP is 15000 or less" is represented natively.

#### Scenario: Running DP budget updates after each pick

- **WHEN** a DP-budget selection has 15000 remaining DP and the player selects an opponent Digimon with 7000 DP
- **THEN** the selection SHALL continue with 8000 remaining DP
- **AND** opponent Digimon whose DP exceeds the remaining budget SHALL NOT be legal picks

#### Scenario: Budget selection requires minimum picks

- **WHEN** a printed mandatory DP-budget effect has at least one legal target
- **THEN** PASS SHALL NOT be legal before the required minimum number of picks is selected
- **AND** PASS SHALL become legal once the minimum is satisfied

### Requirement: Event-bound keyword grants

The engine and DSL SHALL allow triggered effects to grant keywords to Digimon identified by the triggering event, including a Digimon that was just played, with the printed expiry.

#### Scenario: Grant keywords to the played Digimon

- **WHEN** a triggered effect fires because a matching Digimon was played
- **THEN** the effect SHALL be able to target the played Digimon from the event context
- **AND** granted keywords SHALL expire at the printed timing

#### Scenario: Event-bound grant does not over-target

- **WHEN** multiple Digimon match the same trait filter but only one Digimon was played by the triggering event
- **THEN** an event-bound keyword grant SHALL NOT grant keywords to unrelated matching Digimon unless the printed text allows that choice

### Requirement: Gap trackers distinguish substrate gaps from card-authoring backlog

Royal Knights gap documentation SHALL distinguish open reusable primitives from card-local authoring and test backlog.

#### Scenario: Closed substrate is not listed as an open blocker

- **WHEN** a Royal Knights card is blocked only because production YAML has not been authored
- **THEN** gap trackers SHALL classify the item as card-authoring backlog rather than an engine or DSL gap

#### Scenario: True blockers name reusable primitives

- **WHEN** a Royal Knights card remains partial or blocked after this change
- **THEN** its test ignore reason and tracker entry SHALL name the current reusable primitive that prevents faithful implementation
