## ADDED Requirements

### Requirement: Evaluation uses a frozen independent protocol
Experiments SHALL preregister populations, list/variant splits, seeds, budgets, sample sizes, uncertainty methods and promotion thresholds before training. Held-out lists SHALL NOT enter priors, training, or tuning. Policy-only and equal-budget search-assisted results SHALL be reported separately with exact denominators.

#### Scenario: Search improves but the policy does not
- **WHEN** search-assisted win rate improves while policy-only held-out results fail the registered criterion
- **THEN** the report does not claim stronger policy-only deployment and promotion under that criterion fails

### Requirement: Counting and belief quality receive direct probes
Evaluation SHALL include long-game copy exhaustion, recovery/recycling, restricted-card and unfamiliar-variant scenarios, plus support coverage and proper probability scores. True hidden cards SHALL be available only to an isolated post-game scorer.

#### Scenario: Same public facts conceal different actual hands
- **WHEN** hidden truth is changed while entitled facts and seeds remain fixed
- **THEN** action selection remains unchanged and only the isolated scoring result may change

### Requirement: Robustness is measured with fresh fair exploiters
Frozen candidates and baselines SHALL receive equal-budget fresh exploiter training and held-out seat-balanced evaluation under the same information regime. Reports SHALL include uncertainty and identify the measured exploitability estimate as budget-limited, not a certificate.

#### Scenario: Exploiter result is inconclusive
- **WHEN** evidence cannot establish the preregistered exploiter non-regression criterion
- **THEN** robustness is reported as inconclusive and the required promotion gate remains unmet

### Requirement: Promotion requires independent evidence
Promotion SHALL require the registered held-out policy-only uplift, anchored non-regression, and exploiter non-regression criteria across the specified runs. Missing or failed evaluation SHALL block promotion without destroying checkpoints or resume progress.

#### Scenario: Mirror play looks strong but evaluation is absent
- **WHEN** a generation finishes with self-play statistics but required independent evaluation is skipped or fails
- **THEN** it remains a resumable candidate and is not promoted
