# generalist-pilot-pretraining Specification

## Purpose
TBD - created by archiving change generalist-pilot-pretraining. Update Purpose after archive.
## Requirements
### Requirement: Generalist mode samples both player decks from eligible archetypes

The pilot training CLI SHALL provide a generalist pretraining mode that samples both `deck1` and `deck2` from the eligible Rust DSL deck pool at episode reset. The sampler SHALL choose an archetype uniformly from the eligible archetype set, then choose a deck uniformly from that archetype's eligible decks.

#### Scenario: Generalist reset samples deck1 and deck2

- **WHEN** pilot training starts with generalist mode enabled
- **AND** the eligible pool contains multiple archetypes with one or more decks each
- **THEN** each episode reset injects a sampled `deck1` and a sampled `deck2`
- **AND** each sampled deck belongs to an eligible fully implemented Rust DSL archetype

#### Scenario: Archetypes are sampled uniformly before decklists

- **WHEN** one eligible archetype has more decklists than another eligible archetype
- **AND** the generalist sampler runs for many episodes with a fixed curriculum seed
- **THEN** archetype selection follows the uniform archetype distribution rather than the decklist-count distribution
- **AND** decks are sampled uniformly within the selected archetype

### Requirement: Training deck sources are implementation-safe

Every deck used by pilot training SHALL be validated against the Rust implemented-card registry before training begins or before a sampled deck is accepted into a training pool. This validation SHALL apply to live gauntlet decks, frozen deck-pool snapshots, generalist sampled `deck1` decks, and explicit `--deck1` / `--deck2` inputs.

#### Scenario: Explicit deck with unimplemented card fails fast

- **WHEN** the user starts pilot training with an explicit `--deck1` or `--deck2` containing a card ID absent from the Rust implemented-card registry
- **THEN** training fails before environment construction
- **AND** the error message lists the missing card ID or IDs

#### Scenario: Generalist pool excludes unimplemented decks

- **WHEN** generalist mode builds its eligible pool from `data/deck_library.json`
- **THEN** any decklist containing a card ID absent from the Rust implemented-card registry is excluded
- **AND** archetypes with no remaining eligible decks are excluded from sampling

### Requirement: Generalist deck pools are reproducible snapshots

Generalist training SHALL write a frozen deck-pool snapshot at run start unless the run is explicitly configured to reuse an existing snapshot. Snapshot deck records SHALL use stable content-addressed deck IDs derived from canonical deck card counts, not positional indexes from `data/deck_library.json`.

#### Scenario: Run writes a frozen deck-pool snapshot

- **WHEN** pilot training starts in generalist mode without a provided curriculum pool snapshot
- **THEN** the run output contains a deck-pool snapshot file
- **AND** the snapshot includes eligible archetypes, deck records, stable deck IDs, card IDs or counts, and a snapshot hash

#### Scenario: Snapshot reuse is independent of deck library order

- **WHEN** two generalist runs use the same frozen deck-pool snapshot and curriculum seed
- **AND** `data/deck_library.json` has changed order or gained additional decks between the runs
- **THEN** both runs sample the same sequence of deck IDs for `deck1` and `deck2`

### Requirement: Curriculum seeding is deterministic and separated

Generalist training SHALL support a curriculum seed that controls deck-pair sampling independently from the training seed used for model, framework, and environment randomness. Evaluation SHALL support a fixed seed or equivalent deterministic schedule for comparing tensor profiles.

#### Scenario: Same snapshot and curriculum seed reproduce deck-pair schedule

- **WHEN** two generalist runs use the same deck-pool snapshot and curriculum seed
- **THEN** episode N selects the same `deck1` deck ID and `deck2` deck ID in both runs
- **AND** this remains true when the runs use different tensor profiles

#### Scenario: Different curriculum seeds change deck-pair schedule

- **WHEN** two generalist runs use the same deck-pool snapshot but different curriculum seeds
- **THEN** their sampled deck-pair schedules differ
- **AND** both schedules still only contain eligible deck IDs from the snapshot

### Requirement: Generalist metadata records curriculum and tensor provenance

Model metadata for generalist and fine-tuned pilot runs SHALL record the training mode, sampling policy, training seed, curriculum seed, eval seed when present, deck-pool snapshot path or hash, eligible archetypes, eligible deck count, tensor profile, tensor version, tensor size, tensor layout hash, and action-space size.

#### Scenario: Generalist model has provenance metadata

- **WHEN** a generalist training run saves a model
- **THEN** the adjacent metadata sidecar identifies the run as generalist pretraining
- **AND** it records the curriculum and tensor provenance needed to compare or reproduce the run

#### Scenario: Fine-tuned model records base checkpoint

- **WHEN** a specific archetype pilot is fine-tuned from a generalist checkpoint
- **THEN** the saved metadata records the base checkpoint path or identifier
- **AND** it records the fine-tune deck configuration and tensor contract for the resulting model

### Requirement: Generalist base models can initialize archetype fine-tuning

Pilot training SHALL support loading a compatible generalist checkpoint as initialization for a later archetype-specific fine-tuning run. The fine-tune run SHALL reject incompatible tensor profiles, tensor layout hashes, or action-space sizes before training starts.

#### Scenario: Compatible generalist checkpoint starts fine-tuning

- **WHEN** the user starts a fine-tune run with a generalist checkpoint whose tensor and action contracts match the requested training environment
- **THEN** training initializes from that checkpoint
- **AND** the run uses the requested fixed archetype deck and opponent curriculum

#### Scenario: Incompatible checkpoint is rejected

- **WHEN** the user starts a fine-tune run with a checkpoint whose tensor profile, tensor layout hash, or action-space size is incompatible
- **THEN** training fails before learning starts
- **AND** the error message identifies the incompatible contract field

### Requirement: Training runbook documents the generalist workflow

The training runbook SHALL document how to pretrain a generalist pilot, how to reuse a frozen deck-pool snapshot for tensor-profile A/B comparisons, and how to fine-tune a specific archetype pilot from the generalist base.

#### Scenario: User can find generalist pretraining commands

- **WHEN** a contributor reads `docs/TRAINING_RUNBOOK.md`
- **THEN** it includes example commands for generalist pretraining, snapshot reuse, and archetype fine-tuning
- **AND** it explains the relationship between training seed, curriculum seed, eval seed, and tensor-profile comparison

### Requirement: Generalist pretraining runs under the legacy-free cloud image contract

Generalist pilot pretraining SHALL be runnable from the `Dockerfile.training` image with neither `engine_py_legacy` nor `code/server/` present. This pins the cloud-run contract: the generalist entrypoint and deck-pool machinery depend only on the Rust engine (`digimon_engine`), `data_paths`, and `digimon_gym.*`.

#### Scenario: Generalist deck pool resolves in the legacy-free image

- **WHEN** a generalist job (e.g. the six worldwide starter decks) is launched in the training image
- **AND** `engine_py_legacy` and `server` are not importable
- **THEN** the generalist deck pool resolves its archetypes against the live Rust card registry
- **AND** `pilot_training.train` begins stepping the env without import errors

