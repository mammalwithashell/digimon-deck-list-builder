## ADDED Requirements

### Requirement: DigiXros play resolves as a transaction

The engine SHALL resolve a DigiXros play through a transaction that records the played card, controller, selected recipe materials, pre-attached materials, temporary material-zone allowances, per-material cost delta, and final DigiXros material count before the permanent enters play.

#### Scenario: Selected materials reduce play cost and become sources

- **WHEN** a player plays a Digimon by a DigiXros alt path and selects two valid materials from hand or battle area
- **THEN** the final play cost is reduced by the DigiXros path's per-material cost delta twice
- **AND** after the play cost is paid, the played Digimon enters the battle area with the selected materials as digivolution cards
- **AND** the transaction records `digixros_count` as `2`

#### Scenario: Payment failure does not consume materials

- **WHEN** a player selects valid DigiXros materials but the final play cost cannot be paid
- **THEN** the played card remains in hand
- **AND** every selected material remains in its original zone
- **AND** no DigiXros sources are attached

#### Scenario: Normal play without DigiXros is unchanged

- **WHEN** a player plays the same card without choosing a DigiXros alt path
- **THEN** the engine resolves the normal play cost and permanent creation path
- **AND** no DigiXros material selection is offered
- **AND** no DigiXros transaction context is recorded

### Requirement: DigiXros material selection validates recipe slots and origins

The engine SHALL validate selected DigiXros materials against the active recipe slots and the transaction's currently allowed origin zones. A selected card MUST satisfy one unfilled recipe slot, and a single physical card MUST NOT fill more than one slot.

#### Scenario: Invalid recipe material is masked out

- **WHEN** a DigiXros recipe requires `Shoutmon` and `Ballistamon`
- **AND** the player has an unrelated Digimon card in an allowed origin zone
- **THEN** the unrelated card is not a legal pending-selection action for that transaction

#### Scenario: Tamer-stashed source is legal only after zone extension

- **WHEN** a DigiXros transaction starts with hand and battle-area origins only
- **AND** a valid material card is under one of the controller's Tamers
- **THEN** the Tamer-stashed card is not selectable
- **WHEN** a transaction modifier grants under-Tamer material access for that play
- **THEN** the same Tamer-stashed card becomes selectable if it satisfies an unfilled recipe slot

#### Scenario: Trash material is legal only after zone extension

- **WHEN** a valid recipe material is in the controller's trash
- **AND** the active transaction has not been granted trash material access
- **THEN** the trash card is not selectable as a DigiXros material
- **WHEN** a transaction modifier grants trash material access for that play
- **THEN** the trash card becomes selectable if it satisfies an unfilled recipe slot

### Requirement: Cast-time effects can modify a pending DigiXros transaction

The engine SHALL allow eligible before-pay-cost or when-would-play effects to inspect and modify the pending DigiXros transaction before fixed cost is calculated. Supported transaction mutations SHALL include granting extra material origins, adding pre-attached materials, adding one-shot cost deltas, and declining the modifier before mutation.

#### Scenario: Taiki grants under-Tamer materials for one play

- **WHEN** a Taiki-style effect is accepted while its controller is playing a DigiXros Digimon
- **THEN** the effect pays its printed cost
- **AND** the pending DigiXros transaction allows valid material cards under the controller's Tamers for that play only
- **AND** the extra access expires after that play transaction resolves or aborts

#### Scenario: Superior Mode pre-attaches Shoutmon and unlocks trash

- **WHEN** a Superior Mode-style effect selects a valid `Shoutmon` before pay cost
- **THEN** the selected `Shoutmon` is recorded as a pre-attached material in the pending transaction
- **AND** the transaction receives the printed one-shot cost reduction
- **AND** trash becomes an allowed DigiXros material origin for the same transaction

#### Scenario: Declined modifier leaves transaction unchanged

- **WHEN** a transaction modifier is optional and the controller declines it
- **THEN** the pending DigiXros transaction keeps its prior material origins, pre-attached materials, and cost modifiers

### Requirement: DigiXros context is visible to resolving effects

The engine SHALL expose transaction-local context to effects that need to know whether the current play was a DigiXros play and how many materials were used. This context MUST be scoped to the resolving play and MUST NOT leak to later plays.

#### Scenario: On-play effect checks DigiXros count

- **WHEN** a Digimon enters play through DigiXros with three selected or pre-attached materials
- **THEN** its resolving on-play effects can observe that it was DigiXrosed
- **AND** those effects can observe a DigiXros material count of `3`

#### Scenario: Later play has no stale DigiXros context

- **WHEN** a DigiXros play resolves and then the same player performs a normal play
- **THEN** effects resolving during the normal play do not observe the previous DigiXros transaction context

### Requirement: DigiXros choices remain action-mask driven

Every player-visible DigiXros choice SHALL be represented through pending selections and legal action masks. The change MUST NOT expand `ACTION_SPACE_SIZE` or require active tensor-profile changes.

#### Scenario: Material choices appear in the pending-selection mask

- **WHEN** a DigiXros transaction asks the player to select materials
- **THEN** every legal material choice has a legal action in the current action mask
- **AND** every illegal material choice is masked out
- **AND** resolving one legal action advances the transaction according to the pending-selection contract

#### Scenario: No action-space expansion is required

- **WHEN** the DigiXros transaction support is enabled
- **THEN** `digimon_engine.ACTION_SPACE_SIZE` remains unchanged
- **AND** existing action IDs keep their prior meaning
