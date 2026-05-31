# st2-cocytus-blue-coverage Specification

## Purpose
TBD - created by archiving change implement-st2-cocytus-blue-substrate. Update Purpose after archive.
## Requirements
### Requirement: ST-2 Cocytus Blue starter deck composition is represented

The system SHALL provide a verified deck artifact for the English/Worldwide ST-2 Starter Deck Cocytus Blue composition: `ST2-01 x4`, `ST2-02 x4`, `ST2-03 x4`, `ST2-04 x4`, `ST2-05 x4`, `ST2-06 x2`, `ST2-07 x4`, `ST2-08 x4`, `ST2-09 x4`, `ST2-10 x2`, `ST2-11 x2`, `ST2-12 x4`, `ST2-13 x4`, `ST2-14 x4`, `ST2-15 x2`, and `ST2-16 x2`. The artifact SHALL classify `ST2-01` as the 4-card Digi-Egg deck and the remaining cards as the 50-card main deck.

#### Scenario: Deck artifact has official counts

- **WHEN** the ST-2 Cocytus Blue deck artifact is loaded
- **THEN** its card multiset exactly matches the required copy counts
- **AND** it contains 54 playable cards total, 16 unique card IDs, 4 Digi-Egg cards, and 50 main-deck cards

#### Scenario: Deck validates under standard construction rules

- **WHEN** the ST-2 Cocytus Blue deck artifact is passed to the Rust deck validator
- **THEN** validation succeeds with no unknown-card, wrong-size, or Digi-Egg deck errors

### Requirement: Every ST2 card has production DSL YAML

Every unique card in ST-2 Cocytus Blue SHALL have a production DSL YAML file under `code/digimon-engine/cards/st2/`. The YAML SHALL match the card's printed metadata from `data/cards.json` and SHALL represent all printed effects, inherited effects, security effects, keywords, and alternate behaviors. Cards with no printed effect text SHALL still have YAML so they are included in the Rust implemented-card registry.

#### Scenario: All ST2 YAML files exist

- **WHEN** `code/digimon-engine/cards/st2/` is enumerated
- **THEN** `ST2-01.yaml` through `ST2-16.yaml` all exist
- **AND** `ST2-13.yaml` remains a faithful Hammer Spark implementation

#### Scenario: Implemented-card registry includes every ST2 card

- **WHEN** the Rust implemented-card registry is built from the embedded DSL pack
- **THEN** `load_implemented_card_ids()` includes every card ID from `ST2-01` through `ST2-16`

#### Scenario: Vanilla cards are registered without fake behavior

- **WHEN** ST2 vanilla cards with no printed effect text are compiled
- **THEN** they register as implemented cards with correct kind, level, color, cost, DP, form, attribute, traits, and digivolution costs
- **AND** they do not declare no-op effect clauses or placeholder raw Rust behavior

### Requirement: Every printed ST2 effect is faithfully implemented

Each ST2 card's runtime behavior SHALL match printed text from `data/cards.json`. All player choices SHALL be surfaced as pending selections/action masks, and non-choice effects SHALL resolve without artificial prompts. No ST2 card SHALL use a `raw_rust` escape or omit a printed clause to claim readiness.

#### Scenario: Source-trash effects trash bottom sources without source-choice prompts

- **WHEN** ST2-03 or ST2-06 inherited `[When Attacking]` effects resolve
- **THEN** the controller chooses an eligible opponent Digimon as printed
- **AND** the effect trashes the bottom digivolution card under that Digimon without prompting for which source card to trash

#### Scenario: Zudomon trashes up to two bottom sources

- **WHEN** ST2-09's `[When Digivolving]` effect resolves against an opponent Digimon with two or more digivolution cards
- **THEN** exactly the two bottom digivolution cards are trashed in bottom-up order
- **AND** no source-selection prompt is surfaced

#### Scenario: No-source predicates affect only legal targets

- **WHEN** ST2-08, ST2-12, or ST2-14 checks for an opponent Digimon with no digivolution cards
- **THEN** only opponent Digimon with zero source cards satisfy that condition or target filter
- **AND** opponent Digimon with one or more source cards do not satisfy it

#### Scenario: Tsunomon inherited DP is battle-contextual

- **WHEN** a Digimon carrying ST2-01 battles an opponent Digimon with no digivolution cards during the controller's turn
- **THEN** the carrier gets +1000 DP for that battle
- **AND** the same carrier does not get that DP during security checks or battles against an opponent Digimon that has one or more source cards

#### Scenario: Kaiser Nail plays a selected source

- **WHEN** ST2-15 resolves and the controller chooses a Digimon digivolution card placed under one of their Digimon
- **THEN** the chosen source card is removed from that stack and played as a Digimon without paying the cost
- **AND** ownership, on-play behavior, summoning-sickness state, and source cleanup follow the normal engine source-play contract

#### Scenario: Cocytus Breath returns opponent Digimon to hand

- **WHEN** ST2-16 resolves
- **THEN** the controller chooses one opponent Digimon
- **AND** that Digimon's top card returns to its owner's hand while its digivolution cards are routed by normal return-to-hand rules

### Requirement: ST2 behavioral coverage and trackers are complete

The change SHALL add or update Rust behavioral tests for every ST2 card and SHALL reconcile card verdict and gap trackers to the verified state. No enabled test may depend on hidden auto-selection, and no ignored ST2 test may cite a substrate gap that is already closed in current code.

#### Scenario: Behavioral test coverage exists

- **WHEN** the ST2 behavioral tests are enumerated
- **THEN** each card ID from `ST2-01` through `ST2-16` has enabled coverage for card data and any printed behavior
- **AND** the relevant `cards_behavioral` and `dsl` test suites pass

#### Scenario: Verdict ledger marks ST2 implemented

- **WHEN** ST2 implementation is complete
- **THEN** the DSL verdict ledger includes entries for every ST2 card
- **AND** each entry has an implemented verdict with references to production YAML and behavioral coverage

#### Scenario: Gap trackers reflect verified substrate state

- **WHEN** a tracker entry claims ST2 is blocked by a missing primitive
- **THEN** implementation verifies the claim against current source code
- **AND** closed or stale gap claims are moved or annotated in the appropriate tracker
- **AND** any still-open blocker is recorded as a reusable substrate gap rather than a one-card TODO

