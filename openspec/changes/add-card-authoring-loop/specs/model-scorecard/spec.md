## ADDED Requirements

### Requirement: Every worker attempt is ledgered
The system SHALL append one row per worker call to a committed, append-only, union-merged attempt ledger recording attempt id, run id, timestamp, stage, item, model, effort, prompt version, assignment (`routed`, `explore`, or `forced` for an interaction exam routed away from the implementer's family), outcome and usage.

#### Scenario: Concurrent runs
- **WHEN** two runs on different branches each append attempts and are merged
- **THEN** the ledger contains both sets of rows without a merge conflict

### Requirement: Worker artifacts carry provenance
Every record or file a worker produces SHALL carry its attempt id, and every commit the driver makes MUST carry `Loop-Attempt:` and `Loop-Model:` trailers, so any later change can be attributed to the attempt that produced the changed content.

#### Scenario: Blame resolves to an attempt
- **WHEN** a later fix edits YAML lines a worker wrote
- **THEN** the system resolves the edited lines to the original attempt id through the commit trailers

### Requirement: Corrections are recorded per stage, immediate and late
The system SHALL record a correction event linking the corrected attempt to the correcting attempt or human whenever a review rejects, a gate fails, a later attempt or human edits worker-produced content, a terminating verdict is overturned, or an escalation resolves against a worker's call, and MUST distinguish immediate from late corrections.

#### Scenario: Late correction of a triage call
- **WHEN** a `dcgo_quirk` verdict is later overturned to `ours_wrong`
- **THEN** a late correction is recorded against both triage attempts that agreed on `dcgo_quirk`

#### Scenario: Confirmed line that never exercised its clause
- **WHEN** an interaction exam shows a previously confirmed clause line never triggered the clause
- **THEN** a late correction is recorded against the attempt that authored that line

### Requirement: Comparisons use randomized cross-assignment
The router SHALL assign a configured share (default 20%) of each stage's items to the non-default model family by a seeded hash of the item id, and the scorecard MUST partition results by prompt version.

#### Scenario: Reproducible assignment
- **WHEN** the same item is planned twice with the same seed
- **THEN** it receives the same assignment

### Requirement: A human audit sample grounds the scorecard
The system SHALL queue a random sample of accepted worker outputs per stage for human review through a CLI, record each verdict as ground truth, and use it to estimate each model's precision as an author and as a reviewer.

#### Scenario: Lenient reviewer exposed
- **WHEN** human audits disagree with outputs a reviewer model accepted
- **THEN** the scorecard lowers that reviewer's precision and the authors' acceptance rate is reported both with and without that reviewer

### Requirement: The scorecard drives per-stage routing
The scorecard SHALL report, per stage, model and prompt version, the attempts, first-pass acceptance, immediate and late correction rates, and cost per accepted-and-not-later-corrected unit with confidence intervals; routing MUST change a stage's default model only once both models have at least the minimum sample and their intervals separate, and workers MUST NOT see scorecard data.

#### Scenario: Insufficient evidence
- **WHEN** a stage has fewer than 30 attempts for one model
- **THEN** its routing stays at the configured default

#### Scenario: Clear winner
- **WHEN** both models have at least 30 attempts in a stage and one's corrected cost interval lies entirely below the other's
- **THEN** that stage routes to the cheaper model, keeping the exploration share on the other
