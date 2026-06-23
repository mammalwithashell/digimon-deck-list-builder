## ADDED Requirements

### Requirement: Cursor-tracked light in the menu shell
The system SHALL render a soft light that follows the pointer across the desktop menu
shell, updating its position as the pointer moves.

#### Scenario: Light follows the pointer
- **WHEN** the pointer moves within the menu shell at motion `full`
- **THEN** the light's center tracks the pointer position

#### Scenario: Light is present on menu routes
- **WHEN** the user is on any menu route (Home, Play, Decks, Patch Notes, Graphics, Models) at motion `full`
- **THEN** the cursor-follow light is rendered

### Requirement: Light is tinted per theme
The system SHALL tint the cursor light to match the active theme: the dark "Digi-OS"
theme uses an electric accent halo and the light "Adventure '99" theme uses a soft
neutral/teal sheen.

#### Scenario: Dark theme tint
- **WHEN** the active theme is dark and motion is `full`
- **THEN** the light renders with the dark-theme accent tint

#### Scenario: Light theme tint
- **WHEN** the active theme is light and motion is `full`
- **THEN** the light renders with the light-theme sheen tint

### Requirement: Light honors the motion preference
The system SHALL render the cursor-follow light only when the effective motion level is
`full`, and MUST NOT render or track the pointer at `reduced` or `off`.

#### Scenario: Suppressed under reduced/off motion
- **WHEN** the effective motion level is `reduced` or `off`
- **THEN** no cursor-follow light is rendered and no pointer tracking occurs

#### Scenario: Re-enabled when returning to full
- **WHEN** the motion level changes back to `full`
- **THEN** the cursor-follow light renders and resumes tracking without a reload

### Requirement: Light never intercepts input or harms legibility
The system SHALL render the cursor light as a non-interactive overlay
(`pointer-events: none`) that does not block clicks/hovers and does not reduce the
contrast/legibility of foreground content.

#### Scenario: Clicks pass through
- **WHEN** the user clicks a menu control beneath the light
- **THEN** the click reaches the control as if the light were not present

### Requirement: Light is excluded from the in-game board
The system SHALL NOT render the cursor-follow light on the full-bleed in-game board
route in this change.

#### Scenario: No light during a match
- **WHEN** the user is on the in-game board route
- **THEN** the cursor-follow light is not rendered

### Requirement: Pointer tracking does not trigger per-move React renders
The system SHALL update the light position without causing a React re-render on each
pointer move (e.g. via CSS custom properties updated in a throttled handler).

#### Scenario: Throttled, render-free tracking
- **WHEN** the pointer moves continuously
- **THEN** the light position updates at most once per animation frame and the menu component tree does not re-render per move
