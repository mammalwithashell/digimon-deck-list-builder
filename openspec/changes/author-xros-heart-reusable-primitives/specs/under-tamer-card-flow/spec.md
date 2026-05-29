## ADDED Requirements

### Requirement: Cards can be placed under Tamers from authored zones
The engine SHALL support effects that place selected cards from authored zones,
including hand and trash, under an own Tamer. If more than one legal Tamer
destination exists, the destination choice MUST be represented by a pending
selection.

#### Scenario: Place hand card under this Tamer
- **WHEN** a Tamer effect instructs its controller to place one matching Digimon
  card from hand under itself
- **THEN** only matching hand cards are legal selection actions
- **AND** the selected card is moved under that Tamer as a source
- **AND** any follow-up reward such as draw or memory gain resolves after the
  card is placed

#### Scenario: Place card from hand or trash under a Tamer
- **WHEN** an effect instructs its controller to place one matching card from
  hand or trash under a Tamer
- **THEN** matching cards in both zones are legal pending-selection actions
- **AND** cards that fail the printed filter are masked out
- **AND** the selected card is moved from its original zone under the chosen
  Tamer

### Requirement: Cards under Tamers can be selected as effect sources
The engine SHALL support pending selections over cards under one own Tamer or
under any own Tamer. The selected card MUST retain enough origin information for
later effect steps to move, play, or count it correctly.

#### Scenario: Select a card under any own Tamer
- **WHEN** an effect asks the player to choose a matching card from under any of
  their Tamers
- **THEN** matching cards under all own Tamers are legal actions
- **AND** cards under opponent Tamers are not legal actions
- **AND** resolving the selection records both the Tamer origin and source card

#### Scenario: Empty Tamer stash skips selection
- **WHEN** an effect asks for a card under Tamers but no legal source cards
  exist
- **THEN** no pending selection is installed
- **AND** the effect follows its no-target or optional-decline path

### Requirement: Cards can be played from under Tamers
The engine SHALL support playing a selected Digimon card from under an own Tamer
without paying the cost, at a fixed cost, or with a printed play-cost reduction.
The play MUST resolve through the normal play pipeline so play triggers and
replacement hooks remain observable.

#### Scenario: Free play from under Tamer
- **WHEN** an effect selects a Digimon card under an own Tamer to play without
  paying the cost
- **THEN** the selected card is played through the normal play flow with zero
  memory cost
- **AND** the card is removed from the Tamer stack only as the play succeeds
- **AND** its on-play effects can resolve normally

#### Scenario: Cost-reduced play from under Tamer
- **WHEN** an effect selects a level 5 or higher matching card under an own
  Tamer and applies a play-cost reduction
- **THEN** the final play cost reflects the printed reduction
- **AND** payment failure does not consume the selected card from under the
  Tamer

#### Scenario: Security effect plays from under Tamer or other zones
- **WHEN** a security effect allows a matching card to be played from an
  authored zone set
- **THEN** the legal zone choices are exposed through pending selections
- **AND** the selected card is played without hidden auto-selection
