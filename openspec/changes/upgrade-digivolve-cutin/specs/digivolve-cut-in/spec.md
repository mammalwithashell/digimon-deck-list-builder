## ADDED Requirements

### Requirement: Multi-phase digivolution cut-in at full motion
The system SHALL, at motion level `full`, present a multi-phase digivolution cut-in
consisting of a wireframe/grid sweep over the evolving card art, an orbiting ring
spin-up, and a flash reveal of the new card and name.

#### Scenario: Full cut-in on digivolve
- **WHEN** a digivolve event occurs and the motion level is `full`
- **THEN** the cut-in plays the wireframe sweep, then the orbit ring, then the reveal of the new card and name

### Requirement: Cut-in is tinted by card color and theme
The system SHALL tint the cut-in using the digivolving card's color and the active
theme so it reads correctly in both the dark and light themes.

#### Scenario: Color/theme tint applied
- **WHEN** the cut-in plays
- **THEN** its wireframe, ring, and flash are tinted by the digivolving card's color and remain legible in the active theme

### Requirement: Cut-in reuses the existing digivolve event and lifecycle
The system SHALL drive the cut-in from the existing `digivolve` event on the game event
stream, deduping by event sequence so each digivolve shows once, and MUST NOT change
how or when the digivolve event is emitted.

#### Scenario: One cut-in per digivolve
- **WHEN** a single digivolve event is received
- **THEN** exactly one cut-in plays for it (deduped by sequence) and auto-dismisses on its timer

#### Scenario: No event plumbing change
- **WHEN** the cut-in is implemented
- **THEN** the digivolve event emission and game logic are unchanged

### Requirement: Cut-in degrades by motion level
The system SHALL degrade the cut-in by motion level: the full sequence plays at `full`;
at `reduced` it falls back to a simple banner that still identifies the digivolved card;
at `off` the result is shown instantly/minimally with no animated sequence.

#### Scenario: Reduced motion shows simple banner
- **WHEN** a digivolve event occurs and the motion level is `reduced`
- **THEN** the simple banner (no cinematic sequence) plays and still shows what digivolved

#### Scenario: Off shows instant result
- **WHEN** a digivolve event occurs and the motion level is `off`
- **THEN** no animated cut-in plays

### Requirement: Cut-in is a non-interactive, self-cleaning overlay
The system SHALL render the cut-in as a non-interactive overlay (`pointer-events: none`)
that auto-dismisses, can be dismissed early by click as today, clears its timers, and
never persists or blocks board input.

#### Scenario: Does not block the board
- **WHEN** the cut-in is on screen
- **THEN** it does not intercept input to the board and is removed after its show window
