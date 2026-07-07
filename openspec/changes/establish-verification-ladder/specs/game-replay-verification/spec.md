# Spec: game-replay-verification

## ADDED Requirements

### Requirement: Canonical compact game recordings
The engine SHALL support a canonical recording format that fully determines a native game: format/schema versions, a content hash of the card data it was played under, deck references, the RNG seed, and the ordered per-seat action-id sequence — with an accompanying per-step state-digest stream. Because every player decision is an action id in the 2192-action space and all engine randomness derives from the seed, replaying the recording MUST reconstruct the game exactly on the recording's engine version.

#### Scenario: Reconstruction from seed and action logs
- **WHEN** a stored recording is replayed on the engine version and card data it was recorded under
- **THEN** every step's recomputed state digest matches the stored digest stream and the final state digest matches

### Requirement: Replay legality and divergence reporting
The replay runner SHALL, at each step, recompute the action mask and assert the recorded action is legal before applying it, then compare the state digest; on the first legality or digest divergence in a game it SHALL report the game, step index, divergence type, and the digest delta, and stop replaying that game.

#### Scenario: Behavior change localizes to a step
- **WHEN** an engine change alters a decision surface exercised by a corpus game
- **THEN** the runner reports the first diverging step of that game (not a pile of downstream failures), and the existing replay stepper can be pointed at that step for forensics

### Requirement: Determinism guard
A tier-1 check SHALL replay at least one corpus recording twice (including once in a fresh process) and require byte-identical digest streams, and a source-level check SHALL restrict non-seeded RNG (`from_entropy`/`thread_rng`) to the documented no-seed fallback and policy-driver sites.

#### Scenario: Nondeterminism is caught at introduction
- **WHEN** a change introduces iteration-order nondeterminism or an unseeded RNG draw into game resolution
- **THEN** the determinism guard fails in tier 1 on that change, not weeks later

### Requirement: Golden corpus from existing assets
A committed golden corpus SHALL be maintained under `qa/replay-goldens/`, populated from (a) converted training-run recordings and (b) generated seeded games between reference policies over meta decklists from `data/deck_library.json` restricted to implemented cards. The corpus MUST replay within the tier-2 time budget.

#### Scenario: Meta decks become regression coverage
- **WHEN** the corpus generator runs
- **THEN** it produces deterministic games over implemented meta decks (key cards exercised in realistic lines) and stores them in the canonical format with digest streams

### Requirement: Blessing workflow for intended changes
When an engine change intentionally alters behavior, the corpus SHALL be re-blessed via the runner: digests regenerate, games whose recorded actions became illegal are replaced by newly generated games and retired with a reason, and the resulting committed diff constitutes the reviewable behavioral changelog of the change.

#### Scenario: Intended change produces a reviewed digest diff
- **WHEN** a faithfulness fix changes resolution order in games covered by the corpus
- **THEN** `--bless` regenerates the affected goldens and the PR shows exactly which games/steps changed, reviewed like code
