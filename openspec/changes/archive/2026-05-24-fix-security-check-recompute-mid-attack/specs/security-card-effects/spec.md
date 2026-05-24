## ADDED Requirements

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
