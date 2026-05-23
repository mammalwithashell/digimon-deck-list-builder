## ADDED Requirements

### Requirement: Union selections support heterogeneous card sources

The engine and DSL SHALL support a single player-visible union selection whose candidates may come from hand, trash, breeding-area digivolution sources, and battle-area digivolution sources, with per-zone filters and name exclusions.

#### Scenario: Trash or breeding-source play is one selection

- **WHEN** a Royal Knights effect allows playing a card from trash or from under a breeding-area Digimon
- **THEN** the pending selection SHALL present all legal candidates from both zones in one choice surface
- **AND** selecting a candidate SHALL play the chosen card from its actual source zone

#### Scenario: Per-zone exclusions are enforced

- **WHEN** a union selection excludes cards with a specific name from one or more source zones
- **THEN** candidates with that excluded name SHALL NOT be legal
- **AND** other candidates that satisfy the printed filters SHALL remain selectable

#### Scenario: Different-name constraints span source zones

- **WHEN** a union selection chooses multiple Royal Knights with different names across trash and source zones
- **THEN** selecting a candidate SHALL make other candidates with the same name illegal for the same selection

### Requirement: Source-placement costs are payable from union zones

The engine and DSL SHALL support placing a selected card from hand or trash as a digivolution source as a cost before resolving the effect's success body.

#### Scenario: Hand or trash source-cost is payable

- **WHEN** a Royal Knights effect requires placing one eligible card from hand or trash as a digivolution source as a cost
- **THEN** the pending selection SHALL expose all legal hand and trash candidates
- **AND** resolving a selected candidate SHALL place that card as a source before the effect's success body resolves

#### Scenario: Unpayable source-cost suppresses the effect body

- **WHEN** no legal hand or trash candidate exists for a required source-placement cost
- **THEN** the source-placement effect SHALL NOT be offered as payable
- **AND** the effect's success body SHALL NOT resolve

#### Scenario: Optional source-cost can be declined

- **WHEN** a printed optional effect has a payable source-placement cost
- **THEN** PASS SHALL be legal
- **AND** resolving PASS SHALL decline both the cost payment and the effect's success body

### Requirement: Source-play effects bind played cards for follow-up clauses

The engine and DSL SHALL bind cards successfully played from union/source selections so later clauses can target only those cards.

#### Scenario: Played source card suppresses On Play when printed

- **WHEN** a Royal Knights effect plays a Digimon from a digivolution source and printed text says its On Play effects do not activate
- **THEN** the played Digimon SHALL enter battle without enqueueing its On Play effects

#### Scenario: Attach-self follows the played permanent

- **WHEN** a Royal Knights effect plays a card and then places the resolving source card under that played Digimon
- **THEN** the follow-up attach clause SHALL target the permanent played by that effect
- **AND** unrelated permanents with matching traits SHALL NOT receive the attached card

#### Scenario: Keyword or Rush grant follows played cards

- **WHEN** a Royal Knights effect plays one or more cards from union/source selections and then grants a printed keyword or Rush to those cards
- **THEN** the grant SHALL apply only to the cards successfully played by that effect

### Requirement: Union-source operations preserve action-mask contracts

Union/source operations SHALL use existing action and pending-selection contracts unless an explicit action/tensor contract change is planned and approved.

#### Scenario: Existing pending-selection ranges can represent the choice

- **WHEN** a union/source operation can be represented by current pending-selection action ranges
- **THEN** the implementation SHALL reuse those ranges
- **AND** `ACTION_SPACE_SIZE` SHALL NOT change

#### Scenario: Existing ranges cannot represent a required player choice

- **WHEN** a scoped union/source operation cannot faithfully expose a printed player choice through current action ranges
- **THEN** implementation SHALL stop before approximating the effect
- **AND** the required action/tensor contract change SHALL be planned separately with the action and tensor specs
