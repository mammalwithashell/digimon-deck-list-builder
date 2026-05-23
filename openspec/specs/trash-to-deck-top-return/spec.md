# trash-to-deck-top-return Specification

## Purpose
TBD - created by archiving change unblock-medusamon-partial-cards. Update Purpose after archive.
## Requirements
### Requirement: Effects can return selected trash cards to the deck top

An effect SHALL be able to move a player-selected set of cards out of a player's trash and place them on the **top** of that player's deck (the position drawn from first). A card-scripting author MUST be able to specify the destination — deck top or deck bottom — for a trash-return step. When no destination is specified, the step SHALL default to deck bottom so existing card scripts retain their current behavior.

#### Scenario: Selected trash card returned to the deck top

- **WHEN** an effect resolves that returns one player-selected card from a player's trash to the top of that player's deck
- **THEN** the selected card is removed from the trash and placed at the deck top
- **AND** that card is the next card that player draws

#### Scenario: Destination defaults to deck bottom

- **WHEN** a trash-return step is authored without an explicit destination
- **THEN** the returned cards are placed at the deck bottom, identical to the pre-existing behavior

#### Scenario: Multiple cards returned to the top preserve order

- **WHEN** an effect returns an ordered set of selected trash cards to the deck top
- **THEN** the cards are placed at the deck top in the bound order, so the order in which they are drawn is deterministic and matches the selection order

