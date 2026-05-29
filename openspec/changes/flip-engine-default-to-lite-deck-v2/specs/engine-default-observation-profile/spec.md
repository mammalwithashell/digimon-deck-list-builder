## ADDED Requirements

### Requirement: Engine reports a single canonical default observation profile

The `digimon-engine` crate SHALL expose exactly one canonical default observation profile. This default SHALL be readable as a compile-time `TensorProfile` constant at `tensor_profiles::standard::DEFAULT_PROFILE`, returned by the function `tensor_profiles::default_profile()`, and reflected by the top-level constants `tensor::TENSOR_SIZE` and the layout that `tensor::build_tensor` produces. All four surfaces SHALL agree on the same `profile_id`, `tensor_size`, and `feature_schema_version`.

The default profile SHALL be `standard_lite_deck_v2`.

#### Scenario: Default profile constant matches lookup-by-id

- **WHEN** a Rust caller reads `tensor_profiles::default_profile()`
- **THEN** the returned `TensorProfile` has `id == "standard_lite_deck_v2"`
- **AND** is byte-equal to `tensor_profiles::profile_by_id("standard_lite_deck_v2").unwrap()`

#### Scenario: Top-level tensor constant agrees with the default profile

- **WHEN** a caller reads `digimon_engine::tensor::TENSOR_SIZE`
- **THEN** the value equals `tensor_profiles::default_profile().tensor_size`

#### Scenario: Default builder produces the default-profile tensor shape

- **WHEN** `digimon_engine::tensor::build_tensor(game, player_id, registry)` is called against any legal game state
- **THEN** the returned `Vec<f32>` has length `tensor_profiles::default_profile().tensor_size`
- **AND** the populated tensor is byte-equal to `observation::build_observation(game, player_id, registry, &tensor_profiles::default_profile())`

### Requirement: EngineContract reports the default profile shape

The Tauri `EngineContract::current()` (and equivalent ABI surfaces consumed by the desktop manifest gate and the PyO3 bindings) SHALL report `tensor_size` and `action_space_size` equal to the engine default's values. Consumers SHALL gate model compatibility against these reported values.

#### Scenario: Desktop reports the default profile's tensor size

- **WHEN** the desktop build evaluates `EngineContract::current()`
- **THEN** the returned `tensor_size` equals `tensor_profiles::default_profile().tensor_size`
- **AND** the returned `action_space_size` equals `action::space::ACTION_SPACE_SIZE`

#### Scenario: Manifest entries with non-default tensor_size are filtered out

- **WHEN** the desktop model manager receives a manifest entry whose `tensor_size` does not match the engine's reported default
- **THEN** the entry is rejected by the compatibility gate
- **AND** the entry does not appear in the list of installable models

### Requirement: Non-default profiles remain reachable by explicit ID

The engine SHALL continue to expose every profile listed by `all_profile_ids()` (including `standard_compact_v1`, `standard_lite_v2`, and `standard_full_v2`) via `profile_by_id`. Callers that explicitly request a non-default profile SHALL receive tensors built against that profile's layout, regardless of the engine's default.

#### Scenario: Explicit standard_compact_v1 caller is unaffected by the default

- **WHEN** a caller invokes `observation::build_observation(game, pid, registry, &profile_by_id("standard_compact_v1").unwrap())`
- **THEN** the returned tensor has length `profile_by_id("standard_compact_v1").unwrap().tensor_size`
- **AND** the populated values match the v1 layout regardless of what the engine default is

#### Scenario: Python bindings honour explicit profile id

- **WHEN** Python instantiates `RustHeadlessGame(..., tensor_profile="standard_lite_v2")`
- **THEN** observation tensors returned for that game have shape `(tensor_profiles::profile_by_id("standard_lite_v2").tensor_size,)`
- **AND** are unaffected by the engine's `DEFAULT_PROFILE` constant

### Requirement: Default profile shape is constant within a build

The engine default SHALL be a compile-time constant within any given build of `digimon-engine`. No runtime API SHALL change the default observation profile during the lifetime of a process.

#### Scenario: No public mutator exists

- **WHEN** the `digimon-engine` crate's public API is enumerated
- **THEN** no function, method, or static is exposed that allows callers to mutate `DEFAULT_PROFILE` or replace the value returned by `default_profile()`

#### Scenario: Two processes built from the same engine commit agree on the default

- **WHEN** two binaries are built from the same `digimon-engine` source tree and run on the same host
- **THEN** both processes return identical values from `default_profile()`, `tensor::TENSOR_SIZE`, and `EngineContract::current().tensor_size`
