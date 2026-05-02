# Phase 1: Delete the Transpiler Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Delete the C# → Python script transpiler (`tools/transpiler/`) and its dedicated entry points in the admin AI pipeline, keeping the AI pipeline scaffolding intact for future repurposing (user error reports → Rust script/engine fixes). The pipeline may be non-functional in alpha — that's accepted.

**Architecture:** Surgical deletion. Remove the transpiler package, its dedicated audit script, and pipeline call sites that import from it. Strip the transpiler scope profile (`script_engine_transpiler`) from `autofix_apply.py`. Delete dedicated tests and the `implement-set` skill. Pipeline modules that were structured around transpiler workflows (`set_run_orchestrator._score_cards`, `retrieval._infer_python_source_type`, `autofix_apply.SCOPE_SCRIPT_ENGINE_TRANSPILER`) lose their transpiler-specific code paths but keep their general structure.

**Tech Stack:** Python 3.11, pytest, FastAPI (server boot smoke check).

**Spec:** `docs/superpowers/specs/2026-04-25-server-digimon-gym-split-design.md` (Phase 1).

---

## File Map

**Files deleted:**
- `tools/transpiler/` (entire directory — `__init__.py`, `cli.py`, `extractors.py`, `generators.py`, `known_complex_cards.json`, `models.py`, `patterns.py`, `scoring.py`, `validation.py`)
- `tools/audit_transpiled_sets.py`
- `digimon_gym/ai/transpiler_audit.py`
- `tests/ai_pipeline/test_transpiler_scoring.py`
- `tests/ai_pipeline/test_retranspile_integration.py`
- `tests/ai_pipeline/test_set_run_retranspile.py`
- `tests/ai_pipeline/test_prompts_llm_transpile.py`
- `.claude/skills/implement-set/` (entire skill directory)

**Files modified:**
- `digimon_gym/ai/set_run_orchestrator.py` — drop `_score_cards` body (transpiler-specific)
- `digimon_gym/ai/autofix_apply.py` — drop `SCOPE_SCRIPT_ENGINE_TRANSPILER` scope and all branches that key on it
- `digimon_gym/ai/retrieval.py` — drop `tools/transpiler` branch in `_infer_python_source_type` and `_is_transpiler_path`
- `tests/ai_pipeline/test_ai_autofix_apply.py` — drop transpiler-scope tests if any
- `tests/ai_pipeline/test_retrieval.py` — drop transpiler-source-type tests if any
- `tests/ai_pipeline/test_contracts.py` — drop transpiler-scope tests if any
- `docs/TOOLS.md` — drop §C# → Python Script Transpiler section
- `docs/ARCHITECTURE.md` — drop transpiler mention
- `docs/superpowers/specs/2026-04-25-server-digimon-gym-split-design.md` — note that AI pipeline bones are kept for repurposing

---

## Task 1: Baseline check

**Files:** none (read-only verification)

- [ ] **Step 1: Confirm baseline pytest is green**

Run: `python -m pytest tests -v -x --ignore=tests/ai_pipeline 2>&1 | tail -20`

Expected: all tests pass. The `tests/ai_pipeline/` directory is excluded by default (per CLAUDE.md "tests excluded from default runs"); we still run it explicitly to know what's currently green:

Run: `python -m pytest tests/ai_pipeline -v 2>&1 | tail -40`

Expected: capture which ai_pipeline tests pass today. Tests that fail today are not regressions caused by this plan.

- [ ] **Step 2: Capture transpiler import surface**

Run: `python -c "import tools.transpiler; import tools.transpiler.scoring; import tools.transpiler.validation; import tools.transpiler.extractors; print('OK')"`

Expected: `OK`. Confirms the package imports cleanly today.

---

## Task 2: Delete `tools/transpiler/`

**Files:**
- Delete: `tools/transpiler/` (entire directory)

- [ ] **Step 1: Delete the directory**

Run: `git rm -r tools/transpiler/`

Expected: nine files removed (`__init__.py`, `cli.py`, `extractors.py`, `generators.py`, `known_complex_cards.json`, `models.py`, `patterns.py`, `scoring.py`, `validation.py`).

- [ ] **Step 2: Confirm import is now broken**

Run: `python -c "import tools.transpiler" 2>&1`

Expected: `ModuleNotFoundError: No module named 'tools.transpiler'`. Good — that's what we want.

- [ ] **Step 3: Commit**

```bash
git commit -m "$(cat <<'EOF'
chore: delete tools/transpiler/

The C# -> Python script transpiler is dead. Card scripts now live in
Rust (digimon-engine/src/cards/). The Python AI pipeline that consumed
the transpiler is left intact for later repurposing toward user error
reports against Rust scripts.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Delete `tools/audit_transpiled_sets.py`

**Files:**
- Delete: `tools/audit_transpiled_sets.py`

- [ ] **Step 1: Verify it imports the transpiler**

Run: `grep -E "from tools\.transpiler|import tools\.transpiler" tools/audit_transpiled_sets.py`

Expected: at least one match. If zero matches, this script is misnamed — stop and re-read the file before continuing.

- [ ] **Step 2: Delete**

Run: `git rm tools/audit_transpiled_sets.py`

- [ ] **Step 3: Commit**

```bash
git commit -m "$(cat <<'EOF'
chore: delete tools/audit_transpiled_sets.py

Transpiler-only diagnostic. No longer relevant.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Delete `digimon_gym/ai/transpiler_audit.py`

**Files:**
- Delete: `digimon_gym/ai/transpiler_audit.py`

- [ ] **Step 1: Confirm no other module imports it**

Run: `grep -rn "from digimon_gym.ai.transpiler_audit\|import digimon_gym.ai.transpiler_audit\|from \.transpiler_audit\|from digimon_gym\.ai import transpiler_audit" --include="*.py" .`

Expected: zero matches outside the file itself. If any importer exists, surface it before proceeding.

- [ ] **Step 2: Delete**

Run: `git rm digimon_gym/ai/transpiler_audit.py`

- [ ] **Step 3: Commit**

```bash
git commit -m "$(cat <<'EOF'
chore: delete digimon_gym/ai/transpiler_audit.py

Module exists solely to audit transpiled C# -> Python output; obsolete
with the transpiler removal.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Strip transpiler from `set_run_orchestrator._score_cards`

**Files:**
- Modify: `digimon_gym/ai/set_run_orchestrator.py:753-799`

The current `_score_cards` method imports `tools.transpiler.{scoring,validation,extractors}` to compute per-card transpile scores. With the transpiler gone, the scoring concept is meaningless. Replace the body with a no-op that marks every item with `transpile_score = 0.0`.

- [ ] **Step 1: Replace `_score_cards` body**

Open `digimon_gym/ai/set_run_orchestrator.py`. Locate the `_score_cards` method (around line 753). Replace its full body with:

```python
    def _score_cards(
        self,
        items: list[AISetRunItem],
        set_id: str,
        threshold: float,
        cs_dir: str | None = None,
    ) -> list[AISetRunItem]:
        """Compute transpile scores for all items.

        The C#->Python transpiler was removed; scoring is no longer meaningful
        here. Pipeline kept structurally for future repurposing toward user
        error reports against Rust scripts. Every item gets a zero score so
        downstream code paths that branch on the score behave deterministically.
        """
        for item in items:
            item.transpile_score = 0.0
        return items
```

- [ ] **Step 2: Delete the now-orphaned `_find_cs_file` helper if unreferenced**

Run: `grep -n "_find_cs_file" digimon_gym/ai/set_run_orchestrator.py`

If the only remaining match is the method definition (i.e., no callers left after Step 1), delete the `_find_cs_file` static method too. Otherwise leave it.

- [ ] **Step 3: Verify the file compiles**

Run: `python -m py_compile digimon_gym/ai/set_run_orchestrator.py`

Expected: silent (success).

- [ ] **Step 4: Commit**

```bash
git add digimon_gym/ai/set_run_orchestrator.py
git commit -m "$(cat <<'EOF'
refactor(ai): strip transpiler scoring from set_run_orchestrator

_score_cards becomes a no-op that zeroes the transpile_score field.
The pipeline scaffolding stays for future repurposing.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Strip transpiler scope from `autofix_apply.py`

**Files:**
- Modify: `digimon_gym/ai/autofix_apply.py` (multiple locations — see below)

Remove the `SCOPE_SCRIPT_ENGINE_TRANSPILER` scope value and every branch that keys on it.

- [ ] **Step 1: Drop the constant**

In `digimon_gym/ai/autofix_apply.py`, change lines 33–40 from:

```python
SCOPE_SCRIPT = "script"
SCOPE_SCRIPT_ENGINE = "script_engine"
SCOPE_SCRIPT_ENGINE_TRANSPILER = "script_engine_transpiler"
SCOPE_VALUES = {
    SCOPE_SCRIPT,
    SCOPE_SCRIPT_ENGINE,
    SCOPE_SCRIPT_ENGINE_TRANSPILER,
}
```

to:

```python
SCOPE_SCRIPT = "script"
SCOPE_SCRIPT_ENGINE = "script_engine"
SCOPE_VALUES = {
    SCOPE_SCRIPT,
    SCOPE_SCRIPT_ENGINE,
}
```

- [ ] **Step 2: Drop the `TRANSPILER_FILES` constant**

Remove lines 49–54:

```python
# Additional transpiler files loaded for script_engine_transpiler scope.
TRANSPILER_FILES = [
    "tools/transpiler/generators.py",
    "tools/transpiler/extractors.py",
    "tools/transpiler/patterns.py",
]
```

- [ ] **Step 3: Drop the transpiler branch in `_is_allowed_for_scope`**

Locate the function at line 120. Remove lines 134–136 (the `if scope_profile == SCOPE_SCRIPT_ENGINE_TRANSPILER` block). The function should now end with the `script_engine` branch and the final `return False`.

After edit, the function reads:

```python
def _is_allowed_for_scope(
    rel_path: str,
    *,
    scope_profile: str,
    set_id: str,
    module_name: str,
) -> bool:
    primary = f"digimon_gym/engine/data/scripts/{set_id}/{module_name}.py"
    if rel_path == primary:
        return True
    if scope_profile == SCOPE_SCRIPT:
        return False
    if rel_path.endswith(".py") and rel_path.startswith("digimon_gym/engine/"):
        return True
    return False
```

- [ ] **Step 4: Drop transpiler branch in `get_allowed_roots_for_scope`**

Locate the function at line 140. Replace lines 140–146 with:

```python
def get_allowed_roots_for_scope(scope_profile: str) -> list[str]:
    roots = ["digimon_gym/engine/data/scripts/"]
    if scope_profile == SCOPE_SCRIPT_ENGINE:
        roots.append("digimon_gym/engine/")
    return roots
```

- [ ] **Step 5: Drop transpiler branch in `build_file_context_for_task`**

Locate the function at line 162. Change the `if scope_profile in {SCOPE_SCRIPT_ENGINE, SCOPE_SCRIPT_ENGINE_TRANSPILER}:` block (lines 165–168) to:

```python
    if scope_profile == SCOPE_SCRIPT_ENGINE:
        paths.extend(ENGINE_CORE_FILES)
```

(Drop the inner `if scope_profile == SCOPE_SCRIPT_ENGINE_TRANSPILER: paths.extend(TRANSPILER_FILES)` lines entirely.)

- [ ] **Step 6: Sweep for any remaining `transpiler` references in the file**

Run: `grep -n "transpiler\|TRANSPILER" digimon_gym/ai/autofix_apply.py`

Expected: zero matches (other than possibly a docstring you can leave; if any executable code references transpiler, remove or update it).

- [ ] **Step 7: Verify the file compiles**

Run: `python -m py_compile digimon_gym/ai/autofix_apply.py`

Expected: silent (success).

- [ ] **Step 8: Commit**

```bash
git add digimon_gym/ai/autofix_apply.py
git commit -m "$(cat <<'EOF'
refactor(ai): drop SCOPE_SCRIPT_ENGINE_TRANSPILER from autofix_apply

The transpiler-specific scope profile and its file allowlist
(tools/transpiler/{generators,extractors,patterns}.py) are removed.
The script and script_engine scopes remain for future use.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Strip transpiler from `retrieval.py`

**Files:**
- Modify: `digimon_gym/ai/retrieval.py`

- [ ] **Step 1: Drop the `tools/transpiler` branch in `_infer_python_source_type`**

Locate the function around line 111. Replace lines 111–118 with:

```python
def _infer_python_source_type(file_path: str) -> str:
    """Infer source_type from a file path string."""
    normalized = file_path.replace("\\", "/")
    if "digimon_gym/engine" in normalized:
        return "engine_api"
    return "rules"
```

- [ ] **Step 2: Drop `_is_transpiler_path`**

Locate the function around line 370. Delete the entire function:

```python
def _is_transpiler_path(file_path: str) -> bool:
    """Check if a Python file is under tools/transpiler/."""
    normalized = file_path.replace("\\", "/")
    return "tools/transpiler" in normalized
```

- [ ] **Step 3: Sweep for any remaining callers of `_is_transpiler_path` or `tools/transpiler` strings**

Run: `grep -n "transpiler" digimon_gym/ai/retrieval.py`

Expected: zero matches. If any caller of `_is_transpiler_path` remains, replace each call with `False` (the function always returned False once the directory is gone) and remove resulting dead code.

- [ ] **Step 4: Verify the file compiles**

Run: `python -m py_compile digimon_gym/ai/retrieval.py`

Expected: silent (success).

- [ ] **Step 5: Commit**

```bash
git add digimon_gym/ai/retrieval.py
git commit -m "$(cat <<'EOF'
refactor(ai): drop transpiler classification from retrieval

The transpiler source-type and _is_transpiler_path helper are removed.
Engine and rules classifications remain.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Delete transpiler-specific tests

**Files:**
- Delete: `tests/ai_pipeline/test_transpiler_scoring.py`
- Delete: `tests/ai_pipeline/test_retranspile_integration.py`
- Delete: `tests/ai_pipeline/test_set_run_retranspile.py`
- Delete: `tests/ai_pipeline/test_prompts_llm_transpile.py`

- [ ] **Step 1: Verify the four files are transpiler-specific**

Run: `head -20 tests/ai_pipeline/test_transpiler_scoring.py tests/ai_pipeline/test_retranspile_integration.py tests/ai_pipeline/test_set_run_retranspile.py tests/ai_pipeline/test_prompts_llm_transpile.py`

Expected: each file's docstring or import block clearly identifies it as transpiler-coupled (imports from `tools.transpiler.*`, references retranspile/score_card/etc.). If any file does not look transpiler-specific, stop and inspect.

- [ ] **Step 2: Delete**

```bash
git rm tests/ai_pipeline/test_transpiler_scoring.py
git rm tests/ai_pipeline/test_retranspile_integration.py
git rm tests/ai_pipeline/test_set_run_retranspile.py
git rm tests/ai_pipeline/test_prompts_llm_transpile.py
```

- [ ] **Step 3: Commit**

```bash
git commit -m "$(cat <<'EOF'
test: delete transpiler-coupled ai_pipeline tests

Tests for tools/transpiler/ and the retranspile flow inside the
admin AI pipeline. Both surfaces are removed.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Update remaining ai_pipeline tests

**Files:**
- Modify: `tests/ai_pipeline/test_ai_autofix_apply.py`
- Modify: `tests/ai_pipeline/test_retrieval.py`
- Modify: `tests/ai_pipeline/test_contracts.py`

These tests survive but may contain assertions about the deleted transpiler behavior.

- [ ] **Step 1: Find references to drop**

Run: `grep -n "transpiler\|TRANSPILER\|SCOPE_SCRIPT_ENGINE_TRANSPILER\|script_engine_transpiler" tests/ai_pipeline/test_ai_autofix_apply.py tests/ai_pipeline/test_retrieval.py tests/ai_pipeline/test_contracts.py`

For each match, decide:
  - If the match is an entire test function whose subject is the transpiler scope or transpiler source-type, **delete the function**.
  - If the match is a single assertion or parametrize entry inside an otherwise valid test, **delete that line / parametrize entry**.
  - If the match is an import (e.g., `from digimon_gym.ai.autofix_apply import SCOPE_SCRIPT_ENGINE_TRANSPILER`), **delete the import**.

- [ ] **Step 2: Run the three test files**

Run: `python -m pytest tests/ai_pipeline/test_ai_autofix_apply.py tests/ai_pipeline/test_retrieval.py tests/ai_pipeline/test_contracts.py -v`

Expected: pass (or pass-rate matches the baseline captured in Task 1, Step 1). Any new failure caused by this PR must be fixed before continuing.

- [ ] **Step 3: Commit**

```bash
git add tests/ai_pipeline/test_ai_autofix_apply.py tests/ai_pipeline/test_retrieval.py tests/ai_pipeline/test_contracts.py
git commit -m "$(cat <<'EOF'
test: drop transpiler assertions from kept ai_pipeline tests

Removes references to SCOPE_SCRIPT_ENGINE_TRANSPILER, transpiler
source_type, and tools/transpiler paths from the surviving
ai_pipeline test files.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: Delete `.claude/skills/implement-set/`

**Files:**
- Delete: `.claude/skills/implement-set/` (entire directory — `SKILL.md`, `transpiler-fixes.md`, `review-checklist.md`, any other files)

The skill exists solely to drive the C# → Python transpile + review pipeline.

- [ ] **Step 1: Confirm scope**

Run: `ls .claude/skills/implement-set/`

Expected: `SKILL.md`, `transpiler-fixes.md`, `review-checklist.md` (and possibly more).

- [ ] **Step 2: Delete**

Run: `git rm -r .claude/skills/implement-set/`

- [ ] **Step 3: Confirm no other skill references it**

Run: `grep -rn "implement-set" .claude/ docs/ CLAUDE.md AGENTS.md`

Each remaining match: if it's a doc cross-reference that promises the skill exists, update or remove that reference in the relevant doc edit task below.

- [ ] **Step 4: Commit**

```bash
git commit -m "$(cat <<'EOF'
chore: delete .claude/skills/implement-set/

Skill drove the C#->Python transpile + AI review pipeline.
Card scripts now live in Rust; replaced operationally by
batch-implement-cards-rust and assess-archetype-rust.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 11: Update `docs/TOOLS.md`

**Files:**
- Modify: `docs/TOOLS.md`

- [ ] **Step 1: Find the transpiler section**

Run: `grep -n "transpiler\|Transpiler\|TRANSPILER" docs/TOOLS.md`

There should be a heading-level section about the C# → Python transpiler and the `tools/transpiler/cli.py` invocation.

- [ ] **Step 2: Delete the section**

Open `docs/TOOLS.md`. Locate the transpiler section (likely a `## ` heading). Delete the entire section, including its heading, body, code blocks, and any inline references to `tools/transpiler/` elsewhere in the document.

- [ ] **Step 3: Verify zero remaining matches**

Run: `grep -n "transpiler\|Transpiler\|tools/transpiler" docs/TOOLS.md`

Expected: zero matches.

- [ ] **Step 4: Commit**

```bash
git add docs/TOOLS.md
git commit -m "$(cat <<'EOF'
docs(tools): drop C#->Python transpiler section

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 12: Update `docs/ARCHITECTURE.md`

**Files:**
- Modify: `docs/ARCHITECTURE.md`

- [ ] **Step 1: Find references**

Run: `grep -n "transpiler\|Transpiler\|tools/transpiler" docs/ARCHITECTURE.md`

- [ ] **Step 2: Delete each match in context**

For each match: read 5 lines of surrounding context. If the reference is part of a section describing the AI pipeline workflow, rewrite the surrounding text to describe the pipeline as "currently dormant; retained for future repurposing toward user error reports against Rust scripts." If the reference is part of a tools list or architecture diagram bullet, remove the bullet.

- [ ] **Step 3: Verify zero remaining matches**

Run: `grep -n "transpiler\|Transpiler\|tools/transpiler" docs/ARCHITECTURE.md`

Expected: zero matches.

- [ ] **Step 4: Commit**

```bash
git add docs/ARCHITECTURE.md
git commit -m "$(cat <<'EOF'
docs(architecture): drop transpiler references

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 13: Update the spec doc

**Files:**
- Modify: `docs/superpowers/specs/2026-04-25-server-digimon-gym-split-design.md`

The spec describes Phase 1 narrowly as "delete `tools/transpiler/` and skill bits." Reality (this plan) is broader: it also strips the transpiler call sites inside `digimon_gym/ai/` while keeping the AI pipeline scaffolding. Reflect that.

- [ ] **Step 1: Update Phase 1 section**

In `docs/superpowers/specs/2026-04-25-server-digimon-gym-split-design.md`, find the "### Phase 1 — Delete the transpiler" section. Replace it with:

```markdown
### Phase 1 — Delete the transpiler

- Remove `tools/transpiler/` and `tools/audit_transpiled_sets.py`.
- Remove `digimon_gym/ai/transpiler_audit.py`.
- Strip transpiler call sites and the `script_engine_transpiler` scope from `digimon_gym/ai/{set_run_orchestrator,autofix_apply,retrieval}.py`. The AI pipeline scaffolding (`worker`, `dispatcher`, `client`, `prompts`, `git_adapter`, `issue_resolution`, `pattern_learner`, `batch_orchestrator`, `contracts`, plus `AISetRun*` DB models and the `admin_ai` router) is **kept** for future repurposing toward user error reports against Rust scripts. The pipeline may be non-functional in alpha — accepted.
- Delete transpiler-coupled tests in `tests/ai_pipeline/` (`test_transpiler_scoring.py`, `test_retranspile_integration.py`, `test_set_run_retranspile.py`, `test_prompts_llm_transpile.py`). Strip transpiler assertions from the kept `tests/ai_pipeline/` files.
- Delete the `.claude/skills/implement-set/` skill.
- Update `docs/TOOLS.md` and `docs/ARCHITECTURE.md` to drop the transpiler section.

**Standalone, low-risk. No callers in production code outside the dormant AI pipeline.**
```

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/specs/2026-04-25-server-digimon-gym-split-design.md
git commit -m "$(cat <<'EOF'
docs(spec): expand Phase 1 to cover ai-pipeline transpiler call sites

The pipeline scaffolding survives; only the transpiler entry points
are stripped. Pipeline is dormant in alpha and will be repurposed
later.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 14: End-to-end verification

**Files:** none (verification only)

- [ ] **Step 1: Repo-wide grep for stragglers**

Run: `grep -rn "tools/transpiler\|tools\.transpiler\|from tools\.transpiler\|import tools\.transpiler\|SCOPE_SCRIPT_ENGINE_TRANSPILER\|script_engine_transpiler" --include="*.py" --include="*.md" --include="*.json" --include="*.yaml" --include="*.toml" .`

Expected: zero matches outside (a) `docs/superpowers/specs/2026-04-25-server-digimon-gym-split-design.md` (where it appears in the historical context of Phase 1), (b) `docs/superpowers/plans/2026-04-25-server-split-phase-1-delete-transpiler.md` (this file), (c) `docs/plans/2026-02-25-llm-transpiler-and-pattern-learner-*.md` (historical plan docs — leave them, they record history). Any other match must be addressed.

- [ ] **Step 2: Default pytest run**

Run: `python -m pytest tests -v 2>&1 | tail -30`

Expected: all collected tests pass. (Default collection still excludes `tests/ai_pipeline/` per existing config.)

- [ ] **Step 3: Explicit ai_pipeline pytest run**

Run: `python -m pytest tests/ai_pipeline -v 2>&1 | tail -40`

Expected: pass-rate at least matches the Task 1 Step 1 baseline. New failures caused by this PR must be fixed.

- [ ] **Step 4: Server boot smoke**

Run (in one terminal): `python -m uvicorn digimon_gym.api:app --host 127.0.0.1 --port 8765 &`

Wait ~3 seconds.

Run: `curl -s http://127.0.0.1:8765/health`

Expected: 200 OK with health JSON. Then kill the uvicorn process.

This confirms the lifespan startup (which wires `ai_task_worker.start()`) doesn't crash on the now-thinned AI pipeline.

- [ ] **Step 5: Final sanity check on git state**

Run: `git status` and `git log --oneline origin/main..HEAD`

Expected: clean working tree; one commit per task above (12 commits in the series — Tasks 2–13). Task 1 and Task 14 are verification-only (no commits).

---

## Out-of-Scope

Explicitly **not** done in this phase:

- Deleting the AI pipeline scaffolding (`worker`, `dispatcher`, `client`, `prompts`, `git_adapter`, `issue_resolution`, `pattern_learner`, `batch_orchestrator`, `contracts`, `AISetRun*` DB models, `admin_ai` router). Per user direction, kept for repurposing.
- Touching `digimon_gym.engine.*` imports. That's Phase 3.
- Touching `tests/ai_pipeline/conftest.py` unless a transpiler reference appears (verify with grep in Task 9).
- Repurposing the `meta_loader` test (`test_meta_loader.py`) — it's not transpiler-specific; leave it.
- The deletion-trigger / restoration policy for the dormant AI pipeline. Tracked separately when the repurposing work begins.

---

## Plan complete

After Task 14 ships green: open the PR for Phase 1 and request review. Phase 2 (PyO3 binding expansion) gets its own plan when Phase 1 is merged.
