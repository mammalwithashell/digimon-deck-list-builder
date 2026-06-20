## ADDED Requirements

### Requirement: Role-based token layer is the single source of visual truth

The client SHALL express all themeable color, typography, and chrome-treatment
values as a set of semantic CSS custom properties ("role tokens") defined on the
document root. In-scope components SHALL consume role tokens only and MUST NOT
hardcode hex colors or reference page-local palettes (`--ib-*`, `--bld-*`,
`--bg-*`) for themeable values. Every theme MUST provide a value for every role
token so no token resolves empty.

#### Scenario: A primitive resolves color from role tokens

- **WHEN** a primitive component styles a surface using `var(--surface)` and text using `var(--ink-0)`
- **THEN** the rendered values resolve to the active theme's values for those roles
- **AND** no theme-specific hex literal appears in the component's styles

#### Scenario: Token completeness across themes

- **WHEN** the active theme is dark or light
- **THEN** every role token defined by the system resolves to a non-empty value in that theme

### Requirement: Two named themes exist with complete value sets

The system SHALL define exactly two themes — `dark` (Digi-OS) and `light`
(Adventure '99) — each supplying a complete set of role-token values. The dark
theme is the default. The dark theme expresses a phosphor/amber CRT terminal
language (angular framing, scanlines); the light theme expresses a beige
Windows-95 / DIGITALMONSTER-analyzer language (chunky bevels, square corners).

#### Scenario: Default theme on a fresh install

- **WHEN** the application starts with no persisted theme preference
- **THEN** the active theme is `dark`

#### Scenario: Both themes are selectable

- **WHEN** the theme is set to `light`
- **THEN** the active theme is `light` and all role tokens resolve to light-theme values

### Requirement: Theme is selected via a data-theme attribute on the document root

Theme selection SHALL be carried by a `data-theme` attribute on the root
`<html>` element. Per-theme chrome SHALL be expressed in CSS keyed on
`[data-theme="dark"]` / `[data-theme="light"]`. Changing the active theme MUST
update the attribute and restyle the visible UI without a page reload or
component re-mount.

#### Scenario: Switching theme updates the root attribute

- **WHEN** the user switches the theme to `light`
- **THEN** `document.documentElement` carries `data-theme="light"`
- **AND** visible surfaces restyle to the light theme without a reload

### Requirement: Theme is applied before first paint

The persisted theme SHALL be read and applied to the `data-theme` attribute
before the application's first paint, so the user never sees a flash of the
wrong theme on launch. Reading the persisted value MUST NOT depend on React
having mounted.

#### Scenario: Persisted light theme shows no dark flash

- **WHEN** the persisted theme is `light` and the application launches
- **THEN** the first painted frame is already in the light theme
- **AND** no dark-themed frame is shown before it

#### Scenario: No persisted value applies the default pre-paint

- **WHEN** there is no persisted theme and the application launches
- **THEN** `data-theme="dark"` is set before first paint

### Requirement: Theme preference persists across launches

The selected theme SHALL be persisted (via `localStorage`) and restored on the
next launch. On first launch with no persisted value, the default MUST be `dark`
and that default MUST be persisted.

#### Scenario: Selection survives a relaunch

- **WHEN** the user selects the light theme, then closes and relaunches the app
- **THEN** the app restores the light theme on launch

#### Scenario: First launch persists the default

- **WHEN** the app starts and no persisted theme exists
- **THEN** the dark theme is applied and persisted

### Requirement: Game-identity and vendored-art colors are theme-stable

Player and opponent identity colors SHALL remain the same hue family in both
themes — player in the orange family, opponent in the blue family — tuned per
theme only for contrast against that theme's background. Vendored pixel sprites
and card sleeves SHALL NOT be recolored by the active theme.

#### Scenario: Switching theme on the board keeps team colors

- **WHEN** the user switches themes while the game board is visible
- **THEN** player elements remain in the orange family and opponent elements remain in the blue family
- **AND** any rendered Digimon sprite's pixels are unchanged by the switch

### Requirement: In-scope surfaces respond to the active theme

In-scope surfaces SHALL render in the active theme: the launcher, the game
board, and the legacy pages whose chrome is replaced by this change. Switching
the theme MUST update them live, without navigation.

#### Scenario: Launcher reflects a live theme switch

- **WHEN** the user switches the theme while on the launcher
- **THEN** the launcher chrome updates to the new theme without navigating away

#### Scenario: Board reflects a live theme switch

- **WHEN** the user switches the theme while the game board is visible
- **THEN** the board chrome updates to the new theme
- **AND** the in-game player/opponent colors persist per the theme-stable rule

### Requirement: A ThemeSwitch control is reachable

The client SHALL expose a `ThemeSwitch` control in the launcher top bar and in
the settings area, allowing the user to toggle between the two themes. Toggling
MUST apply and persist the new theme.

#### Scenario: User toggles theme from the launcher

- **WHEN** the user activates the ThemeSwitch in the launcher top bar
- **THEN** the active theme flips to the other theme and the preference is persisted

### Requirement: Dark-theme CRT motion respects reduced-motion

Looping CRT atmosphere animations in the dark theme SHALL be disabled when the
user agent reports `prefers-reduced-motion: reduce` — for example scanline
flicker and grid-floor drift.

#### Scenario: Reduced motion disables CRT animation

- **WHEN** the user agent reports `prefers-reduced-motion: reduce` and the dark theme is active
- **THEN** no looping CRT atmosphere animation plays
