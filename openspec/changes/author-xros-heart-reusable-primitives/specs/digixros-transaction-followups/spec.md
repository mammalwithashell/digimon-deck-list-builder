## ADDED Requirements

### Requirement: DigiXros transactions support scoped wildcard requirements
The engine SHALL support effects that allow a specified card to replace one
DigiXros requirement for a scoped duration. A wildcard material MUST satisfy at
most one recipe slot and MUST NOT change the card's identity outside DigiXros
requirement matching.

#### Scenario: Turn-scoped wildcard replaces one requirement
- **WHEN** an effect grants that a specific Digimon may replace one DigiXros
  requirement for the turn
- **AND** the controller later starts a DigiXros transaction
- **THEN** that Digimon is legal for one otherwise-unfilled recipe slot
- **AND** selecting it consumes only one requirement slot
- **AND** the wildcard permission expires when the printed duration ends

#### Scenario: Wildcard does not satisfy unrelated effects
- **WHEN** a card is currently allowed to replace one DigiXros requirement
- **THEN** the card is not treated as having other names or traits for search,
  deletion, attack, or non-DigiXros predicates

### Requirement: DigiXros transaction modifiers can be card-scoped or turn-scoped
The engine SHALL distinguish modifiers that apply only to the current pending
DigiXros transaction from modifiers that apply to a later DigiXros transaction
within a printed duration.

#### Scenario: Current transaction modifier expires after play
- **WHEN** a Tamer suspends to allow cards under Tamers for the current
  DigiXros play
- **THEN** the extra origin access applies only to that pending transaction
- **AND** the access is gone after that play resolves or aborts

#### Scenario: Later transaction modifier waits for the next DigiXros
- **WHEN** an on-play effect grants a wildcard for the next DigiXros this turn
- **THEN** no material selection occurs immediately
- **AND** the next eligible DigiXros transaction can consume the modifier

### Requirement: DigiXros wildcard choices remain mask-driven
The engine SHALL expose wildcard material choices through pending-selection
action masks. Illegal wildcard choices MUST be masked out.

#### Scenario: Wildcard material appears in material prompt
- **WHEN** a DigiXros material prompt has an unfilled requirement and a legal
  wildcard card is in an allowed material zone
- **THEN** the wildcard card has a legal action in the material-selection mask
- **AND** resolving the action records it as the selected material for one slot

#### Scenario: Wildcard is masked after it is consumed
- **WHEN** a wildcard material has already replaced one requirement in the
  current DigiXros transaction
- **THEN** the same wildcard permission cannot satisfy a second slot unless the
  printed effect explicitly allows multiple substitutions
