# format-warm-start-lineage — delta spec

## ADDED Requirements

### Requirement: Per-format warm start is the default
Per-format training SHALL warm-start from the previous format's promoted generalist checkpoint by default; from-scratch initialization is reserved for observation/action contract changes and scheduled rebase checks.

#### Scenario: New format run initializes from the lineage
- **WHEN** a training run is launched for a new format window with no contract change
- **THEN** the run initializes from the prior format's promoted checkpoint and records that provenance in run metadata

#### Scenario: Contract change forbids warm start
- **WHEN** the run's observation tensor layout hash differs from the checkpoint's
- **THEN** warm-start initialization is refused (existing layout-hash gate) and the run must start from scratch

### Requirement: Promotion gates do not inherit
A warm-started checkpoint SHALL be promoted only by passing its own format-scoped anchored evaluation — a seat-balanced, field-weighted matchup panel against the format's gauntlet pool with pre-registered thresholds — never by inheriting the parent's promotion status or any in-run/self-play win rate.

#### Scenario: Lineage child must pass its own gate
- **WHEN** a warm-started run finishes with strong in-run metrics but has not passed the format-scoped anchored panel
- **THEN** it is not promoted as the format's reference checkpoint

### Requirement: Promoted checkpoints freeze with format provenance
Each promoted checkpoint SHALL be frozen into the champion registry with its format window, gauntlet snapshot hash, tensor-layout hash, and anchored/exploiter results, and frozen entries are immutable thereafter.

#### Scenario: Registry entry carries reproducibility provenance
- **WHEN** a checkpoint is promoted for a format
- **THEN** its registry entry records format window, gauntlet snapshot hash, layout hash, and evaluation results

### Requirement: Periodic rebase check against lineage calcification
Every N formats (default 3), an equal-compute from-scratch training run SHALL be executed and compared to the lineage checkpoint on the same anchored frame, and the comparison result SHALL be recorded with the format's evaluation artifacts.

#### Scenario: Rebase A/B is recorded
- **WHEN** the scheduled rebase format arrives and both runs complete
- **THEN** the anchored comparison (lineage vs from-scratch at equal compute) is recorded, and the winner becomes the format's promotion candidate
