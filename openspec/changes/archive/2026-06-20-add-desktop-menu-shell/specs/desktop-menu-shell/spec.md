## ADDED Requirements

### Requirement: Persistent shell across menu screens
The desktop build SHALL render a single persistent navigation shell around every menu route (Home, Play, Decks, Patch Notes, Graphics, Models), and SHALL NOT render that shell on the in-game board route, which remains full-bleed.

#### Scenario: Rail persists across menu navigation
- **WHEN** the user navigates from Home to Play to Decks in the desktop app
- **THEN** the navigation rail remains visible the whole time without unmounting

#### Scenario: Board route is full-bleed
- **WHEN** the user opens an active match at `/game/:id`
- **THEN** the navigation shell is not rendered and the board fills the canvas

### Requirement: Route-driven active indication
The active rail item SHALL reflect the current route and SHALL be highlighted using the `--accent` token, rendered theme-appropriately: a neon-green accent in dark and a solid navy fill in light.

#### Scenario: Active item tracks the route
- **WHEN** the user is on the `/play` route
- **THEN** the Play rail item is marked active (`aria-current="page"`) and the Home item is not

#### Scenario: Accent color follows the theme
- **WHEN** the active item is shown in dark theme versus light theme
- **THEN** dark renders the `--accent` green accent bar and light renders the solid navy `--accent` block with `--accent-ink` text

### Requirement: Collapsible rail with persisted state
The rail SHALL provide a collapse control that reduces it to an icon-only strip, and SHALL persist the collapsed-or-expanded choice so it is restored on the next launch.

#### Scenario: Collapsing the rail
- **WHEN** the user activates the collapse toggle
- **THEN** the rail reduces to an icon-only strip and item labels are hidden

#### Scenario: Collapse state survives relaunch
- **WHEN** the user collapses the rail and reopens the app
- **THEN** the rail is restored to the collapsed state it was left in

### Requirement: Page-contributed contextual sub-navigation
The shell SHALL allow a page to contribute contextual sub-navigation rendered inside the rail, and SHALL remove that contributed sub-navigation when the page unmounts. The deck library SHALL present its Folders and Formats through this mechanism instead of a separate in-page sidebar, and selecting an entry SHALL filter the displayed deck list.

#### Scenario: Deck library contributes Folders/Formats to the rail
- **WHEN** the user opens the deck library
- **THEN** Folders and Formats appear as collapsible sub-sections under Decks in the rail, and the page no longer shows its own separate sidebar

#### Scenario: Selecting a rail folder filters the decks
- **WHEN** the user selects a folder or format entry in the rail
- **THEN** the deck list is filtered to that folder or format

#### Scenario: Contributed sub-nav is removed on leaving
- **WHEN** the user navigates away from the deck library
- **THEN** the Folders/Formats sub-sections are removed from the rail

### Requirement: Themed window chrome on menu panels
Menu-screen content panels SHALL use the design-system `Window` chrome — a green terminal header in dark and a navy title bar in light — on the launcher Play and My-Decks panels, the Play/format chooser, and the deck-library panels. The deck builder / analyzer SHALL remain unchanged.

#### Scenario: Launcher panels carry themed headers
- **WHEN** the launcher home screen is shown
- **THEN** the Play and My-Decks panels render a green `Window` header in dark theme and a navy title bar in light theme

#### Scenario: Deck builder is untouched
- **WHEN** the user opens the deck builder / analyzer
- **THEN** its existing layout and styling are unchanged by this capability

### Requirement: Account and theme controls live in the rail
The shell SHALL present the signed-in identity and the theme toggle inside the navigation rail itself (not a separate content-area header bar), and the rail's collapse control SHALL sit at the top of the rail. The redundant top header bar SHALL NOT be rendered, since the custom title bar already owns the top chrome and the active rail item indicates the current page.

#### Scenario: Account and theme in the rail, no content header
- **WHEN** any menu screen is shown
- **THEN** the "signed in as" label and the theme toggle appear within the rail, the collapse control is at the top of the rail, and no separate header bar is rendered above the page content

### Requirement: Fullscreen hides the title bar and fills the display
The desktop build SHALL hide the custom title bar while the window is in fullscreen and SHALL reserve no top space for it, so the scaled canvas fills the entire display; leaving fullscreen SHALL restore the title bar and the selected window preset.

#### Scenario: Entering fullscreen
- **WHEN** the user enables fullscreen
- **THEN** the custom title bar is not rendered and the canvas fills the full display height with no reserved title-bar strip

#### Scenario: Leaving fullscreen
- **WHEN** the user disables fullscreen
- **THEN** the custom title bar is shown again and the window returns to the selected resolution preset

### Requirement: Reuse of design-system primitives and accent token
The shell SHALL be built from the existing design-system `NavRail` and `Window` components and SHALL source active-state and window-header coloring from the shared `--accent` family, with no remaining `--player`-based active styling in the menu rail, so the app matches the style guide in both themes.

#### Scenario: Active styling uses the shared accent token
- **WHEN** the active rail item is rendered in either theme
- **THEN** its highlight derives from `--accent` (not `--player`), matching the design-system NavRail in the style guide
