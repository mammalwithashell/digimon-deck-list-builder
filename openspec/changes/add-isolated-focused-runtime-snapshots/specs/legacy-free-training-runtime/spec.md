## MODIFIED Requirements

### Requirement: DigimonEnv runs exclusively on the Rust engine backend

An explicitly opted-in isolated training route SHALL construct the Rust runner from a verified immutable snapshot handle. It SHALL validate final decks after opponent/variant wrappers and reject cards or generated references outside the approved executable closure. Pilot resume/inference and Architect worker/cache identities SHALL bind source, projection, pack and observation/action contracts. Default routes remain unchanged; invalid scoped handles MUST NOT fall back to global card data or be counted as simulation draws.

`DigimonEnv` SHALL construct only the Rust `RustHeadlessGame` runner. There SHALL be no Python `HeadlessGame` backend branch. When the `digimon_engine` wheel is not installed, environment construction SHALL raise an actionable error instructing the user to build/install the wheel.

#### Scenario: Runner is always the Rust runner

- **WHEN** a `DigimonEnv` is reset for any supported tensor profile
- **THEN** the underlying runner is a `RustHeadlessGame` instance

#### Scenario: Missing wheel produces an actionable error

- **WHEN** the `digimon_engine` wheel is not importable
- **AND** a `DigimonEnv` is constructed
- **THEN** a `RuntimeError` is raised whose message names the wheel and the install command

#### Scenario: Opted-in wrapper injects an outside-scope deck

- **WHEN** an opponent, deck variant, Architect swap or subprocess supplies a member outside the sealed executable closure
- **THEN** construction or dispatch fails before play
- **AND** the scope failure is not converted into a draw or a default data fallback

### Requirement: The standard_compact_v1 profile is retired from the training env

Introducing an isolated snapshot SHALL NOT restore the retired compact training profile or alter action/tensor contracts. The compatibility reference retirement requirement remains mandatory for the new isolated training capability even if historical repair-branch guidance differs. Any separate change to an ordinary destination profile contract requires its own explicit review; this snapshot change does not authorize one.

The training env SHALL NOT serve the `standard_compact_v1` observation profile. Requesting it SHALL raise a clear error naming `standard_lite_v2` as the supported default. The Python `_legacy_standard_compact_v1` builder SHALL be removed. This requirement governs the training surface only and does not constrain the Rust engine's own tensor builders.

#### Scenario: Requesting compact profile is rejected

- **WHEN** `get_tensor_profile("standard_compact_v1")` is called
- **THEN** a `ValueError` is raised
- **AND** the message names `standard_lite_v2`

#### Scenario: Constructing the env with compact profile is rejected

- **WHEN** `DigimonEnv(tensor_profile="standard_compact_v1")` is constructed
- **THEN** a `ValueError` is raised before any game is created

#### Scenario: Isolated snapshot does not enable compact training

- **WHEN** an isolated training configuration requests standard_compact_v1
- **THEN** it refuses with the supported standard_lite_v2 guidance before game construction
