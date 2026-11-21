## ADDED Requirements

### Requirement: Format selection step in the battle flow

The client SHALL present a format-selection step after the battle-mode selection (Random Match, Room Match, Bot Match) and before deck selection. The step SHALL offer every defined format descriptor by display name, and the chosen format SHALL apply to the match being set up.

#### Scenario: Format is chosen before the deck

- **WHEN** the player selects a battle mode
- **THEN** the format-selection step is presented, and deck selection is not reached until a format is chosen

#### Scenario: Format applies to Random Match

- **WHEN** the player selects Random Match and then chooses a format
- **THEN** that format governs the queue joined and the legality applied for that match

#### Scenario: Format applies to Room Match

- **WHEN** the player selects Room Match and then chooses a format
- **THEN** that format governs both room creation and room joining for that match

#### Scenario: Format applies to Bot Match

- **WHEN** the player selects Bot Match and then chooses a format
- **THEN** that format governs the legality applied to the player's own deck for that match

#### Scenario: Cancelling returns to mode selection

- **WHEN** the player dismisses the format-selection step without choosing
- **THEN** the client returns to the battle-mode selection without connecting or creating a room

### Requirement: Match format defaults from the builder format

The format-selection step SHALL default to the player's persisted builder format, and choosing a different format for a match SHALL NOT change the persisted builder format.

#### Scenario: Default reflects the builder preference

- **WHEN** the format-selection step is presented and the persisted builder format is `singleton`
- **THEN** `singleton` is the pre-selected option

#### Scenario: Match choice does not overwrite the builder preference

- **WHEN** the player's builder format is `singleton` and they choose `standard` for a match
- **THEN** the persisted builder format remains `singleton` after the match

### Requirement: Live queue population per format

The format-selection step SHALL display, for each format, the number of players currently waiting in that format's random-match queue, derived from the Photon room list already delivered to the client.

#### Scenario: Population is shown per format

- **WHEN** the format-selection step is presented and the client holds a current room list
- **THEN** each format displays a count of waiting random-match rooms for that format

#### Scenario: Empty queue is shown as empty

- **WHEN** no random-match rooms exist for a given format
- **THEN** that format displays a count of zero rather than being hidden or disabled

#### Scenario: Population is unavailable before connection

- **WHEN** the format-selection step is presented before a room list has been received
- **THEN** counts are shown as unknown and the player can still choose any format

### Requirement: Deck selection reflects the chosen format

The deck-selection step SHALL indicate which of the player's saved decks are legal under the format chosen for the match, and SHALL prevent selecting an illegal deck for a match in that format.

#### Scenario: Illegal decks are visually distinguished

- **WHEN** deck selection is presented for a chosen format
- **THEN** decks that are illegal under that format are visually distinguished from legal decks

#### Scenario: Illegal deck cannot be taken into a match

- **WHEN** the player attempts to select a deck that is illegal under the chosen format
- **THEN** the selection is refused and the reason is surfaced to the player

#### Scenario: No legal deck is reported clearly

- **WHEN** the player has no deck that is legal under the chosen format
- **THEN** the client tells the player that no legal deck exists for that format rather than presenting an empty or silently unusable list
