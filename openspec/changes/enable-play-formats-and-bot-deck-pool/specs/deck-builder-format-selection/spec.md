## MODIFIED Requirements

### Requirement: Format selection in the deck builder

The deck builder SHALL let the user choose the format for the deck being edited from the formats reported by the engine's `list_formats()`, SHALL persist the chosen format with the deck (`game_mode`), and SHALL restore it when the deck is reopened. The frontend SHALL NOT hardcode a playable format list; both the deck-builder format selector and the play/format queue SHALL be populated from the engine registry-backed format catalog.

#### Scenario: Choosing a format persists it

- **WHEN** the user selects EDEN Singleton in the deck builder and saves the deck
- **THEN** the deck is stored with `game_mode = "eden_singleton"` and reopening the deck shows EDEN Singleton selected

#### Scenario: Catalog matches engine

- **WHEN** the play/format catalog renders
- **THEN** the available formats and their identifiers come from the engine's `list_formats()` rather than a hardcoded frontend list

#### Scenario: Play queue enables registry-playable formats

- **WHEN** the play format queue renders in desktop or hosted/browser mode
- **THEN** Standard, No Banlist, Pauper, EDEN, and EDEN Singleton are selectable when those formats are marked playable by the engine registry

#### Scenario: Updating an existing deck preserves the format

- **WHEN** an existing deck's card list is updated and saved
- **THEN** the deck's stored `game_mode` is retained (the prior bug where the browser update path dropped `game_mode` is fixed)
