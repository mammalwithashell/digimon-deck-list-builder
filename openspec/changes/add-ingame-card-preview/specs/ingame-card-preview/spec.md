## ADDED Requirements

### Requirement: Right-click preview of a hand card

During a game, the system SHALL open a large card-preview overlay when the local player right-clicks (context-menu gesture) one of their own hand cards. The browser's native context menu SHALL be suppressed for this gesture.

#### Scenario: Right-click a hand card opens the preview

- **WHEN** the local player right-clicks a card in their hand
- **THEN** the native context menu is suppressed
- **AND** a large preview overlay opens showing that card's full-size art and full printed text (name, play cost, DP, level, traits, main effect, inherited effect)

#### Scenario: Right-click does not play the card

- **WHEN** the local player right-clicks a hand card
- **THEN** no play action, trash action, or other game action is submitted to the engine
- **AND** the action mask and game state are unchanged

### Requirement: Right-click preview of a field permanent

During a game, the system SHALL open a large card-preview overlay when the local player right-clicks a permanent on the battle area (their own or the opponent's). The overlay SHALL preview the permanent's top card and SHALL surface the permanent's digivolution stack so each source card can be previewed individually.

#### Scenario: Right-click a field permanent shows the top card and its stack

- **WHEN** the local player right-clicks a permanent on the field
- **THEN** the preview overlay opens showing the top card's full-size art and full printed text
- **AND** the permanent's digivolution stack is listed top-card-first with each source card identified

#### Scenario: Preview a source card within the stack

- **WHEN** the preview overlay for a permanent is open and the player selects a source card in the digivolution stack
- **THEN** the overlay shows that source card's full-size art and full printed text (including its inherited effect)

#### Scenario: Right-click does not declare an attack or select a target

- **WHEN** the local player right-clicks a field permanent
- **THEN** no attack, blocker, digivolve, or selection action is submitted to the engine
- **AND** any in-progress attacker selection state is left unchanged

### Requirement: Preview content is sourced from static card metadata

The preview overlay SHALL render card art and printed text from static card metadata keyed by `cardId` (the CDN image source and the card-metadata API the deck builder already uses), independent of engine runtime serialization. The preview SHALL NOT depend on engine-populated effect-text fields in the game-state payload.

#### Scenario: Printed text shows even when engine state omits effect text

- **WHEN** a card is previewed whose engine game-state entry carries empty effect-text fields
- **THEN** the preview still shows the card's full printed main and inherited effect text from card metadata

#### Scenario: Art or metadata unavailable is handled gracefully

- **WHEN** a card's art fails to load or its metadata cannot be fetched
- **THEN** the overlay shows a loading or error placeholder for the missing part
- **AND** the overlay does not crash and remains dismissable

### Requirement: Preview overlay is non-blocking and dismissable

The preview overlay SHALL be dismissable and SHALL NOT block the player from continuing the game once dismissed. Left-click interactions on the board (play, attack, target selection, digivolve) SHALL retain their existing behavior and SHALL NOT be altered by the addition of the right-click preview.

#### Scenario: Dismiss the overlay

- **WHEN** the preview overlay is open and the player presses Escape, clicks the close control, or clicks outside the overlay
- **THEN** the overlay closes
- **AND** the board returns to its prior interactive state

#### Scenario: Left-click behavior is preserved

- **WHEN** the player left-clicks a hand card or field permanent
- **THEN** the existing left-click behavior occurs (e.g., play, select attacker, choose a target) exactly as before this change
