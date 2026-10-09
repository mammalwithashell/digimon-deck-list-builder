## ADDED Requirements

### Requirement: Code orchestrates and models perform single judgment tasks
The card loop SHALL run its state machine, oracle submission, test gates, merges, ledger writes and reports in deterministic code, and SHALL invoke a model worker only for judgment tasks (implementing, authoring, triage, review, Q&A classification), each as a fresh process receiving a task packet and returning a schema-validated result.

#### Scenario: Oracle round trip involves no model
- **WHEN** an authored scenario passes the sim check
- **THEN** the driver submits it to the oracle, waits for the result, diffs it and writes the verdict without invoking a worker

#### Scenario: Invalid worker output
- **WHEN** a worker returns output that fails its stage schema
- **THEN** the driver retries once with the validation error appended, and records a correction against the attempt if the retry also fails

### Requirement: Items end adjudicated, not merely confirmed
Every clause and interaction item MUST end in exactly one terminal state — `confirmed`, `dcgo_quirk` with a citation, `unreachable` with a measured reason, `unavailable`, or `escalated` — and `escalated` MUST NOT count as adjudicated.

#### Scenario: Divergence with the rules on our side
- **WHEN** an item diverges and triage cites a `general_rule.pdf` section or official ruling showing our engine is correct
- **THEN** the item may end `dcgo_quirk` with that citation, subject to the two-family rule

#### Scenario: Report leads with the unadjudicated
- **WHEN** a run stops with any item `unmeasured`, `escalated` or `unavailable`
- **THEN** the first line of the run report states those counts before the confirmed count

### Requirement: Terminating calls need agreement from both model families
A call that ends an item without our engine changing — `dcgo_quirk`, `unreachable`, a Q&A ruling classified `textual` or `not_examinable`, or an `expect_ruling:` encoding — SHALL require independent agreement from a Claude worker and a Codex worker given the same packet without each other's answer; disagreement, a missing citation, or an unavailable second family MUST escalate the item to the human queue.

#### Scenario: Families disagree
- **WHEN** one family classifies a divergence `dcgo_quirk` and the other `ours_wrong`
- **THEN** the item is written to the escalation queue with both arguments and is not adjudicated

#### Scenario: Only one family available
- **WHEN** the Codex CLI is unavailable for the run
- **THEN** every terminating call escalates instead of being decided by one family

### Requirement: Prompt-sequence failures are routed by who disagreed
The driver SHALL route a failed line back to authoring when both engines contradict the scenario's expected prompts, and SHALL route it to triage when our engine and DCGO disagree with each other.

#### Scenario: Scenario wrong
- **WHEN** both engines present a prompt the scenario did not expect
- **THEN** the item returns to authoring with the observed prompt sequence

#### Scenario: Engines disagree on a prompt
- **WHEN** DCGO offers an optional decline that our engine never prompts for
- **THEN** the item goes to triage, not to authoring

### Requirement: Fixes pass the fix gate and engine fixes stay off main
A card or YAML fix SHALL land only with a rule, ruling or DCGO citation, a test that fails before and passes after, and a green `cards_behavioral` run; an engine fix MUST additionally land on its own branch flagged for human review, and the loop MUST NOT push to `main`.

#### Scenario: Engine fix produced
- **WHEN** triage concludes `ours_wrong` and the fix touches engine code
- **THEN** the fix is committed to a separate engine branch with the citation and failing-then-passing test, and the item waits for that branch to merge

#### Scenario: Fix without citation
- **WHEN** a proposed fix cannot cite a rule, ruling or DCGO source
- **THEN** it is logged as a finding and the item escalates

### Requirement: Gaps are a ranked lane that parks and unparks cards
When implementation hits a DSL-vocabulary or engine gap, the loop SHALL park the card with the gap id, rank open gaps by the number of core clauses they block, and return parked cards to the plan when their gap closes.

#### Scenario: One gap blocks many cards
- **WHEN** three core cards park on the same gap and another gap blocks one support card
- **THEN** the three-card gap is scheduled first

#### Scenario: Gap closes
- **WHEN** a gap's fix merges
- **THEN** cards parked on it re-enter implementation on the next resume

### Requirement: Runs stop on budget, caps or plateau
The loop SHALL stop a run on its USD budget, its wall-clock cap, per-item attempt caps, or a plateau of consecutive completed attempts with no new adjudication, and SHALL write its state and report before exiting.

#### Scenario: Attempt cap
- **WHEN** an item's scenario fails authoring three times
- **THEN** the item escalates with its attempt history instead of being retried again

#### Scenario: Plateau
- **WHEN** ten consecutive completed attempts produce no new adjudication
- **THEN** the run stops and reports the plateau
