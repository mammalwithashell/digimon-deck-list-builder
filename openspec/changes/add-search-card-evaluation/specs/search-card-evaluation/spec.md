## ADDED Requirements

### Requirement: Shared card measurements
Anchored policy evaluation SHALL collect the existing per-card and route counters from both players through the inner environment. Reports MUST separate anchors and panels and retain model, deck and event-contract provenance. Errors MUST remain visible as incomplete evaluation.

#### Scenario: Opponent actions and entry routes
- **WHEN** an anchored evaluation runs with card telemetry enabled
- **THEN** opponent actions, raising, promotion and DNA use the same collector semantics as PPO evaluation

#### Scenario: Interrupted window
- **WHEN** evaluation raises or hits a safety cap
- **THEN** completed games and an incomplete active game are saved without reporting a successful complete window

### Requirement: Independent periodic schedule
The worker SHALL evaluate at startup, a configurable interval defaulting to two hours, and on stable new checkpoints. It MUST persist schedule state, avoid concurrent windows and identify unchanged model windows separately from learning progress.

#### Scenario: Long collection phase
- **WHEN** the interval expires while the model is unchanged
- **THEN** the worker emits a new diagnostic window labeled with the same checkpoint hash and current collection progress

#### Scenario: New checkpoint before interval
- **WHEN** a stable new generation model becomes available
- **THEN** it is evaluated without waiting for the interval, using the fixed evaluation setup

### Requirement: Durable scoped publication
The monitor SHALL publish atomic local reports to W&B tables and versioned card-evaluation artifacts. It MUST retry failures, preserve monotonic W&B history, and expose panel, anchor, window and checkpoint identity in reports and dashboard filters. Existing PPO reports MUST remain readable.

#### Scenario: Network failure and recovery
- **WHEN** W&B publication fails and later succeeds
- **THEN** local reports remain available and confirmed artifact versions are not republished on each poll

### Requirement: Training isolation
The evaluator MUST have read-only access to training artifacts and no training control path, and SHALL use bounded resources and execution time. Policy evaluation MUST be labeled policy-only, not search-assisted gameplay telemetry.

#### Scenario: Evaluator failure
- **WHEN** the evaluation process times out or crashes
- **THEN** the failure is recorded and training continues with unchanged search settings
