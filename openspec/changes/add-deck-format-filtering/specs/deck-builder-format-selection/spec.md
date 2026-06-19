## MODIFIED Requirements

### Requirement: Format selection in the deck builder

The deck builder SHALL let the user choose the format for the deck being edited from the formats reported by the engine's `list_formats()`, SHALL persist the chosen format with the deck (`game_mode`), and SHALL restore it when the deck is reopened. The frontend SHALL NOT hardcode a format list; both the deck-builder format selector and the play "CHOOSE FORMAT" window SHALL be populated from `list_formats()` (via the `/decks/formats` endpoint on the hosted runtime and the `rust_list_formats` Tauri command on desktop). The play "CHOOSE FORMAT" window SHALL render exactly the playable formats reported by the registry and SHALL NOT display non-playable concept placeholders that are absent from the registry. The previously drifted mock catalog — the hosted `/formats` route and the desktop `formats_list` command — SHALL be removed (or repointed at the registry) so that the format registry is the single source of truth across both surfaces.

#### Scenario: Choosing a format persists it

- **WHEN** the user selects EDEN Singleton in the deck builder and saves the deck
- **THEN** the deck is stored with `game_mode = "eden_singleton"` and reopening the deck shows EDEN Singleton selected

#### Scenario: Catalog matches engine

- **WHEN** the play/format catalog renders
- **THEN** the available formats and their identifiers come from the engine's `list_formats()` rather than a hardcoded frontend list

#### Scenario: Updating an existing deck preserves the format

- **WHEN** an existing deck's card list is updated and saved
- **THEN** the deck's stored `game_mode` is retained (the prior bug where the browser update path dropped `game_mode` is fixed)

#### Scenario: Play format window matches the deck builder

- **WHEN** the play "CHOOSE FORMAT" window renders on either the hosted or desktop runtime
- **THEN** it lists exactly the playable formats from `list_formats()` (Standard, No Banlist, Pauper, EDEN, EDEN Singleton) — the same set the deck builder offers — each enabled and selectable

#### Scenario: No concept placeholders in the play window

- **WHEN** the play "CHOOSE FORMAT" window renders
- **THEN** the non-playable concept placeholders (Titan, EDH, Draft, Tutorial) do not appear, and any count-based copy (e.g. "06" / "SIX RULESETS") reflects the actual number of formats shown

#### Scenario: Mock format catalog removed

- **WHEN** the codebase is built after this change
- **THEN** the hosted `/formats` route and the desktop `formats_list` Tauri command no longer exist, and the play window obtains formats solely from the engine registry source used by the deck builder
