# search-strength-opponents — delta spec

## ADDED Requirements

### Requirement: Joint deck-and-pilot sampling
The gauntlet SHALL sample the opponent's deck and pilot as a single unit: each deck entry MAY carry a pilot binding (a frozen ONNX policy with its tensor profile and trained-deck provenance), and decks without a binding SHALL fall back to the greedy pilot.

#### Scenario: Bound deck is piloted by its bound policy
- **WHEN** a sampled deck entry carries a pilot binding to a frozen ONNX checkpoint
- **THEN** the opponent seat is driven by that checkpoint for the episode

#### Scenario: Unbound deck falls back to greedy
- **WHEN** a sampled deck entry has no pilot binding
- **THEN** the opponent seat is driven by the greedy policy and the fallback is recorded in episode info

### Requirement: Pilot-deck coherence
A pilot binding SHALL be valid only for decks the policy was trained on — an exact content-addressed deck match, or an archetype-level match when the policy was pool-trained on that archetype — and binding construction MUST reject incoherent pairs.

#### Scenario: Incoherent binding is rejected at construction
- **WHEN** a binding is declared between a checkpoint trained only on ST-deck pools and a Beelstarmon list
- **THEN** gauntlet construction fails with an error naming the incoherent pair

### Requirement: Tensor-profile validation at construction
Pilot bindings SHALL declare the checkpoint's observation tensor profile, and the wrapper MUST fail at construction when a binding's profile mismatches the environment's profile.

#### Scenario: Profile mismatch fails fast
- **WHEN** a bound checkpoint was exported for a different tensor profile than the training environment uses
- **THEN** wrapper construction raises an error before any episode runs

### Requirement: Observable opponent-strength mix
Training telemetry SHALL record, per evaluation window, the fraction of episodes played against each pilot class (greedy vs frozen-policy), so the strength of applied training pressure is measurable.

#### Scenario: Pilot mix appears in telemetry
- **WHEN** an evaluation window elapses during a run with partial pilot coverage
- **THEN** the logged metrics include the per-pilot-class episode fractions for that window
