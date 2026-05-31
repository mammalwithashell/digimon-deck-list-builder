# st3-heavens-yellow-starter-coverage Specification

## Purpose
TBD - created by archiving change add-st3-heavens-yellow-starter. Update Purpose after archive.
## Requirements
### Requirement: All ST3 cards are implemented as production Rust DSL cards

The Rust engine SHALL provide production DSL YAML implementations for every worldwide ST-3 Heaven's Yellow card ID from `ST3-01` through `ST3-16`. Each YAML file SHALL faithfully represent the card's printed metadata, digivolution requirements, and printed effect clauses from `data/cards.json`, including vanilla cards with no effect clauses. No ST3 card behavior may be implemented as a no-op placeholder, hidden auto-selection, legacy Python script, or coarser proxy for the printed text.

#### Scenario: Full ST3 card pool has YAML

- **WHEN** the ST3 card pool is enumerated from `ST3-01` through `ST3-16`
- **THEN** each card has a corresponding `code/digimon-engine/cards/st3/<CARD-ID>.yaml` production DSL file
- **AND** each YAML file parses and compiles into the embedded DSL pack

#### Scenario: ST3 cards register as implemented

- **WHEN** the Rust card-effect registry is built with the DSL YAML loader enabled
- **THEN** `load_implemented_card_ids()` includes every card ID from `ST3-01` through `ST3-16`

#### Scenario: Printed clauses are not approximated

- **WHEN** an ST3 YAML file is reviewed against the printed text in `data/cards.json`
- **THEN** every printed main, inherited, security, timing, keyword, and digivolution requirement clause is represented faithfully
- **AND** no clause is replaced by a no-op, hidden auto-choice, legacy Python implementation, or broader trigger condition than printed

### Requirement: ST3 effects have behavioral test coverage

Every effectful ST3 card SHALL have behavioral tests under `code/digimon-engine/tests/cards_behavioral/st3/` that exercise its printed effect clauses through `DebugRunner` and production DSL loading. Vanilla ST3 cards SHALL have structural/load tests proving their YAML exists, compiles, and carries the expected card metadata.

#### Scenario: Effectful ST3 card behavior is tested

- **WHEN** an ST3 card has non-empty printed effect, inherited effect, security effect, or keyword text
- **THEN** its test file covers each printed clause's timing, condition, target legality, and resulting game-state mutation

#### Scenario: Vanilla ST3 cards are load-tested

- **WHEN** an ST3 card has no printed effect clauses beyond normal play/digivolution metadata
- **THEN** its test coverage verifies the card loads from the embedded DSL pack with correct card ID, kind, level, color, cost, DP, traits, and digivolution requirements

#### Scenario: Security option effects resolve through security flow

- **WHEN** an ST3 Option card with a `[Security]` effect is revealed during a security check
- **THEN** the test drives the normal security-resolution flow
- **AND** the printed security disposition is observed, including activating the main effect or adding the option to hand when printed

### Requirement: ST3 starter deck is loadable as a canonical deck fixture

The repository SHALL provide a canonical ST-3 Heaven's Yellow starter deck list matching the worldwide 54-card product composition: 4 `ST3-01`, 4 `ST3-02`, 4 `ST3-03`, 4 `ST3-04`, 2 `ST3-05`, 4 `ST3-06`, 4 `ST3-07`, 4 `ST3-08`, 4 `ST3-09`, 2 `ST3-10`, 2 `ST3-11`, 4 `ST3-12`, 4 `ST3-13`, 2 `ST3-14`, 4 `ST3-15`, and 2 `ST3-16`. The fixture SHALL be usable by the repository's established deck-loading or starter-deck smoke-test path.

#### Scenario: Canonical starter deck composition is available

- **WHEN** the ST-3 starter fixture or deck-library entry is loaded
- **THEN** it contains exactly 54 cards
- **AND** its card counts match the worldwide Heaven's Yellow product list

#### Scenario: Starter deck contains only implemented cards

- **WHEN** the ST-3 starter deck is validated against `load_implemented_card_ids()`
- **THEN** every card in the deck is present in the implemented-card set

#### Scenario: Rust-backed game can initialize with ST3 deck

- **WHEN** a Rust-backed game or smoke test initializes with the canonical ST-3 deck list
- **THEN** deck parsing and game setup complete without missing-card or unimplemented-card errors

### Requirement: ST3 implementation preserves engine and agent contracts

Implementing ST3 SHALL NOT change the active action-space size, observation tensor profile layouts, PyO3 public API shape, model metadata contract, or frontend constants. Any new player-visible choices required by ST3 card text SHALL use existing action and pending-selection contracts unless a separate action/tensor contract change is proposed.

#### Scenario: No action or tensor contract drift

- **WHEN** ST3 card YAML, tests, and deck fixtures are added
- **THEN** `ACTION_SPACE_SIZE`, observation profile tensor sizes, tensor layout hashes, and exported model metadata requirements remain unchanged

#### Scenario: Player choices use normal pending selections

- **WHEN** an ST3 effect requires the player to choose a target, accept an optional effect, or select a security/hand/trash card
- **THEN** the choice is surfaced through the engine's normal action mask and pending-selection flow
- **AND** no legal choice is resolved only by UI code or hidden engine auto-selection

### Requirement: ST3 gaps are tracked as reusable primitives

If an ST3 printed clause cannot be implemented faithfully with current Rust engine or DSL capabilities, the implementation SHALL document the missing reusable primitive in the appropriate gap tracker and mark only the affected behavioral test as ignored with that gap ID. The implementation SHALL NOT mark the card fully implemented until all printed behavior is real and tested.

#### Scenario: Missing primitive is documented

- **WHEN** implementation discovers that an ST3 clause cannot be expressed faithfully
- **THEN** a reusable gap entry is added or updated in `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, or `qa/dsl-vocab-gaps.md`
- **AND** the affected test's ignore reason cites the same gap

#### Scenario: No blocked card is reported as complete

- **WHEN** an ST3 card still has an ignored behavioral test for a missing primitive
- **THEN** that card is not represented in reports or ledgers as fully implemented
- **AND** the final implementation notes identify the blocked printed clause and reusable gap

