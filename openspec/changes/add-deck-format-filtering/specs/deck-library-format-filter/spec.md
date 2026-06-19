## ADDED Requirements

### Requirement: Filter the Deck Library by format

The Deck Library SHALL provide a format filter that narrows the displayed decks to a single format (`game_mode`), defaulting to "all formats". The filter SHALL be presented in two places — a selectable "Formats" list in the library sidebar and a dropdown in the library toolbar — both bound to a single shared selection so that changing one updates the other. The format filter SHALL compose (AND) with the existing folder and search filters.

#### Scenario: Filter to a single format

- **WHEN** the user selects the EDEN format in the library
- **THEN** only decks whose `game_mode` is `eden` are shown, and decks of other formats are hidden

#### Scenario: Sidebar and toolbar stay in sync

- **WHEN** the user picks a format from the toolbar dropdown
- **THEN** the corresponding sidebar "Formats" entry shows as active (and vice-versa), reflecting the same single selection

#### Scenario: All formats is the default

- **WHEN** the library first loads, or the user chooses "All formats"
- **THEN** decks of every format are shown (the format filter is a no-op)

#### Scenario: Format filter composes with folder and search

- **WHEN** a folder is active and a search term is entered and a format is selected
- **THEN** the result is the set of decks matching the folder AND the search AND the selected format

### Requirement: Per-format deck counts in the sidebar

The sidebar "Formats" list SHALL display, for each listed format, the number of decks in that format across the whole library (independent of the active folder), and SHALL include an "All formats" entry showing the total deck count. The list SHALL include each format that has at least one deck, labeled by its engine-registry display name, falling back to the raw `game_mode` value for any format not present in the registry. If the currently selected format has no decks (for example after the last such deck is deleted), the filter SHALL fall back to "All formats".

#### Scenario: Counts reflect the library

- **WHEN** the library contains 3 Standard decks and 1 Pauper deck
- **THEN** the sidebar shows Standard with a count of 3 and Pauper with a count of 1, and "All formats" with a count of 4

#### Scenario: Legacy or unknown format is still listed

- **WHEN** a deck has a `game_mode` not present in the engine registry
- **THEN** that format still appears in the list (labeled by its raw `game_mode`) and is selectable

#### Scenario: Selected format empties out

- **WHEN** the user is filtered to a format and deletes the last deck of that format
- **THEN** the filter falls back to "All formats" so no empty, unselectable view is shown

### Requirement: Deck format is displayed and searchable

Each deck in the library SHALL display its format as a pill on the deck tile and on the selected-deck detail banner, using the engine-registry display name (falling back to the raw `game_mode`). The library search SHALL match against a deck's `game_mode`.

#### Scenario: Format pill shown on tile and banner

- **WHEN** a deck with `game_mode = "pauper"` is rendered in the library
- **THEN** its tile and the detail banner each show a format pill reading the Pauper display name

#### Scenario: Search matches format

- **WHEN** the user types "eden" into the library search
- **THEN** decks whose `game_mode` is `eden` (or `eden_singleton`) are included in the results
