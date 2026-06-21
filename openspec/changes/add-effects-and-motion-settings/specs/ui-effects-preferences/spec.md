## ADDED Requirements

### Requirement: Motion preference with three levels
The system SHALL provide a user-facing Motion preference with exactly three levels —
`full`, `reduced`, and `off` — that controls how much animation the app renders.
`full` permits all motion; `reduced` permits only functional one-shot feedback and
suppresses ambient, looping, and pointer-tracking effects; `off` suppresses all
non-essential animation so visual feedback resolves as instant state changes.

#### Scenario: Full motion renders all animation
- **WHEN** the Motion preference is `full`
- **THEN** ambient/looping effects, functional one-shot feedback, and pointer-tracking effects are all permitted to animate

#### Scenario: Reduced motion suppresses ambient effects
- **WHEN** the Motion preference is `reduced`
- **THEN** functional one-shot feedback (e.g. card-enter, security-reveal) still plays
- **AND** ambient, looping, and pointer-tracking effects do not animate

#### Scenario: Off removes non-essential animation
- **WHEN** the Motion preference is `off`
- **THEN** no non-essential animation plays and the corresponding visual changes apply instantly

### Requirement: Motion preference defaults from the OS reduced-motion setting
The system SHALL, on first run when no Motion preference has been persisted, derive
the default level from the operating system's `prefers-reduced-motion` setting:
`reduce` maps to a default of `reduced`, otherwise the default is `full`.

#### Scenario: OS requests reduced motion
- **WHEN** no Motion preference is persisted and the OS reports `prefers-reduced-motion: reduce`
- **THEN** the effective Motion level defaults to `reduced`

#### Scenario: OS has no reduced-motion request
- **WHEN** no Motion preference is persisted and the OS does not request reduced motion
- **THEN** the effective Motion level defaults to `full`

### Requirement: Motion and live-background preferences persist across sessions
The system SHALL persist the Motion level and the Live-background toggle, and MUST
restore the persisted values on relaunch; a missing or invalid persisted value MUST
fall back to the derived default rather than erroring.

#### Scenario: Persisted choice survives relaunch
- **WHEN** the user sets Motion to `reduced` and relaunches the app
- **THEN** the Motion preference is `reduced` after relaunch

#### Scenario: Invalid persisted value falls back to default
- **WHEN** the persisted Motion value is missing or not one of the three valid levels
- **THEN** the effective Motion level falls back to the OS-derived default

### Requirement: Effective motion is applied globally pre-paint
The system SHALL apply the effective Motion level to a `data-motion` attribute on the
document root before first paint, so styles keyed on the motion level resolve without
a flash of unintended motion, and MUST keep the attribute in sync when the preference
changes.

#### Scenario: Attribute set before hydration
- **WHEN** the app loads
- **THEN** the document root carries a `data-motion` attribute reflecting the effective level before React hydrates

#### Scenario: Attribute updates on change
- **WHEN** the user changes the Motion preference at runtime
- **THEN** the `data-motion` attribute updates to the new level without requiring a reload

### Requirement: The animation library honors the motion gate
The system SHALL gate the existing animation library behind the effective Motion
level: ambient and looping effects (including the app-shell CRT scan) MUST stop at
`reduced` and `off`, while functional one-shot feedback animations MUST remain at
`reduced` and be suppressed only at `off`.

#### Scenario: Ambient effect stops under reduced motion
- **WHEN** the Motion level is `reduced` or `off`
- **THEN** the app-shell CRT scan and other looping/ambient animations do not animate

#### Scenario: Functional feedback preserved under reduced motion
- **WHEN** the Motion level is `reduced`
- **THEN** functional one-shot feedback animations still play

### Requirement: Live background is gated by the motion level
The system SHALL render live (animated) background atmosphere only when the Motion
level is `full` and the Live-background toggle is on; at `reduced` or `off`, or when
the toggle is off, the background MUST fall back to its static rendering regardless of
the stored toggle value.

#### Scenario: Live background on at full motion
- **WHEN** the Motion level is `full` and the Live-background toggle is on
- **THEN** the live animated background is permitted to render

#### Scenario: Live background forced static under reduced motion
- **WHEN** the Motion level is `reduced` or `off`
- **THEN** the background renders statically even if the Live-background toggle is on

### Requirement: Preferences are exposed in Graphics Settings
The system SHALL surface the Motion level control and the Live-background toggle in
the desktop Graphics Settings page alongside the existing Theme and Fullscreen
controls.

#### Scenario: Controls visible in settings
- **WHEN** the user opens the Graphics Settings page
- **THEN** a Motion level control and a Live-background toggle are shown

#### Scenario: Changing a control takes effect immediately
- **WHEN** the user changes the Motion level or Live-background toggle in Graphics Settings
- **THEN** the change applies immediately without restarting the app

### Requirement: Effective motion level is readable by other modules
The system SHALL expose the effective Motion level through a shared
selector/helper so that other features (cursor lighting, live atmosphere, the
digivolve cut-in) read a single source of truth rather than re-deriving it.

#### Scenario: Downstream feature reads the level
- **WHEN** another feature needs to decide whether to animate
- **THEN** it can read the effective Motion level from the shared helper/store selector
