## ADDED Requirements

### Requirement: Game creation accepts optional seed input

The system SHALL allow bot-game and room-match creation flows to accept an optional user-provided seed. When no seed is provided, the system SHALL generate a fresh random seed before initializing the engine.

#### Scenario: Bot game starts with generated seed

- **WHEN** a user starts a bot game without entering a seed
- **THEN** the game is initialized with a generated random seed
- **AND** the create-game response includes the effective seed as a decimal string

#### Scenario: Bot game starts with explicit seed

- **WHEN** a user starts a bot game with a valid seed value
- **THEN** the engine is initialized with that exact seed
- **AND** starting the same decks with the same seed produces the same initial deck order, opening hands, and initial player

#### Scenario: Invalid seed is rejected

- **WHEN** a user submits a seed that is not a base-10 integer in the inclusive `u64` range
- **THEN** the game is not created
- **AND** the user receives a validation error that explains the accepted seed format

### Requirement: Effective seed is visible and copyable in game

The system SHALL display the effective seed for an active bot or room game after the game is created. The displayed seed SHALL match the seed used to initialize the engine and SHALL be copyable from the in-game surface and the terminal result overlay.

#### Scenario: Player copies seed during active game

- **WHEN** a player is viewing an active created game
- **THEN** the game surface displays the effective seed
- **AND** activating the seed copy control writes the exact decimal seed string to the clipboard

#### Scenario: Player copies seed after result

- **WHEN** a game has ended and the result overlay is visible
- **THEN** the result overlay displays the same effective seed shown during the game
- **AND** activating the seed copy control writes the exact decimal seed string to the clipboard

### Requirement: Room matches expose seed controls

The room-match host SHALL be able to set, replace, or clear an explicit seed before the room match starts. Room state SHALL expose whether the pending match is using an explicit seed or generated-seed mode.

#### Scenario: Host sets room seed before start

- **WHEN** the host enters a valid seed before starting the room match
- **THEN** the room state records that seed as the pending explicit seed
- **AND** both players can see the seed value before or when the match starts

#### Scenario: Host clears room seed before start

- **WHEN** the host clears the pending explicit seed
- **THEN** the room returns to generated-seed mode
- **AND** starting the room creates and returns a fresh effective seed

### Requirement: Room explicit seed determines initial player

The room-match start flow SHALL treat an explicit seed as authoritative for the full initial setup, including initial player. While an explicit room seed is set, the first-player selector SHALL be disabled or read-only in the UI, and the server SHALL NOT mutate the explicit seed to satisfy a separate first-player choice.

#### Scenario: Explicit room seed starts unchanged

- **WHEN** the host starts a room match with an explicit seed
- **THEN** the server initializes the game with that exact seed
- **AND** the initial player is the one produced by that seed

#### Scenario: Generated room seed honors first-player selector

- **WHEN** the host starts a room match without an explicit seed and has selected a first-player option
- **THEN** the server generates an effective seed compatible with the selected first-player behavior
- **AND** the response exposes the generated effective seed

### Requirement: Seed metadata uses lossless wire format

All frontend-facing bot-game and room-match APIs SHALL expose effective seeds as decimal strings. The system MAY accept safe integer values for backward compatibility, but it SHALL preserve and return the effective seed losslessly as a string.

#### Scenario: Large random seed crosses frontend boundary

- **WHEN** the backend or Tauri command generates a seed greater than `Number.MAX_SAFE_INTEGER`
- **THEN** the frontend receives and stores that seed as a decimal string
- **AND** copying and reusing the string initializes a later game with the same seed
