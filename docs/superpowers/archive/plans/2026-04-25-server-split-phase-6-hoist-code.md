# Phase 6 — Hoist Source Under `code/` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Relocate every source tree under a new top-level `code/` directory so the repo root holds only docs, infra, agent config, and runtime data — leaving everything code-side under one prefix.

**Architecture:** Mechanical sweep over 9 source trees + 1 file: `server/`, `digimon_gym/`, `digimon-engine/`, `digimon-engine-py/`, `digimon-dsl/`, `src-tauri/`, `frontend/`, `engine_py_legacy/`, `tools/`, `tests/`, plus `data_paths.py`. Sibling moves preserve all internal relative paths (`path = "../digimon-engine"` Cargo deps, `cd ../frontend` Tauri commands, Python `from server.x import y` dotted imports). The bulk of the work is updating top-level config files (root `Cargo.toml`, `pyproject.toml`, Dockerfiles, CI workflows, `.vscode/launch.json`, `.claude/skills/*`) to point at the new `code/` prefix. `digimon-dsl/` is added to the move list — the design spec omitted it but it is a Cargo workspace member and must travel with the other Rust crates so `digimon-engine`'s `path = "../digimon-dsl"` dep stays valid.

**Tech Stack:** `git mv` (history-preserving), Cargo workspace, setuptools `packages.find` (`where = ["code"]`), pytest `testpaths` + `pythonpath`, maturin, Tauri v2, Vite, Docker multi-stage builds, GitHub Actions.

---

## File Structure

### Files moved (this phase)

| Source (root) | Destination |
|---|---|
| `server/` | `code/server/` |
| `digimon_gym/` | `code/digimon_gym/` |
| `digimon-engine/` | `code/digimon-engine/` |
| `digimon-engine-py/` | `code/digimon-engine-py/` |
| `digimon-dsl/` | `code/digimon-dsl/` |
| `src-tauri/` | `code/src-tauri/` |
| `frontend/` | `code/frontend/` |
| `engine_py_legacy/` | `code/engine_py_legacy/` |
| `tools/` | `code/tools/` |
| `tests/` | `code/tests/` |
| `data_paths.py` | `code/data_paths.py` |

### Files modified (this phase)

| File | What changes |
|---|---|
| `Cargo.toml` (root) | Workspace members rewritten with `code/` prefix |
| `pyproject.toml` (root) | `[tool.setuptools.packages.find] where = ["code"]`, packages list, pytest `testpaths` + `pythonpath` + `addopts` ignore paths, `norecursedirs` |
| `Dockerfile` | All `COPY` source paths gain `code/` prefix; runtime layout stays flat (`/app/server/`, `/app/digimon_gym/`) |
| `Dockerfile.training` | `COPY` source paths gain `code/` prefix; `DIGIMON_CARDS_JSON` env var dropped (data lives at runtime mount, not inside image post-Phase 4) |
| `.github/workflows/deploy-api.yml` | `cargo test --manifest-path code/src-tauri/Cargo.toml`, `working-directory: code/frontend`, cache key bumped |
| `.github/workflows/desktop-release.yml` | `working-directory: code/frontend`, `working-directory: code/src-tauri`, `bundle_glob: 'code/src-tauri/target/release/...'`, cache `workspaces: './code/src-tauri -> target'` |
| `.github/workflows/frozen-integrity.yml` | `python code/tools/check_frozen_integrity.py` |
| `.vscode/launch.json` | Frontend `cwd: ${workspaceFolder}/code/frontend` |
| `alembic/env.py` | No change — `prepend_sys_path = .` in `alembic.ini` plus the `pythonpath = ["code"]` in pyproject pytest section is for tests; alembic's CLI run gets `PYTHONPATH=code` from the Dockerfile env (set in Task 7). Note: this is the only test-vs-runtime split. |
| `alembic.ini` | `prepend_sys_path = code` (replaces `.`) |
| `scripts/train_remote.sh` | `python code/tools/export_onnx.py`, `python code/tools/publish_model.py`, drop the broken `DIGIMON_CARDS_JSON=/app/digimon_gym/engine/data/cards.json` env (it points at a path that doesn't exist post-Phase 4) |
| `.gitignore` | `code/frontend/dist/`, `code/frontend/node_modules/`, `code/frontend/tsconfig.tsbuildinfo`, `code/src-tauri/target/`, `code/src-tauri/binaries/`, `code/src-tauri/resources/models/`, `code/src-tauri/resources/onnxruntime/`, `code/tools/onnxruntime-vendor/**/*.dll` (etc.), `code/tests/api/fixtures/*.onnx` |
| `.claude/skills/*/SKILL.md` (7 files) | Hardcoded path refs gain `code/` prefix |

### Files NOT moved (stay at root)

`.github/`, `.claude/`, `.mcp.json`, `.gitignore`, `.gitattributes`, `.env*`, `CLAUDE.md`, `AGENTS.md`, `README.md`, `GEMINI.md`, `PLAN.md`, `Caddyfile`, `Dockerfile*`, `pyproject.toml`, `requirements*.txt`, `Cargo.toml`, `Cargo.lock`, `alembic.ini`, `alembic/`, `docker-compose*.yml`, `docs/`, `qa/`, `data/`, `DCGO/`, `ops/`, `scripts/`, `training_jobs/`, `target/` (build cache), `models/`, `runs/`.

`alembic/` stays at root because: (a) the spec says `migrations/` lives at root, and (b) `alembic.ini`'s `script_location = alembic` is conventionally repo-relative for the alembic CLI. We update its `prepend_sys_path` so `from server.db.models import Base` still resolves.

### Why each section is split into its own task

The spec says ship as one PR but doesn't mandate one commit. Splitting the move into per-tree tasks lets each `git mv` commit have a pure rename diff (reviewer sees no content changes), and config-update commits stay minimal. The repo is **deliberately broken between commits within this PR**; verification is at the end (Task 12).

---

## Task 1: Pre-flight — baseline + create `code/`

**Files:**
- Modify: none (verification only)
- Create: `code/` (empty directory marker; git tracks via the next task's first move)

- [ ] **Step 1: Confirm clean working tree on the Phase 6 branch**

```bash
git status
```
Expected: `working tree clean` on a branch off the latest Phase 5 main (post-merge).

- [ ] **Step 2: Run the full default pytest suite as the green baseline**

```bash
python -m pytest -q
```
Expected: All tests pass (the same suite that passed at Phase 5 close — ~300 tests). Record the pass count: this is the number Task 12 must match.

- [ ] **Step 3: Run the Rust workspace test suite as the green baseline**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --quiet
cargo test --manifest-path src-tauri/Cargo.toml --quiet
```
Expected: All Rust tests pass.

- [ ] **Step 4: Boot the FastAPI app to confirm import surface is healthy**

```bash
python -c "from server.api import app; print(len(app.routes), 'routes')"
```
Expected: prints `166 routes` (or whatever the post-Phase 5 count is).

- [ ] **Step 5: Build the frontend to confirm Vite + TS state**

```bash
cd frontend && npm run build && cd ..
```
Expected: Vite build succeeds, `frontend/dist/` populated.

No commit. This is a verification gate.

---

## Task 2: Move Rust crates into `code/` + update workspace

**Files:**
- Move: `digimon-engine/` → `code/digimon-engine/`
- Move: `digimon-engine-py/` → `code/digimon-engine-py/`
- Move: `digimon-dsl/` → `code/digimon-dsl/`
- Move: `src-tauri/` → `code/src-tauri/`
- Modify: `Cargo.toml` (root)

- [ ] **Step 1: Create the `code/` directory**

```bash
mkdir -p code
```

- [ ] **Step 2: `git mv` each Rust crate into `code/`**

```bash
git mv digimon-engine code/digimon-engine
git mv digimon-engine-py code/digimon-engine-py
git mv digimon-dsl code/digimon-dsl
git mv src-tauri code/src-tauri
```

Sibling moves: every internal `path = "../digimon-engine"` and `path = "../digimon-dsl"` Cargo dep continues to resolve because all four crates moved together into `code/`. No internal Cargo.toml changes needed.

- [ ] **Step 3: Rewrite root `Cargo.toml` workspace members**

Replace the contents of `Cargo.toml` with:

```toml
[workspace]
members = [
    "code/digimon-dsl",
    "code/digimon-engine",
    "code/digimon-engine-py",
    "code/src-tauri",
    "code/tools/dsl-schema-export",
    "code/tools/dsl-lint",
]
resolver = "2"
```

(`code/tools/dsl-schema-export` and `code/tools/dsl-lint` are anticipated paths — `tools/` doesn't move until Task 4. The workspace will be temporarily broken between this commit and Task 4. That's OK — verification is end-of-phase.)

- [ ] **Step 4: Verify the rewritten Cargo.toml parses**

```bash
cargo metadata --manifest-path Cargo.toml --no-deps --format-version 1 > /dev/null
```
Expected: exits non-zero with a "manifest path not found" error pointing at `code/tools/dsl-schema-export/Cargo.toml`. **This failure is expected and resolves in Task 4.** Note the error to confirm it's only the dsl-tools paths that are unresolved, nothing else.

- [ ] **Step 5: Confirm the engine crate alone still builds**

```bash
cargo build --manifest-path code/digimon-engine/Cargo.toml
```
Expected: build succeeds.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
phase 6 task 2: move Rust crates into code/ + update workspace

git mv digimon-engine, digimon-engine-py, digimon-dsl, src-tauri into code/.
Sibling moves preserve all internal `path = "../X"` Cargo deps. Root
Cargo.toml workspace members rewritten with code/ prefix; tools/dsl-*
member paths anticipate Task 4's tools/ move.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Move `frontend/` into `code/`

**Files:**
- Move: `frontend/` → `code/frontend/`

- [ ] **Step 1: `git mv` the frontend tree**

```bash
git mv frontend code/frontend
```

- [ ] **Step 2: Confirm Tauri's relative path to frontend is preserved**

```bash
grep -E '\.\./frontend' code/src-tauri/tauri.conf.json
```
Expected: matches `"../frontend/dist"` and `"cd ../frontend && ..."`. Both are still correct because `code/src-tauri/` and `code/frontend/` are siblings.

- [ ] **Step 3: Verify the frontend builds from its new location**

```bash
cd code/frontend && npm run build && cd ../..
```
Expected: Vite build succeeds, `code/frontend/dist/` populated.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
phase 6 task 3: move frontend/ into code/

git mv frontend code/frontend. Tauri's `../frontend/dist` and
`cd ../frontend` paths in tauri.conf.json continue to resolve because
src-tauri and frontend remain siblings under code/.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Move `tools/` into `code/`

**Files:**
- Move: `tools/` → `code/tools/`

- [ ] **Step 1: `git mv` the tools tree**

```bash
git mv tools code/tools
```

This brings `tools/dsl-schema-export/` and `tools/dsl-lint/` along, so the workspace member paths set in Task 2 (`code/tools/dsl-schema-export`, `code/tools/dsl-lint`) now resolve. `tools/dsl-lint/Cargo.toml`'s `digimon-engine = { path = "../../digimon-engine" }` dep remains correct: `code/tools/dsl-lint/` → `../../digimon-engine` → `code/digimon-engine/`. Same for `tools/dsl-schema-export/`'s `digimon-dsl = { path = "../../digimon-dsl" }`.

- [ ] **Step 2: Verify the workspace metadata now resolves**

```bash
cargo metadata --no-deps --format-version 1 > /dev/null
```
Expected: succeeds (exit 0). The error from Task 2 step 4 is now resolved.

- [ ] **Step 3: Build the workspace to confirm cross-crate linkage**

```bash
cargo build --workspace --exclude digimon-tcg
```

(Excluding `digimon-tcg` (Tauri) to avoid pulling in webkit/gtk system deps for a Linux compile check.)

Expected: build succeeds.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
phase 6 task 4: move tools/ into code/

git mv tools code/tools. Brings tools/dsl-schema-export and tools/dsl-lint
with it, resolving the workspace member paths set in Task 2. Internal
`path = "../../digimon-{engine,dsl}"` deps continue to resolve.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Move Python source trees + `data_paths.py` into `code/` + pyproject packages

**Files:**
- Move: `server/` → `code/server/`
- Move: `digimon_gym/` → `code/digimon_gym/`
- Move: `engine_py_legacy/` → `code/engine_py_legacy/`
- Move: `data_paths.py` → `code/data_paths.py`
- Modify: `pyproject.toml`

- [ ] **Step 1: `git mv` the Python trees**

```bash
git mv server code/server
git mv digimon_gym code/digimon_gym
git mv engine_py_legacy code/engine_py_legacy
git mv data_paths.py code/data_paths.py
```

`code/data_paths.py` line 36 currently reads `REPO_ROOT: Path = Path(__file__).resolve().parent`. After the hoist, that resolves to `code/` rather than the repo root. We **do not** change it in this commit — it's verified and corrected in step 3.

- [ ] **Step 2: Update `code/data_paths.py` to walk up one more level**

The repo root must continue to resolve to the repo root, not to `code/`. After the hoist, `data_paths.py` lives at `<repo_root>/code/data_paths.py`, so:

Edit `code/data_paths.py:36`:

```python
# was: REPO_ROOT: Path = Path(__file__).resolve().parent
REPO_ROOT: Path = Path(__file__).resolve().parent.parent
```

(This is the inverse of the Phase 5 fix — Phase 5 moved it from `digimon_gym/data_paths.py` (2 deep) to root (1 deep) and changed `.parent.parent` to `.parent`. Phase 6 moves it from root (1 deep) to `code/` (2 deep) and changes `.parent` back to `.parent.parent`.)

- [ ] **Step 3: Confirm REPO_ROOT resolves correctly**

```bash
python -c "import sys; sys.path.insert(0, 'code'); from data_paths import REPO_ROOT, CARDS_JSON; print('REPO_ROOT:', REPO_ROOT); print('CARDS_JSON exists:', CARDS_JSON.exists())"
```
Expected: `REPO_ROOT` is the absolute path of the repo root (the worktree directory), `CARDS_JSON exists: True`.

- [ ] **Step 4: Update root `pyproject.toml` to declare the `code/` source layout**

Replace the contents of `pyproject.toml` with:

```toml
[project]
name = "digimon-gym"
version = "0.1.0"
description = "Reinforcement learning environment for Digimon Trading Card Game"
requires-python = ">=3.11"
dependencies = [
    "numpy>=1.24",
    "gymnasium>=0.29",
    "torch>=2.0",
    "fastapi>=0.100",
    "httpx>=0.24",
    "uvicorn>=0.20",
    "openai>=1.40",
    "python-dotenv>=1.0",
    "pypdf>=4.2",
]

[project.optional-dependencies]
train = [
    "stable-baselines3>=2.0",
    "sb3-contrib>=2.0",
]
dev = [
    "pytest>=7.0",
]

[tool.setuptools.packages.find]
where = ["code"]
include = ["server*", "digimon_gym*"]
exclude = ["engine_py_legacy*", "tests*", "tools*"]

[tool.setuptools]
py-modules = ["data_paths"]

[tool.setuptools.package-dir]
"" = "code"

[tool.pytest.ini_options]
testpaths = ["code/tests"]
pythonpath = ["code"]
addopts = "--ignore=code/tests/test_rl_gym.py --ignore=code/tests/ai_pipeline --ignore=code/engine_py_legacy -v"
norecursedirs = ["code/engine_py_legacy"]
asyncio_mode = "auto"
markers = [
    "ai_pipeline: AI pipeline tests (server.ai.* and DB deps)",
    "behavioral: Behavioral tests using DebugRunner with real card effects",
    "scenario: YAML scenario-based tests",
    "slow: Tests that take >10s (greedy baselines)",
]

[build-system]
requires = ["setuptools>=68.0"]
build-backend = "setuptools.backends._legacy:_Backend"
```

Notes:
- `pythonpath = ["code"]` makes pytest add `code/` to `sys.path`, so `import server`, `import digimon_gym`, `import data_paths` all resolve when running tests from repo root.
- `[tool.setuptools.package-dir] "" = "code"` + `[tool.setuptools.packages.find] where = ["code"]` together tell setuptools that the importable Python packages live under `code/`. This is needed for the wheel built by Dockerfile to package `server/` and `digimon_gym/` correctly.
- `py-modules = ["data_paths"]` packages the standalone `code/data_paths.py` module into the wheel.
- `code/tests`, `code/tools`, `code/engine_py_legacy` are not packaged into the distributable wheel.
- `addopts` and `norecursedirs` paths gain the `code/` prefix.

- [ ] **Step 5: Editable-install the package so the Python entry-points work**

```bash
pip install -e . --quiet
```
Expected: succeeds. After this, `python -c "import server, digimon_gym, data_paths"` should work from any cwd.

- [ ] **Step 6: Smoke-import each top-level package**

```bash
python -c "import server.api; import digimon_gym; import data_paths; print('OK')"
```
Expected: prints `OK`. This is the live confirmation that the moves + pyproject changes hang together.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
phase 6 task 5: hoist server/, digimon_gym/, engine_py_legacy/, data_paths.py into code/

git mv all four trees into code/. Update root pyproject.toml:
  - [tool.setuptools.packages.find] where = ["code"]
  - [tool.setuptools.package-dir] "" = "code"
  - py-modules = ["data_paths"]
  - pytest pythonpath = ["code"], testpaths/addopts/norecursedirs gain
    code/ prefix
Update code/data_paths.py REPO_ROOT to walk up one more level (2 deep
again, matches its pre-Phase-5 depth).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Move `tests/` into `code/`

**Files:**
- Move: `tests/` → `code/tests/`

- [ ] **Step 1: `git mv` the test tree**

```bash
git mv tests code/tests
```

The tests don't change. `code/tests/api/`, `code/tests/rl/`, `code/tests/classifier/`, `code/tests/storage/`, `code/tests/ai_pipeline/` all keep importing `server.x`, `digimon_gym.x`, `data_paths` — those imports already resolve from Task 5's pyproject + editable install.

- [ ] **Step 2: Run the default pytest collection from repo root**

```bash
python -m pytest --collect-only -q
```
Expected: pytest collects tests from `code/tests/` (per `testpaths = ["code/tests"]`). No collection errors. Test count matches Task 1's baseline.

- [ ] **Step 3: Run the full default pytest suite**

```bash
python -m pytest -q
```
Expected: same pass count as Task 1's baseline.

- [ ] **Step 4: Run the legacy engine_py_legacy tests as a separate gate**

```bash
python -m pytest code/engine_py_legacy/tests -q
```
Expected: same pass count as before Phase 6 (~490 tests).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
phase 6 task 6: move tests/ into code/

git mv tests code/tests. pytest discovers via the testpaths/pythonpath
set in Task 5's pyproject.toml. Default suite + engine_py_legacy suite
both green from new locations.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Update Dockerfile + Dockerfile.training

**Files:**
- Modify: `Dockerfile`
- Modify: `Dockerfile.training`

The strategy is to keep the runtime container layout flat (`/app/server/`, `/app/digimon_gym/`, `/app/data_paths.py`, `/app/alembic/`, `/app/data/`) and only update the **source-side** `COPY` paths to gain the `code/` prefix. The CMD line is unchanged because `uvicorn server.api:app` works identically once the files are at `/app/`.

- [ ] **Step 1: Update `Dockerfile` source COPY paths**

Replace the contents of `Dockerfile` with:

```dockerfile
# syntax=docker/dockerfile:1.6

# ── Stage 1: Rust builder (produces the PyO3 wheel) ───────────────────────
FROM rust:1.78-slim AS rust-builder
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3.11 python3.11-dev python3-pip pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN pip3 install --no-cache-dir --break-system-packages maturin==1.5.1
COPY Cargo.toml Cargo.lock ./
COPY code/digimon-dsl/ code/digimon-dsl/
COPY code/digimon-engine/ code/digimon-engine/
COPY code/digimon-engine-py/ code/digimon-engine-py/
# Drop src-tauri + tools/dsl-* from the workspace so Tauri/dsl-tools deps
# don't block the server build.
RUN sed -i -E '/"code\/src-tauri"|"code\/tools\/dsl-/d' Cargo.toml
RUN cd code/digimon-engine-py && maturin build --release --out /wheels

# ── Stage 2: Python wheel builder ─────────────────────────────────────────
FROM python:3.11-slim AS py-builder
WORKDIR /build
COPY requirements-server.txt .
RUN pip wheel --no-cache-dir -r requirements-server.txt -w /wheels
COPY --from=rust-builder /wheels/*.whl /wheels/

# ── Stage 3: Runtime ──────────────────────────────────────────────────────
FROM python:3.11-slim AS runtime
ENV PYTHONDONTWRITEBYTECODE=1 PYTHONUNBUFFERED=1
RUN useradd -u 1001 -m app
WORKDIR /app
COPY --from=py-builder /wheels /wheels
RUN pip install --no-cache-dir /wheels/*.whl && rm -rf /wheels
COPY code/server/ server/
COPY code/digimon_gym/ digimon_gym/
COPY code/data_paths.py .
COPY alembic/ alembic/
COPY alembic.ini .
USER app
EXPOSE 8000
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:8000/health').read()" || exit 1
CMD ["sh", "-c", "alembic upgrade head && uvicorn server.api:app --host 0.0.0.0 --port 8000"]
```

Key changes:
- `COPY code/digimon-engine/` (was `digimon-engine/`), and digimon-dsl + digimon-engine-py likewise.
- `RUN sed -i ...` regex removes `code/src-tauri` and `code/tools/dsl-*` workspace members (the tools/dsl crates need digimon-engine/digimon-dsl but produce build artifacts the API container doesn't need).
- `RUN cd code/digimon-engine-py && maturin build` — the cd path changes, the rest is identical.
- Runtime stage flattens: `COPY code/server/ server/` puts files at `/app/server/`, same for `digimon_gym/` and `data_paths.py`. The CMD `uvicorn server.api:app` is unchanged.

- [ ] **Step 2: Update `Dockerfile.training` source COPY paths**

Replace the contents of `Dockerfile.training` with:

```dockerfile
# Multi-stage training image for Digimon TCG RL
#
# Stage 1: Compile the digimon-engine-py PyO3 wheel (Rust + Python headers)
# Stage 2: Lean Python training runtime — copies the wheel, installs deps
#
# Build: docker build -f Dockerfile.training -t digimon-trainer .
# Run:   docker run --gpus all --rm \
#          -v $(pwd)/models:/app/models \
#          -v $(pwd)/data:/app/data \
#          -e DIGIMON_BACKEND=rust \
#          digimon-trainer training_jobs/my_job.json

# ── Stage 1: Rust build ───────────────────────────────────────────────────
FROM rust:1.82-slim AS rust-builder

WORKDIR /build

RUN apt-get update && apt-get install -y --no-install-recommends \
    python3.11-dev python3-pip libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN pip install --no-cache-dir maturin

# Copy workspace manifests and lock file first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Copy only the crates needed for the PyO3 wheel
COPY code/digimon-dsl/ code/digimon-dsl/
COPY code/digimon-engine/ code/digimon-engine/
COPY code/digimon-engine-py/ code/digimon-engine-py/

# Trim the workspace so unrelated members don't have to resolve.
RUN sed -i -E '/"code\/src-tauri"|"code\/tools\/dsl-/d' Cargo.toml

# Build the wheel — release mode for maximum speed at training time
RUN cd code/digimon-engine-py && maturin build --release --out /wheels


# ── Stage 2: Python training runtime ─────────────────────────────────────
FROM python:3.11-slim

WORKDIR /app

# Install the compiled PyO3 wheel
COPY --from=rust-builder /wheels/*.whl /wheels/
RUN pip install --no-cache-dir /wheels/*.whl && rm -rf /wheels

# Install Python training dependencies
COPY requirements-training.txt .
RUN pip install --no-cache-dir -r requirements-training.txt

# Copy training-side source (flatten code/ → /app/)
COPY code/digimon_gym/ digimon_gym/
COPY code/tools/ tools/
COPY code/data_paths.py .
COPY training_jobs/ training_jobs/

# Card data is mounted at runtime: -v $(pwd)/data:/app/data
# The data_paths module resolves cards.json relative to the repo root,
# which inside the container is /app once code/ is flattened. The
# default DIGIMON_DATA_DIR (DATA_DIR / "data") therefore points at
# /app/data and is satisfied by the volume mount.

# Use the Rust engine backend by default
ENV DIGIMON_BACKEND=rust

# Entrypoint: python tools/run_training_job.py <job_config>
ENTRYPOINT ["python", "tools/run_training_job.py"]
```

Key changes:
- `COPY code/digimon-engine/`, `code/digimon-dsl/`, `code/digimon-engine-py/`.
- New `RUN sed -i ...` to drop unrelated workspace members.
- Runtime stage `COPY code/digimon_gym/`, `code/tools/`, `code/data_paths.py`.
- **Removed**: `ENV DIGIMON_CARDS_JSON=/app/digimon_gym/engine/data/cards.json`. Phase 4 moved the engine to `engine_py_legacy/` so that path was already broken — Phase 6 just stops asserting it. `data_paths.py` defaults `DATA_DIR` to `<repo_root>/data` which inside the container is `/app/data`, satisfied by the documented `-v $(pwd)/data:/app/data` volume mount. The container is opt-in: callers who need cards.json must mount data/.

- [ ] **Step 3: Smoke-build the API Dockerfile (optional but recommended)**

```bash
docker build -f Dockerfile -t digimon-api-phase6-smoke .
```
Expected: succeeds. Skip this step if Docker is unavailable on the dev machine — CI's `Deploy API` workflow will exercise it after the push.

- [ ] **Step 4: Commit**

```bash
git add Dockerfile Dockerfile.training
git commit -m "$(cat <<'EOF'
phase 6 task 7: update Dockerfile + Dockerfile.training for code/ layout

All COPY source paths gain code/ prefix; runtime container layout stays
flat (/app/server/, /app/digimon_gym/, /app/tools/, /app/data_paths.py).
Dockerfile.training drops the broken DIGIMON_CARDS_JSON env (path was
invalidated by Phase 4); cards.json is now expected via a -v data:
volume mount. sed regex updated to strip code/src-tauri and
code/tools/dsl-* workspace members in builder stage.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Update CI workflows

**Files:**
- Modify: `.github/workflows/deploy-api.yml`
- Modify: `.github/workflows/desktop-release.yml`
- Modify: `.github/workflows/frozen-integrity.yml`

- [ ] **Step 1: Update `deploy-api.yml`**

Apply these targeted edits:

`fast-tests` job — change `python -m pytest tests -m "not slow" -x -q` to `python -m pytest -m "not slow" -x -q` (the `tests` arg becomes redundant since pyproject's `testpaths` handles it; explicit removal avoids a path mismatch).

`tauri-tests` job — change:
```yaml
      - uses: swatinem/rust-cache@v2
        with:
          workspaces: "./src-tauri -> target"
      - run: sudo apt-get update && sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
      - run: cargo test --manifest-path src-tauri/Cargo.toml
```
to:
```yaml
      - uses: swatinem/rust-cache@v2
        with:
          workspaces: "./code/src-tauri -> target"
          key: phase6
      - run: sudo apt-get update && sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
      - run: cargo test --manifest-path code/src-tauri/Cargo.toml
```

`e2e` job — change `working-directory: frontend` to `working-directory: code/frontend`. Change `path: frontend/playwright-report` to `path: code/frontend/playwright-report`.

The `build` (Docker) job is unchanged; the `context: .` build context already covers the new `code/` paths from Task 7's Dockerfile rewrite.

- [ ] **Step 2: Update `desktop-release.yml`**

Apply these targeted edits:

`build` matrix `bundle_glob`:
```yaml
          - runner: windows-latest
            target: windows-x86_64
            bundle_glob: 'code/src-tauri/target/release/bundle/nsis/*-setup.exe'
          - runner: ubuntu-latest
            target: linux-x86_64
            bundle_glob: 'code/src-tauri/target/release/bundle/appimage/*.AppImage'
```

Cache:
```yaml
      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: './code/src-tauri -> target'
          key: phase6
```

Node cache:
```yaml
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
          cache-dependency-path: code/frontend/package-lock.json
```

Each `working-directory: frontend` → `working-directory: code/frontend`.
Each `working-directory: src-tauri` → `working-directory: code/src-tauri`.

The `bundle_glob` paths in the publish job `case "$TARGET"` block use shell globbing relative to `artifacts/desktop-$TARGET` — the artifact upload preserves filenames so no change is needed in the publish job.

- [ ] **Step 3: Update `frozen-integrity.yml`**

Replace the contents of `.github/workflows/frozen-integrity.yml` with:

```yaml
name: Frozen Integrity

on:
  pull_request:
  push:
    branches: ["main"]

jobs:
  check-frozen-integrity:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Setup Python
        uses: actions/setup-python@v5
        with:
          python-version: "3.11"

      - name: Run frozen manifest integrity check
        run: python code/tools/check_frozen_integrity.py
```

Only line changed: `python tools/check_frozen_integrity.py` → `python code/tools/check_frozen_integrity.py`.

- [ ] **Step 4: Validate workflow YAML parses**

```bash
python -c "import yaml; [yaml.safe_load(open(f)) for f in ['.github/workflows/deploy-api.yml', '.github/workflows/desktop-release.yml', '.github/workflows/frozen-integrity.yml']]; print('OK')"
```
Expected: prints `OK`. (Catches indentation typos without needing actionlint.)

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/
git commit -m "$(cat <<'EOF'
phase 6 task 8: update CI workflows for code/ layout

deploy-api.yml: cargo test path → code/src-tauri/, frontend
working-directory → code/frontend, rust-cache key bumped.
desktop-release.yml: bundle_glob, working-directory, cache-dependency-path
all gain code/ prefix; rust-cache key bumped.
frozen-integrity.yml: tool path → code/tools/check_frozen_integrity.py.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Update editor + ops + alembic + git config

**Files:**
- Modify: `.vscode/launch.json`
- Modify: `scripts/train_remote.sh`
- Modify: `alembic.ini`
- Modify: `.gitignore`

- [ ] **Step 1: Update `.vscode/launch.json` frontend cwd**

Edit `.vscode/launch.json:24`:

```json
"cwd": "${workspaceFolder}/code/frontend",
```

(was `"${workspaceFolder}/frontend"`)

The backend Uvicorn config is unchanged: `cwd: ${workspaceFolder}` and `module: uvicorn` with `args: ["server.api:app", "--port", "8000"]` continues to work because `pip install -e .` from Task 5 makes `server` importable from any cwd.

- [ ] **Step 2: Update `scripts/train_remote.sh`**

Three changes:

Line 75 (scp of job config) is unchanged.
Line 95 — drop the broken `DIGIMON_CARDS_JSON` env:
```bash
        -v ${REMOTE_MODELS}:/app/models \
        -v ${REMOTE_DIR}/data:/app/data \
        -e DIGIMON_BACKEND=rust \
```
(replace `-e DIGIMON_CARDS_JSON=/app/digimon_gym/engine/data/cards.json` with a `-v ${REMOTE_DIR}/data:/app/data` mount that satisfies the new Dockerfile.training expectation.)

Line 122 — `python tools/export_onnx.py` → `python code/tools/export_onnx.py`.
Line 140 — `python tools/publish_model.py` → `python code/tools/publish_model.py`.

- [ ] **Step 3: Update `alembic.ini`**

Edit line 3:
```ini
prepend_sys_path = code
```
(was `prepend_sys_path = .`)

This makes `from server.db.models import Base` (in `alembic/env.py`) resolve when the alembic CLI is run from repo root.

- [ ] **Step 4: Verify alembic loads**

```bash
alembic check
```
Expected: succeeds (no schema diff or "no migrations" message; confirms `from server.db.models import Base` resolved).

If `alembic check` errors out due to lack of DATABASE_URL, fall back to:
```bash
python -c "import sys; sys.path.insert(0, 'code'); from server.db.models import Base; print('OK', len(Base.metadata.tables))"
```
Expected: prints `OK <N>` where N is the table count.

- [ ] **Step 5: Update `.gitignore` paths**

Apply these line-level edits to `.gitignore`:

```
# Replace these lines individually
frontend/dist/                 → code/frontend/dist/
frontend/node_modules/         → code/frontend/node_modules/
frontend/tsconfig.tsbuildinfo  → code/frontend/tsconfig.tsbuildinfo
src-tauri/target/              → code/src-tauri/target/
src-tauri/binaries/            → code/src-tauri/binaries/
src-tauri/resources/models/    → code/src-tauri/resources/models/
src-tauri/resources/onnxruntime/ → code/src-tauri/resources/onnxruntime/
tools/onnxruntime-vendor/**/*.dll → code/tools/onnxruntime-vendor/**/*.dll
tools/onnxruntime-vendor/**/*.so  → code/tools/onnxruntime-vendor/**/*.so
tools/onnxruntime-vendor/**/*.dylib → code/tools/onnxruntime-vendor/**/*.dylib
tests/api/fixtures/*.onnx      → code/tests/api/fixtures/*.onnx
```

Lines that are already path-agnostic (`__pycache__/`, `*.pyc`, `target/`, `**/*.rs.bk`, `models/`, `runs/`, `.env*`, etc.) are not changed.

- [ ] **Step 6: Commit**

```bash
git add .vscode/launch.json scripts/train_remote.sh alembic.ini .gitignore
git commit -m "$(cat <<'EOF'
phase 6 task 9: update editor + ops + alembic + .gitignore for code/ layout

.vscode/launch.json: frontend cwd → code/frontend.
scripts/train_remote.sh: tool paths gain code/ prefix; broken
DIGIMON_CARDS_JSON env replaced with -v data: volume mount.
alembic.ini: prepend_sys_path = code (was .) so `from server.db.models
import Base` resolves when alembic CLI runs from repo root.
.gitignore: tree-specific entries gain code/ prefix.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: Audit `.claude/skills/` for hardcoded paths

**Files:**
- Modify: `.claude/skills/assess-archetype-rust/SKILL.md`
- Modify: `.claude/skills/batch-fix-cards/SKILL.md`
- Modify: `.claude/skills/batch-implement-cards-rust/SKILL.md`
- Modify: `.claude/skills/fix-card/skill.md`
- Modify: `.claude/skills/gameplay-qa/SKILL.md`
- Modify: `.claude/skills/implement-archetype/SKILL.md`
- Modify: `.claude/skills/review-archetype/SKILL.md`

These 7 skill files contain hardcoded path references that must be updated. The files are referenced by sub-agent dispatchers and by the user via `/<skill-name>` slash commands.

- [ ] **Step 1: Re-grep to identify exact lines that need updating**

```bash
grep -nE 'digimon_gym/|digimon-engine[/-]|src-tauri/|frontend/|^cd tools/|tools/|^cd tests/|tests/|engine_py_legacy/' .claude/skills/*/SKILL.md .claude/skills/*/skill.md 2>/dev/null
```

Inspect each match. For each match, the rewrite is mechanical:
- `digimon_gym/` → `code/digimon_gym/` (when path-like, i.e., used in `cat`, `ls`, `cd`, file refs)
- `digimon-engine/` → `code/digimon-engine/`
- `digimon-engine-py/` → `code/digimon-engine-py/`
- `src-tauri/` → `code/src-tauri/`
- `frontend/` → `code/frontend/`
- `engine_py_legacy/` → `code/engine_py_legacy/`
- `tools/` → `code/tools/` (when shell command, e.g., `python tools/X.py` → `python code/tools/X.py`)
- `tests/` → `code/tests/` (when path, e.g., `pytest tests/Y` → `pytest code/tests/Y`)
- `cargo test --manifest-path digimon-engine/Cargo.toml` → `cargo test --manifest-path code/digimon-engine/Cargo.toml`

**Do NOT rewrite Python dotted module names** like `digimon_gym.engine.x` or `server.api`. Those are unaffected by the directory move because `pip install -e .` makes them importable under their existing dotted names.

**Do NOT rewrite C# / DCGO references** (`DCGO/Assets/Scripts/...`) — `DCGO/` stays at root.

**Do NOT rewrite docs paths** like `docs/X.md`, `qa/X/Y.md`, `data/cards.json` — `docs/`, `qa/`, `data/` stay at root.

- [ ] **Step 2: Apply edits to each skill file**

For each file in the grep output, edit the matched lines to apply the rewrites in step 1's table. The edits are mechanical line-by-line text substitutions; no logic changes.

- [ ] **Step 3: Re-grep to confirm zero stale path references**

```bash
grep -nE '(?<!code/)(?<!\.)(?<!\w)(digimon_gym/|digimon-engine[/-]|src-tauri/|frontend/|engine_py_legacy/)' .claude/skills/*/SKILL.md .claude/skills/*/skill.md 2>/dev/null
```

Expected: empty output. (Negative lookbehind `(?<!code/)` excludes already-rewritten paths. The `(?<!\.)` excludes Python module dotted refs like `digimon_gym.x`. The `(?<!\w)` ensures we're at a word boundary.)

If grep doesn't support PCRE lookbehinds (e.g., BSD grep on macOS), use ripgrep:
```bash
rg --pcre2 '(?<!code/)(?<!\.)(?<!\w)(digimon_gym/|digimon-engine[/-]|src-tauri/|frontend/|engine_py_legacy/)' .claude/skills/
```

- [ ] **Step 4: Commit**

```bash
git add .claude/skills/
git commit -m "$(cat <<'EOF'
phase 6 task 10: update .claude/skills/ path refs to code/ layout

7 skill files audited. Path-style refs (digimon_gym/, digimon-engine/,
src-tauri/, frontend/, tools/, tests/, engine_py_legacy/) gain code/
prefix; Python dotted module names (digimon_gym.x, server.x) and
unmoved roots (docs/, qa/, data/, DCGO/) unchanged.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 11: Audit broader repo for stale path references

**Files:** verification only — fixes inline as needed.

The previous tasks covered every config file we know about. This task is a defensive sweep for things we missed.

- [ ] **Step 1: Repo-wide grep for stale path references in non-doc files**

```bash
rg --pcre2 -tpy -ttoml -tsh -tjson -tyaml '(?<!code/)(?<!\.)(?<!\w)(digimon_gym/|digimon-engine/|digimon-engine-py/|digimon-dsl/|src-tauri/|frontend/|engine_py_legacy/|/data_paths\.py)' \
   --glob '!docs/**' --glob '!qa/**' --glob '!**/*.md'
```

Expected: empty output. Any hit is a path the previous tasks missed.

If hits appear in **plan documents** (`docs/superpowers/plans/`) or **specs** (`docs/superpowers/specs/`), ignore them — those are historical and Phase 7 sweeps docs separately.

If hits appear in any other tracked file, fix inline using `sed -i` or an Edit operation, then commit:
```bash
git add <files>
git commit -m "phase 6 task 11: fix missed path refs in <list of files>"
```

- [ ] **Step 2: Repo-wide grep for unmoved Cargo manifests**

```bash
find . -name 'Cargo.toml' -not -path './target/*' -not -path './.git/*' -not -path './code/*'
```

Expected output: `./Cargo.toml` (the workspace root) and nothing else. Any other hit is an un-moved Rust crate.

- [ ] **Step 3: Repo-wide grep for unmoved Python packages**

```bash
find . -maxdepth 2 -name '__init__.py' -not -path './code/*' -not -path './.git/*'
```

Expected output: empty. Any hit is an un-moved Python package.

- [ ] **Step 4: Verify `data_paths.py` is gone from the root**

```bash
ls data_paths.py 2>&1
```
Expected: `ls: cannot access 'data_paths.py': No such file or directory`.

- [ ] **Step 5: No commit if no fixes were needed**

If steps 1–4 produced any inline fixes, commit them under one message:
```bash
git commit -m "phase 6 task 11: defensive sweep — fix missed path refs"
```

If no fixes were needed, report `clean` and proceed.

---

## Task 12: Final verification — green gate on every channel

**Files:** verification only.

This task is the contract that closes Phase 6. It must pass before opening the PR.

- [ ] **Step 1: Default pytest suite from repo root**

```bash
python -m pytest -q
```
Expected: same pass count as Task 1 baseline. **No skips, no errors.**

- [ ] **Step 2: Engine_py_legacy test suite**

```bash
python -m pytest code/engine_py_legacy/tests -q
```
Expected: same pass count as before Phase 6 (~490 tests).

- [ ] **Step 3: Cargo workspace tests**

```bash
cargo test --workspace --exclude digimon-tcg --quiet
```

(Excluding `digimon-tcg` (Tauri) on machines without webkit2gtk dev headers; CI's tauri-tests job will exercise it.)

Expected: all tests pass.

- [ ] **Step 4: Tauri crate tests (skip if webkit2gtk-4.1-dev not installed)**

```bash
cargo test --manifest-path code/src-tauri/Cargo.toml --quiet
```

Expected: succeeds, OR a clearly-system-deps-missing error (linker errors against `webkit2gtk`). System dep failures are not blocking on dev machines — CI gates this.

- [ ] **Step 5: maturin develop builds the PyO3 wheel**

```bash
cd code/digimon-engine-py && maturin develop --quiet && cd ../..
```
Expected: succeeds. After this, `python -c "import digimon_engine; print(digimon_engine.__name__)"` prints `digimon_engine`.

- [ ] **Step 6: FastAPI app boots cleanly**

```bash
python -c "from server.api import app; print(len(app.routes), 'routes')"
```
Expected: prints `166 routes` (or whatever Task 1 baseline showed).

- [ ] **Step 7: DigimonEnv smoke (Python backend)**

```bash
python -c "from digimon_gym.digimon_gym import DigimonEnv; env = DigimonEnv(); obs, info = env.reset(); print(obs.shape, info['action_mask'].shape)"
```
Expected: prints obs shape and mask shape (matches the Task 1 baseline output).

- [ ] **Step 8: DigimonEnv smoke (Rust backend)**

```bash
DIGIMON_BACKEND=rust python -c "from digimon_gym.digimon_gym import DigimonEnv; env = DigimonEnv(); obs, info = env.reset(); print(obs.shape, info['action_mask'].shape)"
```
Expected: same shapes as step 7.

- [ ] **Step 9: Frontend builds**

```bash
cd code/frontend && npm run build && cd ../..
```
Expected: Vite build succeeds, `code/frontend/dist/` populated.

- [ ] **Step 10: Tauri smoke build (optional, requires Rust + system deps)**

```bash
cd code/src-tauri && cargo build --quiet && cd ../..
```
Expected: succeeds. Skip if system deps missing — CI gates the bundled build.

- [ ] **Step 11: Push the branch**

```bash
git push -u origin <branch-name>
```

- [ ] **Step 12: Update PR #358**

Update PR title to `server split: spec + Phases 1-6 (transpiler delete, PyO3 bindings, caller cutover, engine_py_legacy, server extraction, code/ hoist)` and append a Phase 6 section to the PR body summarizing what shipped:
- 11 source trees + `data_paths.py` moved into `code/` via `git mv` (history preserved).
- Root `Cargo.toml` workspace members rewritten with `code/` prefix; `digimon-dsl` added to the move list (omitted by spec, but a Cargo workspace member).
- Root `pyproject.toml` updated with `[tool.setuptools.package-dir] "" = "code"`, `packages.find where = ["code"]`, `py-modules = ["data_paths"]`, pytest `pythonpath = ["code"]`, `testpaths = ["code/tests"]`.
- Dockerfile + Dockerfile.training: source COPYs gain `code/` prefix; runtime container layout stays flat. `Dockerfile.training` drops the broken `DIGIMON_CARDS_JSON` env (path was invalidated by Phase 4) — replaced with `-v data:/app/data` mount expectation.
- CI workflows updated; rust-cache keys bumped to `phase6`.
- `.vscode/launch.json`, `scripts/train_remote.sh`, `alembic.ini`, `.gitignore`, 7 `.claude/skills/*` files updated for `code/` paths.
- Phase 7 (docs sweep) still pending — `CLAUDE.md`, `AGENTS.md`, `docs/*.md` not yet rewritten.

---

## Self-Review Notes

**Spec coverage check:**
- Spec move list (10 items): server, digimon_gym, digimon-engine, digimon-engine-py, src-tauri, frontend, engine_py_legacy, tools, tests, data_paths.py → all in Tasks 2–6.
- Spec adds `digimon-dsl/` (this plan adds it because it's a Cargo workspace member; spec omitted) → Task 2.
- Spec config updates: Cargo workspace (Task 2), Python packaging (Task 5), pytest (Task 5), maturin (Task 12 verifies — no internal change), Tauri (Task 3 verifies — no internal change), frontend (Task 3 verifies — no internal change) → covered.
- Spec CI updates: pytest paths (Task 5/6 via pyproject), cargo manifest paths (Task 8), npm cd (Task 8), uvicorn (no change — module name unchanged), cache keys bumped (Task 8) → covered.
- Spec `.claude/` audit → Task 10.
- Spec stays-at-root list confirmed against current repo: `.github/`, `Dockerfile*`, `.mcp.json`, `.claude/`, `CLAUDE.md`, `AGENTS.md`, `README.md`, `docs/`, `qa/`, `data/`, `DCGO/`, `pyproject.toml`, `requirements*.txt`, `Cargo.toml`, lint configs → all confirmed not moved.
- Spec validation gates (pytest, cargo test, maturin develop, server boot, npm build, tauri build) → all in Task 12.
- Phase 7 (docs sweep) is explicitly out of scope — flagged in Task 12 step 12 as still-pending.

**Gaps from spec:**
- Spec mentions `migrations/` as future home; current repo uses `alembic/` at root. This plan keeps `alembic/` at root (consistent with spec's "migrations stay at root") and updates `alembic.ini`'s `prepend_sys_path` accordingly (Task 9).
- `Dockerfile.training` had a pre-existing broken env var `DIGIMON_CARDS_JSON=/app/digimon_gym/engine/data/cards.json` (broken since Phase 4 deleted that path). This plan opportunistically removes it in Task 7 — flagged in the commit message.

**Type/path consistency check:**
- All Cargo workspace member paths use `code/` prefix consistently across Task 2 (root Cargo.toml), Task 7 (Dockerfile sed regex), Task 8 (CI workflows).
- `data_paths.py` REPO_ROOT depth verified: pre-Phase 5 (in digimon_gym/) → 2 deep → `.parent.parent`; Phase 5 (root) → 1 deep → `.parent`; Phase 6 (code/) → 2 deep → `.parent.parent`. Task 5 step 2 reverses Phase 5's adjustment.
- `pythonpath = ["code"]` consistent with `where = ["code"]` consistent with `package-dir "" = "code"`.
- Skill audit grep negative-lookbehind pattern `(?<!code/)` consistent across Tasks 10 and 11.

No placeholder text. No "TBD" or "TODO". Every code block contains the actual content needed.
