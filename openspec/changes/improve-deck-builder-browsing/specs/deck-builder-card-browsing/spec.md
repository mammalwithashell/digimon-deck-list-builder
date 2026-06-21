## ADDED Requirements

### Requirement: Colour filter matches the intersection of selected colours

The deck builder card pool SHALL, when two or more colours are selected, show only cards whose colour identity contains **all** of the selected colours; with a single colour selected it SHALL show every card containing that colour, and with no colour selected it SHALL apply no colour restriction. A card's colour identity is the set of its primary colour and, when present, its secondary colour.

#### Scenario: Two colours show only dual-colour cards

- **WHEN** the user selects both Green and Blue in the colour filter
- **THEN** the pool shows only cards that are Green and Blue (e.g. a Green/Blue dual-colour card)
- **AND** a mono-Green card and a mono-Blue card are excluded

#### Scenario: A single colour shows all cards containing it

- **WHEN** the user selects only Green
- **THEN** the pool shows mono-Green cards and dual-colour cards that include Green

#### Scenario: No colour selected applies no restriction

- **WHEN** no colour is selected
- **THEN** the colour filter excludes no cards

#### Scenario: Selecting three colours yields no cards

- **WHEN** the user selects three colours
- **THEN** the pool is empty, because no card's colour identity contains three colours
- **AND** the result count reflects zero matches

### Requirement: Card pool offers GRID, DETAIL, and DECKLIST view modes with a persisted preference

The deck builder SHALL provide a toggle among a GRID view that emphasises the card pool, a DETAIL view that emphasises the selected card and its effect text, and a DECKLIST view that emphasises the deck contents (a two-column deck list with the card pool reduced to a compact add strip); it SHALL default to GRID and SHALL persist the chosen view so it is restored on the next visit. The toggle SHALL change layout proportions only; it SHALL NOT change which cards are shown, the deck contents, or any filter state.

#### Scenario: Toggling to DETAIL emphasises the selected card

- **WHEN** the user switches the view toggle to DETAIL
- **THEN** the selected-card preview and its effect text occupy a larger share of the display and the pool tiles shrink to make room
- **AND** the set of cards shown and the current filters are unchanged

#### Scenario: Toggling to DECKLIST emphasises the deck contents

- **WHEN** the user switches the view toggle to DECKLIST
- **THEN** the deck-contents panel occupies the dominant share of the display and renders the deck list in two columns
- **AND** the card pool collapses to a compact add strip while remaining usable to add cards
- **AND** the set of cards shown and the current filters are unchanged

#### Scenario: View preference persists across sessions

- **WHEN** the user selects DETAIL and later reopens the deck builder
- **THEN** the deck builder opens in DETAIL view

#### Scenario: Default view is GRID

- **WHEN** the user opens the deck builder with no stored preference
- **THEN** the deck builder opens in GRID view

#### Scenario: A stale or invalid stored preference falls back to the default

- **WHEN** the stored view preference is missing or not one of the known values (GRID, DETAIL, or DECKLIST)
- **THEN** the deck builder opens in GRID view

### Requirement: Selected-card preview and effect text are enlarged for readability

The deck builder SHALL render the selected-card preview image larger and its effect text at a larger size than the pre-change 11px in both view modes, and SHALL render them larger still in DETAIL view. The enlarged preview SHALL preserve the existing card-text token styling (timing pills, name references, keyword pills).

#### Scenario: Effect text is readable in GRID view

- **WHEN** a card is selected in GRID view
- **THEN** its effect text renders larger than the previous 11px size with the existing token styling intact

#### Scenario: DETAIL view renders the preview larger than GRID view

- **WHEN** the same card is viewed in DETAIL versus GRID view
- **THEN** the preview card image and effect text are larger in DETAIL view

### Requirement: Reset control clears all card-pool filters

The deck builder SHALL provide a reset control that returns every card-pool filter — search text, selected colours, type, level, rarity, and the inherited-only, security-only, and format-legal-only toggles — to its default state in a single action.

#### Scenario: Reset returns filters to defaults

- **WHEN** the user has set a search term, selected colours, and enabled the format-legal-only toggle, then activates the reset control
- **THEN** the search is cleared, no colours are selected, type/level/rarity return to "all", and all three checkbox toggles are unchecked
- **AND** the pool reflects the unfiltered card set
