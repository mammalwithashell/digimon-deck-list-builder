# Fix Pipeline: Target Frozen-Lane Scripts — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Switch the autofix apply pipeline from targeting `scripts/generated/` (untracked, missing in worktrees) to `scripts/{set_id}/` (frozen lane, in git), so fix tasks can actually apply edits.

**Architecture:** Four path references in `autofix_apply.py` and one in `dispatcher.py` currently hardcode the `generated/` segment. We remove that segment so all fix-related reads, writes, and validations target frozen-lane scripts. We also add a worktree health check and clean up 115+ stale worktree directories.

**Tech Stack:** Python, git worktrees, pytest

---

### Task 1: Clean up stale worktrees

**Files:**
- None (shell commands only)

**Step 1: Remove all stale worktree directories**

```bash
rm -rf data/run/ai_worktrees/
git worktree prune
```

**Step 2: Verify no worktrees remain**

```bash
git worktree list
```

Expected: Only the main worktree listed.

---

### Task 2: Switch `autofix_apply.py` paths from generated to frozen lane

**Files:**
- Modify: `digimon_gym/ai/autofix_apply.py:122` (`_is_allowed_for_scope`)
- Modify: `digimon_gym/ai/autofix_apply.py:136` (`get_allowed_roots_for_scope`)
- Modify: `digimon_gym/ai/autofix_apply.py:145` (`get_primary_script_path`)
- Modify: `digimon_gym/ai/autofix_apply.py:299` (`derive_targeted_tests`)

**Step 1: Update `_is_allowed_for_scope` primary path (line 122)**

Change:
```python
primary = f"digimon_gym/engine/data/scripts/generated/{set_id}/{module_name}.py"
```
To:
```python
primary = f"digimon_gym/engine/data/scripts/{set_id}/{module_name}.py"
```

**Step 2: Update `get_allowed_roots_for_scope` (line 136)**

Change:
```python
roots = ["digimon_gym/engine/data/scripts/generated/"]
```
To:
```python
roots = ["digimon_gym/engine/data/scripts/"]
```

**Step 3: Update `get_primary_script_path` (line 145)**

Change:
```python
return f"digimon_gym/engine/data/scripts/generated/{set_id}/{module_name}.py"
```
To:
```python
return f"digimon_gym/engine/data/scripts/{set_id}/{module_name}.py"
```

**Step 4: Update `derive_targeted_tests` (line 299)**

Change:
```python
if rel.startswith("digimon_gym/engine/data/scripts/generated/"):
    parts = rel.split("/")
    if len(parts) >= 6:
        touched_sets.add(parts[4].lower())
```
To:
```python
if rel.startswith("digimon_gym/engine/data/scripts/"):
    parts = rel.split("/")
    if len(parts) >= 6:
        touched_sets.add(parts[5].lower())
```

Note: The parts index changes from `4` to `5` because without `generated/`, the set_id is at index 5 in the full path `digimon_gym/engine/data/scripts/{set_id}/{module}.py`.

Wait — let's count: `digimon_gym(0)/engine(1)/data(2)/scripts(3)/{set_id}(4)/{module}(5)`. Actually the index stays as `4` but the prefix is shorter now. However, `scripts/` also matches `scripts/_frozen_manifest.json` and other non-set paths. We need to be more specific OR keep the index logic correct.

Actually the old path was: `digimon_gym/engine/data/scripts/generated/{set_id}/{module}.py` — split gives `[digimon_gym, engine, data, scripts, generated, {set_id}, {module}.py]` — `parts[5]` was the set_id (index 5), but the code used `parts[4]` which was `generated`. That looks like a pre-existing bug. Let me re-check.

Split of `digimon_gym/engine/data/scripts/generated/bt14/bt14_003.py`:
- parts[0] = `digimon_gym`
- parts[1] = `engine`
- parts[2] = `data`
- parts[3] = `scripts`
- parts[4] = `generated`
- parts[5] = `bt14`
- parts[6] = `bt14_003.py`

The old code: `if len(parts) >= 6: touched_sets.add(parts[4].lower())` — `parts[4]` is `generated`, not the set_id! This is indeed a pre-existing bug — it would always add `"generated"` to `touched_sets`, which would look for `tests/test_generated_scripts.py` (doesn't exist). The tests still ran because `has_engine_change` caught them via the broader prefix.

New path `digimon_gym/engine/data/scripts/bt14/bt14_003.py`:
- parts[0] = `digimon_gym`
- parts[1] = `engine`
- parts[2] = `data`
- parts[3] = `scripts`
- parts[4] = `bt14`
- parts[5] = `bt14_003.py`

So `parts[4]` is now correctly `bt14`. The condition `len(parts) >= 6` stays correct (6 parts). The index `parts[4]` is now the set_id. No index change needed!

But we need to avoid matching non-script paths under `scripts/` (like `scripts/_frozen_manifest.json`). The `len(parts) >= 6` check handles this — `_frozen_manifest.json` splits to only 5 parts.

Corrected Step 4:

Change:
```python
if rel.startswith("digimon_gym/engine/data/scripts/generated/"):
```
To:
```python
if rel.startswith("digimon_gym/engine/data/scripts/") and not rel.startswith("digimon_gym/engine/data/scripts/_"):
```

The `parts[4]` index and `len(parts) >= 6` check remain the same. The `_` guard excludes `_frozen_manifest.json` and `__init__.py`.

**Step 5: Run tests**

Run: `python -m pytest tests/test_ai_pipeline.py -v --tb=short`
Expected: Some tests fail due to `scripts/generated/` paths in test fixtures (fixed in Task 4).

---

### Task 3: Switch `dispatcher.py` script reader from generated to frozen lane

**Files:**
- Modify: `digimon_gym/ai/dispatcher.py:69-73` (`_read_generated_script`)

**Step 1: Rename function and update path**

Change:
```python
def _read_generated_script(set_id: str, module_name: str) -> str:
    path = SCRIPTS_ROOT / "generated" / set_id / f"{module_name}.py"
    if not path.exists():
        return ""
    return path.read_text(encoding="utf-8")
```
To:
```python
def _read_script(set_id: str, module_name: str) -> str:
    """Read script from frozen lane, falling back to generated."""
    path = SCRIPTS_ROOT / set_id / f"{module_name}.py"
    if not path.exists():
        # Fall back to generated lane for sets not yet promoted
        path = SCRIPTS_ROOT / "generated" / set_id / f"{module_name}.py"
        if not path.exists():
            return ""
    return path.read_text(encoding="utf-8")
```

**Step 2: Update all call sites in dispatcher.py**

Replace `_read_generated_script` → `_read_script` at lines 235 and 383.

---

### Task 4: Add worktree health check to `git_adapter.py`

**Files:**
- Modify: `digimon_gym/ai/git_adapter.py:94-129` (`prepare_worktree_for_task`)

**Step 1: Add health check before worktree reuse**

Replace the existing `prepare_worktree_for_task` method with:

```python
def prepare_worktree_for_task(
    self, *, task_id: str, run_mode: str
) -> WorktreeContext:
    wt = self.worktree_path_for_task(task_id=task_id)
    wt.parent.mkdir(parents=True, exist_ok=True)
    branch = self.branch_name_for_task(task_id=task_id)

    if wt.exists() and (wt / ".git").exists():
        # Health check: verify the worktree is functional
        try:
            _run(["git", "rev-parse", "--git-dir"], cwd=wt)
        except GitCommandError:
            # Corrupted worktree — remove and recreate
            shutil.rmtree(wt, ignore_errors=True)
            _run(["git", "worktree", "prune"], cwd=self.repo_root)
        else:
            # Worktree is healthy — reset to latest origin/main
            target_branch = branch if run_mode == "pr" else DEFAULT_BRANCH
            _run(["git", "fetch", "origin", DEFAULT_BRANCH], cwd=self.repo_root)
            try:
                _run(["git", "checkout", "-B", target_branch, f"origin/{DEFAULT_BRANCH}"], cwd=wt)
            except GitCommandError:
                # Branch checkout failed, nuke and recreate
                shutil.rmtree(wt, ignore_errors=True)
                _run(["git", "worktree", "prune"], cwd=self.repo_root)
            else:
                self.restore_worktree_to_head(worktree_path=wt)
                return WorktreeContext(
                    worktree_path=wt,
                    branch_name=target_branch,
                )

    # Create fresh worktree
    _run(["git", "fetch", "origin", DEFAULT_BRANCH], cwd=self.repo_root)
    if run_mode == "pr":
        _run(
            ["git", "worktree", "add", "-B", branch, str(wt), f"origin/{DEFAULT_BRANCH}"],
            cwd=self.repo_root,
        )
        return WorktreeContext(worktree_path=wt, branch_name=branch)

    _run(
        ["git", "worktree", "add", str(wt), f"origin/{DEFAULT_BRANCH}"],
        cwd=self.repo_root,
    )
    return WorktreeContext(worktree_path=wt, branch_name=DEFAULT_BRANCH)
```

Key improvements:
- Validates worktree health with `git rev-parse --git-dir` before reuse
- Always resets reused worktrees to `origin/main` (not stale commit)
- Falls through to fresh creation if health check or reset fails

---

### Task 5: Update test fixtures with frozen-lane paths

**Files:**
- Modify: `tests/test_ai_pipeline.py`

**Step 1: Update all `scripts/generated/` references in test fixtures**

Replace all occurrences of `digimon_gym/engine/data/scripts/generated/` with `digimon_gym/engine/data/scripts/` in test fixture data (edit paths, expected paths, monkeypatched return values).

Specific locations (line numbers approximate):
- Line 322: `"path": "digimon_gym/engine/data/scripts/generated/bt13/bt13_006.py"` → `"path": "digimon_gym/engine/data/scripts/bt13/bt13_006.py"`
- Line 341: `path="digimon_gym/engine/data/scripts/generated/bt13/bt13_006.py"` → same without `generated/`
- Line 408: same pattern
- Line 428: `lambda **_kwargs: ["digimon_gym/engine/data/scripts/generated/bt13/bt13_006.py"]` → same without `generated/`
- Line 475: same pattern
- Line 1220: `"digimon_gym/engine/data/scripts/generated/bt24/bt24_099.py"` → `"digimon_gym/engine/data/scripts/bt24/bt24_099.py"`
- Line 1448: `"digimon_gym/engine/data/scripts/generated/bt13/bt13_001.py"` → `"digimon_gym/engine/data/scripts/bt13/bt13_001.py"`

**Step 2: Run all tests**

Run: `python -m pytest tests/test_ai_pipeline.py -v --tb=short`
Expected: All tests pass (except the pre-existing CRLF hash mismatch in `test_promote_generated_script_updates_manifest`).

**Step 3: Commit**

```bash
git add digimon_gym/ai/autofix_apply.py digimon_gym/ai/dispatcher.py digimon_gym/ai/git_adapter.py tests/test_ai_pipeline.py
git commit -m "fix: switch autofix pipeline from generated to frozen-lane scripts

Generated scripts are untracked and missing in git worktrees, causing
every fix apply to fail with 'Cannot edit missing file'. Frozen-lane
scripts are committed and available in worktrees.

Also adds worktree health check to detect corrupted/stale worktrees
and reset them to origin/main before reuse."
```

---

### Task 6: Verify with live BT14 run

**Step 1: Re-run BT14 set run**

Create a new set run for BT14 with `max_fix_iterations=3` and verify:
- Fix tasks create worktrees successfully
- Edits target `scripts/bt14/*.py` (frozen lane)
- Edits apply without "Cannot edit missing file" errors
- Profile checks pass in the worktree
- Iterative loop works if fixes are applied

**Step 2: Check results**

Query the run items to verify `fix_apply_status` is `applied` for at least some items, not `failed` for all 85.
