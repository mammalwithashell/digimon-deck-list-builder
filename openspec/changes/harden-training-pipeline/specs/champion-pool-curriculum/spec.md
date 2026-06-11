## ADDED Requirements

### Requirement: Opponent pools are derivable from the champion registry

The system SHALL build an `OpponentPool` manifest from the champion registry filtered to a given tensor-layout hash, via both a programmatic constructor (`OpponentPool.from_champion_registry(registry_path, layout_hash)`) and a CLI subcommand (`champion_admin.py emit-pool --out <manifest.json>`). Entries SHALL carry uniform weights by default (sampling-time PFSP remains the place for adaptive weighting). An empty compatible set SHALL be an explicit error, not an empty pool.

#### Scenario: Manifest from registry

- **WHEN** the registry holds two champions compatible with the requested layout hash and one incompatible
- **THEN** the emitted manifest contains exactly the two compatible champions with equal weights

#### Scenario: No compatible champions

- **WHEN** the registry holds no champion matching the requested layout hash
- **THEN** pool construction fails with an error naming the layout hash and the registry path

#### Scenario: Training against the derived pool

- **WHEN** a run is launched with `opponent="pool"` and a registry-derived manifest
- **THEN** opponents are sampled from the registered champions and the manifest path is recorded in the run metadata

### Requirement: The promotion cadence is documented in the training runbook

`docs/TRAINING_RUNBOOK.md` SHALL contain a standing-cadence section describing the loop: train against the registry-derived pool → run anchored eval and the Elo ladder on the result → gated promotion (≥55% vs the compatible champion panel) → registry grows → the next run derives a larger pool. The section SHALL state that in-run/self-play mirror metrics are never used for promotion decisions (CLAUDE.md rule 30).

#### Scenario: Runbook describes the loop

- **WHEN** an operator reads the runbook's cadence section
- **THEN** it specifies, in order, the commands for anchored eval, Elo ladder, gated promotion, and pool derivation for the next run

### Requirement: The promotion loop is exercised for the starter-flat control model

The change SHALL run a recorded promotion decision for `starter1_6_flat_control_v1`: play the gate panel against the compatible champions, and either register it (gate passed, or `--force` with the anchored-vs-greedy evidence recorded in the champion's `source` field) or document the failing verdict. The outcome — either way — SHALL be visible in `models/champions/registry.json` or the runbook's cadence section.

#### Scenario: Gate decision is recorded

- **WHEN** the gate panel for `starter1_6_flat_control_v1` completes
- **THEN** either the registry contains the new champion with its provenance noted, or the failing panel result is documented with the verdict
