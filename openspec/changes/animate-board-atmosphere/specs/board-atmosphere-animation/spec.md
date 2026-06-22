## ADDED Requirements

### Requirement: Board atmosphere layers animate when gated on
The system SHALL animate the in-game board's existing atmosphere layers when effective
live-background is on: the corner binary becomes calm digital rain, the scanlines roll
slowly, and the grid mat drifts slowly.

#### Scenario: Board comes alive during a match
- **WHEN** the user is on the in-game board with effective live-background on
- **THEN** the binary rain, scanline roll, and grid drift animate

### Requirement: Board atmosphere reuses the shared engine
The system SHALL render the board's digital rain via the shared `LiveAtmosphere`
engine's board surface variant rather than a separate rain implementation.

#### Scenario: Single rain implementation
- **WHEN** the board rain renders
- **THEN** it is produced by the shared atmosphere engine (board surface), not a duplicate renderer

### Requirement: Atmosphere stays behind gameplay
The system SHALL keep the board atmosphere strictly behind gameplay: permanents,
board chrome, the memory gauge, and event-driven VFX (digivolve/battle/security/phase)
MUST all render above the atmosphere, and the atmosphere MUST NOT reduce the legibility
of the board state.

#### Scenario: Cards and VFX render above atmosphere
- **WHEN** atmosphere is animating and a permanent or an event VFX is on screen
- **THEN** the permanent / VFX renders above the atmosphere and remains fully legible

### Requirement: Board atmosphere is subtler than menu atmosphere
The system SHALL tune the board atmosphere to a lower intensity than the menu
atmosphere so it reads as texture rather than focus during play.

#### Scenario: Lower intensity on the board
- **WHEN** the board atmosphere animates
- **THEN** its rain density and scanline/drift amplitude are lower than the menu defaults

### Requirement: Gated off renders today's static board
The system SHALL render the board's current static atmosphere (no animation) when
effective live-background is off (motion `reduced`/`off` or the toggle off), with no
visual regression from the pre-change board.

#### Scenario: Static fallback equals current board
- **WHEN** effective live-background is off on the board
- **THEN** the board atmosphere renders statically, matching the pre-change look

### Requirement: Board atmosphere respects the fixed-canvas performance budget
The system SHALL size the rain canvas to the board's fixed internal resolution (scaled
with the rest of the board), cap its frame rate, and pause when the document is hidden,
so animation cost is bounded and does not reflow.

#### Scenario: Fixed-size, capped, paused-when-hidden
- **WHEN** the board atmosphere is active
- **THEN** the canvas matches the internal board resolution, updates at a capped frame rate, and stops while the document is hidden

#### Scenario: No layout reflow
- **WHEN** the window is resized
- **THEN** the board atmosphere scales with the board and does not reflow the board layout
