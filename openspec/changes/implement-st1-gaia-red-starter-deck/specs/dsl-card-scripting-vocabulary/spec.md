## ADDED Requirements

### Requirement: DSL can express when-this-Digimon-is-blocked effects
The DSL SHALL allow a card author to express an effect that triggers only when the source permanent or inherited carrier is the attacking Digimon and that attack becomes blocked by a declared blocker. The trigger SHALL expose enough event context to distinguish the blocked attacker from other battle-area observers and from the original attack target. The effect SHALL work for face-up and inherited source scopes without hidden auto-resolution.

#### Scenario: Inherited source triggers when its carrier is blocked
- **WHEN** a Digimon attacks with an inherited source carrying a "when this Digimon is blocked" clause
- **AND** the defender declares a legal blocker
- **THEN** the inherited clause is enqueued for that attacking carrier
- **AND** resolving the clause runs its process body exactly once

#### Scenario: Other allied Digimon do not trigger
- **WHEN** a Digimon attacks and is blocked
- **AND** another allied Digimon has a "when this Digimon is blocked" clause but is not the attacker
- **THEN** the other allied Digimon's clause is not enqueued

#### Scenario: Unblocked attacks do not trigger
- **WHEN** a Digimon attacks and the defender declines to block or has no legal blocker
- **THEN** "when this Digimon is blocked" clauses do not trigger

#### Scenario: Non-block attack target changes do not trigger
- **WHEN** an attack target changes for a reason other than a declared blocker
- **THEN** "when this Digimon is blocked" clauses do not trigger
