## Context

The training stack is already Rust-first, but `digimon_gym.py` imports `engine_py_legacy` at module scope, so the legacy-free `Dockerfile.training` image cannot import it. We confirmed (via call-graph reading, not assumption) that all legacy usage sits on Python-backend branches the Rust path never runs, and that the only thing keeping the Python backend alive is the `standard_compact_v1` profile — which the project has decided to drop. That makes "excise the Python backend from training" the correct move rather than "lazily defer the imports."

Grounding facts (current `main`/worktree state):
- `code/digimon_gym/agents/training_config.py:63` → default `tensor_profile = "standard_lite_v2"`; `pilot_training` passes `cfg.tensor_profile` everywhere. Compact is never the training default.
- `_make_runner` (`digimon_gym.py:56`) routes `standard_compact_v1` → Python `HeadlessGame`, all else → `RustHeadlessGame`. The compact default *param value* on `_make_runner` is a vestige, not a live default.
- `greedy_policy` (`digimon_gym.py:670`) early-returns via `env.runner.greedy_action()` (Rust-native heuristic) at line 698; the geometry-constant/`PendingAction` code below is dead under Rust.
- Geometry constants are used ONLY inside `greedy_policy` helpers (lines 759–806); `PendingAction` only at line 845. Once the tail is removed they are unreferenced — **no vendoring required**.
- `RustHeadlessGame` accepts `seed=` and `observation_profile=` directly (`code/digimon-engine-py/src/lib.rs`), so removing the Python `random.seed` shim loses nothing.
- `Dockerfile.training` copies all of `code/tools/`; eight of those scripts import `engine_py_legacy` and are dead weight + latent crash-on-invoke in the image.
- CI smoke (`.github/workflows/training-image.yml`) runs the entrypoint with `--dry-run`, which returns in `run_training_job.run_job` *before* `from digimon_gym.agents.pilot_training import train` — so import-time breakage ships green.

## Goals / Non-Goals

**Goals**
- `digimon_gym` env + training entrypoint chain + `Dockerfile.training` import zero `engine_py_legacy`.
- Training runs Rust-only; missing-wheel errors are actionable.
- A mechanical guardrail prevents regression; CI catches import-time failures.
- The starter-deck cloud run is unblocked by the *first* implementation step (so we can resume training before the full clean-up lands).

**Non-Goals**
- No change to the hosted API (`code/server/`) — see `excise-legacy-engine-from-hosted-api`.
- No change to the Rust engine, including its `standard_compact_v1` builder.
- No removal of `code/engine_py_legacy/` itself (it remains sunset reference + its own excluded test tree).
- Not re-homing the geometry constants into a shared module (unnecessary once the dead tail is removed; if a future Rust-native Python greedy needs them, export from `digimon_engine` then).

## Decisions

### D1 — Excise the Python backend (Option B), do not lazy-defer (Option A)
Lazy imports would leave `engine_py_legacy` a soft dependency and keep dead Python-backend code in a Rust-only product. Since compact is being dropped, the Python branch has no consumer. Delete `_make_runner`'s Python branch, the `HeadlessGame` import, the `seeded_choice` shim, and `greedy_policy`'s Python tail. Net effect: `_make_runner` always builds `RustHeadlessGame`; if `RustHeadlessGame is None`, raise the existing actionable "wheel not installed" error regardless of profile.

### D2 — Retire `standard_compact_v1` from the training env with a hard error, not a silent fallback
`get_tensor_profile("standard_compact_v1")` and `DigimonEnv(tensor_profile="standard_compact_v1")` raise `ValueError` naming `standard_lite_v2` as the replacement. Delete `_legacy_standard_compact_v1()` and the compact dispatch arms in `tensor_profiles.py`, and the compact arm in `tensor_profile_gauntlet.py`. Rationale: a silent reroute to a different-shaped tensor would corrupt any caller that still asked for 1375 floats; failing loud is safer. The Rust engine keeps its own compact builder, so this is a training-surface retirement only.

### D3 — `architect_simulator.py`: quarantine, don't migrate (this change)
`architect_simulator.py` imports `HeadlessGame` but is not in the training entrypoint chain (`pilot_training` does not import it). Migrating it to the Rust runner is a separate concern (the Q-DeckRec architect agents). For this change: move its `engine_py_legacy` import to function scope and add a module docstring note that it is **not** part of the legacy-free training surface and will not load under the training image. This keeps the guardrail (D6) scoped to the real entrypoint chain without dragging architect work in. (If `architect_simulator` is later wanted in the image, that's its own change.)

### D4 — `server.digilab_client` import in `run_training_job.py` must fail soft
The scoped-meta path imports `from server.digilab_client import get_scoped_meta`. `code/server/` is not in the training image. The import is already function-local; wrap it so that only jobs that actually request scoped meta hit it, and a generalist job (no scoped meta) never imports `server.*`. If a scoped-meta job runs in an image without `server`, raise a clear "scoped-meta requires the full server package; use the hosted runner" error.

### D5 — Lean image via allowlist COPY, not denylist
Rather than `COPY code/tools/ tools/` then trying to prune, copy only what the entrypoint needs: `run_training_job.py` plus any sibling tool modules it imports at runtime (verified by reading its import list — currently none beyond stdlib + `digimon_engine` + `data_paths` + `digimon_gym.*`). Keep `code/data_paths.py`, `code/digimon_gym/`, `training_jobs/`. This both shrinks the image and removes the eight latent legacy-importing tools. Document in the Dockerfile *why* the wholesale `code/tools/` copy was replaced.

### D6 — Guardrail = import test + real-import CI smoke
Two layers:
1. A pytest in the default `code/tests` tree that sets `sys.modules["engine_py_legacy"] = None` (forces `ImportError` on any legacy import) and `sys.modules.setdefault("server", None)`, then imports `digimon_gym.digimon_gym`, `digimon_gym.agents.pilot_training`, and `tools.run_training_job`. A clean import proves the contract. (Blocking via `None` is the standard idiom: any `import engine_py_legacy.x` then raises immediately.)
2. The CI image smoke does `python -c "import tools.run_training_job; import digimon_gym.agents.pilot_training"` inside the built image (no `--dry-run` short-circuit), so a future top-level legacy import fails the build instead of shipping.

### D7 — Sequence so the cloud run unblocks first
Order the tasks so the *entrypoint-import* fix (delete the 3 imports + Python branch/tail) and `pyyaml` land and verify before the image-trim and guardrail polish. After the first group, a rebuilt image (or an in-place patch of the running pod) can resume training; the remaining tasks harden the build.

## Risks / Trade-offs

- **A consumer still asks for `standard_compact_v1`.** Mitigation: grep confirms the only references are CLI help text, `tensor_profile_gauntlet.py` (updated here), and `_make_runner`'s vestigial default (removed). The hard error (D2) surfaces any missed caller loudly rather than silently mis-shaping tensors.
- **`greedy_action()` not present on some Rust runner variant.** Mitigation: `_make_runner` only ever builds `RustHeadlessGame`, which exposes `greedy_action`; the guardrail import test plus an existing greedy smoke confirm the path. If absent, that's a binding gap to fix in `digimon-engine-py`, not a reason to keep the Python tail.
- **Image trim accidentally drops a runtime-needed tool.** Mitigation: D5 derives the copy list from the entrypoint's actual imports; the real-import CI smoke (D6.2) catches a missing module.
- **Blocking `engine_py_legacy` via `sys.modules[...] = None` is coarse.** It is exactly the contract we want (no legacy import anywhere in the chain) and matches how the image fails. Acceptable.

## Migration Notes

- `standard_compact_v1` is removed from the *training* env only. Anyone needing 1375-float tensors for offline parity work uses the Rust `build_tensor_standard_compact_v1` builder directly or the excluded `engine_py_legacy` test tree; this is documented in the error message and `docs/TENSOR_SPEC.md` is given a one-line note.
- No data migration; no model-format change (training already emits `standard_lite_v2`-shaped observations).
- **`code/tests/rl/test_rust_python_parity.py` is deleted.** Its sole purpose was comparing the Python backend vs Rust *through `DigimonEnv`* at the `standard_compact_v1` profile — both of which this change removes (no Python backend branch; compact retired). The authoritative cross-engine parity oracle remains `code/engine_py_legacy/tests/engine/test_rust_backend_parity.py` (excluded from default collection, constructs `HeadlessGame` directly) plus the DCGO replay oracle. The DigimonEnv-level dual-backend test is obsolete under Option B.

## Open Questions

- Should `architect_simulator.py` be migrated to the Rust runner in a fast-follow rather than quarantined? (Default per D3: quarantine now, migrate later under its own change.)
