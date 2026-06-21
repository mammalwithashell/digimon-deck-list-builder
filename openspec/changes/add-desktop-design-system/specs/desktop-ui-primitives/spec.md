## ADDED Requirements

### Requirement: A theme-agnostic primitive component library exists

The client SHALL provide a library of reusable React primitive components under
the design module. Call sites SHALL NOT branch on the active theme to produce
chrome; the structural markup and props are shared across themes, and per-theme
chrome differences are expressed entirely in CSS keyed on `[data-theme]`.

#### Scenario: One component renders both chromes from identical props

- **WHEN** a `Window` primitive is rendered with the same props under each theme
- **THEN** in the dark theme it renders the terminal-frame chrome
- **AND** in the light theme it renders the Windows-95 window chrome
- **AND** the call site passes no theme-conditional chrome prop

#### Scenario: Primitives consume design tokens only

- **WHEN** a primitive component is styled
- **THEN** its themeable colors and chrome reference design role tokens (not page-local palettes or theme-specific hex)

### Requirement: Core structural primitives are provided

The library SHALL provide at least these structural primitives: `Backdrop`,
`Frame`/`Panel`, `Window` (composed of `TitleBar` + body), `StatusBar`,
`NavRail`, `Screen`, and `Button` with primary / ghost / accent / danger
variants.

#### Scenario: Button variant renders accent chrome per theme

- **WHEN** a `Button` with the `primary` variant is rendered
- **THEN** it renders the active theme's accent treatment (phosphor fill in dark, title-blue bevel in light)

#### Scenario: Window composes a title bar and body

- **WHEN** a `Window` is rendered with a title and children
- **THEN** it renders a `TitleBar` carrying the title and a panel body containing the children

### Requirement: Domain primitives are provided

The library SHALL provide at least these domain primitives: `AnalyzerFrame`
(with a Digimon-sprite art slot), `CardTile`, `CardBack`/`CardSleeve`,
`DigimonSprite`, `StatChip`, `MemoryGauge`, `Badge`, `DeckColorBadge`, and
`ThemeSwitch`.

#### Scenario: AnalyzerFrame renders a sprite in its art slot

- **WHEN** an `AnalyzerFrame` is given a Digimon sprite reference and stat values
- **THEN** it renders the active theme's analyzer chrome with the sprite shown in the art slot and the stats in stat chips

### Requirement: An in-app style-guide route renders every primitive in both themes

The desktop client SHALL provide a `/style-guide` route that renders every
primitive component in both the dark and light themes side by side. The route
MUST be reachable only in the desktop build (`VITE_BUILD_TARGET === 'desktop'`).

#### Scenario: Style guide shows all primitives in both themes

- **WHEN** the user navigates to `/style-guide` in the desktop build
- **THEN** each primitive component is shown rendered in the dark theme and in the light theme

#### Scenario: Style guide is absent from non-desktop builds

- **WHEN** the application runs in a non-desktop build
- **THEN** the `/style-guide` route is not reachable
