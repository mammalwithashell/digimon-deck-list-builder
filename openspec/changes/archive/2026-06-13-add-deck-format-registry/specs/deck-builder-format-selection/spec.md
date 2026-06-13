## ADDED Requirements

### Requirement: Format selection in the deck builder

The deck builder SHALL let the user choose the format for the deck being edited from the formats reported by the engine's `list_formats()`, SHALL persist the chosen format with the deck (`game_mode`), and SHALL restore it when the deck is reopened. The frontend SHALL NOT hardcode a format list; the play/format catalog SHALL be populated from `list_formats()`.

#### Scenario: Choosing a format persists it

- **WHEN** the user selects EDEN Singleton in the deck builder and saves the deck
- **THEN** the deck is stored with `game_mode = "eden_singleton"` and reopening the deck shows EDEN Singleton selected

#### Scenario: Catalog matches engine

- **WHEN** the play/format catalog renders
- **THEN** the available formats and their identifiers come from the engine's `list_formats()` rather than a hardcoded frontend list

#### Scenario: Updating an existing deck preserves the format

- **WHEN** an existing deck's card list is updated and saved
- **THEN** the deck's stored `game_mode` is retained (the prior bug where the browser update path dropped `game_mode` is fixed)

### Requirement: Format-aware card pool filtering and badges

The deck builder card pool SHALL offer a filter that restricts results to cards legal in the selected format, using the engine's per-card legality query. Each card SHALL display its format-specific status (e.g. legal, banned, copy limit, counts toward anomaly limit) using engine-provided legality data, with no format rules re-implemented in the frontend.

#### Scenario: Filter to legal cards

- **WHEN** the user enables the "format-legal only" filter under Pauper
- **THEN** the pool shows only common and uncommon cards (plus Digi-Eggs) legal in Pauper

#### Scenario: Per-card limit badge

- **WHEN** a deck is being built under EDEN Singleton
- **THEN** each pool card shows a maximum-copies indicator of 1 sourced from the engine legality query

#### Scenario: Banned-card indication

- **WHEN** a card is banned in the selected format
- **THEN** the builder marks it as banned and prevents adding it (or flags it) based on the engine legality result

### Requirement: Format-aware validation and play

The deck builder's validate action SHALL validate against the selected format, and saving SHALL store the deck under that format. All five formats (Standard, No Banlist, Pauper, EDEN, EDEN Singleton) SHALL be queueable in matchmaking, with players matched within the same format.

#### Scenario: Validate respects selected format

- **WHEN** the user validates a deck with EDEN selected
- **THEN** validation runs under EDEN rules (anomaly protocol, EDEN banlist) and reports EDEN-specific results

#### Scenario: Queue an EDEN deck

- **WHEN** a player queues for a match with an EDEN deck
- **THEN** matchmaking accepts `game_mode = "eden"` and pairs them only with other EDEN-format queue entries

#### Scenario: New formats accepted by persistence layer

- **WHEN** a deck is saved with `game_mode` of `pauper` or `eden_singleton`
- **THEN** the database accepts it (the game-mode check constraints include the new modes)
