## ADDED Requirements

### Requirement: Representative TS Olympos deck is Rust-training eligible

The representative TS Olympos deck resolved from the deck resolver SHALL be eligible for Rust-backed training only when every unique card in that representative deck has production Rust DSL YAML and enabled behavioral test coverage. No representative-card clause SHALL be omitted, approximated, hidden behind a no-op, auto-selected when a player choice exists, or implemented as a raw-Rust card escape when the behavior is expressible in DSL.

#### Scenario: Representative card pool fully authored

- **WHEN** the TS Olympos representative deck pool is resolved
- **THEN** each unique representative card ID has a production YAML file under `code/digimon-engine/cards/<set>/`
- **AND** the remaining representative cards `BT24-095`, `BT10-042`, `BT24-085`, `BT24-088`, `BT24-090`, `BT24-035`, `BT24-051`, `BT24-030`, `BT24-041`, `BT24-034`, `BT24-083`, and `BT24-091` each have faithful YAML

#### Scenario: Representative cards have behavioral tests

- **WHEN** the change is complete
- **THEN** each unique representative TS Olympos card has enabled behavioral coverage under `code/digimon-engine/tests/cards_behavioral/`
- **AND** no representative-card test remains ignored for a primitive closed by this change

#### Scenario: Training pool rejects incomplete representative deck

- **WHEN** a TS Olympos training deck contains a representative card absent from the Rust implemented-card registry
- **THEN** the deck is excluded from Rust-backed training pool admission
- **AND** the exclusion identifies the missing card IDs

### Requirement: Broad TS Olympos pool remains accounted for

The system SHALL track the full broad TS Olympos resolved card pool separately from the representative training unlock target. Broad-pool cards that remain unauthored SHALL be documented as residual implementation work and SHALL NOT block the representative-deck training unlock unless they appear in the representative deck selected for training.

#### Scenario: Broad-pool residuals are documented

- **WHEN** the TS Olympos QA ledger is updated after representative unlock
- **THEN** it records both the representative-deck implemented count and the broad-pool implemented count
- **AND** any broad-pool card that remains unauthored is listed or linked from the QA tracker

### Requirement: TS Olympos gap trackers reflect verified Rust state

TS Olympos QA and gap documents SHALL reflect verified source state after the change. A gap closed by this change SHALL be moved or annotated as resolved with passing verification commands, while any still-open gap SHALL cite a primitive whose absence was confirmed against current Rust engine or DSL source.

#### Scenario: Closed gaps are no longer listed as open

- **WHEN** effect-driven Option use, source-stack aggregates, formula-valued De-Digivolve amounts, or predicate-scoped timing suppression are implemented and tested
- **THEN** the corresponding TS Olympos gap entries are marked resolved or moved to resolved-gap documentation
- **AND** the closure note includes the focused tests that prove the primitive

#### Scenario: Remaining blockers cite verified missing primitives

- **WHEN** a TS Olympos card remains blocked after this change
- **THEN** its QA entry cites an open reusable primitive verified missing from current source
- **AND** the card is not counted as Rust-training eligible
