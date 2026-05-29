## ADDED Requirements

### Requirement: DSL supports no-choice bottom-source trash

The DSL SHALL provide a step for trashing the bottom N digivolution source cards from a resolved permanent target without presenting a source-card choice to the player. The step SHALL accept a target binding and a positive count, SHALL trash sources from the bottom of the target's stack in bottom-up order, SHALL cap naturally at the number of available source cards, and SHALL route each trashed source card to its owner's trash.

This primitive is for printed text such as "Trash the digivolution card at the bottom of 1 of your opponent's Digimon" and "Trash 2 digivolution cards at the bottom of 1 of your opponent's Digimon." It SHALL NOT replace `select_own_sources`, `select_opponent_sources`, or other player-choice source selectors when printed text says the player chooses source cards.

#### Scenario: One bottom source is trashed with no source prompt

- **WHEN** a DSL process selects an opponent Digimon and then executes bottom-source trash with count 1
- **THEN** the bottom source card under that opponent Digimon is moved to its owner's trash
- **AND** no pending source-selection prompt is installed

#### Scenario: Two bottom sources are trashed in order

- **WHEN** a target permanent has three source cards under its top card
- **AND** a DSL process executes bottom-source trash with count 2
- **THEN** the two lowest source cards are moved to their owners' trash in bottom-up order
- **AND** the remaining source and top card stay on the permanent

#### Scenario: Count caps to available sources

- **WHEN** a target permanent has one source card under its top card
- **AND** a DSL process executes bottom-source trash with count 2
- **THEN** the one available source card is trashed
- **AND** the top card is not trashed
- **AND** the engine does not panic

#### Scenario: Player-choice source selectors remain distinct

- **WHEN** printed text requires the controller to choose a source card
- **THEN** the card YAML SHALL use a source-selection primitive rather than bottom-source trash
- **AND** the action mask SHALL expose the legal source choices

### Requirement: DSL can evaluate the opposing battled Digimon's source count

The DSL SHALL provide a battle-context predicate usable by inherited or aura-style effects to test the currently opposing battled Digimon's source count. The predicate SHALL only evaluate as true while a Digimon-vs-Digimon battle context exists, SHALL inspect the opposing battle participant relative to the source carrier, and SHALL be false during security checks, player attacks, and other non-Digimon-battle contexts.

#### Scenario: Opposing battler has no sources

- **WHEN** a Digimon carrying an inherited effect battles an opponent Digimon whose stack contains only its top card
- **AND** the inherited effect condition checks that the opposing battled Digimon has no source cards
- **THEN** the condition evaluates true for that battle

#### Scenario: Opposing battler has sources

- **WHEN** the opposing battled Digimon has one or more source cards
- **THEN** the no-source battled-opponent predicate evaluates false

#### Scenario: No battle opponent context exists

- **WHEN** the carrier attacks a player or performs security checks
- **THEN** the no-source battled-opponent predicate evaluates false
- **AND** any DP or keyword grant gated by that predicate is not applied for that non-battle context

#### Scenario: Predicate resolves relative to the carrier

- **WHEN** both players have Digimon involved in the battle
- **THEN** the predicate inspects the opponent of the carrier permanent, not merely any no-source Digimon on either battle area
