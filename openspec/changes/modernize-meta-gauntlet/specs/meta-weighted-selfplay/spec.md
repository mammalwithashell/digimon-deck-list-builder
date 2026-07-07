# meta-weighted-selfplay — delta spec

## ADDED Requirements

### Requirement: Gauntlet exports a weighted self-play pool
The MetaGauntlet SHALL export its deduplicated deck pool with final per-deck sampling weights as a self-play pool file consumable by the Rust selfplay driver, stamped with the source snapshot hash.

#### Scenario: Export carries weights and provenance
- **WHEN** `export_selfplay_pool` is invoked on a loaded, format-windowed gauntlet
- **THEN** the emitted file contains every eligible deck with its card list, normalized weight, and the gauntlet snapshot hash

### Requirement: Weighted deck-pair sampling in the selfplay driver
The selfplay driver SHALL sample each game's deck pair from per-deck weights when the pool file provides them, independently per seat with mirror matches allowed, and SHALL retain uniform sampling when weights are absent.

#### Scenario: Weighted pool skews pair frequencies
- **WHEN** a pool assigns one deck weight 0.5 and nine decks weight 0.055... and a many-game generation runs
- **THEN** the heavy deck appears in approximately half of all seat assignments, within sampling tolerance

#### Scenario: Weightless pool preserves current behavior
- **WHEN** the pool file contains no weights
- **THEN** deck pairs are sampled uniformly, matching pre-change semantics

### Requirement: Deterministic, manifest-recorded sampling
Weighted pair sampling MUST be a pure function of the master seed, and the generation manifest SHALL record the weights, pool snapshot hash, and seed such that replaying the manifest reproduces the identical deck-pair sequence.

#### Scenario: Same seed and pool reproduce the pair sequence
- **WHEN** two driver invocations use the same pool file and master seed
- **THEN** their manifests record identical game-by-game deck-pair assignments

### Requirement: Orchestrator pass-through
The generation orchestrator SHALL accept a gauntlet-exported pool file and pass it to every driver process unchanged, recording the pool's snapshot hash in the generation summary.

#### Scenario: Generation summary carries pool provenance
- **WHEN** a generation runs with an exported weighted pool
- **THEN** the generation summary records the pool snapshot hash alongside the existing seed/config fields
