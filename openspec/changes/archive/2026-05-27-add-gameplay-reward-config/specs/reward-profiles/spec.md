## REMOVED Requirements

### Requirement: Default profile preserves byte-identical legacy reward shape

**Reason**: The legacy `DigimonEnv._compute_reward` shape is retired in favor of the gameplay-config-driven aggression shape. The `_default` profile no longer encodes legacy values — it becomes a thin pass-through `inherits: gameplay`. The byte-identical guarantee is intentionally dropped.

**Migration**:
- For NEW training runs: no action — the new gameplay shape applies automatically as the universal default.
- For RESUMES of old checkpoints trained under the legacy shape: pass `--reward-profiles-override-mismatch` on the resume command. The reward signal will differ from the checkpoint's training signal; this is an explicit operator decision.
- For external consumers asserting "default reward equals legacy": those assertions no longer hold. Re-baseline against the new gameplay shape via fresh training, or pin to the pre-change version of the codebase.
- The `test_default_profile_byte_identical.py` and `test_digivolve_shaped_profile_parity.py` regression tests are DELETED. Their replacement is `test_terminal_landscape.py` which asserts the NEW gameplay shape's terminal scalar values at representative `(turn, outcome)` cases.

## MODIFIED Requirements

### Requirement: Deprecation path for legacy reward-shaping fields

The `TrainingConfig` fields `digivolve_reward` and `dna_digivolve_bonus` SHALL remain present in this change but SHALL be unread by the reward computation pipeline (the loaded gameplay + profile components drive all shaping). Setting either field to a non-default value SHALL emit a `DeprecationWarning` from `TrainingConfig._validate` directing the user to define a custom profile or edit `gameplay.yaml`.

The `digivolve_shaping` boolean SHALL remain accepted with no warning, but it SHALL become **inert** in this change — it no longer maps to a `_digivolve_shaped` profile (that profile is removed). Setting `digivolve_shaping=True` in a config has no effect on the active reward shape; the gameplay default already includes digivolve weights universally.

The v1 deprecation timeline (warning in v1, removal in v2) for `digivolve_reward` / `dna_digivolve_bonus` is unchanged. `digivolve_shaping` is reserved for v2 removal alongside.

This change SHALL NOT remove the fields. Their removal is reserved for a follow-up proposal.

#### Scenario: Non-default digivolve_reward still fires deprecation warning

- **WHEN** `TrainingConfig(digivolve_reward=0.5)` is constructed
- **THEN** a `DeprecationWarning` SHALL be raised by `_validate`
- **AND** the warning message SHALL reference editing `gameplay.yaml` or defining a custom profile

#### Scenario: Default values do not warn

- **WHEN** `TrainingConfig()` is constructed with the default `digivolve_reward = 0.1` and `dna_digivolve_bonus = 3.9`
- **THEN** no `DeprecationWarning` SHALL fire for those fields

#### Scenario: digivolve_shaping=True is now inert (no profile mapping)

- **WHEN** `TrainingConfig(digivolve_shaping=True)` is constructed with otherwise-default values
- **THEN** no warning SHALL fire (preserves v1 contract)
- **AND** the active profile SHALL resolve via the standard archetype/override path (NOT to `_digivolve_shaped`, which no longer exists)
- **AND** training SHALL proceed under the new universal gameplay shape

## ADDED Requirements

### Requirement: Profile loader cross-file inheritance

The `ProfileLoader` SHALL support inheritance chains that cross file boundaries. When a profile in `profiles.yaml` declares `inherits: gameplay`, the loader SHALL resolve that reference against the merged namespace (`gameplay.yaml`'s profiles + `profiles.yaml`'s profiles).

Override semantics, key-params override de-dup, and `key_cards:` expansion SHALL all work transparently across file boundaries — a child profile in `profiles.yaml` can override components inherited from a parent in `gameplay.yaml`.

Profile name collisions across the two files SHALL fail at parse time (per the gameplay-reward-config capability's "Two-file loader merges namespaces" requirement). Inheritance cycles SHALL fail at parse time with a cycle path that names all involved profiles regardless of which file they live in.

#### Scenario: Cross-file inheritance resolves correctly

- **GIVEN** `gameplay.yaml` defines profile `gameplay` with `security_remove { weight: 1.5 }`
- **AND** `profiles.yaml` defines `aggressive` with `inherits: gameplay` and override `security_remove { weight: 3.0 }`
- **WHEN** the loader resolves `aggressive`
- **THEN** the resolved component list SHALL include `security_remove` with `weight: 3.0` (child override wins)
- **AND** all other components from `gameplay` SHALL be inherited unchanged
