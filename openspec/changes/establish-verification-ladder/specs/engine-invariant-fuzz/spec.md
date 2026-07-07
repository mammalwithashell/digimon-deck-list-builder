# Spec: engine-invariant-fuzz

## ADDED Requirements

### Requirement: Seeded random-game invariant smoke
A tier-2 suite SHALL play N seeded random-policy games over decks drawn from the implemented card pool and assert game-agnostic invariants: no panics; every action offered by the mask resolves without an engine error; no pending selection is stranded unresolvable; memory and zone counts stay within legal bounds; and cloning the game mid-selection then resolving on the clone yields the same digest as resolving on the original.

#### Scenario: Mask-legality invariant catches offer/resolve mismatches
- **WHEN** an engine change causes the mask to offer an action the resolver rejects (the historical no-op-loop class)
- **THEN** a fuzz game hits it within the smoke budget and fails with the seed, step, and action id needed to reproduce deterministically

#### Scenario: Clone-equivalence invariant
- **WHEN** a fuzz game is cloned at a pending selection and both sides resolve identically
- **THEN** their state digests match; any divergence fails with the reproduction seed

### Requirement: Deterministic reproduction of fuzz failures
Every fuzz failure SHALL be reproducible from its reported seed and step alone (the fuzz driver derives all deck choices and policy decisions from the seed), so a failure can be replayed as a debug session without rerunning the whole smoke.

#### Scenario: One-command repro
- **WHEN** the fuzz suite reports a failure with seed S at step K
- **THEN** rerunning the driver with seed S reproduces the identical game and failure at step K
