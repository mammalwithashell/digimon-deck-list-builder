## ADDED Requirements

### Requirement: DSL expresses an inherited-source substitute-return-to-deck replacement clause

The DSL SHALL provide a clause that, scoped to an inherited (digivolution-source) card, substitutes "return this card to the bottom of the deck" for the card's would-be trash. The clause SHALL lower onto the existing replacement-effect framework. The clause SHALL be authorable in card YAML without a `raw_rust` escape.

This closes the gap tagged `G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH` and unblocks EX5-015 Clause C.

#### Scenario: Inherited card substitutes return-to-deck for trash

- **WHEN** an inherited source card that carries the substitute clause would be trashed
- **AND** the player elects to apply the replacement and pays its cost
- **THEN** the card is returned to the bottom of the owner's deck instead of being trashed

#### Scenario: Replacement is optional and declinable

- **WHEN** an inherited source card carrying the substitute clause would be trashed
- **THEN** the player is offered the replacement through `pending_selection`
- **AND** declining lets the original trash proceed unchanged

### Requirement: The substitute clause exposes its cost as a player choice

When the printed text gates the substitution behind a cost (e.g. a multi-pick of cards to move from one zone to another), that cost SHALL be exposed through `pending_selection`. The cost and the substitution SHALL resolve atomically: if the player cannot or does not complete the cost, the substitution is cancelled and the original trash proceeds, with no partial cost applied.

#### Scenario: Cost is a visible multi-pick selection

- **WHEN** the substitute clause requires the player to select cards as its cost
- **THEN** the selection is presented as a `pending_selection` with the printed minimum and maximum counts
- **AND** the selected cards move per the printed text only if the substitution resolves

#### Scenario: Atomic cost-then-cancel guard

- **WHEN** the player begins the substitute clause's cost but the cost cannot be completed
- **THEN** no part of the cost is applied
- **AND** the inherited card is trashed as originally scheduled
