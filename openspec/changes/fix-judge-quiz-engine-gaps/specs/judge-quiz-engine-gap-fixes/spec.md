## ADDED Requirements

### Requirement: General state-based ≤0-DP rules-check

The engine SHALL delete every battle-area Digimon whose effective DP is `≤ 0` via a general state-based rules-check, invoked at top-level resolution boundaries — after an effect or rule action fully resolves (the effect queue is empty), after combat DP changes resolve, and at phase transitions — and NOT mid-effect. The check SHALL re-run until no battle-area Digimon remains at `≤ 0` DP, and SHALL route deletions through the batched deletion flow so `OnDeletion` handlers fire correctly.

#### Scenario: Digimon reduced to ≤0 DP by an effect is deleted after it resolves

- **WHEN** an effect reduces a battle-area Digimon's effective DP to `≤ 0` and that effect's resolution completes
- **THEN** the state-based rules-check deletes that Digimon
- **AND** the deletion routes through the batched deletion flow (its `OnDeletion` effects fire)

#### Scenario: Deletion is deferred until the ongoing effect finishes

- **WHEN** a Digimon is at `0` DP partway through an ongoing effect's resolution but is brought back above `0` (or the effect otherwise completes) before that effect finishes
- **THEN** the Digimon is NOT deleted mid-effect — the rules-check runs only once the effect/rule action has fully resolved
- **AND** a Digimon still at `≤ 0` DP when the effect finishes is then deleted

#### Scenario: Healthy Digimon is never deleted by the rules-check

- **WHEN** the state-based rules-check runs and every battle-area Digimon has effective DP `> 0`
- **THEN** no Digimon is deleted

### Requirement: Digi-Egg cards route to the digitama deck on return-to-deck

Any engine movement that returns a card to the deck (bottom or top) SHALL route a `CardKind::DigiEgg` card to the controller's digitama (Digi-Egg) deck rather than the main deck, while still counting the card as moved for any dependent cost or count. A Digi-Egg SHALL never be placed into the main deck.

#### Scenario: Digi-Egg returned to deck bottom goes to the digitama deck

- **WHEN** a `CardKind::DigiEgg` card is returned from trash to the bottom of the deck
- **THEN** it is placed at the bottom of the controller's digitama deck
- **AND** it is NOT placed in the main deck

#### Scenario: Non-Digi-Egg card is unaffected

- **WHEN** a non-Digi-Egg card is returned to the deck
- **THEN** it is placed in the main deck as before

#### Scenario: Return still satisfies a dependent cost

- **WHEN** an effect's cost is "return N cards from trash to the bottom of the deck" and some returned cards are Digi-Eggs routed to the digitama deck
- **THEN** the cost counts all N returned cards as satisfied (the downstream effect proceeds)

### Requirement: Inherited on-trash triggered effects defer and re-check trash-presence

An inherited triggered effect on a card that is trashed (e.g. a digivolution-source "when this card is trashed" effect) SHALL be enqueued and resolved AFTER the current top-level effect completes, and at resolution SHALL activate only if its carrier card still remains in the trash. This SHALL NOT change the synchronous firing of intra-effect observers that a secondary clause of the same resolving effect consumes.

#### Scenario: On-trash effect does not resolve if its card left the trash first

- **WHEN** multiple cards are trashed (queuing their inherited on-trash effects) and a later part of the same effect removes some of those cards from the trash before the on-trash effects resolve
- **THEN** only the on-trash effects whose carrier still remains in the trash resolve
- **AND** the effects for removed cards do not activate

#### Scenario: Intra-effect observer consumption stays synchronous

- **WHEN** an effect trashes cards and a secondary clause of the SAME resolving effect must observe those just-trashed cards
- **THEN** that secondary clause still sees the trashed cards (the EX10-036 behavior is preserved)

### Requirement: Gap-exposing judge-quiz tests pass

The judge-quiz tests that these gaps blocked SHALL be un-`#[ignore]`-d and pass once the corresponding fix lands, and the closed gaps SHALL be moved to `qa/resolved-gaps.md`.

#### Scenario: Q22 pins after the Digi-Egg routing fix

- **WHEN** the Digi-Egg routing fix lands
- **THEN** `q22_digi_egg_returned_to_deck_bottom_routes_to_digitama_deck` is un-ignored and passes
- **AND** `G-RETURN-TRASH-DIGI-EGG-ROUTING` is moved to `qa/resolved-gaps.md`

#### Scenario: ≤0-DP probe pins after the rules-check fix

- **WHEN** the state-based rules-check lands
- **THEN** `zero_dp_probe_reduced_digimon_deleted_after_effect_resolves` is un-ignored and passes
- **AND** `G-NO-GENERAL-ZERO-DP-RULES-CHECK` is moved to `qa/resolved-gaps.md`

#### Scenario: No regression in the protected suites

- **WHEN** all fixes have landed
- **THEN** the `combat`, `option_flow`, `deletion_batching`, and EX10-036 behavioral tests pass with no regression
