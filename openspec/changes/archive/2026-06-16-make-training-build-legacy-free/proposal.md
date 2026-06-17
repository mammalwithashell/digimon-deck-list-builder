## Why

The RL training stack already runs on the Rust engine (`DIGIMON_BACKEND=rust`, default profile `standard_lite_v2`), yet `code/digimon_gym/digimon_gym.py` still imports `engine_py_legacy` at module load (the Python `HeadlessGame` runner, the `FIELD_SLOTS/TARGETS_PER_ATTACKER/FIELDS_PER_HAND/SECURITY_TARGET/BREEDING_SLOT` geometry constants, and `PendingAction`). The training Docker image (`Dockerfile.training`) intentionally does **not** ship `code/engine_py_legacy/`, so the very first `import digimon_gym...` inside the image dies with `ModuleNotFoundError: No module named 'engine_py_legacy'`. This blocked the starter-deck cloud run and is invisible to CI because the image's smoke test is `--dry-run`, which returns before the real training import chain executes.

Two facts make this cheap to fix correctly rather than patch around:

1. **Every `engine_py_legacy` symbol in `digimon_gym.py` is reachable only on Python-backend code paths the Rust build never executes.** `HeadlessGame` is constructed only in `_make_runner`'s non-Rust branch (guarded by `use_rust`); the geometry constants and `PendingAction` are used only in `greedy_policy`'s Python tail, which is dead code under Rust because `greedy_policy` calls `env.runner.greedy_action()` and returns *before* touching them.
2. **The compact profile is the only thing pinning the Python backend.** `_make_runner` routes `standard_compact_v1` → Python `HeadlessGame`; everything else → Rust. With the project's decision to **drop `standard_compact_v1`** from the training env, the Python backend has no remaining consumer and can be deleted outright (the Rust engine retains its own `build_tensor_standard_compact_v1` builder independently — this change does not touch the engine).

This change makes the **training build** (`digimon_gym` env + the `run_training_job.py` → `pilot_training.train` entrypoint chain + `Dockerfile.training`) genuinely `engine_py_legacy`-free, Rust-only, and self-contained — satisfying rule 22 in letter and spirit — and adds a guardrail so it cannot silently regress. It deliberately does **not** touch the hosted API (`code/server/`), which still runs games on the Python engine; that migration is captured separately in `excise-legacy-engine-from-hosted-api` and is **not** scheduled by this change.

## What Changes

- **BREAKING (training env)** Remove the Python `HeadlessGame` backend from `DigimonEnv`. `_make_runner` becomes Rust-only: it constructs `RustHeadlessGame` for every profile and raises a clear error if the Rust wheel is missing. The `random.seed`/`seeded_choice` Python-seeding shim is removed (the Rust runner takes `seed` directly).
- **BREAKING (training env)** Retire `standard_compact_v1` from the training env. `DigimonEnv`/`get_tensor_profile` reject it with an actionable error pointing at `standard_lite_v2`. The Python-side `_legacy_standard_compact_v1()` builder and its `engine_py_legacy` imports in `tensor_profiles.py` are deleted, as is the compact arm in `tensor_profile_gauntlet.py`.
- Delete the three top-level `engine_py_legacy` imports from `digimon_gym.py` (lines 17, 124–127, 128). Remove `greedy_policy`'s Python tail (everything after the `env.runner.greedy_action()` early return); the geometry constants and `PendingAction` become unreferenced and need no vendoring. Convert the `Optional[HeadlessGame]` type annotation to a backend-neutral type.
- `code/digimon_gym/agents/architect_simulator.py` (imports `HeadlessGame`, not in the training chain) is updated to the Rust runner **or** explicitly quarantined behind a lazy import with a module-level note that it is not part of the legacy-free training surface. (Decision recorded in design.)
- Add `pyyaml` to `requirements-training.txt` (the training entrypoint imports `yaml`; currently missing — second crash discovered on the pod).
- Make the `from server.digilab_client import get_scoped_meta` call in `run_training_job.py` fail soft: the generalist (no-scoped-meta) path must not require `code/server/` to be present in the image.
- **Lean image** Trim `Dockerfile.training` so it no longer copies the eight `code/tools/*.py` scripts that import `engine_py_legacy` (`resolve_deck`, `meta_loader`, `ingest_cards`, `run_scenario`, `run_qa_batch`, `promote_script`, `train_card_autoencoder`, `check_frozen_integrity`) — copy only `run_training_job.py` and the modules the training entrypoint actually imports.
- **Guardrail** Add a test (collected by the default `code/tests` run) that imports the full training entrypoint chain (`digimon_gym.agents.pilot_training`, `tools.run_training_job`) with `engine_py_legacy` blocked via `sys.modules["engine_py_legacy"] = None`, asserting a clean import. Strengthen the CI image smoke step to exercise a real (non-`--dry-run`) import of the entrypoint so import-time regressions fail the build.

## Capabilities

### New Capabilities
- `legacy-free-training-runtime`: defines that the RL training runtime — the `DigimonEnv` Gymnasium env, the `run_training_job.py` → `pilot_training.train` entrypoint chain, and the `Dockerfile.training` image — runs exclusively on the Rust engine backend and imports zero `engine_py_legacy` symbols, with a mechanical guardrail enforcing the no-legacy-import contract.

### Modified Capabilities
- `generalist-pilot-pretraining`: no behavioral change to the deck-pool / generalist logic, but the spec adds a scenario asserting the generalist training entrypoint imports and runs without `engine_py_legacy` or `code/server/` present (the cloud-image contract).

## Impact

- **Affected code (training)**
  - `code/digimon_gym/digimon_gym.py` — delete 3 legacy imports, Python branch of `_make_runner`, `greedy_policy` Python tail; fix `Optional[HeadlessGame]` annotation
  - `code/digimon_gym/tensor_profiles.py` — remove `_legacy_standard_compact_v1()` and its legacy imports; remove compact dispatch arms
  - `code/digimon_gym/agents/tensor_profile_gauntlet.py` — drop the `standard_compact_v1` arm
  - `code/digimon_gym/agents/architect_simulator.py` — Rust runner or quarantine (design decision)
  - `code/tools/run_training_job.py` — soft-guard the `server.digilab_client` import
- **Affected build / infra**
  - `Dockerfile.training` — trim the `COPY code/tools/` step to the training entrypoint's real deps
  - `requirements-training.txt` — add `pyyaml`
  - `.github/workflows/training-image.yml` — smoke step does a real import, not just `--dry-run`
- **New tests**
  - `code/tests/rl/` (or `code/tests/`) — legacy-free import guardrail
- **No changes to**
  - The Rust engine (`code/digimon-engine/`), including its `standard_compact_v1` tensor builder
  - The hosted API (`code/server/`) — its legacy coupling is tracked by `excise-legacy-engine-from-hosted-api` and is out of scope here
  - Action space, reward shaping, deck-pool / gauntlet behavior, recording format
