# st4-giga-green-starter-deck Specification

## Purpose
TBD - created by archiving change implement-st4-giga-green-starter-deck. Update Purpose after archive.
## Requirements
### Requirement: ST-4 cards are authored and registered in Rust DSL

Every unique card in ST-4 Giga Green (`ST4-01` through `ST4-16`) SHALL have a production card spec under the Rust engine card directory. The specs SHALL make each card visible to Rust implemented-card discovery, including cards with no printed effect text.

#### Scenario: Every ST-4 card has a Rust card spec
- **WHEN** implemented-card IDs are loaded from the Rust engine registry
- **THEN** the result includes `ST4-01`, `ST4-02`, `ST4-03`, `ST4-04`, `ST4-05`, `ST4-06`, `ST4-07`, `ST4-08`, `ST4-09`, `ST4-10`, `ST4-11`, `ST4-12`, `ST4-13`, `ST4-14`, `ST4-15`, and `ST4-16`

#### Scenario: Vanilla cards do not gain behavior
- **WHEN** a vanilla ST-4 card with no printed effect resolves through normal play, digivolution, battle, security, and inherited-effect checks
- **THEN** it has no gameplay behavior beyond its printed stats, costs, type, color, level, and normal rules text implied by card metadata

### Requirement: ST-4 effects are faithful to printed card behavior

Each non-vanilla ST-4 card SHALL implement its full printed behavior from `data/cards.json`. No printed clause SHALL be omitted, replaced with a no-op, auto-selected without a player-facing choice, or approximated by a broader trigger.

#### Scenario: Reveal-search effects preserve eligible picks and remainder placement
- **WHEN** ST4-03 or ST4-10 resolves its reveal-search effect
- **THEN** only printed-eligible revealed cards are addable to hand
- **AND** every unretrieved revealed card is placed on the bottom of the deck using the deck-ordering behavior required by the printed text

#### Scenario: Inherited attack DP applies only to attacks on Digimon
- **WHEN** a Digimon with ST4-04 or ST4-06 as an inherited source attacks an opponent's Digimon
- **THEN** that attacking Digimon gains the printed DP bonus for the turn
- **AND** the same inherited effect does not grant the bonus when the attack target is the opponent player

#### Scenario: Keyword and memory-loss effects resolve together
- **WHEN** ST4-08 is present as a Digimon
- **THEN** it has `<Blocker>`
- **AND** when it attacks, its controller loses the printed memory amount through normal memory mutation

#### Scenario: Opponent attack and block suppression expires correctly
- **WHEN** ST4-12 resolves its when-digivolving effect against an opponent Digimon
- **THEN** that Digimon cannot attack or block until the end of its controller's next turn
- **AND** the restriction no longer applies after that expiry window

#### Scenario: Digi-Burst suspend consumes the selected sources
- **WHEN** ST4-13 activates its main effect and the controller pays the printed Digi-Burst cost
- **THEN** the selected digivolution cards are trashed as the cost
- **AND** one eligible opponent Digimon is suspended
- **AND** ST4-13 has `<Piercing>` while present as a Digimon

#### Scenario: Tamer suspend-as-cost memory gain is optional
- **WHEN** an opponent Digimon becomes suspended during the ST4-14 controller's turn and ST4-14 is unsuspended
- **THEN** the controller is offered the printed optional activation
- **AND** accepting suspends ST4-14 and gains the printed memory
- **AND** declining leaves ST4-14 unsuspended and gains no memory

#### Scenario: Option security effects distinguish add-to-hand behavior
- **WHEN** ST4-15 is checked from security
- **THEN** it activates its main effect and is added to its owner's hand after resolution
- **WHEN** ST4-16 is checked from security
- **THEN** it activates its main effect without the ST4-15 add-to-hand rider

### Requirement: ST4-11 battle-deletion inherited trigger is exact

ST4-11's inherited effect SHALL trigger only when the source carrier deletes its own battle opponent in battle, the source carrier survives that battle, and the once-per-turn gate is available. It SHALL NOT trigger from unrelated battle deletions, attacks on players, effect deletion, or mutual destruction where the source carrier does not survive.

#### Scenario: Carrier deletes battle opponent and survives
- **WHEN** a Digimon with ST4-11 as an inherited source battles an opponent Digimon
- **AND** the opponent Digimon is deleted by that battle while the source carrier remains in the battle area
- **THEN** the ST4-11 controller trashes the top card of the opponent's security stack
- **AND** the once-per-turn gate is consumed

#### Scenario: Carrier does not survive the battle
- **WHEN** a Digimon with ST4-11 as an inherited source battles an opponent Digimon
- **AND** both battle participants are deleted or the source carrier otherwise does not survive
- **THEN** ST4-11 does not trash opponent security

#### Scenario: Other friendly Digimon deletes an opponent Digimon
- **WHEN** a different friendly Digimon deletes an opponent Digimon in battle while an ST4-11 inherited carrier is present elsewhere
- **THEN** ST4-11 does not trash opponent security

#### Scenario: Once-per-turn suppresses repeated triggers
- **WHEN** a Digimon with ST4-11 as an inherited source deletes and survives against two opponent Digimon in battle during the same turn
- **THEN** only the first qualifying battle trashes opponent security

### Requirement: ST-4 starter deck recipe is playable

The project SHALL expose a canonical ST-4 Giga Green starter-deck recipe with the worldwide 54-card composition: 4 Digitama cards and a legal 50-card main deck using only ST4 card IDs.

#### Scenario: Starter recipe has the expected size
- **WHEN** the ST-4 Giga Green starter-deck recipe is loaded
- **THEN** it contains exactly 4 Digitama cards
- **AND** it contains exactly 50 main-deck cards

#### Scenario: Starter recipe references implemented cards only
- **WHEN** the ST-4 Giga Green starter-deck recipe is validated against Rust implemented-card discovery
- **THEN** every card ID in the recipe is present in the implemented-card ID set

#### Scenario: Starter recipe can initialize a Rust-backed game
- **WHEN** a Rust-backed headless game is initialized with ST-4 Giga Green as a player deck
- **THEN** reset completes without missing-card, deck-size, or implemented-card filtering errors

### Requirement: ST-4 behavioral tests prove runtime behavior

Every non-vanilla ST-4 printed effect SHALL have Rust behavioral coverage that exercises runtime state changes, pending selections, timing gates, and negative cases. Tests SHALL be added before or alongside implementation code.

#### Scenario: Focused card tests pass
- **WHEN** the ST-4 behavioral test subset is run
- **THEN** all ST-4 tests pass without ignored tests for implemented behavior

#### Scenario: Existing DSL and card suites do not regress
- **WHEN** the relevant Rust engine `dsl` and `cards_behavioral` test suites are run
- **THEN** they pass with no regressions caused by ST-4 implementation

