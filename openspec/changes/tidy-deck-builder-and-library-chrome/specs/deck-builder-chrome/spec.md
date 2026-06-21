## ADDED Requirements

### Requirement: The deck builder has no sideboard

The deck builder SHALL compose only a Main deck and an Egg deck; it SHALL NOT present a sideboard (no SIDE counts pill, no SIDE deck-contents tab, and no sideboard section).

#### Scenario: No sideboard chrome

- **WHEN** the deck builder is shown
- **THEN** the counts bar shows no SIDE pill
- **AND** the deck-contents panel offers only MAIN and EGG tabs

### Requirement: Counts bar separates type and level tallies

The deck builder's top counts bar SHALL visually separate the card-type tallies (Egg, Digimon, Tamer, Option) from the per-level tallies (L2 through L7+).

#### Scenario: Visible separation between the two groups

- **WHEN** the counts bar renders
- **THEN** a gap separates the last type tally (Option) from the first level tally (L2)

### Requirement: Deck-contents list groups Digimon by exact level

The deck-contents list SHALL group Digimon into distinct sections by exact level — Lv2, Lv3, Lv4, Lv5, Lv6, and Lv7 — and SHALL NOT combine Lv6 and Lv7 into a single "Lv6+" bucket.

#### Scenario: Lv6 and Lv7 are separate sections

- **WHEN** a deck contains both a Lv6 and a Lv7 Digimon
- **THEN** the list shows a distinct Lv6 section and a distinct Lv7 section
- **AND** no section combines Lv6 and Lv7 together

#### Scenario: Lv7 Digimon are not placed in the Lv6 section

- **WHEN** a deck contains a Lv7 Digimon
- **THEN** it appears in the Lv7 section, not the Lv6 section
