## ADDED Requirements

### Requirement: Gaia Red starter deck card pool is fully represented
The Rust engine SHALL represent the worldwide ST-1 Gaia Red starter deck as the exact printed 54-card deck: 4 `ST1-01` Digi-Eggs and a 50-card main deck using only `ST1-02` through `ST1-16` counts from the source decklist. The fixture SHALL distinguish Digi-Eggs from main-deck cards and SHALL be usable by engine smoke tests without hand-editing card lists.

#### Scenario: Starter deck fixture has exact card counts
- **WHEN** the Gaia Red starter deck fixture is loaded
- **THEN** it contains exactly 54 cards
- **AND** it contains exactly 4 `ST1-01` cards routed as Digi-Eggs
- **AND** it contains exactly 50 non-Digi-Egg main-deck cards
- **AND** every card ID belongs to `ST1-01` through `ST1-16`

#### Scenario: Engine routes Gaia Red zones correctly
- **WHEN** a game is constructed with the Gaia Red fixture for both players
- **THEN** `ST1-01` cards are routed to each player's digitama deck
- **AND** all other ST-1 cards are routed to each player's main deck
- **AND** game construction succeeds without missing-card errors

### Requirement: Every ST-1 card has production DSL coverage
Every unique card in the worldwide `ST1-01` through `ST1-16` Gaia Red starter deck SHALL have a production YAML spec under `code/digimon-engine/cards/st1/`. Effect-bearing cards SHALL faithfully implement all printed effect, inherited, and security text from `data/cards.json`; vanilla cards SHALL compile as no-effect DSL cards rather than being left JSON-only.

#### Scenario: All ST-1 card IDs are registered as implemented
- **WHEN** the Rust implemented-card registry is built with the DSL YAML loader enabled
- **THEN** `load_implemented_card_ids()` includes every card ID from `ST1-01` through `ST1-16`

#### Scenario: Vanilla cards have no hidden behavior
- **WHEN** a no-effect ST-1 card such as `ST1-02`, `ST1-04`, `ST1-05`, or `ST1-10` is loaded from its YAML
- **THEN** the compiled card has no triggered, declarative, replacement, delay, or partition effects
- **AND** the card remains playable and digivolvable according to printed metadata and ordinary game rules

#### Scenario: Effect-bearing cards match printed text
- **WHEN** an ST-1 card's YAML is reviewed against `data/cards.json`
- **THEN** every printed main, inherited, security, keyword, and continuous effect is represented
- **AND** no printed effect is stubbed, no-op'd, auto-selected, or hidden behind `raw_rust`

### Requirement: ST-1 card behavior has regression coverage
Every effect-bearing ST-1 card SHALL have Rust behavioral test coverage under `code/digimon-engine/tests/cards_behavioral/st1/` or a shared DSL, combat, or security test where the behavior is a reusable primitive. Tests SHALL cover both positive and negative cases where the printed effect has conditions, expiration, or target choices.

#### Scenario: ST-1 behavioral suite passes
- **WHEN** the ST-1 behavioral tests are run
- **THEN** each effect-bearing ST-1 card has an enabled test for its printed behavior
- **AND** no ST-1 test remains ignored for a gap closed by this change

#### Scenario: Shared primitives are tested before card YAML relies on them
- **WHEN** `ST1-09` or `ST1-14` YAML uses a newly added DSL or engine primitive
- **THEN** a focused shared test proves the primitive's general behavior
- **AND** the ST-1 card-specific test proves the printed card behavior uses that primitive correctly

### Requirement: ST-1 implementation keeps action and tensor contracts stable
Implementing Gaia Red SHALL NOT expand the action space or active observation tensor contracts. Any discovered need for new player-visible action IDs, pending-selection ranges, or observation fields SHALL be split into a separate action/tensor contract change before the ST-1 deck is marked complete.

#### Scenario: Contract constants are unchanged
- **WHEN** the ST-1 change is complete
- **THEN** `ACTION_SPACE_SIZE` remains unchanged
- **AND** active observation layout metadata remains unchanged
- **AND** PyO3 exports and frontend action constants do not require shape updates for this change
