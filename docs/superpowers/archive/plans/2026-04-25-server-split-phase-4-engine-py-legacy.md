# Phase 4: Move Python Engine + Tests to `engine_py_legacy/` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Relocate the sunsetting Python engine and its coupled test trees into a sealed `engine_py_legacy/` package. Every production caller still on the Python engine (the parity-doc residue from Phase 3) is rewritten to import from `engine_py_legacy.engine.*`. After this phase, no production code or non-legacy test imports `digimon_gym.engine.*` (the only allowed leftover is `digimon_gym.engine.onnx_policy`, which is *not* engine-coupled and stays in place until Phase 5).

**Architecture:** Two `git mv` operations (engine tree, test trees) plus three mechanical find-replace passes (intra-engine imports, test imports including `tests.helpers.*` cross-references, production caller imports). The pivotal carve-outs are:
1. `digimon_gym/engine/onnx_policy.py` and `digimon_gym/engine/__init__.py` stay put — they carry the ONNX loader that Phase 5 will relocate.
2. `tests/engine/test_rust_bindings_surface.py` is *not* engine-coupled — it tests the `digimon_engine` Rust bindings — so it relocates one level up to `tests/test_rust_bindings_surface.py` rather than into `engine_py_legacy/tests/engine/`.
3. `tests/conftest.py`'s autouse `reset_registry` and `debug_runner` fixtures are only meaningful for engine-coupled tests; their bodies move into `engine_py_legacy/tests/conftest.py` and the root `tests/conftest.py` is deleted.

**Tech Stack:** Python 3.11, pytest, git mv (history preservation).

**Spec:** `docs/superpowers/specs/2026-04-25-server-digimon-gym-split-design.md` (Phase 4).

---

## Pre-flight context

Running this plan presumes Phase 3 has shipped, i.e., the `digimon_engine` PyO3 bindings replace most former `digimon_gym.engine.*` callers. The remaining production callers — listed in `docs/RUST_PYTHON_PARITY.md` § "Phase 3 residue" — are the ones this phase rewrites to `engine_py_legacy.engine.*`.

Files being moved (verbatim, history preserved via `git mv`):

| From | To |
|---|---|
| `digimon_gym/engine/` (everything **except** `__init__.py` and `onnx_policy.py`) | `engine_py_legacy/engine/` |
| `tests/engine/` (**except** `test_rust_bindings_surface.py`) | `engine_py_legacy/tests/engine/` |
| `tests/behavioral/` | `engine_py_legacy/tests/behavioral/` |
| `tests/runners/` | `engine_py_legacy/tests/runners/` |
| `tests/scenarios/` | `engine_py_legacy/tests/scenarios/` |
| `tests/helpers/` | `engine_py_legacy/tests/helpers/` |
| `tests/tools/` | `engine_py_legacy/tests/tools/` |
| `tests/engine/test_rust_bindings_surface.py` | `tests/test_rust_bindings_surface.py` |

Files staying:

- `digimon_gym/engine/__init__.py` (now empty package marker — was already empty)
- `digimon_gym/engine/onnx_policy.py` (Phase 5 will relocate)
- `tests/api/`, `tests/rl/`, `tests/classifier/`, `tests/storage/`, `tests/ai_pipeline/`, `tests/test_decklist_analysis.py`, `tests/test_store_night.py`, `tests/e2e_smoke.mjs`

Production callers with `digimon_gym.engine.*` imports that get rewritten to `engine_py_legacy.engine.*`:

```
digimon_gym/digimon_gym.py
digimon_gym/agents/features_extractor.py
digimon_gym/agents/architect_simulator.py     (HeadlessGame only — onnx_policy import stays)
digimon_gym/db/routers/admin_ai.py
digimon_gym/db/routers/training.py
digimon_gym/db/routers/decks.py
digimon_gym/routers/ws_manager.py
digimon_gym/routers/ws_games.py
digimon_gym/routers/state.py
digimon_gym/routers/simulations.py
digimon_gym/routers/replays.py
digimon_gym/routers/games.py                  (HeadlessGame, InteractiveGame, PlayerType, model_utils — onnx_policy import stays)
digimon_gym/routers/debug_games.py
digimon_gym/routers/recordings.py
digimon_gym/routers/lobby.py
digimon_gym/routers/matchmaking.py
tools/train_card_autoencoder.py
tools/run_scenario.py
tools/run_qa_batch.py
tools/promote_script.py
tools/meta_loader.py
tools/ingest_cards.py
tools/archive/bootstrap_frozen_manifest.py
```

`tools/export_random_onnx.py` and any other call site importing only `digimon_gym.engine.onnx_policy.*` keep their imports unchanged.

---

## Task 1: Create `engine_py_legacy/` skeleton

**Files:**
- Create: `engine_py_legacy/__init__.py`
- Create: `engine_py_legacy/README.md`
- Create: `engine_py_legacy/tests/__init__.py`

- [ ] **Step 1: Create the package directories**

```bash
mkdir -p engine_py_legacy/tests
```

- [ ] **Step 2: Add the legacy package marker**

Write `engine_py_legacy/__init__.py` with this exact content:

```python
"""Sunset Python engine. Reference material only — see README.md."""
```

- [ ] **Step 3: Add the tests subpackage marker**

Write `engine_py_legacy/tests/__init__.py` as an empty file:

```python
```

- [ ] **Step 4: Add the sunset README**

Write `engine_py_legacy/README.md` with this content:

```markdown
# `engine_py_legacy/` — Sunset Python Engine

This package is **sunset reference material** for the Rust engine parity effort.

## Rules

- **Do not extend.** No new features, no new card scripts, no new tests.
- **Do not import from production code** unless the call site is tracked in
  `docs/RUST_PYTHON_PARITY.md` § "Phase 3 residue" (these are short-lived
  escape hatches with explicit removal triggers).
- **Tests here run on demand only.** They are excluded from default `pytest`
  collection. Run them explicitly with:
  ```bash
  python -m pytest engine_py_legacy/tests
  ```

## Deletion trigger

Delete this directory when `docs/RUST_PYTHON_PARITY.md` shows zero entries in
the residue table — i.e., when every parity-doc-tracked caller has migrated
to `digimon_engine` (Rust via PyO3) and the divergences table is empty.

## Layout

- `engine/` — verbatim move of the former `digimon_gym/engine/` (minus
  `onnx_policy.py`, which stays at `digimon_gym/engine/onnx_policy.py` until
  Phase 5 relocates it to `digimon_gym/inference/onnx_policy.py`).
- `tests/` — verbatim move of `tests/{engine,behavioral,runners,scenarios,helpers,tools}/`.
```

- [ ] **Step 5: Commit**

```bash
git add engine_py_legacy/
git commit -m "engine_py_legacy: create package skeleton"
```

---

## Task 2: Carve out `test_rust_bindings_surface.py` before the bulk move

This test imports only `digimon_engine` (the Rust binding), not `digimon_gym.engine`. It must not move into `engine_py_legacy/`. Do this carve-out *before* the bulk `git mv` of `tests/engine/` so git tracks the rename cleanly.

**Files:**
- Move: `tests/engine/test_rust_bindings_surface.py` → `tests/test_rust_bindings_surface.py`

- [ ] **Step 1: Move the file**

```bash
git mv tests/engine/test_rust_bindings_surface.py tests/test_rust_bindings_surface.py
```

- [ ] **Step 2: Verify it still collects and passes**

```bash
python -m pytest tests/test_rust_bindings_surface.py -v
```

Expected: PASS (51 tests).

- [ ] **Step 3: Commit**

```bash
git add tests/test_rust_bindings_surface.py
git commit -m "tests: hoist rust bindings surface test out of tests/engine before legacy move"
```

---

## Task 3: Move the engine tree to `engine_py_legacy/engine/`

**Files:**
- Move: `digimon_gym/engine/{core,data,debug,game,interfaces,runners,validation,events.py,loggers.py,recording.py,state_filter.py,model_utils.py}` → `engine_py_legacy/engine/`
- Keep at original path: `digimon_gym/engine/__init__.py`, `digimon_gym/engine/onnx_policy.py`

- [ ] **Step 1: Move every engine subpath except `__init__.py` and `onnx_policy.py`**

```bash
git mv digimon_gym/engine/core engine_py_legacy/engine/core
git mv digimon_gym/engine/data engine_py_legacy/engine/data
git mv digimon_gym/engine/debug engine_py_legacy/engine/debug
git mv digimon_gym/engine/game engine_py_legacy/engine/game
git mv digimon_gym/engine/interfaces engine_py_legacy/engine/interfaces
git mv digimon_gym/engine/runners engine_py_legacy/engine/runners
git mv digimon_gym/engine/validation engine_py_legacy/engine/validation
git mv digimon_gym/engine/events.py engine_py_legacy/engine/events.py
git mv digimon_gym/engine/loggers.py engine_py_legacy/engine/loggers.py
git mv digimon_gym/engine/recording.py engine_py_legacy/engine/recording.py
git mv digimon_gym/engine/state_filter.py engine_py_legacy/engine/state_filter.py
git mv digimon_gym/engine/model_utils.py engine_py_legacy/engine/model_utils.py
```

- [ ] **Step 2: Add an `__init__.py` for the new engine subpackage**

Write `engine_py_legacy/engine/__init__.py` as an empty file:

```python
```

- [ ] **Step 3: Verify the carve-out is intact**

```bash
ls digimon_gym/engine/
```

Expected output (exactly two entries plus `__pycache__`):
```
__init__.py
onnx_policy.py
```

```bash
ls engine_py_legacy/engine/ | sort
```

Expected: `__init__.py core data debug events.py game interfaces loggers.py model_utils.py recording.py runners state_filter.py validation`.

- [ ] **Step 4: Commit**

```bash
git add digimon_gym/engine/ engine_py_legacy/engine/
git commit -m "engine_py_legacy: move python engine tree (onnx_policy stays put)"
```

---

## Task 4: Rewrite intra-engine imports inside `engine_py_legacy/engine/`

Every Python file inside the moved tree imports siblings via `digimon_gym.engine.*`. Rewrite to `engine_py_legacy.engine.*`. The `digimon_gym.engine.onnx_policy` import in `interactive_game.py` is allowed to stay — `onnx_policy` is still at that path. We use a precise replacement that doesn't catch it: replace `digimon_gym.engine.` only when the next token is one of the moved subpaths.

**Files:**
- Modify: every `*.py` under `engine_py_legacy/engine/` containing `from digimon_gym.engine.` or `import digimon_gym.engine.`

- [ ] **Step 1: Run the rewrite**

```bash
python - <<'PY'
import re
from pathlib import Path

# Subpaths that moved. Anything else (notably onnx_policy) stays.
moved = ["core", "data", "debug", "events", "game", "interfaces", "loggers",
         "model_utils", "recording", "runners", "state_filter", "validation"]
pattern = re.compile(r"\bdigimon_gym\.engine\.(" + "|".join(moved) + r")\b")

count = 0
for p in Path("engine_py_legacy/engine").rglob("*.py"):
    text = p.read_text(encoding="utf-8")
    new = pattern.sub(r"engine_py_legacy.engine.\1", text)
    if new != text:
        p.write_text(new, encoding="utf-8")
        count += 1
        print(f"rewrote: {p}")
print(f"\n{count} files rewritten")
PY
```

Expected: ~50–80 files rewritten (engine internals plus card scripts).

- [ ] **Step 2: Verify no stale intra-engine imports remain**

```bash
python -m pytest --co -q engine_py_legacy/tests 2>&1 | head -5  # collection sanity (will run in Task 6)
```

Run this grep:

```bash
python - <<'PY'
import re
from pathlib import Path
moved = ["core", "data", "debug", "events", "game", "interfaces", "loggers",
         "model_utils", "recording", "runners", "state_filter", "validation"]
pattern = re.compile(r"\bdigimon_gym\.engine\.(" + "|".join(moved) + r")\b")
hits = []
for p in Path("engine_py_legacy/engine").rglob("*.py"):
    for i, line in enumerate(p.read_text(encoding="utf-8").splitlines(), 1):
        if pattern.search(line):
            hits.append(f"{p}:{i}: {line.strip()}")
if hits:
    print("REMAINING:")
    print("\n".join(hits))
else:
    print("OK — no stale digimon_gym.engine.* imports remain (onnx_policy preserved).")
PY
```

Expected output: `OK — no stale digimon_gym.engine.* imports remain (onnx_policy preserved).`

- [ ] **Step 3: Verify `interactive_game.py` still routes to live onnx_policy**

```bash
grep -n "onnx_policy" engine_py_legacy/engine/runners/interactive_game.py
```

Expected: `        from digimon_gym.engine.onnx_policy import load_onnx_policy` (unchanged path; `onnx_policy.py` still lives at `digimon_gym/engine/onnx_policy.py`).

- [ ] **Step 4: Smoke-test that the package imports**

```bash
python -c "import engine_py_legacy.engine.runners.headless_game; print('ok')"
```

Expected: `ok`. If this fails with `ModuleNotFoundError`, find the missed import and rewrite it.

- [ ] **Step 5: Commit**

```bash
git add engine_py_legacy/engine/
git commit -m "engine_py_legacy: rewrite intra-engine imports to engine_py_legacy.engine.*"
```

---

## Task 5: Move the engine-coupled test trees to `engine_py_legacy/tests/`

**Files:**
- Move: `tests/{engine,behavioral,runners,scenarios,helpers,tools}/` → `engine_py_legacy/tests/{engine,behavioral,runners,scenarios,helpers,tools}/`

- [ ] **Step 1: Move each tree with `git mv`**

```bash
git mv tests/engine engine_py_legacy/tests/engine
git mv tests/behavioral engine_py_legacy/tests/behavioral
git mv tests/runners engine_py_legacy/tests/runners
git mv tests/scenarios engine_py_legacy/tests/scenarios
git mv tests/helpers engine_py_legacy/tests/helpers
git mv tests/tools engine_py_legacy/tests/tools
```

- [ ] **Step 2: Verify the carve-out**

```bash
ls tests/ | sort
```

Expected (exactly):
```
ai_pipeline
api
classifier
conftest.py
e2e_smoke.mjs
rl
storage
test_decklist_analysis.py
test_rust_bindings_surface.py
test_store_night.py
```

```bash
ls engine_py_legacy/tests/ | sort
```

Expected: `__init__.py behavioral engine helpers runners scenarios tools`.

- [ ] **Step 3: Commit**

```bash
git add tests/ engine_py_legacy/tests/
git commit -m "engine_py_legacy: move engine-coupled test trees"
```

---

## Task 6: Rewrite imports inside `engine_py_legacy/tests/`

The moved tests import:
1. `digimon_gym.engine.*` → must become `engine_py_legacy.engine.*` (skipping `onnx_policy`).
2. `tests.helpers.*` → must become `engine_py_legacy.tests.helpers.*`.

**Files:**
- Modify: every `*.py` under `engine_py_legacy/tests/` containing those import patterns.

- [ ] **Step 1: Run both rewrites**

```bash
python - <<'PY'
import re
from pathlib import Path

moved = ["core", "data", "debug", "events", "game", "interfaces", "loggers",
         "model_utils", "recording", "runners", "state_filter", "validation"]
engine_re = re.compile(r"\bdigimon_gym\.engine\.(" + "|".join(moved) + r")\b")
helpers_re = re.compile(r"\btests\.helpers\b")

count = 0
for p in Path("engine_py_legacy/tests").rglob("*.py"):
    text = p.read_text(encoding="utf-8")
    new = engine_re.sub(r"engine_py_legacy.engine.\1", text)
    new = helpers_re.sub("engine_py_legacy.tests.helpers", new)
    if new != text:
        p.write_text(new, encoding="utf-8")
        count += 1
print(f"{count} files rewritten")
PY
```

Expected: ~250+ files rewritten (most behavioral test files plus engine tests).

- [ ] **Step 2: Verify no stale imports remain**

```bash
python - <<'PY'
import re
from pathlib import Path
moved = ["core", "data", "debug", "events", "game", "interfaces", "loggers",
         "model_utils", "recording", "runners", "state_filter", "validation"]
engine_re = re.compile(r"\bdigimon_gym\.engine\.(" + "|".join(moved) + r")\b")
helpers_re = re.compile(r"\btests\.helpers\b")
hits = []
for p in Path("engine_py_legacy/tests").rglob("*.py"):
    for i, line in enumerate(p.read_text(encoding="utf-8").splitlines(), 1):
        if engine_re.search(line) or helpers_re.search(line):
            hits.append(f"{p}:{i}: {line.strip()}")
if hits:
    print("REMAINING:")
    print("\n".join(hits[:50]))
else:
    print("OK — all test imports rewritten.")
PY
```

Expected: `OK — all test imports rewritten.`

- [ ] **Step 3: Verify pytest can collect the moved tree**

```bash
python -m pytest --co -q engine_py_legacy/tests 2>&1 | tail -5
```

Expected: a collection summary like `XX tests collected` with no `ImportError` or `ModuleNotFoundError`. If imports fail, find the missing pattern and add to the regex.

- [ ] **Step 4: Commit**

```bash
git add engine_py_legacy/tests/
git commit -m "engine_py_legacy: rewrite test imports (digimon_gym.engine.* and tests.helpers.*)"
```

---

## Task 7: Move root-conftest fixtures into `engine_py_legacy/tests/conftest.py`

The `reset_registry` autouse fixture and `debug_runner` factory in `tests/conftest.py` only matter for engine-coupled tests. Move their bodies into `engine_py_legacy/tests/conftest.py` with imports rewritten, then delete `tests/conftest.py` (its remaining tests don't need engine fixtures — verified during plan-writing: `tests/{api,rl,classifier,storage,ai_pipeline}` have no `CardRegistry` or `digimon_gym.engine` references).

**Files:**
- Create: `engine_py_legacy/tests/conftest.py`
- Delete: `tests/conftest.py`

- [ ] **Step 1: Write `engine_py_legacy/tests/conftest.py`**

```python
"""Engine_py_legacy test conftest — fixtures for the sunsetting Python engine.

Provides:
- reset_registry: autouse fixture that resets CardRegistry between tests
- debug_runner: factory fixture for creating DebugRunner with archetype decks
"""

import json
import pytest

from digimon_gym.data_paths import DECK_LIBRARY
from engine_py_legacy.engine.data.card_registry import CardRegistry


@pytest.fixture(autouse=True)
def reset_registry():
    """Reset CardRegistry before and after each test for isolation."""
    CardRegistry.reset()
    yield
    CardRegistry.reset()


@pytest.fixture
def debug_runner():
    """Factory fixture for creating DebugRunner from archetype names or card lists.

    Usage:
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner = debug_runner(deck1=[...], deck2=[...], skip_shuffle=True)
    """
    from engine_py_legacy.engine.runners.debug_runner import DebugRunner

    _cache = {}

    def _load_deck(archetype_name: str) -> list[str]:
        if archetype_name not in _cache:
            with open(DECK_LIBRARY, "r", encoding="utf-8") as f:
                library = json.load(f)
            arch = library["archetypes"].get(archetype_name)
            if not arch or not arch.get("decklists"):
                raise ValueError(f"No decklists for archetype: {archetype_name}")
            _cache[archetype_name] = json.loads(arch["decklists"][0]["decklist"])
        return list(_cache[archetype_name])

    def _create(
        deck1=None,
        deck2=None,
        archetype1=None,
        archetype2=None,
        **kwargs,
    ) -> DebugRunner:
        d1 = deck1 or _load_deck(archetype1 or "Puppets")
        d2 = deck2 or _load_deck(archetype2 or "Puppets")
        return DebugRunner(d1, d2, **kwargs)

    return _create
```

- [ ] **Step 2: Delete the root conftest**

```bash
git rm tests/conftest.py
```

- [ ] **Step 3: Confirm fixtures still resolve in moved tests**

```bash
python -m pytest engine_py_legacy/tests/engine/test_game_startup.py -v --no-header 2>&1 | tail -20
```

Expected: PASS (or skip with reason — but no `fixture 'reset_registry' not found` / `fixture 'debug_runner' not found` errors).

- [ ] **Step 4: Confirm root tests still collect cleanly without conftest**

```bash
python -m pytest --co -q tests 2>&1 | tail -5
```

Expected: collection summary, no `ImportError`.

- [ ] **Step 5: Commit**

```bash
git add engine_py_legacy/tests/conftest.py
git commit -m "engine_py_legacy: move engine fixtures into legacy conftest; drop root conftest"
```

---

## Task 8: Rewrite production callers (parity-doc residue)

Every parity-doc-tracked production caller still importing `digimon_gym.engine.*` (excluding `onnx_policy`) gets a literal find-replace.

**Files modified:** see the residue list at the top of this plan.

- [ ] **Step 1: Run the rewrite across the production tree**

```bash
python - <<'PY'
import re
from pathlib import Path

moved = ["core", "data", "debug", "events", "game", "interfaces", "loggers",
         "model_utils", "recording", "runners", "state_filter", "validation"]
engine_re = re.compile(r"\bdigimon_gym\.engine\.(" + "|".join(moved) + r")\b")

roots = ["digimon_gym", "tools"]
count = 0
hits = []
for root in roots:
    for p in Path(root).rglob("*.py"):
        if p.parts[0] == "digimon_gym" and len(p.parts) >= 2 and p.parts[1] == "engine":
            continue  # skip — engine subtree is supposed to be empty of moved subpaths
        text = p.read_text(encoding="utf-8")
        new = engine_re.sub(r"engine_py_legacy.engine.\1", text)
        if new != text:
            p.write_text(new, encoding="utf-8")
            count += 1
            hits.append(str(p))
print(f"\n{count} production files rewritten:")
for h in hits:
    print(f"  {h}")
PY
```

Expected: roughly 23 files rewritten (matching the residue list).

- [ ] **Step 2: Verify zero stale `digimon_gym.engine.*` imports remain in production code**

```bash
python - <<'PY'
import re
from pathlib import Path
moved = ["core", "data", "debug", "events", "game", "interfaces", "loggers",
         "model_utils", "recording", "runners", "state_filter", "validation"]
engine_re = re.compile(r"\bdigimon_gym\.engine\.(" + "|".join(moved) + r")\b")
hits = []
for root in ["digimon_gym", "tools"]:
    for p in Path(root).rglob("*.py"):
        if p.parts[0] == "digimon_gym" and len(p.parts) >= 2 and p.parts[1] == "engine":
            continue
        for i, line in enumerate(p.read_text(encoding="utf-8").splitlines(), 1):
            if engine_re.search(line):
                hits.append(f"{p}:{i}: {line.strip()}")
if hits:
    print("REMAINING (parity-doc allowed exceptions only — none expected):")
    print("\n".join(hits))
else:
    print("OK — no production code imports digimon_gym.engine.{moved subpaths}.")
PY
```

Expected: `OK — no production code imports digimon_gym.engine.{moved subpaths}.`

- [ ] **Step 3: Verify `onnx_policy` callers are still wired correctly**

```bash
grep -rn "from digimon_gym\.engine\.onnx_policy\|from digimon_gym\.engine import onnx_policy" --include="*.py" digimon_gym tools 2>&1
```

Expected output (still pointing at the in-place `onnx_policy.py`):
```
digimon_gym/agents/architect_simulator.py:90:        from digimon_gym.engine.onnx_policy import load_onnx_policy
digimon_gym/routers/games.py:... from digimon_gym.engine.onnx_policy import ...   (if present)
tools/export_random_onnx.py:141:    from digimon_gym.engine.onnx_policy import load_onnx_policy
tools/export_random_onnx.py:153:    from digimon_gym.engine.onnx_policy import load_onnx_policy
```

(`engine_py_legacy/engine/runners/interactive_game.py:57` will also still reference this path — that's correct, it lives in legacy now but still consumes the live onnx_policy.)

- [ ] **Step 4: Smoke-import every rewritten production module**

```bash
python -c "
import digimon_gym.api
import digimon_gym.digimon_gym
import digimon_gym.routers.games
import digimon_gym.routers.debug_games
import digimon_gym.routers.lobby
import digimon_gym.routers.matchmaking
import digimon_gym.routers.recordings
import digimon_gym.routers.replays
import digimon_gym.routers.simulations
import digimon_gym.routers.state
import digimon_gym.routers.ws_games
import digimon_gym.routers.ws_manager
import digimon_gym.db.routers.admin_ai
import digimon_gym.db.routers.training
import digimon_gym.db.routers.decks
import digimon_gym.agents.features_extractor
import digimon_gym.agents.architect_simulator
print('ok')
"
```

Expected: `ok`. Any `ModuleNotFoundError` indicates a missed rewrite — fix and re-run Step 1.

- [ ] **Step 5: Commit**

```bash
git add digimon_gym/ tools/
git commit -m "phase 4: rewrite parity-doc residue callers to engine_py_legacy.engine.*"
```

---

## Task 9: Update `pyproject.toml` and parity doc

The spec calls for `engine_py_legacy/tests` to be excluded from default pytest collection while remaining runnable explicitly. Since `testpaths = ["tests"]` already excludes anything outside `tests/`, the change is belt-and-suspenders: an explicit `--ignore=engine_py_legacy` so `pytest` from any working directory still skips it.

**Files:**
- Modify: `pyproject.toml`
- Modify: `docs/RUST_PYTHON_PARITY.md`

- [ ] **Step 1: Update `pyproject.toml` pytest config**

In `pyproject.toml`, change:

```toml
[tool.pytest.ini_options]
testpaths = ["tests"]
addopts = "--ignore=tests/test_rl_gym.py --ignore=tests/ai_pipeline -v"
```

to:

```toml
[tool.pytest.ini_options]
testpaths = ["tests"]
addopts = "--ignore=tests/test_rl_gym.py --ignore=tests/ai_pipeline --ignore=engine_py_legacy -v"
norecursedirs = ["engine_py_legacy"]
```

- [ ] **Step 2: Update parity doc residue paths**

Open `docs/RUST_PYTHON_PARITY.md` and find the "Phase 3 residue" table. Replace every `engine.<subpath>` reference's caller column to reflect the new import root. Specifically, change the surface column entries from `engine.runners.*` / `engine.data.*` etc. (which were already shorthand) to be unchanged in *name* but add a one-line note above the table:

Find the existing intro (lines ~1019–1024):

```markdown
## Phase 3 residue (callers still on Python engine)

These imports survived the Phase 3 cutover because the Rust counterpart
isn't in `digimon_engine` yet. Each entry is a checklist: when the
binding lands, remove the Python import and the row.
```

Replace with:

```markdown
## Phase 3 residue (callers still on Python engine)

These imports survived the Phase 3 cutover because the Rust counterpart
isn't in `digimon_engine` yet. Each entry is a checklist: when the
binding lands, remove the Python import and the row.

**As of Phase 4** (2026-04-25), all surface paths are rooted at
`engine_py_legacy.engine.*` — the Python engine moved to
`engine_py_legacy/`. The "Surface" column below uses the unqualified
shorthand (e.g., `engine.runners.headless_game.HeadlessGame`); read it
as `engine_py_legacy.engine.runners.headless_game.HeadlessGame`. The
sole exception is `engine.onnx_policy.load_onnx_policy`, which still
lives at `digimon_gym.engine.onnx_policy.load_onnx_policy` until Phase 5
relocates it to `digimon_gym/inference/`.
```

- [ ] **Step 3: Verify pytest config**

```bash
python -m pytest --co -q 2>&1 | tail -10
```

Expected: collection summary that does **not** include any `engine_py_legacy/` paths.

```bash
python -m pytest --co -q engine_py_legacy/tests 2>&1 | tail -5
```

Expected: explicit-path collection still works (collects the legacy tests).

- [ ] **Step 4: Commit**

```bash
git add pyproject.toml docs/RUST_PYTHON_PARITY.md
git commit -m "phase 4: exclude engine_py_legacy from default pytest; update parity doc"
```

---

## Task 10: Verification — full test suite + cargo + final grep

- [ ] **Step 1: Default pytest run (excluding engine_py_legacy)**

```bash
python -m pytest -v 2>&1 | tail -30
```

Expected: same green-or-pre-existing-fail baseline as the end of Phase 3 (39 known Windows-cp1252 failures from before Phase 1 are tolerated; **no new failures** introduced by this phase). If there's a new failure, it's almost certainly a missed import rewrite — find it and fix.

- [ ] **Step 2: Explicit legacy pytest run (smoke only — full run is ~60s+)**

```bash
python -m pytest engine_py_legacy/tests/engine -x -q 2>&1 | tail -20
```

Expected: collection succeeds; tests pass or fail consistently with their pre-move baseline. The point here is *collection + import* health, not pass/fail status.

- [ ] **Step 3: Rust engine smoke**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --quiet 2>&1 | tail -5
```

Expected: PASS (Rust tests are independent of this phase but we verify nothing broke).

- [ ] **Step 4: Final inbound-import audit**

```bash
python - <<'PY'
import re
from pathlib import Path
moved = ["core", "data", "debug", "events", "game", "interfaces", "loggers",
         "model_utils", "recording", "runners", "state_filter", "validation"]
engine_re = re.compile(r"\bdigimon_gym\.engine\.(" + "|".join(moved) + r")\b")

allowed_roots = {"engine_py_legacy"}
hits = []
for root in ["digimon_gym", "tools", "tests", "digimon-engine-py"]:
    if not Path(root).exists():
        continue
    for p in Path(root).rglob("*.py"):
        # digimon_gym/engine/__init__.py and onnx_policy.py are not engine-coupled internally
        for i, line in enumerate(p.read_text(encoding="utf-8").splitlines(), 1):
            if engine_re.search(line):
                hits.append(f"{p}:{i}: {line.strip()}")
print(f"{len(hits)} stale references in production / non-legacy tests:")
for h in hits[:20]:
    print(f"  {h}")
PY
```

Expected: `0 stale references in production / non-legacy tests:`.

- [ ] **Step 5: Verify `digimon_gym/engine/onnx_policy.py` import path is alive**

```bash
python -c "from digimon_gym.engine.onnx_policy import load_onnx_policy; print('ok')"
```

Expected: `ok`.

- [ ] **Step 6: Server boot smoke**

```bash
python -c "from digimon_gym.api import app; print('app ok'); print(len(app.routes), 'routes')"
```

Expected: `app ok` followed by the route count (matches Phase 3 baseline).

---

## Task 11: Push and PR

- [ ] **Step 1: Confirm all phase 4 commits are local**

```bash
git log --oneline origin/main..HEAD | head -20
```

Expected: a clean commit list — six (or so) Phase 4 commits, in order from Tasks 1–9.

- [ ] **Step 2: Push to the existing PR branch**

```bash
git push
```

Expected: push succeeds; PR #358 picks up the new commits.

- [ ] **Step 3: Update the PR description**

```bash
gh pr edit 358 --body-file - <<'EOF'
[Existing body remains; append a "Phase 4" section noting:
- Python engine moved to engine_py_legacy/
- Test trees moved to engine_py_legacy/tests/
- 23 production callers rewritten to engine_py_legacy.engine.*
- onnx_policy.py preserved at digimon_gym/engine/ for Phase 5
- pytest excludes engine_py_legacy from default collection]
EOF
```

(In practice: read the current body via `gh pr view 358 --json body -q .body`, append the Phase 4 paragraph, write it back.)

---

## Self-review notes (post-write)

- **Spec coverage:** Each phase-4 spec bullet is mapped — verbatim file move (Tasks 3, 5), import rewrite inside moved tree (Tasks 4, 6), README sunset (Task 1), `onnx_policy.py` carve-out (Tasks 3 step 1, Task 4 regex), `pyproject.toml` exclusion (Task 9), parity-doc-tracked exception verification (Task 10 step 4).
- **Placeholder scan:** All scripts are full Python one-liners with concrete regex patterns; no TODO/TBD entries.
- **Type consistency:** The moved-subpath list `["core", "data", "debug", "events", "game", "interfaces", "loggers", "model_utils", "recording", "runners", "state_filter", "validation"]` is used identically in Tasks 4, 6, 8, and 10. `digimon_gym.engine.onnx_policy` is consistently excluded by leaving it out of the moved list.
- **Cross-phase coherence:** Phase 5 (next) will move `digimon_gym/engine/onnx_policy.py` → `digimon_gym/inference/onnx_policy.py`, at which point the four remaining `digimon_gym.engine.onnx_policy` callers (architect_simulator, routers/games, export_random_onnx, engine_py_legacy/.../interactive_game) will be rewritten in one mechanical pass.
