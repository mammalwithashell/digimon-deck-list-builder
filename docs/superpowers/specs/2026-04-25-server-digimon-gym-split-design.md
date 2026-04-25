# Server / `digimon_gym` Split + Repo Reorg — Design Spec

**Date:** 2026-04-25
**Status:** Approved (design phase)
**Owner:** james

## Goal

Two structural changes shipped together:

1. **Split today's monolithic `digimon_gym/` package** into two focused packages and sunset the dead Python engine + transpiler:
   - **`server/`** — the FastAPI service (HTTP/WS, DB, auth, AI pipeline, storage, admin).
   - **`digimon_gym/`** — RL training and the Gymnasium environment, depending on `digimon_engine` (Rust via PyO3) for game rules.
   - **`engine_py_legacy/`** — the sunsetting Python engine and its tests, sealed off and scheduled for deletion once the Rust engine reaches parity.
   - **Deleted:** `tools/transpiler/` (C# → Python script transpiler, no longer used).

2. **Move all source code into a `code/` folder** so the repo root is dominated by docs, plans, QA, agent config, and infra files rather than mixed with code. This makes scoping (grep, sub-agent dispatch, tool permissions) sharper.

After both changes, `server` consumes `digimon_gym` and `digimon_engine`. `digimon_gym` consumes only `digimon_engine`. `engine_py_legacy/` has no inbound imports from any production package. Trained model artifacts land in a top-level `models/` directory outside `code/`.

## Non-Goals

- Migrating in-flight RL training runs. No models have been trained against today's `digimon_gym.engine`, so no checkpoint-compat work is required.
- Reaching Rust/Python engine parity. That tracker (`docs/RUST_PYTHON_PARITY.md`) is independent and continues on its own cadence; this refactor only shifts who imports what.
- Reorganizing the Rust engine internals, frontend internals, or Tauri shell internals. Those packages move locations but their internal layout is unchanged.
- Adding new features. This is a structural refactor.

## Target Repo Layout

### Root (non-code)

```
.github/                  # CI workflows — must stay (GitHub convention)
Dockerfile, *.dockerfile  # Docker build contexts
.mcp.json                 # MCP server config
.claude/                  # agent config, skills, plugins
CLAUDE.md                 # project guide
AGENTS.md                 # RL agent architecture
README.md
docs/                     # documentation
  superpowers/specs/      # design specs (this file lives here)
qa/                       # QA reports, validated_cards*.json, archetype-qa
data/                     # runtime game data (cards.json, deck_library.json, ...)
migrations/               # DB migrations (when introduced)
models/                   # trained model artifacts — NEW, see below
DCGO/                     # git submodule, behavioral source of truth
pyproject.toml            # Python package metadata, points into code/
requirements.txt          # full server install
requirements-training.txt # RL CLI install
Cargo.toml                # Rust workspace root, members in code/
.gitignore, .gitattributes, .pre-commit-config.yaml, etc.
```

`models/` is the canonical output location for trained model artifacts (SB3 zips, ONNX exports, training metadata). It is gitignored. Conventions:

- Training entrypoints (`pilot_training`, `architect_training`, etc.) write to `models/<run_id>/` by default.
- ONNX exports (`tools/export_onnx.py`) write to `models/<run_id>/<model>.onnx`.
- The hosted API's `/models/manifest.json` is built by scanning `models/` (or a curated subset) and serves artifacts from there to the desktop client.
- Tauri's runtime model cache (`dirs::data_dir()/digimon-tcg/models/`) is unchanged — that's the *consumer* side; `models/` at repo root is the *producer* side.

### `code/` (everything else)

```
code/
  server/                       # FastAPI service
    __init__.py
    api.py
    env.py                      #   moved from digimon_gym/env.py
    routers/                    #   engine-only routers
    db/                         #   models, auth, db routers
    ai/                         #   admin AI pipeline
    storage/                    #   model_resolver, spaces
    classifier/                 #   deck_tagger, meta_tier (consumed by db/routers/decks)
    digilab_client.py
    workers/
      training_worker.py        #   moved from digimon_gym/agents/
      gauntlet_orchestrator.py  #   moved from digimon_gym/agents/

  digimon_gym/                  # RL only
    __init__.py
    digimon_gym.py              #   DigimonEnv (Gymnasium)
    agents/                     #   pilot_training, gauntlet, architect_*,
                                #   deck_pool, features_extractor, maskable_recurrent
    inference/
      onnx_policy.py            #   moved from digimon_gym/engine/onnx_policy.py

  digimon-engine/               # Rust crate — source of truth (moved from root)
  digimon-engine-py/            # PyO3 bindings (moved from root, expanded — see §2)
  src-tauri/                    # Tauri shell (moved from root)
  frontend/                     # React app (moved from root)

  engine_py_legacy/             # SUNSET — slated for deletion
    engine/                     #   verbatim move of digimon_gym/engine/
    tests/                      #   verbatim move of tests/{engine,behavioral,
                                #     runners,scenarios,helpers,tools}
    README.md                   #   sunset warning, deletion trigger

  tools/                        # CLI scripts (moved from root)
                                #   tools/transpiler/ DELETED in phase 1

  tests/                        # pytest tree (moved from root)
    api/                        #   server tests
    rl/                         #   RL training tests
    classifier/                 #   classifier tests
    ai_pipeline/                #   admin AI pipeline tests
    # engine/, behavioral/, runners/, scenarios/, helpers/, tools/
    # all migrated to code/engine_py_legacy/tests/

  data_paths.py                 # shared utility (moved from digimon_gym/data_paths.py)
```

### Dependency direction

```
                 +--------------------+
                 |       server       |
                 +---------+----------+
                           |
                  +--------+--------+
                  |                 |
                  v                 v
          +--------------+  +----------------+
          |  digimon_gym |  | digimon_engine |   (Rust via PyO3)
          +-------+------+  +-------^--------+
                  |                 |
                  +-----------------+
```

`engine_py_legacy/` is not in the graph. It has no inbound imports from `server`, `digimon_gym`, `digimon-engine-py`, or any retained `tools/` script. It is reference material until deletion.

## 1. Package Responsibilities

### `code/digimon_gym/`

- **DigimonEnv** (`digimon_gym.py`) — Gymnasium environment. Drives `digimon_engine` (Rust) for rules.
- **Agents** (`agents/`) — pilot training (MLP/LSTM), gauntlet sampling, architect deck-optimizer agents, recurrent+mask PPO. CLI-driven; no DB, no HTTP. Training outputs to `<repo_root>/models/<run_id>/`.
- **Inference** (`inference/onnx_policy.py`) — ONNX policy loader used by both RL eval and live agent inference on the server.

`digimon_gym` does **not** import from `server`, `engine_py_legacy`, or any DB/FastAPI module.

### `code/server/`

- **FastAPI app** (`api.py`) — lifespan wiring, middleware, router registration.
- **Routers** (`routers/`) — engine-only HTTP routes (games, deck_tools, simulations, replays, health).
- **DB layer** (`db/`) — SQLAlchemy models, auth, DB-backed routers (auth, users, decks, friends, issues, patch_notes, admin_*, training, assets).
- **AI pipeline** (`ai/`) — admin set-run orchestrator, dispatcher, client, retrieval. Server-only.
- **Storage** (`storage/`) — DigitalOcean Spaces / model resolver. The `/models/manifest.json` endpoint reads from `<repo_root>/models/`.
- **Classifier** (`classifier/`) — deck tagger and meta-tier classifier. Live consumer is `server/db/routers/decks.py`.
- **Workers** (`workers/`) — DB-bound background tasks: `training_worker` (RL job queue, dispatches into `digimon_gym.agents`), `gauntlet_orchestrator` (gauntlet sweeps). Started/stopped from the FastAPI lifespan.
- **`digilab_client.py`** — outbound client to the digilab service.

`server` imports `digimon_gym` (for `DigimonEnv` from training_worker, `inference.onnx_policy` for live agent inference) and `digimon_engine` (for gameplay routes).

### `code/engine_py_legacy/`

- **`engine/`** — verbatim move of `digimon_gym/engine/`.
- **`tests/`** — verbatim move of the Python-engine-coupled test trees: `tests/engine/`, `tests/behavioral/`, `tests/runners/`, `tests/scenarios/`, `tests/helpers/`, `tests/tools/`. Anything that imports `digimon_gym.engine.*` lives here.
- **`README.md`** — explicit warning: "This package is sunset reference material for the Rust engine parity effort. Do not extend. Do not import from production code. Delete the directory when `docs/RUST_PYTHON_PARITY.md` is empty."

### Top-level utilities

- **`code/data_paths.py`** — canonical paths to `data/cards.json`, `data/deck_library.json`, etc. Imported by `server`, `digimon_gym`, `tools/`, and the kept tests. Located at `code/data_paths.py` (not nested in either package) to avoid forcing `tools/` to depend on either side. The `data/` directory it points to remains at repo root.

## 2. PyO3 Binding Expansion

`code/digimon-engine-py/` today exposes only `RustHeadlessGame`. The cutover from `digimon_gym.engine.*` to `digimon_engine` requires adding the following Python-visible exports. The Rust crate already implements the underlying logic for most of these — the work is wrapping, not rewriting.

| New PyO3 export | Replaces | Used by |
|---|---|---|
| `CardDatabase` (load + lookup) | `digimon_gym.engine.data.card_database.CardDatabase` | `server.api`, `server/routers/{deck_tools,deck_optimizer,debug_games}`, `digimon_gym/agents/*` |
| `parse_deck`, `validate_deck`, `parse_tts`, `expand_deck_dict`, `RESTRICTED_LIST` | `digimon_gym.engine.data.deck_loader` | `server/routers/deck_tools.py`, `digimon_gym/agents/{deck_pool,gauntlet,pilot_training}` |
| `CardKind`, `GamePhase`, `PlayerType`, `PendingAction` | `digimon_gym.engine.data.enums` | server routers, RL agents |
| `load_tested_cards`, `out_of_set_cards` | `digimon_gym.engine.data.tested_cards` | `server/routers/deck_tools.py` |
| `CardRegistry` (capacity + embedding access) | `digimon_gym.engine.data.card_registry` | `server.api`, `digimon_gym/agents/features_extractor.py` |
| `get_models_dir` | `digimon_gym.engine.model_utils` | `server/db/routers/admin_models.py` (returns `<repo_root>/models/`) |
| `load_implemented_card_ids` | `digimon_gym.engine.data.deck_finder` | `digimon_gym/agents/architect_*` |

Any export that turns out to lack Rust-side support is logged in `docs/RUST_PYTHON_PARITY.md`. Its caller stays on the legacy import path (i.e., temporarily reaches into `engine_py_legacy.engine.*`) until parity lands. Reaching into `engine_py_legacy` from production code is the explicit, short-lived escape hatch — every such site is tracked in the parity doc with an owner and a removal trigger.

## 3. Phasing

Seven PRs, each ships green. The repo reorg is its own phase to keep the rename diff reviewable.

### Phase 1 — Delete the transpiler

- Remove `tools/transpiler/` and any tests in `tests/tools/` that exercise it.
- Remove `/implement-set` skill steps that invoke the transpiler. If the skill becomes empty, delete it; otherwise leave a note that script generation is no longer in scope.
- Update `docs/TOOLS.md` to drop the transpiler section.

**Standalone, low-risk. No callers in production code.**

### Phase 2 — Expand `digimon-engine-py` bindings

- Implement the seven export groups in §2.
- Add `tests/engine/test_rust_bindings_surface.py` covering each new export (smoke-level: load, lookup, parse a known TTS deck, etc.).
- No callers cut over yet — bindings ship behind their existing `digimon_gym.engine.*` counterparts.

### Phase 3 — Cut callers over to `digimon_engine`

- Update imports in:
  - `digimon_gym/api.py`, `digimon_gym/routers/*`, `digimon_gym/db/routers/admin_models.py` (server-side).
  - `digimon_gym/agents/*`, `digimon_gym/digimon_gym.py` (RL-side).
  - Retained `tools/*` scripts that don't go to `engine_py_legacy/`.
- Anything that hits a binding gap stays on the Python engine import and gets a row in `docs/RUST_PYTHON_PARITY.md` with an owner.
- Verify: `pytest` (excluding ai_pipeline) green, `cargo test` green, `maturin develop` clean.

### Phase 4 — Move Python engine + tests to `engine_py_legacy/`

- Verbatim file move: `digimon_gym/engine/` → `engine_py_legacy/engine/`, and the Python-engine-coupled test trees (`tests/engine/`, `tests/behavioral/`, `tests/runners/`, `tests/scenarios/`, `tests/helpers/`, `tests/tools/`) → `engine_py_legacy/tests/`.
- Update internal imports inside the moved tree from `digimon_gym.engine.*` → `engine_py_legacy.engine.*`.
- Add `engine_py_legacy/README.md` (sunset warning, deletion trigger).
- `digimon_gym/engine/onnx_policy.py` is intentionally **not** moved here — it's an ONNX loader, not engine-coupled. It stays at `digimon_gym/engine/onnx_policy.py` until phase 5 relocates it to `digimon_gym/inference/`.
- Update `pyproject.toml` / `pytest.ini` to exclude `engine_py_legacy/tests` from default collection; tests still runnable explicitly.
- Confirm zero inbound imports from `server`, `digimon_gym`, `digimon-engine-py`, or retained `tools/` (other than the parity-doc-tracked exceptions).

### Phase 5 — Extract `server/` from `digimon_gym/`

- Move:
  - `digimon_gym/api.py` → `server/api.py`
  - `digimon_gym/env.py` → `server/env.py`
  - `digimon_gym/routers/` → `server/routers/`
  - `digimon_gym/db/` → `server/db/`
  - `digimon_gym/ai/` → `server/ai/`
  - `digimon_gym/storage/` → `server/storage/`
  - `digimon_gym/classifier/` → `server/classifier/`
  - `digimon_gym/digilab_client.py` → `server/digilab_client.py`
  - `digimon_gym/agents/training_worker.py` → `server/workers/training_worker.py`
  - `digimon_gym/agents/gauntlet_orchestrator.py` → `server/workers/gauntlet_orchestrator.py`
- Move `digimon_gym/data_paths.py` → top-level `data_paths.py` (still at repo root for now; phase 6 relocates it under `code/`).
- Move `digimon_gym/engine/onnx_policy.py` → `digimon_gym/inference/onnx_policy.py` (it survived phase 4 because it's not engine-coupled, just an ONNX loader).
- Update **all** import sites (server code, RL code, tools, kept tests, `tests/api/`, `tests/rl/`, `tests/classifier/`).
- Update `uvicorn` entrypoint everywhere: `digimon_gym.api:app` → `server.api:app`.
- Split requirements:
  - `requirements.txt` (server) — full dep set.
  - `requirements-training.txt` (RL CLI) — drops FastAPI, SQLAlchemy, asyncpg, etc., now that the boundary is real.
- Update `.mcp.json`, `.claude/settings.json` paths if any reference the old module path.
- Update training entrypoints to write to `<repo_root>/models/<run_id>/`. Add `models/` to `.gitignore` (if not already).

### Phase 6 — Repo reorg: hoist source into `code/`

This phase is a single mechanical sweep. The goal is to land it as one PR so the rename is atomic in git history (use `git mv` to preserve blame).

**Moves:**

- `server/` → `code/server/`
- `digimon_gym/` → `code/digimon_gym/`
- `digimon-engine/` → `code/digimon-engine/`
- `digimon-engine-py/` → `code/digimon-engine-py/`
- `src-tauri/` → `code/src-tauri/`
- `frontend/` → `code/frontend/`
- `engine_py_legacy/` → `code/engine_py_legacy/`
- `tools/` → `code/tools/`
- `tests/` → `code/tests/`
- `data_paths.py` → `code/data_paths.py`

**Stays at root:** `.github/`, `Dockerfile*`, `.mcp.json`, `.claude/`, `CLAUDE.md`, `AGENTS.md`, `README.md`, `docs/`, `qa/`, `data/`, `migrations/`, `models/`, `DCGO/`, `pyproject.toml`, `requirements*.txt`, `Cargo.toml`, lint/format configs.

**Configuration updates:**

- **Rust workspace.** Root `Cargo.toml` becomes a workspace:
  ```toml
  [workspace]
  resolver = "2"
  members = [
      "code/digimon-engine",
      "code/digimon-engine-py",
      "code/src-tauri",
  ]
  ```
  Member crates' internal `path = "../digimon-engine"` deps continue to work because they're sibling moves.
- **Python packaging.** Root `pyproject.toml` updates `tool.setuptools.packages.find` (or equivalent) to `where = ["code"]` and lists `server`, `digimon_gym` as included packages. `pip install -e .` still runs from root.
- **pytest.** Root `pyproject.toml` `[tool.pytest.ini_options]` updates `testpaths = ["code/tests"]` and `norecursedirs` adds `code/engine_py_legacy/tests`. `conftest.py` files inside `code/tests/` are unchanged.
- **maturin.** `code/digimon-engine-py/pyproject.toml` is unchanged internally; developers run `maturin develop` from `code/digimon-engine-py/` (one extra `cd`).
- **Frontend.** `code/frontend/package.json` is unchanged internally. `npm install`, `npm run dev`, `npm run build` run from `code/frontend/`.
- **Tauri.** `code/src-tauri/tauri.conf.json` `frontendDist` and `beforeBuildCommand` paths re-point to the new `code/frontend/` location. Tauri commands run from `code/src-tauri/`.

**CI updates (`.github/workflows/`):**

- Every `pytest` invocation gains the `code/tests` path or relies on `pyproject.toml` `testpaths`.
- Every `cargo test --manifest-path X` gets the `code/` prefix, or switches to workspace-level `cargo test` from root.
- Every `npm` step `cd`s into `code/frontend`.
- Every `python -m uvicorn server.api:app` runs from root (Python finds `server` via `pyproject.toml` `code/` path).
- Cache keys bumped to invalidate stale path-keyed caches.

**Skill / agent prompts (`.claude/`):**

Audit every skill and plugin prompt for hardcoded paths: `digimon_gym/`, `digimon-engine/`, `digimon-engine-py/`, `frontend/`, `src-tauri/`, `tools/`, `tests/`. Update each to the `code/` path.

### Phase 7 — Docs sweep

- **`CLAUDE.md`** — Project Layout tree (full rewrite), Service Boundaries, Commands section (every command gets the new working directory or path), Working Rules:
  - Renumber/rewrite rules naming `digimon_gym.engine`, `digimon_gym.routers`, `digimon_gym.db`, `digimon_gym.ai`.
  - Add a rule that `code/engine_py_legacy/` must not be imported by production code.
  - Add a rule that trained model artifacts go to `<repo_root>/models/`.
- **`AGENTS.md`** — RL wrapper chain references: `training_worker` and `gauntlet_orchestrator` move out to `server/workers/`; remaining `digimon_gym` module paths are unchanged in dotted form but live under `code/` on disk.
- **`docs/ARCHITECTURE.md`, `docs/INDEX.md`, `docs/TRAINING_RUNBOOK.md`, `docs/TOOLS.md`, `docs/RUST_PYTHON_PARITY.md`** — path and command updates.
- **`docs/RUST_ENGINE_API.md`** — TDD walkthrough paths update (`digimon-engine/tests/` → `code/digimon-engine/tests/`).

## Validation

Each phase ends with the following green:

- `python -m pytest` (default collection — excludes `engine_py_legacy/tests` from phase 4 onward; runs from `code/tests` from phase 6 onward).
- `cargo test` from repo root (workspace-level from phase 6 onward).
- `maturin develop` succeeds (run from `code/digimon-engine-py/` from phase 6).
- `python -m uvicorn server.api:app` (after phase 5) starts cleanly; `curl /health` succeeds; a smoke game step via `/games` succeeds.
- Frontend `npm run build` green from `code/frontend/` (after phase 6).
- Tauri `cargo tauri build` succeeds from `code/src-tauri/` (after phase 6).

## Risks and Mitigations

- **PyO3 binding gaps discovered late.** Mitigation: phase 2 explicitly inventories and tests every replacement export before any caller flips. The parity doc owns escape-hatch tracking.
- **Hidden test imports referencing the old paths.** Mitigation: phase 5 ends with a repo-wide grep for `digimon_gym\.(api|routers|db|ai|storage|classifier|digilab_client|env|data_paths)` returning zero hits anywhere (these symbols don't exist in `engine_py_legacy/`, which only houses `digimon_gym.engine.*`).
- **Skill prompts referencing old paths break sub-agent runs.** Mitigation: phases 6 and 7 sweeps are mandatory. Spot-check by running `/pm sync` and one batch-fix-cards iteration after each sweep.
- **CI caching of old module paths.** Mitigation: bump cache keys in CI workflow on phase 5 and again on phase 6.
- **Git blame loss on phase 6.** Mitigation: use `git mv` (not delete + add). Reviewers need `git log --follow` or the GitHub "show history before this rename" UI.
- **Rust workspace compile-time regression.** Mitigation: phase 6 verification includes `cargo build --workspace` and `cargo test --workspace` from root before merge.
- **Tauri config drift.** Mitigation: phase 6 includes a manual `cargo tauri dev` smoke before merge — `tauri.conf.json` path updates are the easiest thing to get wrong.

## Out-of-Scope Tracking

These are not part of this refactor but are mentioned for completeness — they intersect the touched areas:

- **`docs/RUST_PYTHON_PARITY.md`** continues to track Rust/Python engine divergences. New entries may appear during phase 2 if binding wraps surface gaps.
- **`/implement-set` skill** — phase 1 may leave it empty; whether to delete it is a separate decision.
- **Training requirements split** — phase 5 finally makes `requirements-training.txt` minimal. Cleaning up transitive deps that were only there for FastAPI is a follow-up if needed.
- **`models/` directory layout convention** — this spec establishes it as the output root; defining the per-run subdirectory schema (metadata files, checkpoint naming, ONNX export naming) is a follow-up if the current ad-hoc layout proves insufficient.
