## ADDED Requirements

### Requirement: DSL can place a selected permanent into its owner's security
The card-scripting DSL SHALL provide a reusable way to place a selected battle-area permanent into that permanent's owner's security stack, preserving printed top/bottom and face-up/face-down choices and routing through normal leave-field and security-add replacement handling.

#### Scenario: Selected own permanent goes to own security
- **WHEN** a DSL effect selects one of the resolving player's permanents and resolves owner-routed security placement
- **THEN** that permanent is moved to the resolving player's security stack at the requested position
- **AND** the move uses the normal replacement-aware permanent-to-security path

#### Scenario: Selected opponent permanent goes to opponent security
- **WHEN** a DSL effect selects an opponent's permanent and resolves owner-routed security placement
- **THEN** that permanent is moved to the opponent's security stack at the requested position
- **AND** the selected permanent is not placed into the resolving player's security stack

#### Scenario: Placement exposes target choice
- **WHEN** both players have legal target permanents for an owner-routed security placement effect
- **THEN** every legal target is represented in the pending selection's valid action IDs
- **AND** no target is chosen automatically by owner or field order

### Requirement: DSL can gate follow-up effects on successful security placement
The card-scripting DSL SHALL let card authors run follow-up steps only when a permanent-to-security placement actually succeeds.

#### Scenario: Follow-up runs after successful placement
- **WHEN** an effect places a selected permanent into security and the placement succeeds
- **THEN** the DSL records that success for the running effect body
- **AND** subsequent result-gated steps can resolve

#### Scenario: Follow-up is skipped after failed placement
- **WHEN** an effect attempts to place a selected permanent into security and replacement or legality prevents the placement
- **THEN** the DSL records no successful placement for that effect body
- **AND** result-gated follow-up steps do not resolve

### Requirement: DSL can model security-stack costs for triggered effects
The card-scripting DSL SHALL support security-stack costs needed by Mastemon cards, including trashing the controller's top security and placing a selected permanent into security as a cost that gates the rest of an effect.

#### Scenario: Top-security trash cost is payable
- **WHEN** a triggered effect requires trashing the controller's top security as a cost and the controller has at least one security card
- **THEN** the controller can accept the effect, the top security card is trashed, and the effect body continues

#### Scenario: Top-security trash cost is not payable
- **WHEN** a triggered effect requires trashing the controller's top security as a cost and the controller has no security cards
- **THEN** the effect body does not resolve
- **AND** no hidden fallback action is performed

#### Scenario: Placement cost gates a tail
- **WHEN** a triggered effect requires placing a selected Digimon or Tamer into security as a cost
- **THEN** the target and placement choices are visible to the player
- **AND** the tail resolves only after the placement cost succeeds

### Requirement: DSL formulas support trash-until-security-threshold effects
The card-scripting DSL SHALL allow effects to trash security until a player has a printed threshold number of security cards remaining.

#### Scenario: Opponent security is trashed down to threshold
- **WHEN** an effect says to trash the opponent's security until they have 4 security cards and the opponent has more than 4 security cards
- **THEN** the DSL effect trashes exactly the excess number of top security cards

#### Scenario: No trash below threshold
- **WHEN** an effect says to trash security until a threshold and the target player has that many or fewer security cards
- **THEN** the DSL effect trashes zero security cards

#### Scenario: Both players trash to threshold
- **WHEN** an effect says both players trash security until they have 3 security cards
- **THEN** each player's trash count is computed independently from their current security count
