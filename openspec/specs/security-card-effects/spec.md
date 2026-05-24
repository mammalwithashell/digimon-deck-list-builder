# security-card-effects Specification

## Purpose
TBD - created by archiving change unblock-medusamon-partial-cards. Update Purpose after archive.
## Requirements
### Requirement: Declinable `[Security]` triggered effects resolve to completion

A revealed security card carrying an optional (`you may`) `[Security]` triggered effect SHALL resolve to a terminal outcome whether the controller accepts or declines it. The security-resolution state machine MUST NOT re-enqueue the `SecuritySkill` timing for the same revealed card after the effect's first drain, so a declined optional effect cannot re-install its selection indefinitely.

#### Scenario: Player declines an optional `[Security]` effect

- **WHEN** a security card with an optional `[Security]` "you may" effect is revealed during a security check and the controller chooses to decline it
- **THEN** the security-resolution state machine advances past the security-skill phase and the security check proceeds to its battle/dispose phases
- **AND** no `pending_selection` remains installed for that revealed card

#### Scenario: Player accepts an optional `[Security]` effect

- **WHEN** a security card with an optional `[Security]` effect is revealed and the controller chooses to accept it, including any follow-up selections the effect installs
- **THEN** the effect resolves fully and the security check then proceeds to its remaining phases exactly once

#### Scenario: Revealed card has no `[Security]` effect

- **WHEN** a revealed security card carries no `[Security]` triggered effect
- **THEN** the security-skill phase completes without parking and resolution proceeds normally

### Requirement: Effects can trash a chosen non-top security card

An effect SHALL be able to trash a specific security card chosen by the controlling player at any position in a security stack, not only the top or bottom card. The choice MUST be surfaced as a player selection so every legal target is exposed to the action space.

#### Scenario: Effect trashes a selected non-top security card

- **WHEN** an effect resolves that lets a player trash any one of a target player's security cards, and the target has multiple security cards
- **THEN** the player is offered a selection of every security card in the stack
- **AND** the card the player selects is removed from the security stack and placed in its owner's trash

#### Scenario: Single-card security stack

- **WHEN** the same effect resolves and the target player has exactly one security card
- **THEN** that card is the sole selectable target and is trashed when chosen

#### Scenario: Empty security stack

- **WHEN** the same effect resolves and the target player has no security cards
- **THEN** no selection is installed and the effect proceeds to its remaining steps without trashing anything

### Requirement: Security-check loop re-evaluates `<Security A.>` each iteration

During a player-targeted security-attack resolution, the engine SHALL recompute the active attacker's effective `<Security A.>` total (printed keyword bonus + `SecurityAttackChange` modifier sum + `ChangeSAttack` payloads + dynamic aura bonus, with `InvertSAttackEffect` applied via the same path used at attack declaration) at each iteration boundary, after the post-check drain finishes and before deciding whether to pop the next security card. The continuation predicate MUST compare cumulative `checks_performed` against the freshly-recomputed strike, so a mid-attack change to the active permanent — including digivolution into a new top card, modifier gain/loss, or keyword grant/revoke — affects every subsequent check in the same attack. The recompute MUST NOT extend an already-completed loop after `SecurityCheckSurvived` or `AttackerDeletedBySecurity` is decided.

#### Scenario: Mid-attack digivolve gains `<Security A. +1>`

- **WHEN** a Digimon without `<Security A. +N>` attacks the opponent's security, the first check resolves, and a post-check trigger digivolves the attacker into a Digimon with `<Security A. +1>` before the loop decides whether to continue
- **THEN** the loop recomputes the attacker's effective strike as 2, observes `checks_performed = 1 < 2`, and pops a second security card
- **AND** the second check resolves through the standard `SecuritySkill` / battle / `OnSecurityCheck` / `OnLoseSecurity` / dispose phases against the new active permanent

#### Scenario: Mid-attack de-digivolve loses `<Security A. +1>`

- **WHEN** an attacker with `<Security A. +1>` performs its first security check, a security effect or post-check trigger de-digivolves the attacker so the new top card no longer has `<Security A. +N>`, and the loop reaches its continuation decision
- **THEN** the loop recomputes the attacker's effective strike as 1, observes `checks_performed = 1 >= 1`, and ends with `SecurityCheckSurvived`
- **AND** no additional security card is popped

#### Scenario: Attacker deleted mid-check

- **WHEN** the active attacker is deleted during the `OnSecurityCheck` / battle phase of a security resolution
- **THEN** the loop terminates with `AttackerDeletedBySecurity` regardless of the strike recompute result
- **AND** the recompute is not invoked on a stale handle

#### Scenario: Attacker `<Security A.>` unchanged across checks

- **WHEN** an attacker with a stable effective strike of N performs its first security check and no mid-attack effect alters the strike
- **THEN** the loop recomputes the same N at each iteration and pops exactly N security cards (or terminates earlier if the defender's security stack empties first, triggering a `GameWon` outcome)

