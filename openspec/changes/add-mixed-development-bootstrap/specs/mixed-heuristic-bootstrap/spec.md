## ADDED Requirements

### Requirement: Reproducible balanced teacher schedule
Bootstrap collection SHALL support explicit greedy-only, development-only, and
50/50 mixed teacher configurations, with fixed per-player assignments during
each game, balanced archetype/seat exposure, independent assignment randomness,
and recorded schedule/seed/deck provenance.

#### Scenario: Mixed thirteen-archetype corpus
- **WHEN** the default mixed qualification corpus collects 416 games over the
  frozen thirteen-archetype pool
- **THEN** it schedules 64 player-game appearances per archetype, 32 per teacher,
  including G/G, D/D, G/D and D/G matchups and balanced opening seats

#### Scenario: Worker count changes
- **WHEN** worker count or completion order changes for the same frozen schedule
- **THEN** teacher assignment, decks, and RNG seeds for each global game ID remain
  identical and each player keeps its assigned teacher through interruptions

### Requirement: Demonstrations remain distinct from search targets
The collector SHALL label teacher actions as demonstrations and preserve
terminal-outcome value targets with whole-game held-out separation. It MUST
exclude all rows of incomplete games from both losses and keep demonstration
data separate from search replay unless a separate explicit experiment defines
otherwise. Training SHALL balance configured teacher/archetype sampling mass
and report realized row weights.

#### Scenario: One teacher creates longer games
- **WHEN** development games produce more decision rows than greedy games
- **THEN** the optimization sampler preserves the configured teacher mixture
  instead of implicitly overweighting the longer games

#### Scenario: Capped qualification game
- **WHEN** a qualification game reaches the decision cap
- **THEN** its recording is retained, all its rows are ineligible for both
  losses, and fitting stops pending investigation or a new complete corpus

### Requirement: Common-anchor evaluation and route evidence
Teacher mixtures SHALL be compared under equal initialization, corpus fitting,
and search budgets using common fixed evaluation opponents and seat-balanced
cohorts. Reports MUST include paired uncertainty, tactical failures, route
opportunities/choices, net resource changes, game length, and truncations.

#### Scenario: Matched comparison
- **WHEN** greedy-only, development-only, and mixed models are evaluated
- **THEN** each faces the same heuristic and same frozen baseline model on the
  same seeds/list cohort, with meta-held-out and pet-same-list panels separated

#### Scenario: DNA denominator
- **WHEN** DNA usage is reported across this mixed deck pool
- **THEN** reports identify DNA-capable deck cohorts and legal DNA opportunities
  separately from decks without that capability

#### Scenario: More development with uncertain strength
- **WHEN** a mixed model evolves more often but its paired win-rate confidence
  interval does not establish the declared noninferiority criterion
- **THEN** the report marks playing-strength evidence inconclusive and does not
  automatically promote or deploy it

### Requirement: Preserve active experiments and compatibility
The feature SHALL be opt-in for a separately frozen future bootstrap. Existing
greedy defaults, model/tensor/action contracts, and active campaign inputs,
weights, replay, services, and evaluation anchors MUST remain unchanged.

#### Scenario: Preparing a comparison during a live run
- **WHEN** a mixed-teacher proposal or corpus is prepared while cloud training runs
- **THEN** all comparison artifacts use separate paths and no live campaign
  configuration or model is replaced
