## ADDED Requirements

### Requirement: Deck Library primary actions live in the top bar

The Deck Library top bar SHALL present the primary actions — Home, Import, and New Deck — and SHALL NOT render a separate descriptor panel duplicating the page title. Import SHALL open the builder's import flow.

#### Scenario: Top-bar actions

- **WHEN** the Deck Library is shown
- **THEN** the top bar offers Home, Import, and New Deck
- **AND** there is no "armory" descriptor panel between the deck banner and the search toolbar

#### Scenario: Import opens the import flow

- **WHEN** the user activates Import
- **THEN** the builder opens with its import flow ready (the `?import=1` entry)

### Requirement: Double-clicking a deck opens it for editing

A deck tile in the Deck Library SHALL open that deck in the builder when double-clicked, while a single click SHALL only select it (loading its preview/analytics).

#### Scenario: Double-click opens the builder

- **WHEN** the user double-clicks a deck tile
- **THEN** the builder opens for that deck

#### Scenario: Single click selects without leaving the library

- **WHEN** the user single-clicks a deck tile
- **THEN** the deck becomes the selected deck (its banner and analytics load) and the library stays open

#### Scenario: Double-clicking the pin control does not open the builder

- **WHEN** the user double-clicks a deck tile's pin control
- **THEN** the builder does not open
