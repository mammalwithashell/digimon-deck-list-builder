# st6-venomous-violet-coverage Specification

## Purpose
TBD - created by archiving change implement-st6-venomous-violet-starter. Update Purpose after archive.
## Requirements
### Requirement: Every ST6 card has faithful Rust DSL coverage

Every unique card in ST-6: Starter Deck Venomous Violet (`ST6-01` through `ST6-16`) SHALL have a production Rust DSL YAML file under `code/digimon-engine/cards/st6/`. Each YAML file SHALL faithfully represent the card's printed text from `data/cards.json`, including timing, optionality, inherited effects, Security effects, keywords, and player-visible selections. No ST6 card clause may be omitted, stubbed, represented by a hidden auto-selection, or replaced by a no-op placeholder.

#### Scenario: ST6 card pool fully authored

- **WHEN** the ST6 card pool is enumerated from `data/cards.json`
- **THEN** each card ID from `ST6-01` through `ST6-16` has a corresponding `code/digimon-engine/cards/st6/<CARD-ID>.yaml` file
- **AND** each YAML file compiles into the embedded Rust DSL card pack

#### Scenario: Card clauses are complete

- **WHEN** an ST6 YAML file is reviewed against that card's printed text in `data/cards.json`
- **THEN** every printed effect, inherited effect, Security effect, keyword, and digivolution path is represented
- **AND** no player-visible choice is collapsed into an automatic engine decision

#### Scenario: Vanilla cards are still registry eligible

- **WHEN** an ST6 card has no printed effect text beyond standard play and digivolution metadata
- **THEN** it still has production YAML sufficient for the Rust implemented-card registry to include the card ID
- **AND** that YAML does not introduce artificial behavior

### Requirement: ST6 behavioral tests cover effect-bearing cards

Every effect-bearing ST6 card SHALL have enabled Rust behavioral tests under `code/digimon-engine/tests/cards_behavioral/st6/`. Tests SHALL exercise the positive and negative cases needed to prove printed timing, optionality, target filtering, and zone movement behavior. Tests SHALL be written before or alongside the YAML they verify.

#### Scenario: Effect-bearing card has enabled tests

- **WHEN** the ST6 implementation is complete
- **THEN** every effect-bearing ST6 card has an enabled behavioral test file or module coverage under `code/digimon-engine/tests/cards_behavioral/st6/`
- **AND** no ST6 behavioral test remains ignored for a primitive that current source proves available

#### Scenario: Inherited effects are dispatched from sources

- **WHEN** a Digimon stack attacks or is deleted while carrying an ST6 inherited source with a matching trigger
- **THEN** the inherited effect fires from the source card
- **AND** the effect mutates the correct controller's zones or Digimon according to printed text

#### Scenario: Optional choices expose decline paths

- **WHEN** an ST6 printed effect says "you may" or "up to"
- **THEN** the engine exposes the legal choices through `PendingSelection` or the action mask
- **AND** declining the optional choice preserves any mandatory printed tail that follows it

### Requirement: ST6 signature mechanics are faithfully represented

The ST6 implementation SHALL faithfully cover the deck's signature mechanics: purple trashing, trash-to-hand recursion, self-sacrifice deletion, Blocker memory loss, Retaliation grants, Digi-Burst source trashing, Security effects, and free play from trash with On Play suppression.

#### Scenario: Draw then trash from hand

- **WHEN** an inherited ST6 effect instructs the controller to draw and then trash a card in hand
- **THEN** the controller draws the required card count
- **AND** the controller chooses which hand card to trash through a legal action

#### Scenario: Trash recursion filters candidates

- **WHEN** an ST6 effect returns a card from trash to hand
- **THEN** only candidates matching the printed purple/card-kind/play-cost filter are selectable
- **AND** the selected card moves from trash to the controller's hand

#### Scenario: Retaliation grant targets up to two own Digimon

- **WHEN** `ST6-12` resolves its `[When Digivolving]` effect with eligible own Digimon
- **THEN** the controller may choose zero, one, or two legal own Digimon
- **AND** each chosen Digimon gains `Retaliation` until the end of the opponent's next turn

#### Scenario: Digi-Burst plays a level 3 purple Digimon from trash

- **WHEN** `ST6-13` activates its `[Main] <Digi-Burst 2>` effect
- **THEN** the controller chooses exactly two source cards from that Digimon to trash as the Digi-Burst cost
- **AND** after the cost is paid, the controller may play one purple level 3 Digimon card from trash without paying its memory cost

#### Scenario: Nail Bone suppresses played On Play effects

- **WHEN** `ST6-16` plays one or more Digimon cards from trash through its Main or Security effect
- **THEN** the played Digimon enter play without paying their memory costs
- **AND** any `[On Play]` effects on Digimon played by `ST6-16` do not activate

### Requirement: Venomous Violet starter deck is available as a playable fixture

The system SHALL provide a deterministic Venomous Violet starter-deck fixture or deck-library entry with the official ST6 product composition. The fixture SHALL include four `ST6-01` Digi-Eggs and the fifty-card main deck counts for `ST6-02` through `ST6-16`. It SHALL be labeled as starter/manual product data and SHALL NOT fabricate tournament meta-share or conversion-rate statistics.

#### Scenario: Starter deck counts are exact

- **WHEN** the Venomous Violet starter deck fixture is loaded
- **THEN** it contains exactly four `ST6-01` cards in the Digi-Egg deck
- **AND** its main deck contains exactly fifty cards with counts `ST6-02` x4, `ST6-03` x4, `ST6-04` x4, `ST6-05` x2, `ST6-06` x4, `ST6-07` x4, `ST6-08` x4, `ST6-09` x4, `ST6-10` x2, `ST6-11` x4, `ST6-12` x2, `ST6-13` x2, `ST6-14` x4, `ST6-15` x4, and `ST6-16` x2

#### Scenario: Starter deck has no synthetic meta stats

- **WHEN** the Venomous Violet starter deck appears in deck tooling
- **THEN** its source metadata identifies it as a starter/manual fixture
- **AND** it does not contribute fabricated DigiLab tournament share or conversion-rate values to meta sampling

### Requirement: ST6 is Rust-training and smoke-test eligible

After implementation, every card in the Venomous Violet starter deck SHALL be present in the Rust implemented-card registry, and the deck SHALL be usable by the Rust headless runner for smoke games. Eligibility SHALL be based on executable Rust effects and tests, not JSON metadata presence alone.

#### Scenario: Implemented-card registry includes ST6

- **WHEN** `digimon_engine.load_implemented_card_ids()` is called after building the Rust/PyO3 engine
- **THEN** every ST6 card ID from `ST6-01` through `ST6-16` is present in the returned set

#### Scenario: Headless smoke game accepts the starter deck

- **WHEN** a Rust headless game is started with the Venomous Violet starter deck as one or both players' decks
- **THEN** reset succeeds
- **AND** the initial action mask contains at least one legal action
- **AND** repeated legal actions can be executed without an unimplemented-card failure

#### Scenario: Contract sizes remain unchanged

- **WHEN** the ST6 implementation is complete
- **THEN** `ACTION_SPACE_SIZE` and active observation tensor layouts are unchanged
- **AND** no model metadata or frontend action/tensor constants require migration

