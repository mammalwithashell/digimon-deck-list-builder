## ADDED Requirements

### Requirement: DSL can gate inherited effects on source-carrier battle deletion survival

The DSL SHALL provide a reusable predicate or helper that allows an inherited effect to detect that its source carrier deleted that carrier's battle opponent in battle and that the source carrier survived the battle. The predicate/helper SHALL compose with existing timing, owner, cause, and once-per-turn gates. It SHALL NOT match unrelated battle deletions caused by another friendly Digimon, attacks on players, effect deletion, or battles where the source carrier does not remain in the battle area.

#### Scenario: Predicate matches source carrier deleting its battle opponent
- **WHEN** an inherited effect uses the battle-deletion-survivor predicate/helper
- **AND** its source carrier deletes the opposing battle participant by battle
- **AND** the source carrier remains in the battle area after battle resolution
- **THEN** the predicate/helper evaluates true for that trigger context

#### Scenario: Predicate rejects unrelated friendly battle deletion
- **WHEN** an inherited effect uses the battle-deletion-survivor predicate/helper
- **AND** another friendly Digimon deletes an opponent Digimon by battle
- **THEN** the predicate/helper evaluates false for the source carrier that was not a participant in that battle

#### Scenario: Predicate rejects mutual destruction
- **WHEN** an inherited effect uses the battle-deletion-survivor predicate/helper
- **AND** its source carrier and the opponent battle participant are both deleted by battle
- **THEN** the predicate/helper evaluates false because the source carrier did not survive

#### Scenario: Predicate rejects non-battle deletion
- **WHEN** an inherited effect uses the battle-deletion-survivor predicate/helper
- **AND** an opponent Digimon is deleted by an effect rather than by battle
- **THEN** the predicate/helper evaluates false
