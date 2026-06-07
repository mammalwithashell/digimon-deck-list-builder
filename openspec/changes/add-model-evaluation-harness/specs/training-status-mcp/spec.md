## ADDED Requirements

### Requirement: Elo ladder query tool
The training-inspection MCP SHALL expose a read-only tool that returns the Elo/TrueSkill ladder and pairwise matchup matrix for a run's checkpoints and anchors, computed from local `runs/` + `models/` artifacts without importing `server.*` or any binding crate.

#### Scenario: Ladder requested for a run
- **WHEN** a client calls the Elo-ladder tool with a run name
- **THEN** the MCP returns each model's rating, uncertainty, cohort key, and the matchup matrix

### Requirement: Champion standings query tool
The training-inspection MCP SHALL expose a read-only tool that returns the current champion registry (names, provenance, profiles) and each champion's standing.

#### Scenario: Champion standings requested
- **WHEN** a client calls the champion-standings tool
- **THEN** the MCP returns the registered champions with their provenance and ratings

### Requirement: Exploitability query tool
The training-inspection MCP SHALL expose a read-only tool that returns recorded approximate-exploitability results for a run or model, including the exploiter's compute budget.

#### Scenario: Exploitability requested
- **WHEN** a client calls the exploitability tool for a run or model
- **THEN** the MCP returns the approximate exploitability value, labeled as a budget-bound lower bound, with the exploiter budget
