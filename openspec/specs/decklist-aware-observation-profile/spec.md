# decklist-aware-observation-profile Specification

## Purpose
TBD - created by archiving change add-standard-lite-deck-v2-profile. Update Purpose after archive.
## Requirements
### Requirement: standard_lite_deck_v2 profile exists

The system SHALL expose an observation profile named `standard_lite_deck_v2` that is derived from `standard_lite_v2` and adds an own-original-decklist section without changing the action space.

#### Scenario: Profile is listed

- **WHEN** callers list available observation profiles
- **THEN** `standard_lite_deck_v2` is included alongside existing profiles

#### Scenario: Profile preserves action-space size

- **WHEN** a game uses the `standard_lite_deck_v2` observation profile
- **THEN** the action mask length remains equal to the existing `ACTION_SPACE_SIZE`

### Requirement: Own original decklist is encoded as unique rows

The `standard_lite_deck_v2` tensor SHALL encode the observing player's original submitted decklist as unique-card rows sorted by stable registry index. Each populated row SHALL include a card ID, normalized original copy count, main-deck flag, and Digi-Egg flag.

#### Scenario: Multiple copies share one row

- **WHEN** the observing player's original submitted deck contains multiple copies of the same card ID
- **THEN** the decklist section contains one populated row for that card ID with the original copy count encoded

#### Scenario: Rows are stable

- **WHEN** two games use the same observing player's original submitted deck with different shuffle seeds
- **THEN** the populated decklist rows appear in the same order

#### Scenario: Main and Digi-Egg cards are distinguished

- **WHEN** the observing player's original submitted deck contains both main-deck cards and Digi-Egg cards
- **THEN** each populated row marks whether the card belongs to the main deck or Digi-Egg deck

### Requirement: Decklist encoding preserves hidden information

The `standard_lite_deck_v2` tensor SHALL NOT expose opponent decklist composition, current shuffled deck order, topdeck identity, or face-down security identity.

#### Scenario: Opponent decklist is not encoded

- **WHEN** a tensor is built for player 1 using `standard_lite_deck_v2`
- **THEN** the decklist section contains only player 1 original deck composition and no player 2 original deck composition

#### Scenario: Shuffle order is not encoded

- **WHEN** two games use the same original submitted deck but different shuffled deck orders
- **THEN** the decklist section is identical for the same observing player

#### Scenario: Hidden security identity is not encoded

- **WHEN** cards are placed face-down in security during setup
- **THEN** the decklist section continues to encode only original composition and does not identify which original cards are in security

### Requirement: Layout metadata covers decklist card IDs

The `standard_lite_deck_v2` layout metadata SHALL include decklist card ID fields in `card_id_positions` and all non-card decklist fields in `scalar_positions`.

#### Scenario: Card embedding receives decklist card IDs

- **WHEN** Python obtains layout metadata for `standard_lite_deck_v2`
- **THEN** decklist row card ID offsets are included in `card_id_positions`

#### Scenario: Tensor positions cover the profile exactly once

- **WHEN** the profile's `card_id_positions` and `scalar_positions` are combined
- **THEN** every tensor index is covered exactly once with no overlap

