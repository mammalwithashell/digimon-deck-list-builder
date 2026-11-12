## ADDED Requirements

### Requirement: Format descriptor set

DCGO SHALL represent a play format as a descriptor comprising a stable id, a display name, a rarity policy, a banlist reference, and a maximum copies-per-card-id value. The client SHALL define exactly four descriptors: `standard`, `no_restriction`, `singleton`, and `eden`. Format-dependent behaviour SHALL be derived from the descriptor rather than from branching on a format id.

#### Scenario: Standard descriptor

- **WHEN** the format `standard` is resolved
- **THEN** its banlist is the official list already fetched by the client, its maximum copies value is 4, and its rarity policy admits all rarities

#### Scenario: No Banlist descriptor

- **WHEN** the format `no_restriction` is resolved
- **THEN** it applies no banned cards, no restricted limits, and no banned-pair or choice-group rules, its maximum copies value is 4, and its rarity policy admits all rarities

#### Scenario: Singleton descriptor

- **WHEN** the format `singleton` is resolved
- **THEN** it applies no banlist, its maximum copies value is 1, and its rarity policy admits all rarities

#### Scenario: EDEN descriptor

- **WHEN** the format `eden` is resolved
- **THEN** its banlist is EDEN's own list, its maximum copies value is 4, and its rarity policy is EDEN's

#### Scenario: Unknown format id resolves to Standard

- **WHEN** a format id is read from persistence or received from another client and does not match any defined descriptor
- **THEN** the client resolves it to `standard` rather than failing or entering an undefined state

### Requirement: Singleton format definition

The `singleton` format SHALL require a main deck of exactly 50 cards, a Digi-Egg deck of 0 to 5 cards, and at most one copy of any card ID counted across the main deck and the Digi-Egg deck together. It SHALL apply no banlist of any kind and SHALL admit all rarities. It SHALL NOT define a commander or any equivalent designated card.

#### Scenario: Duplicate main-deck card is illegal

- **WHEN** a deck under `singleton` contains two or more copies of the same main-deck card ID
- **THEN** the deck is reported illegal for `singleton`, identifying the offending card ID

#### Scenario: Duplicate Digi-Egg is illegal

- **WHEN** a deck under `singleton` contains two or more copies of the same Digi-Egg card ID
- **THEN** the deck is reported illegal for `singleton`, identifying the offending card ID

#### Scenario: Five distinct eggs are legal

- **WHEN** a deck under `singleton` contains five Digi-Egg cards with five distinct card IDs and a 50-card main deck of distinct card IDs
- **THEN** the deck is reported legal for `singleton`

#### Scenario: Banned and restricted cards are legal under Singleton

- **WHEN** a deck under `singleton` contains a single copy of a card that is banned or limited on the official banlist
- **THEN** the deck is not reported illegal on account of that banlist entry

#### Scenario: Banned-pair rules do not apply

- **WHEN** a deck under `singleton` contains one copy each of two cards that form a banned pair or choice group on the official banlist
- **THEN** the deck is not reported illegal on account of that pairing

### Requirement: Format supersedes the useBanlist boolean

The format axis SHALL replace the `useBanlist` boolean as the client's representation of deck-legality mode, such that `useBanlist == true` corresponds exactly to `standard` and `useBanlist == false` corresponds exactly to `no_restriction`. The client SHALL NOT maintain the boolean as an independent axis that can contradict the selected format.

#### Scenario: Legacy true maps to Standard

- **WHEN** the client evaluates deck legality under `standard`
- **THEN** the outcome is identical to the outcome produced by the previous `useBanlist == true` behaviour for the same deck

#### Scenario: Legacy false maps to No Banlist

- **WHEN** the client evaluates deck legality under `no_restriction`
- **THEN** the outcome is identical to the outcome produced by the previous `useBanlist == false` behaviour for the same deck

### Requirement: Format preference persistence and migration

The client SHALL persist the selected builder format across sessions in the preference slot previously occupied by `UseBanlist`, and SHALL migrate an existing stored boolean exactly once on first run of the new build.

#### Scenario: Existing preference is migrated

- **WHEN** a client that previously stored `UseBanlist` starts for the first time after the update
- **THEN** a stored value of true becomes `standard`, a stored value of false becomes `no_restriction`, and the migration does not run again on subsequent starts

#### Scenario: Fresh install defaults to Standard

- **WHEN** a client with no stored format preference and no stored `UseBanlist` value starts
- **THEN** the builder format is `standard`

#### Scenario: Corrupted preference degrades safely

- **WHEN** the stored format preference cannot be resolved to a defined descriptor
- **THEN** the client uses `standard` and does not fail to start
