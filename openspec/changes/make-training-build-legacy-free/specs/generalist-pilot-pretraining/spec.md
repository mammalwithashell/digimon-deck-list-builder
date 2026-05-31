## ADDED Requirements

### Requirement: Generalist pretraining runs under the legacy-free cloud image contract

Generalist pilot pretraining SHALL be runnable from the `Dockerfile.training` image with neither `engine_py_legacy` nor `code/server/` present. This pins the cloud-run contract: the generalist entrypoint and deck-pool machinery depend only on the Rust engine (`digimon_engine`), `data_paths`, and `digimon_gym.*`.

#### Scenario: Generalist deck pool resolves in the legacy-free image

- **WHEN** a generalist job (e.g. the six worldwide starter decks) is launched in the training image
- **AND** `engine_py_legacy` and `server` are not importable
- **THEN** the generalist deck pool resolves its archetypes against the live Rust card registry
- **AND** `pilot_training.train` begins stepping the env without import errors
