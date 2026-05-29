## ADDED Requirements

### Requirement: Security costs can choose top or bottom position
Security-stack cost handling SHALL support costs that trash the controller's top or bottom security card by player choice. The cost MUST be paid before the effect body runs and MUST preserve no-approximations semantics for decline, empty security, and replacement/prevention cases.

#### Scenario: Controller chooses top security as cost
- **WHEN** an effect cost allows trashing the top or bottom security card and the controller chooses top
- **THEN** the top security card is trashed as a cost
- **AND** the effect body resolves after the cost succeeds

#### Scenario: Controller chooses bottom security as cost
- **WHEN** the same cost is offered and the controller chooses bottom
- **THEN** the bottom security card is trashed as a cost
- **AND** the effect body resolves after the cost succeeds

#### Scenario: Security cost is prevented
- **WHEN** a replacement or prevention effect stops the selected security card from being trashed
- **THEN** the cost is not considered paid
- **AND** the gated effect body does not resolve

### Requirement: Effect-trashed security cards can run card-local follow-up effects
When an effect trashes a card from security, the engine SHALL surface that event to the trashed card's relevant DSL clauses so card-local text such as "When effects trash this card from the security stack, play this card" or "activate this card's [Main] effect" can resolve faithfully.

#### Scenario: Effect trashes security Digimon with play trigger
- **WHEN** an effect trashes a security card whose text says it may be played when effects trash it from security
- **THEN** the card's `on_discard_security` clause is queued
- **AND** accepting the trigger plays that physical card from security without paying the cost

#### Scenario: Effect trashes security Option with Main activation trigger
- **WHEN** an effect trashes a security Option whose text says to activate its `[Main]` effect
- **THEN** the card's `on_discard_security` clause can dispatch the same body used by its `[Main]` effect
- **AND** the Option is not treated as normally used from hand

#### Scenario: Normal security check does not count as effect trash
- **WHEN** a card is revealed and removed by a normal security check
- **THEN** `on_discard_security` clauses requiring effect-trash provenance do not fire
