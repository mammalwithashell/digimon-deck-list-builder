# Phase 5: Extract `server/` from `digimon_gym/` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move every FastAPI / DB / AI-pipeline / storage / classifier / digilab / DB-bound worker module out of `digimon_gym/` and into a new top-level `server/` package, leaving `digimon_gym/` as the pure RL package (Gym env + agents + inference). Move `data_paths.py` to repo root and `onnx_policy.py` to `digimon_gym/inference/`. Cut over every import site, the uvicorn entrypoint, and `alembic/env.py`. After this phase, `server` consumes `digimon_gym` and `digimon_engine`; `digimon_gym` consumes only `digimon_engine` (plus the still-resident `engine_py_legacy.engine.*` for the parity-doc residue).

**Architecture:** Mechanical multi-target `git mv` followed by a single regex-driven import rewrite that reroutes nine module prefixes:
- `digimon_gym.api` → `server.api`
- `digimon_gym.env` → `server.env`
- `digimon_gym.routers.X` → `server.routers.X`
- `digimon_gym.db.X` → `server.db.X`
- `digimon_gym.ai.X` → `server.ai.X`
- `digimon_gym.storage.X` → `server.storage.X`
- `digimon_gym.classifier.X` → `server.classifier.X`
- `digimon_gym.digilab_client` → `server.digilab_client`
- `digimon_gym.agents.training_worker` → `server.workers.training_worker`
- `digimon_gym.agents.gauntlet_orchestrator` → `server.workers.gauntlet_orchestrator`
- `digimon_gym.data_paths` → `data_paths` (repo-root module)
- `digimon_gym.engine.onnx_policy` → `digimon_gym.inference.onnx_policy`

Two preliminary moves (`data_paths.py` and `onnx_policy.py`) ship as standalone tasks because their callers span all sides of the refactor and they're stable building blocks. The big server move ships next as a single atomic file-move + import-rewrite pair so the repo only spends one commit broken.

**Tech Stack:** Python 3.11, pytest, FastAPI/uvicorn, alembic, git mv.

**Spec:** `docs/superpowers/specs/2026-04-25-server-digimon-gym-split-design.md` (Phase 5).

---

## Pre-flight context

Tasks are sequential — each depends on the prior task's commit. Don't parallelize.

The two preliminary moves (Tasks 2, 3) make the big move (Task 5) cleaner: after Task 2, no rewrite touches `data_paths`; after Task 3, the `onnx_policy` carve-out from Phase 4 is resolved and the regex in Task 5 doesn't need a special case.

**Files moving** (from → to):

```
digimon_gym/data_paths.py          → data_paths.py                              (Task 2)
digimon_gym/engine/onnx_policy.py  → digimon_gym/inference/onnx_policy.py       (Task 3)
digimon_gym/api.py                 → server/api.py                              (Task 4)
digimon_gym/env.py                 → server/env.py                              (Task 4)
digimon_gym/digilab_client.py      → server/digilab_client.py                   (Task 4)
digimon_gym/routers/*              → server/routers/*                           (Task 4)
digimon_gym/db/*                   → server/db/*                                (Task 4)
digimon_gym/ai/*                   → server/ai/*                                (Task 4)
digimon_gym/storage/*              → server/storage/*                           (Task 4)
digimon_gym/classifier/*           → server/classifier/*                        (Task 4)
digimon_gym/agents/training_worker.py     → server/workers/training_worker.py   (Task 4)
digimon_gym/agents/gauntlet_orchestrator.py → server/workers/gauntlet_orchestrator.py (Task 4)
```

**`digimon_gym/engine/__init__.py` deletion:** After Task 3 moves `onnx_policy.py` to `digimon_gym/inference/`, the `digimon_gym/engine/` directory becomes empty save for `__init__.py` (already empty). Delete the directory in Task 3 — leaving an empty engine subpackage on disk would invite confusion about whether it's the live engine or a vestige.

**Files staying** in `digimon_gym/`:
```
digimon_gym/__init__.py
digimon_gym/digimon_gym.py            (DigimonEnv)
digimon_gym/inference/__init__.py     (created in Task 3)
digimon_gym/inference/onnx_policy.py  (created in Task 3)
digimon_gym/agents/* (minus training_worker, gauntlet_orchestrator)
```

**Production callers being rewritten:** every Python file under `digimon_gym/` (the RL leftovers + agents), `tools/`, `tests/`, plus `engine_py_legacy/` (sunset legacy) and `alembic/env.py`. Approximately 100 files in total — same scale as Phase 4.

**Things outside scope** (Phase 6 or 7 will address):
- Hoisting `server/`, `digimon_gym/`, etc. into `code/` (Phase 6).
- Rewriting `CLAUDE.md`, `AGENTS.md`, `README.md`, `GEMINI.md` prose mentions of `digimon_gym.api` (Phase 7). However, **operational** uvicorn invocations in `Dockerfile`, `.vscode/launch.json` are infrastructure and DO get updated in Task 6.
- Splitting `pyproject.toml`'s `[project.dependencies]` between server and RL.
- Re-organizing the `models/` directory layout (the `<run_id>/` subdirectory convention is documented as a follow-up in the spec).

---

## Task 1: Create `server/` skeleton

**Files:**
- Create: `server/__init__.py`
- Create: `server/workers/__init__.py`

- [ ] **Step 1: Make the directories**

```bash
mkdir -p server/workers
```

- [ ] **Step 2: Add the package marker**

Write `server/__init__.py`:

```python
"""FastAPI service: HTTP/WS, DB, auth, AI pipeline, storage, admin."""
```

- [ ] **Step 3: Add the workers subpackage marker**

Write `server/workers/__init__.py` as an empty file:

```python
```

- [ ] **Step 4: Verify**

```bash
ls server/ && ls server/workers/
```

Expected:
```
__init__.py
workers
__init__.py
```

- [ ] **Step 5: Commit**

```bash
git add server/
git commit -m "server: create package skeleton"
```

---

## Task 2: Move `data_paths.py` to repo root

`digimon_gym/data_paths.py` is a top-level utility — not RL, not server, not engine. Many tools, tests, RL agents, and the legacy engine import it. Move it now so the big server-move task has one fewer alias to handle.

**Files:**
- Move: `digimon_gym/data_paths.py` → `data_paths.py`

- [ ] **Step 1: Move the file**

```bash
git mv digimon_gym/data_paths.py data_paths.py
```

- [ ] **Step 2: Rewrite all importers**

```bash
python - <<'PY'
import re
from pathlib import Path

pattern = re.compile(r"\bdigimon_gym\.data_paths\b")

count = 0
roots = ["digimon_gym", "server", "tools", "tests", "engine_py_legacy", "qa", "alembic", "data_paths.py"]
for root in roots:
    p_root = Path(root)
    if not p_root.exists():
        continue
    if p_root.is_file():
        files = [p_root]
    else:
        files = list(p_root.rglob("*.py"))
    for p in files:
        text = p.read_text(encoding="utf-8")
        new = pattern.sub("data_paths", text)
        if new != text:
            p.write_text(new, encoding="utf-8")
            count += 1
            print(f"rewrote: {p}")
print(f"\n{count} files rewritten")
PY
```

Expected: ~30 files rewritten.

- [ ] **Step 3: Audit — zero stale `digimon_gym.data_paths` references**

```bash
python - <<'PY'
import re
from pathlib import Path
pattern = re.compile(r"\bdigimon_gym\.data_paths\b")
hits = []
for root in ["digimon_gym", "server", "tools", "tests", "engine_py_legacy", "qa", "alembic"]:
    if not Path(root).exists():
        continue
    for p in Path(root).rglob("*.py"):
        for i, line in enumerate(p.read_text(encoding="utf-8").splitlines(), 1):
            if pattern.search(line):
                hits.append(f"{p}:{i}: {line.strip()}")
if hits:
    print("REMAINING:")
    print("\n".join(hits))
else:
    print("OK — no stale digimon_gym.data_paths references.")
PY
```

Expected: `OK — no stale digimon_gym.data_paths references.`

- [ ] **Step 4: Smoke import**

```bash
python -c "from data_paths import CARDS_JSON, DECK_LIBRARY, TESTED_CARDS, ARCHETYPE_ALIASES, CARD_OVERRIDES; print('ok')"
```

Expected: `ok`.

- [ ] **Step 5: Commit**

```bash
git add data_paths.py digimon_gym/ server/ tools/ tests/ engine_py_legacy/ qa/ alembic/
git commit -m "data_paths: hoist to repo root; rewrite all importers"
```

---

## Task 3: Move `onnx_policy.py` to `digimon_gym/inference/`

The Phase 4 carve-out (`onnx_policy.py` left at `digimon_gym/engine/onnx_policy.py`) gets resolved here. The new home is `digimon_gym/inference/onnx_policy.py`. After this task, `digimon_gym/engine/` is empty and gets deleted, eliminating a confusing vestigial directory.

**Files:**
- Create: `digimon_gym/inference/__init__.py`
- Move: `digimon_gym/engine/onnx_policy.py` → `digimon_gym/inference/onnx_policy.py`
- Delete: `digimon_gym/engine/` (now empty)

- [ ] **Step 1: Create the inference package**

```bash
mkdir -p digimon_gym/inference
```

Write `digimon_gym/inference/__init__.py`:

```python
"""ONNX policy loader for live agent inference (no PyTorch dependency)."""
```

- [ ] **Step 2: Move the file**

```bash
git mv digimon_gym/engine/onnx_policy.py digimon_gym/inference/onnx_policy.py
```

- [ ] **Step 3: Delete the now-empty engine directory**

The remaining `digimon_gym/engine/__init__.py` is empty (verified during planning). Remove the dir:

```bash
git rm digimon_gym/engine/__init__.py
rmdir digimon_gym/engine
```

If `rmdir` complains about `__pycache__`, remove it first:

```bash
rm -rf digimon_gym/engine/__pycache__ 2>/dev/null
rmdir digimon_gym/engine
```

- [ ] **Step 4: Rewrite the four importers**

```bash
python - <<'PY'
import re
from pathlib import Path

pattern = re.compile(r"\bdigimon_gym\.engine\.onnx_policy\b")
count = 0
for root in ["digimon_gym", "server", "tools", "tests", "engine_py_legacy"]:
    if not Path(root).exists():
        continue
    for p in Path(root).rglob("*.py"):
        text = p.read_text(encoding="utf-8")
        new = pattern.sub("digimon_gym.inference.onnx_policy", text)
        if new != text:
            p.write_text(new, encoding="utf-8")
            count += 1
            print(f"rewrote: {p}")
print(f"\n{count} files rewritten")
PY
```

Expected: 4 files rewritten:
- `digimon_gym/agents/architect_simulator.py`
- `tools/export_random_onnx.py` (two import lines, same file)
- `engine_py_legacy/engine/runners/interactive_game.py`

- [ ] **Step 5: Audit**

```bash
python - <<'PY'
import re
from pathlib import Path
pattern = re.compile(r"\bdigimon_gym\.engine\.onnx_policy\b")
hits = []
for root in ["digimon_gym", "server", "tools", "tests", "engine_py_legacy"]:
    if not Path(root).exists():
        continue
    for p in Path(root).rglob("*.py"):
        for i, line in enumerate(p.read_text(encoding="utf-8").splitlines(), 1):
            if pattern.search(line):
                hits.append(f"{p}:{i}: {line.strip()}")
if hits:
    print("REMAINING:")
    print("\n".join(hits))
else:
    print("OK.")
PY
```

Expected: `OK.`

Also confirm `digimon_gym/engine/` no longer exists:

```bash
test ! -d digimon_gym/engine && echo "engine dir gone" || echo "STILL THERE"
```

Expected: `engine dir gone`.

- [ ] **Step 6: Smoke import**

```bash
python -c "from digimon_gym.inference.onnx_policy import load_onnx_policy; print('ok')"
```

Expected: `ok`.

- [ ] **Step 7: Commit**

```bash
git add digimon_gym/inference/ digimon_gym/agents/ tools/ engine_py_legacy/
git commit -m "onnx_policy: move to digimon_gym/inference/; remove empty engine dir"
```

---

## Task 4: Move all server modules and workers

Big atomic file-move task. NO import rewrites in this task — those land in Task 5. The repo is broken between Tasks 4 and 5; the two should be reviewed together but committed separately so each diff is reviewable on its own.

**Files moved:**

| From | To |
|---|---|
| `digimon_gym/api.py` | `server/api.py` |
| `digimon_gym/env.py` | `server/env.py` |
| `digimon_gym/digilab_client.py` | `server/digilab_client.py` |
| `digimon_gym/routers/` | `server/routers/` |
| `digimon_gym/db/` | `server/db/` |
| `digimon_gym/ai/` | `server/ai/` |
| `digimon_gym/storage/` | `server/storage/` |
| `digimon_gym/classifier/` | `server/classifier/` |
| `digimon_gym/agents/training_worker.py` | `server/workers/training_worker.py` |
| `digimon_gym/agents/gauntlet_orchestrator.py` | `server/workers/gauntlet_orchestrator.py` |

- [ ] **Step 1: Move the standalone files**

```bash
git mv digimon_gym/api.py server/api.py
git mv digimon_gym/env.py server/env.py
git mv digimon_gym/digilab_client.py server/digilab_client.py
git mv digimon_gym/agents/training_worker.py server/workers/training_worker.py
git mv digimon_gym/agents/gauntlet_orchestrator.py server/workers/gauntlet_orchestrator.py
```

- [ ] **Step 2: Move the subpackages**

```bash
git mv digimon_gym/routers server/routers
git mv digimon_gym/db server/db
git mv digimon_gym/ai server/ai
git mv digimon_gym/storage server/storage
git mv digimon_gym/classifier server/classifier
```

- [ ] **Step 3: Verify the moves**

```bash
ls digimon_gym/ | sort
```

Expected (exactly):
```
__init__.py
agents
digimon_gym.py
inference
```

```bash
ls server/ | sort
```

Expected (exactly):
```
__init__.py
ai
api.py
classifier
db
digilab_client.py
env.py
routers
storage
workers
```

```bash
ls digimon_gym/agents/ | sort
```

Expected (exactly — no `training_worker.py`, no `gauntlet_orchestrator.py`):
```
__init__.py
architect_agent.py
architect_cotraining.py
architect_env.py
architect_explain.py
architect_optimizer.py
architect_pool.py
architect_simulator.py
architect_training.py
deck_pool.py
features_extractor.py
gauntlet.py
league_wrapper.py
maskable_recurrent
pilot_training.py
training_metrics.py
```

- [ ] **Step 4: Commit**

```bash
git add digimon_gym/ server/
git commit -m "server: move modules and workers (no import rewrites yet)"
```

**No pytest in this task.** The repo's imports are broken between Tasks 4 and 5.

---

## Task 5: Rewrite all imports

Single regex-driven pass across the whole tree to reroute nine module prefixes. The `digimon_gym.agents.{training_worker,gauntlet_orchestrator}` rewrites must run BEFORE the generic `digimon_gym.agents.X` rewrite (there isn't a generic one in this task — but the order still matters if anything else gets added later). The order in the script below handles this correctly.

**Files modified:** every `*.py` under `digimon_gym/`, `server/`, `tools/`, `tests/`, `engine_py_legacy/`, `qa/`, `alembic/` that contains a moved-prefix import.

- [ ] **Step 1: Run the rewrite**

```bash
python - <<'PY'
import re
from pathlib import Path

# (pattern, replacement) — order matters: more specific before more general.
substitutions = [
    # Workers (more specific than the bare "digimon_gym.agents." prefix, must come first)
    (re.compile(r"\bdigimon_gym\.agents\.training_worker\b"), "server.workers.training_worker"),
    (re.compile(r"\bdigimon_gym\.agents\.gauntlet_orchestrator\b"), "server.workers.gauntlet_orchestrator"),
    # Standalone server-side modules
    (re.compile(r"\bdigimon_gym\.api\b"), "server.api"),
    (re.compile(r"\bdigimon_gym\.env\b"), "server.env"),
    (re.compile(r"\bdigimon_gym\.digilab_client\b"), "server.digilab_client"),
    # Subpackages — the trailing dot/word-boundary anchors prevent partial matches
    (re.compile(r"\bdigimon_gym\.routers\b"), "server.routers"),
    (re.compile(r"\bdigimon_gym\.db\b"), "server.db"),
    (re.compile(r"\bdigimon_gym\.ai\b"), "server.ai"),
    (re.compile(r"\bdigimon_gym\.storage\b"), "server.storage"),
    (re.compile(r"\bdigimon_gym\.classifier\b"), "server.classifier"),
]

count = 0
roots = ["digimon_gym", "server", "tools", "tests", "engine_py_legacy", "qa", "alembic"]
for root in roots:
    p_root = Path(root)
    if not p_root.exists():
        continue
    for p in p_root.rglob("*.py"):
        text = p.read_text(encoding="utf-8")
        new = text
        for pattern, replacement in substitutions:
            new = pattern.sub(replacement, new)
        if new != text:
            p.write_text(new, encoding="utf-8")
            count += 1
print(f"{count} files rewritten")
PY
```

Expected: ~80–100 files rewritten.

- [ ] **Step 2: Audit zero stale references**

```bash
python - <<'PY'
import re
from pathlib import Path

# Match the moved prefixes — exclude anything that legitimately stays
# (e.g., digimon_gym.agents.architect_*, digimon_gym.agents.gauntlet — the gauntlet
# module is still in agents/, only gauntlet_orchestrator moved to server.workers).
moved = re.compile(
    r"\bdigimon_gym\.("
    r"api|env|digilab_client|routers|db|ai|storage|classifier"
    r"|agents\.(training_worker|gauntlet_orchestrator)"
    r")\b"
)
hits = []
for root in ["digimon_gym", "server", "tools", "tests", "engine_py_legacy", "qa", "alembic"]:
    if not Path(root).exists():
        continue
    for p in Path(root).rglob("*.py"):
        for i, line in enumerate(p.read_text(encoding="utf-8").splitlines(), 1):
            if moved.search(line):
                hits.append(f"{p}:{i}: {line.strip()}")
if hits:
    print("REMAINING:")
    print("\n".join(hits[:40]))
else:
    print("OK — no stale references to moved digimon_gym.* prefixes.")
PY
```

Expected: `OK — no stale references to moved digimon_gym.* prefixes.`

- [ ] **Step 3: Smoke-import each surface**

```bash
python -c "
import server.api
import server.env
import server.digilab_client
import server.workers.training_worker
import server.workers.gauntlet_orchestrator
import server.routers.games
import server.routers.deck_tools
import server.routers.deck_optimizer
import server.routers.lobby
import server.routers.matchmaking
import server.routers.recordings
import server.routers.replays
import server.routers.simulations
import server.routers.state
import server.routers.ws_games
import server.routers.ws_manager
import server.routers.ws_matchmaking
import server.routers.debug_games
import server.routers.health
import server.db.database
import server.db.models
import server.db.auth
import server.db.routers.admin_ai
import server.db.routers.admin_models
import server.db.routers.admin_releases
import server.db.routers.assets
import server.db.routers.auth
import server.db.routers.decks
import server.db.routers.friends
import server.db.routers.issues
import server.db.routers.patch_notes
import server.db.routers.training
import server.db.routers.users
import server.ai.worker
import server.ai.client
import server.ai.dispatcher
import server.ai.set_run_orchestrator
import server.ai.retrieval
import server.ai.batch_orchestrator
import server.ai.pattern_learner
import server.ai.issue_resolution
import server.ai.autofix_apply
import server.ai.contracts
import server.ai.git_adapter
import server.ai.prompts
import server.storage.spaces
import server.storage.model_resolver
import server.classifier.deck_tagger
import server.classifier.meta_tier
import digimon_gym.digimon_gym
import digimon_gym.agents.pilot_training
import digimon_gym.agents.architect_simulator
import digimon_gym.inference.onnx_policy
print('ok')
"
```

Expected: `ok`. Any `ModuleNotFoundError` indicates a missed rewrite — find and fix.

- [ ] **Step 4: Server boot smoke**

```bash
python -c "from server.api import app; print('app ok'); print(len(app.routes), 'routes')"
```

Expected: `app ok` and ~166 routes (matches Phase 4 baseline).

- [ ] **Step 5: Commit**

```bash
git add digimon_gym/ server/ tools/ tests/ engine_py_legacy/ qa/ alembic/
git commit -m "server: rewrite imports for moved modules"
```

---

## Task 6: Update operational entrypoints

The uvicorn target moved from `digimon_gym.api:app` to `server.api:app`. Update infrastructure files that reference the old path. **Documentation prose** (CLAUDE.md, AGENTS.md, README.md, GEMINI.md, docs/runbooks/desktop-release.md, .claude/plans/model-admin-api.md) is left for Phase 7 — those are dev guides, not running infrastructure.

**Files modified:**
- `Dockerfile`
- `.vscode/launch.json`
- `alembic/env.py` (already rewritten in Task 5 by the regex pass; verify here)

- [ ] **Step 1: Update `Dockerfile`**

Replace the CMD line:

```dockerfile
CMD ["sh", "-c", "alembic upgrade head && uvicorn digimon_gym.api:app --host 0.0.0.0 --port 8000"]
```

with:

```dockerfile
CMD ["sh", "-c", "alembic upgrade head && uvicorn server.api:app --host 0.0.0.0 --port 8000"]
```

- [ ] **Step 2: Update `.vscode/launch.json`**

Replace `"digimon_gym.api:app"` with `"server.api:app"`. Use Edit tool with these exact strings:

Find:
```json
        "digimon_gym.api:app",
```

Replace with:
```json
        "server.api:app",
```

- [ ] **Step 3: Verify alembic env.py landed cleanly**

```bash
grep -n "from server.db.models import Base\|from digimon_gym.db.models import Base" alembic/env.py
```

Expected: a single line showing `from server.db.models import Base`. If you see the old `digimon_gym.db.models` line instead, the Task 5 regex missed it — manually edit and report.

- [ ] **Step 4: Smoke alembic config**

```bash
python -c "
import sys
sys.path.insert(0, '.')
from alembic.config import Config
from alembic import command
cfg = Config('alembic.ini')
print('alembic config loaded ok')
"
```

Expected: `alembic config loaded ok`. If alembic actually tries to connect to a DB, that's a different error and not what this is checking — we're verifying the env.py imports resolve.

- [ ] **Step 5: Commit**

```bash
git add Dockerfile .vscode/launch.json alembic/env.py
git commit -m "server: update operational entrypoints (Dockerfile, vscode, alembic)"
```

(`alembic/env.py` may already be in HEAD from Task 5's commit; if `git status` shows it clean, omit it from the `git add` and the commit just covers Dockerfile + launch.json.)

---

## Task 7: Default RL training output to `<repo_root>/models/`

The spec mandates trained models land in `<repo_root>/models/`. Today, `pilot_training.py` defaults `models_dir="models"` (relative to CWD). Make the default explicit so it's robust regardless of CWD. Same change in `architect_training.py` and any other training entrypoint that writes a model artifact.

**Files modified:**
- `digimon_gym/agents/pilot_training.py`
- `digimon_gym/agents/architect_training.py`

- [ ] **Step 1: Inspect current default in `pilot_training.py`**

```bash
grep -n "models_dir" digimon_gym/agents/pilot_training.py | head -10
```

Expected: shows lines like `models_dir: str = "models"` and `os.makedirs(models_dir, ...)`.

- [ ] **Step 2: Add a repo-root helper at the top of `pilot_training.py`**

Add this after the existing top-of-file imports (look for the last `import` line near the top):

```python
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_MODELS_DIR = str(_REPO_ROOT / "models")
```

Then change every `models_dir: str = "models"` in this file to `models_dir: str = DEFAULT_MODELS_DIR`. Use grep to find all occurrences before editing:

```bash
grep -n 'models_dir.*"models"' digimon_gym/agents/pilot_training.py
```

Edit each one to use `DEFAULT_MODELS_DIR` instead. (There may be only one default-arg site plus a few constructor calls that pass through.)

- [ ] **Step 3: Apply the same change to `architect_training.py`**

```bash
grep -n 'models_dir\|"models"' digimon_gym/agents/architect_training.py | head -10
```

If `architect_training.py` has a `models_dir` default arg, apply the same pattern: import the helper from `pilot_training` (single source of truth) or define an equivalent locally if the indirection is awkward. Preferred — add at the top of `architect_training.py`:

```python
from digimon_gym.agents.pilot_training import DEFAULT_MODELS_DIR
```

Then replace `"models"` defaults with `DEFAULT_MODELS_DIR`.

If `architect_training.py` doesn't already write models or use `models_dir`, skip this step and note it in the report.

- [ ] **Step 4: Verify `.gitignore` already excludes `models/`**

```bash
grep -n "^models/" .gitignore
```

Expected: line `12:models/` (and `42:src-tauri/resources/models/`). Already present from prior phases. No change needed.

- [ ] **Step 5: Smoke-test the default**

```bash
python -c "
from digimon_gym.agents.pilot_training import DEFAULT_MODELS_DIR
from pathlib import Path
print(DEFAULT_MODELS_DIR)
assert DEFAULT_MODELS_DIR.endswith('models'), 'unexpected default'
assert Path(DEFAULT_MODELS_DIR).is_absolute(), 'default must be absolute'
print('ok')
"
```

Expected: prints the absolute path and `ok`.

- [ ] **Step 6: Commit**

```bash
git add digimon_gym/agents/pilot_training.py digimon_gym/agents/architect_training.py
git commit -m "agents: default models_dir to <repo_root>/models/"
```

---

## Task 8: Verification

Verify default pytest, explicit legacy pytest, server boot, and the final stale-import audit.

- [ ] **Step 1: Default pytest run**

```bash
python -m pytest -v 2>&1 | tail -30
```

Expected: same green-or-pre-existing-fail baseline as the end of Phase 4. **No new failures**. Pre-existing failures (Windows `cp1252` cards.json issues, `moto` API drift in `tests/api/test_admin_models.py` and `tests/api/test_matchmaking_inline_deck.py`) are unchanged.

If there's a new failure, the most likely cause is a missed import rewrite — run the audit (Step 4) first to localize.

- [ ] **Step 2: Explicit legacy pytest smoke**

```bash
python -m pytest engine_py_legacy/tests/engine -x -q 2>&1 | tail -10
```

Expected: same pass count as Phase 4 end (490 pass / 2 skipped, give or take).

- [ ] **Step 3: Rust engine smoke (sanity — should be untouched)**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --quiet 2>&1 | tail -5
```

Expected: same baseline as Phase 4 (185+ pass; 2 pre-existing dsl failures unrelated to this phase).

- [ ] **Step 4: Final inbound-import audit**

```bash
python - <<'PY'
import re
from pathlib import Path

moved = re.compile(
    r"\bdigimon_gym\.("
    r"api|env|digilab_client|routers|db|ai|storage|classifier|data_paths"
    r"|engine\.onnx_policy"
    r"|agents\.(training_worker|gauntlet_orchestrator)"
    r")\b"
)
hits = []
for root in ["digimon_gym", "server", "tools", "tests", "engine_py_legacy", "qa", "alembic"]:
    if not Path(root).exists():
        continue
    for p in Path(root).rglob("*.py"):
        for i, line in enumerate(p.read_text(encoding="utf-8").splitlines(), 1):
            if moved.search(line):
                hits.append(f"{p}:{i}: {line.strip()}")
print(f"{len(hits)} stale references:")
for h in hits[:40]:
    print(f"  {h}")
PY
```

Expected: `0 stale references:`.

- [ ] **Step 5: Server boot smoke (post-rewrite)**

```bash
python -c "from server.api import app; print('app ok'); print(len(app.routes), 'routes')"
```

Expected: `app ok` followed by the route count (~166).

- [ ] **Step 6: DigimonEnv smoke (RL package still works)**

```bash
python -c "
from digimon_gym.digimon_gym import DigimonEnv
env = DigimonEnv()
obs, info = env.reset()
print('env ok', obs.shape, info['action_mask'].shape)
"
```

Expected: `env ok` followed by shape info matching Phase 4 baseline.

- [ ] **Step 7: Rust-backend RL smoke**

```bash
DIGIMON_BACKEND=rust python -c "
from digimon_gym.digimon_gym import DigimonEnv
env = DigimonEnv()
obs, info = env.reset()
print('rust env ok', obs.shape)
"
```

Expected: `rust env ok` followed by shape info.

- [ ] **Step 8: Tools smoke**

```bash
python -c "
import importlib
for m in ['tools.build_registry', 'tools.build_tested_cards', 'tools.ingest_cards', 'tools.meta_loader', 'tools.resolve_deck', 'tools.export_random_onnx']:
    importlib.import_module(m)
print('tools ok')
"
```

Expected: `tools ok`.

**No commit in this task.** If a step fails, stop and report which step + the exact error.

---

## Task 9: Push and update PR

- [ ] **Step 1: Inspect commit list**

```bash
git log --oneline origin/main..HEAD | head -20
```

Expected: 7 new Phase-5 commits on top of Phase 4's 11 commits — single linear history.

- [ ] **Step 2: Push**

```bash
git push
```

- [ ] **Step 3: Update PR title and body**

```bash
gh pr edit 358 --title "server split: spec + Phases 1-5 (transpiler delete, PyO3 bindings, caller cutover, engine_py_legacy, server extraction)"
```

Update the PR body via `gh pr view 358 --json body -q .body`, append a "Phase 5" section listing:
- Server modules extracted to `server/` (api, env, routers, db, ai, storage, classifier, digilab_client)
- Workers extracted to `server/workers/` (training_worker, gauntlet_orchestrator)
- `data_paths.py` moved to repo root
- `onnx_policy.py` moved to `digimon_gym/inference/`; empty `digimon_gym/engine/` deleted
- ~80–100 import sites rewritten across `digimon_gym/`, `server/`, `tools/`, `tests/`, `engine_py_legacy/`, `qa/`, `alembic/`
- Operational entrypoints updated: `Dockerfile`, `.vscode/launch.json`, `alembic/env.py`
- Default RL `models_dir` set to `<repo_root>/models/`
- Documentation prose updates deferred to Phase 7

Use a HEREDOC to overwrite the body cleanly (mirroring the Phase 4 push pattern).

---

## Self-review notes (post-write)

- **Spec coverage:** Every Phase 5 spec bullet maps to a task — module moves (Task 4), data_paths to root (Task 2), onnx_policy relocate (Task 3), import-site updates (Task 5), uvicorn entrypoint (Task 6), models_dir default (Task 7). Two spec items intentionally deferred: `pyproject.toml` dependency split is non-load-bearing (no install-time errors result from leaving `fastapi` in the base deps) and Phase 7 is the natural home for it; documentation prose updates (CLAUDE.md / README.md / etc.) are explicitly Phase 7's job per the design spec.
- **Placeholder scan:** All `git mv` commands have explicit source/dest paths. All Python regex blocks are runnable as-is. Smoke-import lists name every module that should exist post-rewrite.
- **Type/path consistency:** The same nine-prefix substitution table appears in Task 5's rewrite and Task 8's audit (latter accepts a few additions: data_paths and engine.onnx_policy). Module names match the directory listings in Task 4 Step 3.
- **Ordering:** Tasks 2 (data_paths) and 3 (onnx_policy) ship first because their callers span both sides of the upcoming split — moving them later would force the big rewrite to handle them as additional cases. Task 4 (move) and Task 5 (rewrite) are deliberately separate commits so the file-move diff is reviewable as a pure rename (`git diff --stat HEAD~ HEAD` shows only renames in Task 4's commit), and Task 5's diff shows only one-line `import` changes.
- **Coordination with Phase 6 (next):** Phase 6 hoists `server/`, `digimon_gym/`, `data_paths.py`, etc. into a `code/` folder. Phase 5 keeps everything at repo root so Phase 6 has a clean atomic `git mv` operation. Nothing in Phase 5 prejudges the Phase 6 layout.
