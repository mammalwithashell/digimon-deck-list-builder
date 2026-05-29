## ADDED Requirements

### Requirement: Resolved Mastemon full pool is tracked
The system SHALL treat the resolver output for `Mastemon (Tribal)` as the source of truth for full-pool authoring coverage. The full-pool baseline SHALL distinguish total resolved cards, best-deck cards, cards with production YAML, cards with behavioral tests, and cards still blocked by reusable substrate.

#### Scenario: Resolver produces the full pool baseline
- **WHEN** `code/tools/resolve_deck.py "Mastemon (Tribal)"` is run with UTF-8 output
- **THEN** the resolved archetype is `Mastemon (Tribal)`
- **AND** the resolved pool contains 93 unique card IDs
- **AND** the resolved best deck contains 20 unique card IDs

#### Scenario: Coverage report distinguishes best deck from full pool
- **WHEN** Mastemon coverage is reported
- **THEN** the report identifies the completed 20-card best-deck set separately from the remaining full-pool cards
- **AND** the report does not mark the full pool ready until every resolved card has faithful production YAML and behavioral tests or an explicit accepted exclusion

### Requirement: Full-pool cards have faithful Rust DSL coverage
Every non-excluded card in the resolved `Mastemon (Tribal)` pool SHALL have production Rust YAML and at least one behavioral test proving its gameplay-relevant printed text. A card SHALL NOT be counted as full-pool ready if it relies on a no-op stub, raw-Rust card escape, hidden auto-selection, or a comment claiming unimplemented printed behavior.

#### Scenario: Remaining non-best-deck card is authored
- **WHEN** a remaining Mastemon full-pool card is implemented
- **THEN** its YAML exists under `code/digimon-engine/cards/<set>/`
- **AND** a focused behavioral test exists under `code/digimon-engine/tests/cards_behavioral/<set>/`
- **AND** every gameplay-affecting player choice in its printed text is visible through an action or `PendingSelection`

#### Scenario: Card remains blocked by reusable substrate
- **WHEN** a card cannot be implemented faithfully with current DSL/engine support
- **THEN** the blocker is recorded as a reusable DSL or engine capability gap
- **AND** the card is not counted as ready

### Requirement: Best-deck readiness remains intact
Full-pool work SHALL preserve the completed Mastemon best-deck readiness established by `unblock-mastemon-rust-dsl`.

#### Scenario: Full-pool work touches shared substrate
- **WHEN** shared DSL or engine substrate is changed for the full pool
- **THEN** all Mastemon best-deck behavioral tests still pass
- **AND** no best-deck YAML is replaced with a partial or approximate implementation

### Requirement: Mastemon full-pool work preserves RL contracts
Mastemon full-pool authoring SHALL NOT change `ACTION_SPACE_SIZE`, active observation tensor sizes, feature schema versions, or layout hashes as part of card unlock work. If a required full-pool choice cannot be represented through existing action ranges, the work SHALL stop and a separate action/tensor contract change SHALL be proposed.

#### Scenario: Full-pool substrate is added
- **WHEN** the full-pool substrate and card implementations are added
- **THEN** `digimon_engine.ACTION_SPACE_SIZE` remains unchanged from the pre-change baseline
- **AND** `standard_lite_v2` and `standard_compact_v1` layout metadata remain unchanged from the pre-change baseline

#### Scenario: New action IDs appear necessary
- **WHEN** a full-pool card requires a player-visible choice that cannot be represented with existing actions or `PendingSelection` kinds
- **THEN** this change does not add the new action IDs
- **AND** a separate action/tensor contract proposal is required before continuing that blocker
