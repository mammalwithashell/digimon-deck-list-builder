## ADDED Requirements

### Requirement: Effects can move source cards under Tamers
The engine SHALL support effects that move all, up to N, or filtered source
cards from an own permanent's digivolution stack under an own Tamer. The source
selection and Tamer destination MUST be pending-selection driven whenever the
printed effect gives the player a choice.

#### Scenario: Move all source cards under a Tamer
- **WHEN** an effect instructs the player to place all Digimon cards from one
  own Xros Heart Digimon's digivolution cards under one of their Tamers
- **THEN** the player chooses the source permanent if more than one is legal
- **AND** the player chooses the Tamer destination if more than one is legal
- **AND** all matching source cards move under the chosen Tamer in a
  deterministic order

#### Scenario: Move up to N filtered source cards on leave battle
- **WHEN** a permanent would leave the battle area and its effect allows up to
  N matching source cards to be placed under a Tamer
- **THEN** the eligible cards are read from the pre-removal source snapshot
- **AND** the count cap is enforced by the pending selection
- **AND** ineligible source cards are masked out

### Requirement: Effects can count moved sources for follow-up cost reduction
The engine SHALL expose the number of source cards moved by an effect to later
steps in the same effect process when printed text uses that count for play-cost
reduction or another scalar.

#### Scenario: Play cost reduction equals moved source count
- **WHEN** an effect moves three source cards under a Tamer and then plays a
  matching Digimon from hand with cost reduced by one per moved card
- **THEN** the play-cost reduction is three
- **AND** if no source cards were moved, the reduction is zero

### Requirement: Effects can trash opponent source stacks
The engine SHALL support effects that trash a fixed number of stacked cards from
an opponent's Digimon. The selected target MUST be legal according to the
printed filter.

#### Scenario: Trash top N stacked cards
- **WHEN** an effect selects one opponent Digimon and trashes the top ten
  stacked cards of that Digimon
- **THEN** up to ten topmost cards from that stack are moved to trash
- **AND** if the stack has fewer than ten cards, all available stacked cards are
  trashed
- **AND** the target's remaining visible card state is legal after the movement

### Requirement: Effects can target no-source Digimon for deck return
The engine SHALL support targeting opponent Digimon with no digivolution cards
for return-to-deck effects.

#### Scenario: Return no-source target to bottom deck
- **WHEN** an attacking effect asks the player to choose an opponent Digimon
  with no digivolution cards
- **THEN** only opponent Digimon with empty source stacks are legal targets
- **AND** the selected Digimon is returned to the bottom of its owner's deck
  according to existing return-to-deck semantics
