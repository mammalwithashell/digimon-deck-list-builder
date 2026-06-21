## ADDED Requirements

### Requirement: Live animated background behind the menu shell
The system SHALL render an animated, non-interactive background behind the desktop menu
shell content when effective live-background is on, layered below content and with
`pointer-events: none`.

#### Scenario: Animated background on menus
- **WHEN** the user is on a menu route with effective live-background on
- **THEN** an animated background renders behind the content and does not intercept input

### Requirement: Dark "Digi-OS" rain variant is calm by default
The system SHALL render the dark theme variant as a slow, sparse digital rain plus a
faint drifting grid, tuned to feel ambient rather than a fast screensaver, with rain
speed and density as explicit tunables defaulting to slow/sparse.

#### Scenario: Calm rain in dark theme
- **WHEN** the active theme is dark and effective live-background is on
- **THEN** digital rain renders at the calm default speed/density

### Requirement: Light "Adventure '99" idle-desktop variant
The system SHALL render the light theme variant as an idle laptop desktop: a slowly
breathing teal gradient, a gently parallaxing dot-grid, a blinking terminal cursor, and
an occasional analyzer sweep line.

#### Scenario: Idle desktop in light theme
- **WHEN** the active theme is light and effective live-background is on
- **THEN** the idle-desktop scene renders (breathing gradient, parallax dots, blink cursor, occasional sweep)

### Requirement: Background is gated and falls back to static
The system SHALL animate the background only when effective live-background is on
(motion `full` and the Live-background toggle on); otherwise it MUST render a static
version of the same scene with no animation.

#### Scenario: Static fallback when gated off
- **WHEN** effective live-background is off (motion `reduced`/`off`, or the toggle off)
- **THEN** the background renders statically with no animation

#### Scenario: Live resumes when gate turns on
- **WHEN** effective live-background turns on at runtime
- **THEN** the background begins animating without a reload

### Requirement: Animation pauses when the document is hidden
The system SHALL pause the background animation while the document/tab is hidden and
resume it when the document becomes visible again.

#### Scenario: Pause on hide
- **WHEN** the document becomes hidden
- **THEN** the background animation loop stops

#### Scenario: Resume on show
- **WHEN** the document becomes visible again
- **THEN** the background animation resumes

### Requirement: Atmosphere engine is reusable across surfaces
The system SHALL implement the atmosphere as a reusable component that distinguishes a
menu surface from a board surface, so the in-game board change can drive its atmosphere
from the same engine.

#### Scenario: Board surface reuse
- **WHEN** the engine is rendered with the board surface variant
- **THEN** it produces board-appropriate atmosphere from the same component without duplicating the rain implementation

### Requirement: No duplicate static atmosphere
The system SHALL ensure the live atmosphere is the single source of menu-shell
background atmosphere, so the pre-existing static `.ds-backdrop` scanline/grid styling
is not rendered on top of or beneath the live layer simultaneously.

#### Scenario: Single atmosphere layer
- **WHEN** the live atmosphere is active on a menu route
- **THEN** the legacy static `.ds-backdrop` atmosphere does not also render, avoiding doubled scanlines/grid
