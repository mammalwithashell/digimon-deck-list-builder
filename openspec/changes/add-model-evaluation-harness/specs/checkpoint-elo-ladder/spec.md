## ADDED Requirements

### Requirement: Round-robin Elo over checkpoints and anchors
The harness SHALL compute a relative rating (Elo or TrueSkill) for a set of models by playing a seat-balanced round-robin among `{a run's checkpoints, registered champions, the greedy anchor}` and reporting a rating per model.

#### Scenario: A run's checkpoints are ranked
- **WHEN** the ladder tool is run over `models/<run>/checkpoints/` plus the greedy anchor
- **THEN** it outputs a rating per checkpoint and per anchor derived from their pairwise results

### Requirement: Greedy-anchored comparable scale
The rating scale SHALL be pinned by the greedy anchor so that ratings are comparable across training modes and across runs that share the anchor.

#### Scenario: Two runs compared on one scale
- **WHEN** the ladder includes the greedy anchor in two separate runs' ladders
- **THEN** the two runs' model ratings are expressed on the same greedy-anchored scale

### Requirement: Observation-profile cohorts
Model-versus-model games SHALL only be played between models sharing an observation profile and tensor-layout hash; the ladder SHALL record the cohort key and SHALL refuse to play two models with mismatched layout hashes.

#### Scenario: Mismatched models are not paired
- **WHEN** the ladder is given two models with different tensor-layout hashes
- **THEN** it does not play them against each other and records them in separate cohorts, bridged only by the profile-agnostic anchors

### Requirement: Cycling and forgetting detection
The ladder SHALL surface the full pairwise matchup matrix (not only the scalar ratings) so that non-transitive (rock-paper-scissors) dynamics and forgetting are visible.

#### Scenario: A later checkpoint losing to an earlier one is visible
- **WHEN** a later checkpoint has a sub-50% seat-balanced win rate against an earlier checkpoint
- **THEN** that cell is present in the reported matchup matrix

### Requirement: Rating stability reporting
The ladder SHALL enforce a minimum number of games per pair and SHALL report uncertainty (confidence interval or rating variance) alongside each rating.

#### Scenario: Ratings carry uncertainty
- **WHEN** the ladder reports a model's rating
- **THEN** it also reports the number of games and an uncertainty measure for that rating
