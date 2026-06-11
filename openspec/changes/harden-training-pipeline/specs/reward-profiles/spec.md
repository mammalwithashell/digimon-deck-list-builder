## ADDED Requirements

### Requirement: Configured reward YAML paths fail fast when missing

When `reward_profiles_path` or `reward_gameplay_path` is set to a non-null path (including the shipped defaults) and that file does not exist at training start, `train()` SHALL raise an error naming the missing path before any training step. The silent fallback to legacy rewards on a missing file is removed. Explicitly opting into legacy rewards SHALL require setting `reward_profiles_path` to null.

#### Scenario: Typo'd profiles path fails at startup

- **WHEN** a run is configured with `reward_profiles_path: "code/digimon_gym/agents/reward/profilez.yaml"` and the file does not exist
- **THEN** training fails before step 0 with an error naming that path

#### Scenario: Missing file on a cloud image fails loudly

- **WHEN** a cloud image lacks the default `gameplay.yaml` and a run starts with default config
- **THEN** the run fails at startup naming the missing file, rather than silently training with legacy rewards

#### Scenario: Explicit legacy opt-out still works

- **WHEN** a run is configured with `reward_profiles_path: null`
- **THEN** training proceeds using the legacy reward path with no profile files required
