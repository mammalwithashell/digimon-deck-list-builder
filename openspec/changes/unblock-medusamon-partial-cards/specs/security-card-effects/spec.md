## ADDED Requirements

### Requirement: Declinable `[Security]` triggered effects resolve to completion

A revealed security card carrying an optional (`you may`) `[Security]` triggered effect SHALL resolve to a terminal outcome whether the controller accepts or declines it. The security-resolution state machine MUST NOT re-enqueue the `SecuritySkill` timing for the same revealed card after the effect's first drain, so a declined optional effect cannot re-install its selection indefinitely.

#### Scenario: Player declines an optional `[Security]` effect

- **WHEN** a security card with an optional `[Security]` "you may" effect is revealed during a security check and the controller chooses to decline it
- **THEN** the security-resolution state machine advances past the security-skill phase and the security check proceeds to its battle/dispose phases
- **AND** no `pending_selection` remains installed for that revealed card

#### Scenario: Player accepts an optional `[Security]` effect

- **WHEN** a security card with an optional `[Security]` effect is revealed and the controller chooses to accept it, including any follow-up selections the effect installs
- **THEN** the effect resolves fully and the security check then proceeds to its remaining phases exactly once

#### Scenario: Revealed card has no `[Security]` effect

- **WHEN** a revealed security card carries no `[Security]` triggered effect
- **THEN** the security-skill phase completes without parking and resolution proceeds normally

### Requirement: Effects can trash a chosen non-top security card

An effect SHALL be able to trash a specific security card chosen by the controlling player at any position in a security stack, not only the top or bottom card. The choice MUST be surfaced as a player selection so every legal target is exposed to the action space.

#### Scenario: Effect trashes a selected non-top security card

- **WHEN** an effect resolves that lets a player trash any one of a target player's security cards, and the target has multiple security cards
- **THEN** the player is offered a selection of every security card in the stack
- **AND** the card the player selects is removed from the security stack and placed in its owner's trash

#### Scenario: Single-card security stack

- **WHEN** the same effect resolves and the target player has exactly one security card
- **THEN** that card is the sole selectable target and is trashed when chosen

#### Scenario: Empty security stack

- **WHEN** the same effect resolves and the target player has no security cards
- **THEN** no selection is installed and the effect proceeds to its remaining steps without trashing anything
