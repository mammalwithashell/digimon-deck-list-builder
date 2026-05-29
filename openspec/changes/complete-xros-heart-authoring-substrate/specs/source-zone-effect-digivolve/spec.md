## ADDED Requirements

### Requirement: Source-zone cards can be selected for effect-initiated digivolution

The engine SHALL support effects that select a card from a source-like zone,
including cards under the controller's Tamers, and use that selected card as the
digivolution card for an effect-initiated digivolution.

#### Scenario: Digivolve using a card under a Tamer

- **WHEN** an effect instructs the controller to digivolve one of their Digimon
  into a matching card under one of their Tamers without paying the cost
- **THEN** the engine presents legal under-Tamer cards through pending selection
  and commits the selected card as the new top card only after the digivolution
  is legal

#### Scenario: No matching source-zone card exists

- **WHEN** the controller has no source-zone card satisfying the printed
  digivolution predicate
- **THEN** the effect offers no hidden auto-selection and resolves without
  changing the Digimon

### Requirement: Source-zone digivolution preserves normal digivolution timing

An effect-initiated digivolution from a source-like zone SHALL use normal
digivolution commitment, source attachment, and when-digivolving timing after
the selected card is placed as the new top card.

#### Scenario: Selected source-zone card has a when-digivolving effect

- **WHEN** a Digimon digivolves using a card selected from under a Tamer
- **THEN** the new top card's when-digivolving effects are eligible to trigger
  under the same timing rules as other effect-initiated digivolutions

#### Scenario: Later legality check prevents digivolution

- **WHEN** a selected source-zone card cannot legally digivolve the chosen
  Digimon
- **THEN** the card remains in its original source-like zone and the battle area
  permanent is unchanged
