## ADDED Requirements

### Requirement: Training runtime imports zero engine_py_legacy symbols

The RL training runtime SHALL NOT import any `engine_py_legacy.*` module. This applies to the `DigimonEnv` Gymnasium environment (`code/digimon_gym/digimon_gym.py`), the training entrypoint chain (`code/tools/run_training_job.py` → `code/digimon_gym/agents/pilot_training.py` and everything they transitively import for a generalist job), and any module that loads when those are imported.

#### Scenario: Env module imports with engine_py_legacy blocked

- **WHEN** `engine_py_legacy` is made unimportable (e.g. `sys.modules["engine_py_legacy"] = None`)
- **AND** `code/server` is absent or unimportable
- **THEN** `import digimon_gym.digimon_gym` succeeds without raising
- **AND** `import digimon_gym.agents.pilot_training` succeeds without raising
- **AND** `import tools.run_training_job` succeeds without raising

#### Scenario: A reintroduced legacy import fails the guardrail

- **WHEN** any module in the training entrypoint chain adds a top-level `import engine_py_legacy...`
- **THEN** the guardrail test in `code/tests` fails
- **AND** the failure names the offending import path

### Requirement: DigimonEnv runs exclusively on the Rust engine backend

`DigimonEnv` SHALL construct only the Rust `RustHeadlessGame` runner. There SHALL be no Python `HeadlessGame` backend branch. When the `digimon_engine` wheel is not installed, environment construction SHALL raise an actionable error instructing the user to build/install the wheel.

#### Scenario: Runner is always the Rust runner

- **WHEN** a `DigimonEnv` is reset for any supported tensor profile
- **THEN** the underlying runner is a `RustHeadlessGame` instance

#### Scenario: Missing wheel produces an actionable error

- **WHEN** the `digimon_engine` wheel is not importable
- **AND** a `DigimonEnv` is constructed
- **THEN** a `RuntimeError` is raised whose message names the wheel and the install command

### Requirement: The standard_compact_v1 profile is retired from the training env

The training env SHALL NOT serve the `standard_compact_v1` observation profile. Requesting it SHALL raise a clear error naming `standard_lite_v2` as the supported default. The Python `_legacy_standard_compact_v1` builder SHALL be removed. This requirement governs the training surface only and does not constrain the Rust engine's own tensor builders.

#### Scenario: Requesting compact profile is rejected

- **WHEN** `get_tensor_profile("standard_compact_v1")` is called
- **THEN** a `ValueError` is raised
- **AND** the message names `standard_lite_v2`

#### Scenario: Constructing the env with compact profile is rejected

- **WHEN** `DigimonEnv(tensor_profile="standard_compact_v1")` is constructed
- **THEN** a `ValueError` is raised before any game is created

### Requirement: The training image is self-contained and legacy-free

The `Dockerfile.training` image SHALL contain only the modules the training entrypoint needs and SHALL NOT contain `engine_py_legacy` or any `code/tools/*` module that imports `engine_py_legacy`. The image SHALL list `pyyaml` (and every other runtime import of the entrypoint chain) among its installed dependencies.

#### Scenario: Entrypoint imports inside the built image

- **WHEN** the built training image runs `python -c "import tools.run_training_job; import digimon_gym.agents.pilot_training"`
- **THEN** the command exits 0 with no `ModuleNotFoundError`

#### Scenario: CI smoke exercises a real import, not only --dry-run

- **WHEN** the training-image CI workflow runs its smoke step
- **THEN** the step performs a non-`--dry-run` import of the training entrypoint chain
- **AND** an import-time failure causes the CI job to fail

### Requirement: Generalist jobs do not require the hosted-server package

A generalist training job (one that does not request scoped meta) SHALL run without `code/server/` present. The `server.digilab_client` import SHALL be confined to the scoped-meta code path; when that path runs without the server package, it SHALL raise a clear error rather than failing at module import time.

#### Scenario: Generalist job runs without server package

- **WHEN** a generalist job config is executed in an environment where `import server` fails
- **THEN** the job proceeds without importing `server.*`

#### Scenario: Scoped-meta job without server fails clearly

- **WHEN** a job requesting scoped meta is executed where `import server` fails
- **THEN** an error is raised that names the missing server package and the scoped-meta requirement
