## ADDED Requirements

### Requirement: DSL can author under-Tamer card flow
The card-scripting DSL SHALL provide declarative steps for placing cards under
Tamers, selecting cards from under Tamers, and playing selected cards from under
Tamers with free, fixed, or reduced costs.

#### Scenario: Author places matching hand card under source Tamer
- **WHEN** YAML declares a step to select one matching Digimon from hand and
  place it under the source Tamer
- **THEN** the compiler lowers the step to pending-selection driven movement
- **AND** unsupported zone or destination fields fail compilation explicitly

#### Scenario: Author plays selected under-Tamer card
- **WHEN** YAML declares a step to select a matching card under any own Tamer
  and play it without paying the cost
- **THEN** the compiler lowers the selection and play through the under-Tamer
  play primitive
- **AND** no raw Rust placeholder is required

### Requirement: DSL can author source-stack payoff effects
The card-scripting DSL SHALL provide declarative steps for moving source cards
under Tamers, counting moved cards for later formulas, trashing opponent stacked
cards, and filtering no-source targets.

#### Scenario: Author move-sources then reduced play
- **WHEN** YAML declares an effect that moves all matching sources under a
  Tamer and then plays a matching card from hand with reduction per moved card
- **THEN** the compiler binds the moved-source count for the cost formula
- **AND** the generated process preserves player-visible choices

#### Scenario: Author trash top stacked cards
- **WHEN** YAML declares a step to trash the top N stacked cards of a selected
  opponent Digimon
- **THEN** the compiler lowers it to the stack-trashing primitive
- **AND** unsupported stack positions fail compilation explicitly

### Requirement: DSL can author DigiXros wildcard modifiers
The card-scripting DSL SHALL provide declarative vocabulary for scoped DigiXros
wildcard requirement substitution.

#### Scenario: Author turn-scoped wildcard substitution
- **WHEN** YAML declares that the source card may replace one DigiXros
  requirement for the turn
- **THEN** the compiler registers a scoped DigiXros wildcard modifier
- **AND** the modifier is not represented as a global name or trait change

### Requirement: DSL can author effect-driven attack windows
The card-scripting DSL SHALL provide declarative steps for optional immediate
attack windows and effect-driven attacks after setup selections.

#### Scenario: Author played Digimon may attack
- **WHEN** YAML declares a trigger that lets a just-played matching Digimon
  attack after a cost is paid
- **THEN** the compiler lowers it to an optional temporary attack window
- **AND** attack legality remains action-mask driven

#### Scenario: Author option mode attack player
- **WHEN** YAML declares an option mode that performs setup selections and then
  attacks a player with one own Digimon
- **THEN** the compiler lowers each choice to pending selections
- **AND** the final attack resolves through normal attack handling
