## ADDED Requirements

### Requirement: Decoded legal-action list is exposed to production clients

The system SHALL expose the engine's decoded legal-action list (`legal_decoded_actions(game, player)`) to the production desktop (Tauri) and hosted-API (REST) clients, in addition to the existing debug/MCP `LiveGame` surface. Each returned entry SHALL carry, at minimum, `action_id`, `kind`, `source_zone`, `source_index`, `card_id`, `card_name`, `effect_name`, and `label`. The endpoint SHALL be served by an engine-only path (no DB/auth coupling on the hosted API) and SHALL return an empty list when the queried player is not the current decision player.

#### Scenario: Desktop client receives decoded actions

- **WHEN** the desktop client requests the legal actions for the current decision player during the Main phase
- **THEN** it receives a list of decoded entries, one per set bit in the action mask, each populated with `card_name` and (where present) `effect_name`

#### Scenario: Hosted-API client receives decoded actions

- **WHEN** a browser game requests the legal actions via the REST endpoint for the current decision player
- **THEN** it receives the same decoded entries the engine produces, with no DB or auth dependency required to serve the response

#### Scenario: Inactive player receives empty list

- **WHEN** a client requests legal actions for a player who is not the current decision player
- **THEN** the returned list is empty

### Requirement: Action bar surfaces every activatable [Main] effect category

The action bar SHALL render one entry for every currently-legal activatable effect, across all engine-emitted categories: field [Main], Digiburst, breeding `<Training>`, trash [Main], hand [Main], and delayed-Option [Main]. The action bar SHALL derive these entries from the decoded legal-action list, NOT by re-deriving action semantics from raw mask bit-ranges. An entry SHALL appear if and only if its action is legal at that decision point.

#### Scenario: Field [Main] / Digiburst is surfaced

- **WHEN** a Digimon on the field has a legal field [Main] activated effect (e.g. a Digiburst)
- **THEN** the action bar shows an activatable entry whose action submits that effect's action id

#### Scenario: Trash [Main] is surfaced without slot mis-decode

- **WHEN** a card in the player's trash has a legal trash [Main] effect (engine action id in `1150–1194`)
- **THEN** the action bar shows an entry for it labeled by the trash card, and does NOT render it as a nonexistent battle slot (e.g. `Effect 15:0`)

#### Scenario: Hand [Main] is surfaced and not mis-bucketed as discard

- **WHEN** a card in hand has a legal hand [Main] effect (engine action id in `30–59`)
- **THEN** the action bar shows an activatable-effect entry for it, and does NOT mis-classify it as a "trash from hand" / discard action

#### Scenario: Breeding Training and delayed-Option [Main] are surfaced

- **WHEN** a breeding-area Digimon has a legal `<Training>` activation, or a parked Option has a legal delayed [Main] activation
- **THEN** each appears as its own activatable entry in the action bar

#### Scenario: No false entries

- **WHEN** an activatable effect is not currently legal (condition fails, OPT exhausted, or wrong phase)
- **THEN** no entry for it appears in the action bar

### Requirement: Activatable-effect entries are labeled by source card and effect name

Each activatable-effect entry SHALL be labeled `"{card name}: {effect name}"` using the source card's name and the matched effect's name. When the matched effect has no name, the entry SHALL fall back to `"{card name}"` alone. The board slot SHALL be appended (e.g. `"{card name} (slot N): {effect name}"`) ONLY when two or more surfaced entries share the same card name; a unique card name SHALL NOT show a slot. The entry SHALL provide the source card's main effect text as hover/tooltip content, resolved from game state by `source_zone` + `source_index`.

#### Scenario: Single card shows name without slot

- **WHEN** exactly one surfaced activatable entry comes from a card named `"Cresgarurumon"` with effect name `"Digiburst"`
- **THEN** its label reads `"Cresgarurumon: Digiburst"` with no slot shown

#### Scenario: Duplicate card names disambiguate by slot

- **WHEN** two surfaced activatable entries both come from cards named `"Cresgarurumon"`
- **THEN** each label includes its board slot to disambiguate (e.g. `"Cresgarurumon (slot 1): Digiburst"`)

#### Scenario: Missing effect name falls back to card name

- **WHEN** a surfaced activatable entry's matched effect has no name
- **THEN** the label is just the source card's name

#### Scenario: Tooltip shows the source card's main effect text

- **WHEN** the player hovers an activatable-effect entry
- **THEN** the tooltip shows the source card's main effect text for that zone and index
