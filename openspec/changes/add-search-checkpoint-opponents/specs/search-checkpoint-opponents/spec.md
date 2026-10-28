## ADDED Requirements

### Requirement: Frozen compatible historical opponents
The system SHALL accept weighted historical ONNX checkpoints and a recent completed-generation window, freeze their hashes and metadata for each generation, and reject missing, duplicate, current-model, incompatible or mutated artifacts.

#### Scenario: Resume after an artifact changes
- **WHEN** a pool model, metadata, mixing setting or selected entry differs on resume
- **THEN** collection fails before any worker starts and preserves existing evidence.

#### Scenario: Bootstrap has no history
- **WHEN** a requested history window has no eligible completed checkpoint and there are no explicit pins
- **THEN** the run fails with an actionable error instead of silently changing its opponent mixture.

### Requirement: Reproducible balanced actor assignment
The system SHALL use a separate deterministic seed stream and global game pairs to select an opponent and alternate the current model's opening order, independently of dynamic workers, chunk boundaries and retries.

#### Scenario: A pair is split across chunks
- **WHEN** two adjacent global games execute in separate chunks
- **THEN** they retain their assigned opponent and opposite learner opening orders.

### Requirement: Preserve search and learner target ownership
Both actors SHALL search with their own evaluator at real decisions using identical existing search settings. Historical-opponent rows MUST NOT enter the learner's training shards. Current-current games SHALL retain both actors' rows. Default no-pool behavior SHALL preserve the existing targets and RNG streams.

#### Scenario: Historical actor commits a paged choice
- **WHEN** the historical actor searches and executes a complete choice across pages
- **THEN** the engine executes legal navigation and commitment without emitting that actor's training rows or changing the temperature clock.

### Requirement: Auditable generation and cloud configuration
The system SHALL expose pool configuration in native, generation and cloud-plan entrypoints; record per-game identities, opening order, outcomes, truncations and learner rows; and bind pool assets into scheduler identity. Existing plans SHALL remain unchanged unless explicitly opted in.

#### Scenario: Diagnose apparent game coverage
- **WHEN** historical opponents were sampled
- **THEN** saved receipts distinguish total deck appearances from learner appearances and eligible training rows.
