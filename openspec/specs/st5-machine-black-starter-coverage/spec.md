# st5-machine-black-starter-coverage Specification

## Purpose
TBD - created by archiving change implement-st5-machine-black-starter. Update Purpose after archive.
## Requirements
### Requirement: Exact ST-5 Machine Black starter deck is represented

The system SHALL represent ST-5: Starter Deck Machine Black as the exact starter composition of 4 Digi-Egg cards and 50 main-deck cards: ST5-01 x4, ST5-02 x4, ST5-03 x4, ST5-04 x4, ST5-05 x4, ST5-06 x4, ST5-07 x4, ST5-08 x2, ST5-09 x4, ST5-10 x4, ST5-11 x2, ST5-12 x2, ST5-13 x2, ST5-14 x4, ST5-15 x4, and ST5-16 x2.

#### Scenario: Starter deck resolves to 54 physical cards

- **WHEN** the Machine Black starter deck fixture is loaded
- **THEN** it contains exactly 54 card entries
- **AND** the Digi-Egg deck contains 4 copies of ST5-01
- **AND** the main deck contains exactly 50 cards across ST5-02 through ST5-16 with the specified counts

#### Scenario: Starter deck uses only implemented Rust cards

- **WHEN** the Machine Black starter deck fixture is validated against the Rust implemented-card registry
- **THEN** every unique ST5 card ID in the fixture is present in the registry
- **AND** no card in the fixture requires a legacy Python script fallback

### Requirement: Every ST5 card has faithful Rust DSL implementation

Every unique card in ST-5: Starter Deck Machine Black SHALL have a production DSL YAML file under `code/digimon-engine/cards/st5/` whose effects faithfully implement the full printed card text from local card data and the starter-deck card list. No printed clause may be omitted, stubbed, replaced by a no-op, or resolved through a hidden auto-choice.

#### Scenario: All ST5 YAML files exist

- **WHEN** the ST5 card pool is enumerated from ST5-01 through ST5-16
- **THEN** each card ID has a corresponding `code/digimon-engine/cards/st5/<CARD-ID>.yaml` file
- **AND** each YAML file compiles into the Rust DSL card registry

#### Scenario: Inherited and security clauses are authored in the correct effect sections

- **WHEN** ST5 YAML files are reviewed against printed card text
- **THEN** inherited effects are authored as inherited effects even when legacy JSON stored them in a generic effect field
- **AND** security effects for ST5-14, ST5-15, and ST5-16 are authored as security effects even when legacy JSON stored them in a non-security field

#### Scenario: Printed choices remain visible to the action system

- **WHEN** an ST5 effect gives a player a choice, including Tai Kamiya's optional Tamer suspension or target selection effects
- **THEN** the choice is surfaced through the engine's pending-selection/action-mask contract
- **AND** the effect does not select, decline, or target automatically on the player's behalf

### Requirement: ST5 behavioral tests cover printed effects

Every non-vanilla ST5 card SHALL have Rust behavioral tests under `code/digimon-engine/tests/` that exercise its printed effects through the Rust engine. Tests SHALL be written before or alongside the DSL implementation they cover and SHALL include negative cases for conditional effects.

#### Scenario: Non-vanilla card tests exist

- **WHEN** the ST5 non-vanilla card IDs are enumerated
- **THEN** behavioral tests exist for ST5-01, ST5-03, ST5-04, ST5-06, ST5-08, ST5-09, ST5-11, ST5-12, ST5-13, ST5-14, ST5-15, and ST5-16
- **AND** the tests exercise the card text through normal game resolution rather than directly mutating final state

#### Scenario: Conditional effects have positive and negative coverage

- **WHEN** a conditional ST5 effect is tested
- **THEN** at least one test proves the effect applies when its condition is true
- **AND** at least one test proves the effect does not apply when its condition is false

#### Scenario: Behavioral tests pass without ignored ST5 cases

- **WHEN** the ST5 behavioral test suite is run
- **THEN** all ST5 tests pass
- **AND** no ST5 behavioral test is marked ignored for an unresolved implementation gap

### Requirement: ST5-04 and ST5-06 inherited draw effects are faithful

ST5-04 ToyAgumon and ST5-06 Greymon SHALL implement their inherited `[End of Opponent's Turn]` draw effect faithfully: if the opponent did not attack with a Digimon during that turn, the controller draws 1 card. The effect SHALL not trigger when the opponent did attack with a Digimon during that turn.

#### Scenario: Controller draws when opponent did not attack with a Digimon

- **WHEN** ST5-04 or ST5-06 is in a Digimon's digivolution cards
- **AND** the opponent reaches end of turn without attacking with a Digimon
- **THEN** the ST5 inherited effect draws 1 card for the controller

#### Scenario: Controller does not draw after opponent attacked with a Digimon

- **WHEN** ST5-04 or ST5-06 is in a Digimon's digivolution cards
- **AND** the opponent attacked with a Digimon during that turn
- **THEN** the ST5 inherited effect does not draw a card

### Requirement: ST5-14 Tai Kamiya reacts only to blocker usage

ST5-14 Tai Kamiya SHALL implement its opponent-turn effect faithfully: when the controller uses `<Blocker>` to suspend one of their Digimon, the controller may suspend Tai Kamiya to unsuspend 1 of their Digimon.

#### Scenario: Tai can unsuspend after a blocker redirects an attack

- **WHEN** the ST5-14 controller has an unsuspended Tai Kamiya and uses one of their Digimon's `<Blocker>` to redirect an opponent's attack
- **THEN** the controller is offered Tai Kamiya's optional effect
- **AND** accepting suspends Tai Kamiya and lets the controller choose 1 of their Digimon to unsuspend

#### Scenario: Tai does not trigger for non-blocker attack redirection

- **WHEN** an attack target changes for a reason other than using one of the ST5-14 controller's Digimon's `<Blocker>`
- **THEN** ST5-14 Tai Kamiya's optional effect is not offered

#### Scenario: Declining Tai leaves permanents unchanged

- **WHEN** ST5-14 Tai Kamiya's optional effect is offered
- **AND** the controller declines the effect
- **THEN** Tai Kamiya remains unsuspended
- **AND** no Digimon is unsuspended by Tai Kamiya's effect

### Requirement: ST5 deck smoke checks run on Rust backend

The exact ST5 starter deck SHALL be usable in a Rust headless smoke test after all card implementations land.

#### Scenario: Rust headless game resets with ST5 deck

- **WHEN** a Rust headless game or `DigimonEnv` reset is configured with the exact ST5 Machine Black deck
- **THEN** reset succeeds without missing-card, script, or registry errors
- **AND** the returned action mask has the configured Rust action-space size

#### Scenario: Implemented-card loader reports all ST5 IDs

- **WHEN** `digimon_engine.load_implemented_card_ids()` is called after this change
- **THEN** it includes ST5-01 through ST5-16

