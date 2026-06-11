## ADDED Requirements

### Requirement: The single-perspective self-play opponent mode is retired

The training system SHALL reject `opponent="self-play"` at environment-construction time with an error that (a) states the structural reason (observations are built from Player 1's perspective only, so the agent would select Player 2's actions against wrong-perspective input) and (b) names the replacement recipe (`opponent="pool"` with a champion-registry-derived manifest). The `--self-play` CLI flag SHALL remain parseable and SHALL fail with the same actionable message.

#### Scenario: make_env rejects self-play

- **WHEN** `make_env(..., opponent="self-play")` is called
- **THEN** a `ValueError` is raised whose message mentions the Player-1-perspective limitation and the `opponent="pool"` replacement

#### Scenario: CLI flag fails with migration guidance

- **WHEN** `python -m digimon_gym.agents.pilot_training --self-play ...` is invoked
- **THEN** the run exits with an error before any training step, and the message includes the pool-based replacement recipe

#### Scenario: Job-runner configs fail loudly

- **WHEN** a training job config sets `"opponent": "self-play"`
- **THEN** the job fails at startup with the same actionable error, not partway through training

### Requirement: Every active opponent mode drives Player 2 through an explicit opponent policy

For every accepted `opponent` value, the constructed environment chain SHALL route all Player 2 decisions through `OpponentWrapper` with a concrete `opponent_fn` (heuristic, pool-sampled, or loaded agent). No accepted configuration may leave the Player 2 seat to be stepped by the learner or left passive.

#### Scenario: Accepted modes wrap the opponent seat

- **WHEN** `make_env` is called with any accepted `opponent` value
- **THEN** the resulting env chain contains an `OpponentWrapper` whose `opponent_fn` is not None

#### Scenario: Regression guard for future opponent modes

- **WHEN** the test suite enumerates all accepted `opponent` values
- **THEN** a test asserts each constructed chain auto-plays Player 2 (a full episode driven only by agent-side actions reaches a terminal state with both players having acted)
