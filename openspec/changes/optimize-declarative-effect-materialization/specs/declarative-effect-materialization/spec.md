## ADDED Requirements

### Requirement: Incremental declarative-effect materialization

The engine SHALL re-materialize declarative (continuous) effects only when game
state has changed in a way that can affect them, and the resulting materialized
modifier state MUST be byte-identical to a full clear-and-rebuild from all active
declarative sources. The always-rebuild path SHALL remain available as the
reference baseline (selectable via config/env) so the two can be compared and any
divergence bisected. Per-tick `String` allocation per board card SHALL be removed
(carry a `Copy` interned card id / registry index instead), and the redundant
multiple `tick_declarative_effects` calls per action SHALL be reduced to the
minimum that preserves identical behavior.

#### Scenario: Unchanged board skips the rebuild
- **WHEN** an action does not change any declarative source or any state a
  declarative condition reads, and the engine reaches a declarative tick
- **THEN** the tick performs no clear-and-rebuild (it is a no-op) and the
  materialized modifier state is identical to what a full rebuild would produce

#### Scenario: Source change re-materializes correctly
- **WHEN** a declarative source enters or leaves the battle area, breeding area,
  or face-up security (or a stack source changes)
- **THEN** the engine re-materializes the affected declarative modifiers, and the
  resulting state equals a full clear-and-rebuild from all active sources

#### Scenario: Dynamic-condition declaratives stay correct
- **WHEN** a declarative effect's condition reads dynamic state (turn/phase,
  memory, suspended, DP, or board counts) and that state changes
- **THEN** that declarative is re-evaluated so its materialized modifier matches a
  full rebuild evaluated against the new state

### Requirement: Correctness oracle for the incremental path

Debug/test builds SHALL verify, at each materialization, that the incremental
(dirty-flag) path yields the same materialized modifiers as a fresh full rebuild,
and SHALL fail loudly on any divergence. Release builds SHALL run only the fast
path. The full behavioral, card, archetype, and parity test suites SHALL pass
while running under this oracle.

#### Scenario: Oracle confirms equivalence across the corpus
- **WHEN** the behavioral / card / archetype / parity suites run in a build with
  the oracle enabled
- **THEN** every materialization's incremental result matches its full-rebuild
  result and all suites pass

#### Scenario: Missed invalidation is caught
- **WHEN** a game mutation that should invalidate declarative state fails to set
  the dirty flag
- **THEN** the oracle detects the mismatch between the stale incremental state and
  a full rebuild and fails a test, rather than silently shipping a wrong modifier

### Requirement: Engine-step throughput is measured and behavior-preserving

A bare-engine benchmark SHALL measure engine steps/sec (and the per-phase split of
construct / mask-build / policy / engine-step) for greedy and random self-play,
and the optimization SHALL improve engine steps/sec without changing any
behavioral test outcome. The change SHALL NOT ship unless the behavioral suites
are green (under the oracle) and the benchmark shows no regression.

#### Scenario: Benchmark reports the engine-step throughput
- **WHEN** the bare-engine benchmark runs greedy and random ST-1 self-play in
  release
- **THEN** it reports games/sec, steps/sec, and the construct / mask / policy /
  engine-step breakdown

#### Scenario: Optimization improves throughput without behavior change
- **WHEN** the optimization is applied and the benchmark + behavioral suites are re-run
- **THEN** engine steps/sec is greater than the pre-change baseline AND every
  behavioral / card / archetype / parity test produces the same result as before
