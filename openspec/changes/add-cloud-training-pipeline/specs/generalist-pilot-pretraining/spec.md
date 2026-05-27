## ADDED Requirements

### Requirement: Eligible deck pool honors a declared allowed-archetype filter

The pilot training CLI and the training-job config schema SHALL accept an optional `allowed_archetypes` set that scopes the generalist eligible deck pool. When provided, the eligible pool SHALL be the intersection of (a) archetypes present in `data/deck_library.json`, (b) the DSL-IMPLEMENTED archetype set already enforced for implementation safety, and (c) the canonicalized members of `allowed_archetypes`. `MetaGauntlet` SHALL apply the same filter to gauntlet-mode opponent sampling so the scoping mechanism is shared between generalist and gauntlet curricula. Names in `allowed_archetypes` SHALL be canonicalized through the existing archetype alias index before intersection, so aliases and printed names resolve to the same canonical archetype as the deck library uses.

#### Scenario: Job config restricts the generalist pool

- **WHEN** a generalist training job config sets `allowed_archetypes` to `["Rocks", "Yellow Hybrid"]`
- **AND** both archetypes are present in `data/deck_library.json` and DSL-IMPLEMENTED
- **THEN** the eligible pool contains only those two archetypes
- **AND** every sampled `deck1` and `deck2` belongs to one of them

#### Scenario: CLI flag matches job-config behavior

- **WHEN** a generalist run is launched via `pilot_training --generalist --archetypes Rocks,Yellow-Hybrid` without a job config
- **THEN** the resolved eligible pool is identical to the pool produced by an equivalent job config

#### Scenario: Unrecognized archetype names are warned, not fatal

- **WHEN** `allowed_archetypes` contains a name that does not canonicalize to any entry in `data/deck_library.json`
- **THEN** training startup logs a warning naming the unrecognized entry
- **AND** training continues with the remaining recognized entries
- **AND** the run does not silently fall back to the full archetype set

#### Scenario: Alias name matches canonical archetype

- **WHEN** `allowed_archetypes` contains an alias (for example `"Red Hybrid"`) for a canonical archetype (for example `"Red Hybrid (AncientGreymon)"`)
- **THEN** the canonical archetype is included in the eligible pool
- **AND** the canonical name is what appears in the curriculum-pool snapshot record

#### Scenario: Implementation-safety floor still applies

- **WHEN** `allowed_archetypes` names an archetype that is not in the DSL-IMPLEMENTED set
- **THEN** that archetype is excluded from the eligible pool even though it was explicitly allowed
- **AND** training startup logs the exclusion with its reason

#### Scenario: Gauntlet opponent sampling honors the filter

- **WHEN** a gauntlet-mode run is launched with `allowed_archetypes` set to a non-empty subset of the implemented archetypes
- **THEN** every sampled opponent deck belongs to that subset

### Requirement: Resolved deck pool snapshot reflects the declared filter

The curriculum-pool snapshot written under `models/<run_id>/deck_pool_snapshot.json` SHALL contain only archetypes and decks that survived the `allowed_archetypes` intersection at load time. The snapshot's `eligible_archetypes` array SHALL match the resolved set after canonicalization and DSL-implementation filtering, in canonical form. Reusing the snapshot via `--curriculum-pool` SHALL reproduce the same eligible pool regardless of later changes to `data/deck_library.json` or the DSL-implementation ledger.

#### Scenario: Snapshot records the resolved pool

- **WHEN** a generalist run completes with `allowed_archetypes` set to a subset of the implemented archetypes
- **THEN** the snapshot's `eligible_archetypes` array equals that subset in canonical form
- **AND** every `decks[*].archetype` value belongs to the resolved subset

#### Scenario: Snapshot reuse reproduces the resolved pool

- **WHEN** a later run loads the snapshot via `--curriculum-pool`
- **AND** `data/deck_library.json` or the DSL ledger has changed in the meantime
- **THEN** the loaded pool's eligible archetypes and decks match the snapshot exactly
- **AND** `allowed_archetypes` from the new run's config is ignored in favor of the snapshot's resolved set
