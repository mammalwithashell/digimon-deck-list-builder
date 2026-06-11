## ADDED Requirements

### Requirement: Run metadata captures git SHA and bounty parameters

`TrainingRunMetadata` SHALL include `git_sha` (best-effort `git rev-parse HEAD` at run start; the literal `"unknown"` when not resolvable, e.g. cloud images without `.git`), `bounty_threshold`, and `bounty_bonus`. These fields SHALL be present in the persisted sidecar for every new run.

#### Scenario: SHA captured in a git checkout

- **WHEN** a run starts inside a git working tree
- **THEN** the sidecar's `git_sha` equals the current HEAD commit hash

#### Scenario: Graceful degradation outside git

- **WHEN** a run starts in an environment where `git rev-parse HEAD` fails
- **THEN** the run proceeds and the sidecar records `git_sha: "unknown"`

#### Scenario: Bounty knobs auditable after the run

- **WHEN** a gauntlet run with a non-default `bounty_threshold` completes
- **THEN** the sidecar records the threshold and bonus actually used

### Requirement: Checkpoint contract validates action-space structure

Checkpoint metadata (`.meta.json`) SHALL record an `action_space_structure` tuple `(SOURCE_SELECT_END, BREEDING_SOURCE_SELECT_START, BREEDING_SOURCE_SELECT_END)` in addition to the existing `action_space_size`. `resume_from`/`init_from` validation SHALL compare the tuple when present in the checkpoint metadata and fail on mismatch; checkpoints written before this field exists SHALL produce a warning, not an error.

#### Scenario: Structure mismatch rejected

- **WHEN** a checkpoint whose recorded sub-range boundaries differ from the current engine's is loaded via `resume_from`
- **THEN** loading fails with an error naming both tuples, even if the total action-space size matches

#### Scenario: Legacy checkpoint warns only

- **WHEN** a checkpoint without `action_space_structure` is loaded
- **THEN** loading proceeds with a logged warning that structure validation was skipped

### Requirement: The job runner forwards init_from

`tools/run_training_job.py` SHALL forward an `init_from` job-config key to training, and `train()` SHALL accept `init_from` as a direct parameter (weights-only initialization, mutually exclusive with `resume_from`), so continue-from-checkpoint cloud jobs need no in-container patching.

#### Scenario: Cloud fine-tune job config

- **WHEN** a job config sets `"init_from": "/workspace/models/v22/final.zip"`
- **THEN** the launched run initializes from those weights and the base checkpoint path is recorded in the run metadata

#### Scenario: Mutual exclusion preserved

- **WHEN** a job config sets both `init_from` and `resume_from`
- **THEN** the job fails at startup with the existing mutual-exclusion error
