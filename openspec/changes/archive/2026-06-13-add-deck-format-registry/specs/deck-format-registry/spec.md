## ADDED Requirements

### Requirement: Format configuration is loaded from editable data

The engine SHALL load all format definitions, named card restrictions, and the anomaly protocol from a single data file `data/deck_formats.json`, baked into the engine via `include_str!` and read at runtime by the hosted API. No format, banlist, restriction limit, choice group, or anomaly definition SHALL be hardcoded in Rust or Python logic. The previously hardcoded `OFFICIAL_ENG_RESTRICTION` and `EDEN_RESTRICTION` `LazyLock`s and the `is_eden_anomaly` name-substring heuristic SHALL be removed in favor of this file.

#### Scenario: Banlist edited via data file

- **WHEN** a card ID is added to the `eden` restriction's `banned` list in `data/deck_formats.json` and the engine is rebuilt
- **THEN** validating an EDEN deck containing that card reports it as banned, with no Rust or Python source change required

#### Scenario: Anomaly list expanded via data file

- **WHEN** a card ID is appended to `anomaly_protocol.extra_card_ids` in `data/deck_formats.json` and the engine is rebuilt
- **THEN** that card is treated as an EDEN Anomaly Protocol card (legal in EDEN despite being rare-or-higher, counting toward the anomaly total)

#### Scenario: Malformed config is caught in CI

- **WHEN** `deck_formats.json` references a `banlist` name that has no entry under `restrictions`, or a `rarity_policy` that is not a known policy
- **THEN** the structural-validation test fails

### Requirement: Single format registry as the source of truth

The engine SHALL expose a `FormatDescriptor` registry parsed once from `deck_formats.json`, where each descriptor carries id, display name, description, deck size, egg-deck maximum, rarity policy, resolved card restriction, singleton flag, default max copies, and a playable flag. The `Rules` preset for a format and the `CardRestriction` for a named banlist SHALL be derived from this registry rather than from independent hand-written definitions, and each `GameMode` variant SHALL map one-to-one to a descriptor id.

#### Scenario: Adding a format requires only a data row

- **WHEN** a new format object is added to the `formats` array in `deck_formats.json` referencing existing restriction and rarity-policy names
- **THEN** the format appears in `list_formats()` and is accepted by validation without any change to the validator's branching logic

#### Scenario: Rules derive from descriptor

- **WHEN** `Rules` are built for a format id
- **THEN** the resulting `deck_size`, `egg_deck_max`, `singleton`, allowed-rarity gate, and `restriction` match that format's descriptor

#### Scenario: list_formats reports playable formats

- **WHEN** `list_formats()` is called
- **THEN** it returns Standard, No Banlist, Pauper, EDEN, and EDEN Singleton, each with its display metadata and `playable` flag

### Requirement: Deck validation is derived from the format descriptor

Deck validation SHALL apply all checks generically from the descriptor with no per-format conditional branches: deck size and egg-deck size, per-card database copy limits, an effective copy limit of `min(restriction_limit_or_default, singleton ? 1 : default_max_copies)`, the rarity policy, the anomaly total cap, and choice-group exclusivity. Existing Standard, EDEN, and Pauper validation results (including error message wording) SHALL remain unchanged.

#### Scenario: Standard validation unchanged

- **WHEN** a deck is validated under `standard`
- **THEN** the banned/restricted/choice-group/size results and error strings are identical to the prior implementation

#### Scenario: Rarity gate via policy

- **WHEN** a deck under a `common_uncommon` rarity policy contains a rare-or-higher non-egg card
- **THEN** validation reports that card as an illegal rarity for the format

#### Scenario: Singleton enforced generically

- **WHEN** a deck under a singleton format contains two or more copies of any single card
- **THEN** validation reports a copy-limit violation for that card

### Requirement: EDEN Anomaly Protocol is data-driven

The EDEN anomaly protocol SHALL be defined by `max_total`, a list of category rules (each matching by card kind, optional name substring, and a set of legal rarities), and an explicit `extra_card_ids` list. A rare-or-higher card SHALL be legal in an `eden_anomaly` format only if it matches a category or appears in `extra_card_ids`, and the total count of anomaly cards in the deck SHALL NOT exceed `max_total`.

#### Scenario: Anomaly card within cap

- **WHEN** an EDEN deck contains four or fewer total anomaly-protocol cards (e.g. rare Tamers and rare/super-rare/promo Memory Boosts)
- **THEN** validation accepts them

#### Scenario: Anomaly cap exceeded

- **WHEN** an EDEN deck contains more than `max_total` anomaly-protocol cards
- **THEN** validation reports the anomaly-limit violation

#### Scenario: Rare card outside the anomaly definition

- **WHEN** an EDEN deck contains a rare-or-higher card that matches no anomaly category and is not in `extra_card_ids`
- **THEN** validation reports that card's rarity as illegal for EDEN

### Requirement: EDEN Singleton format

The engine SHALL provide an `eden_singleton` format combining the EDEN anomaly rarity policy, the EDEN banlist, and the singleton rule (every card limited to one copy). The anomaly total cap SHALL apply independently of the singleton copy limit.

#### Scenario: EDEN Singleton enforces one copy

- **WHEN** an `eden_singleton` deck contains two copies of any card
- **THEN** validation reports a copy-limit violation

#### Scenario: EDEN Singleton keeps the anomaly cap

- **WHEN** an `eden_singleton` deck contains more than `max_total` distinct anomaly-protocol cards (each a single copy)
- **THEN** validation reports the anomaly-limit violation

#### Scenario: EDEN Singleton applies the EDEN banlist

- **WHEN** an `eden_singleton` deck contains a card on the EDEN `banned` list
- **THEN** validation reports it as banned

### Requirement: Per-card legality query

The engine SHALL provide a `card_legality(card_id, format)` query returning whether the card is legal in that format, its maximum allowed copies, and a human-readable reason when not legal (or when it is constrained, e.g. counts toward the anomaly limit). This query SHALL be exposed through the PyO3 bindings, a Tauri command, and a hosted-API endpoint, and SHALL be derived from the same descriptor logic used by deck validation.

#### Scenario: Legal card

- **WHEN** `card_legality` is called for a common card under EDEN
- **THEN** it returns legal with a maximum-copies value matching the format default

#### Scenario: Banned card

- **WHEN** `card_legality` is called for a card on the format's banlist
- **THEN** it returns not legal with maximum copies 0 and a reason indicating it is banned

#### Scenario: Singleton copy cap

- **WHEN** `card_legality` is called for any card under EDEN Singleton
- **THEN** it returns a maximum-copies value of 1

#### Scenario: Exposed on all surfaces

- **WHEN** the deck builder runs in either the desktop (Tauri) or browser/hosted runtime
- **THEN** the same legality result is available for a given card and format
