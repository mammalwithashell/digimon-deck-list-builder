# Fix Pipeline: Target Frozen-Lane Scripts

## Problem

The autofix apply pipeline targets `scripts/generated/{set_id}/` scripts, but those
files are untracked (never committed to git). When a worktree checks out `origin/main`,
the generated scripts don't exist, causing every fix apply to fail with
`Cannot edit missing file`.

## Root Cause

`build_file_context_for_task()` and `get_primary_script_path()` in `autofix_apply.py`
hardcode the `generated/` path segment. The AI receives the generated script content
and returns edits targeting that path. The worktree-based apply then can't find the file.

## Solution

Switch the fix pipeline to target frozen-lane scripts (`scripts/{set_id}/{module}.py`)
instead of generated scripts. Frozen-lane scripts ARE in git and exist in worktrees.

### Changes

**`autofix_apply.py`** (4 locations):
- `_is_allowed_for_scope` L122: primary path → `scripts/{set_id}/{module}.py`
- `get_allowed_roots_for_scope` L136: root → `scripts/`
- `get_primary_script_path` L145: remove `generated/` segment
- `derive_targeted_tests` L299: update prefix match

**`dispatcher.py`**:
- `_read_generated_script`: read from frozen lane path

**`git_adapter.py`**:
- Add worktree health check in `prepare_worktree_for_task`
- Validate worktree is on a recent commit before reuse

**Cleanup**:
- Delete `data/run/ai_worktrees/` stale directories
- Run `git worktree prune`

**Tests**:
- Update assertions referencing `scripts/generated/` in fix-related tests

### Scope validation unchanged

`_is_allowed_for_scope` still restricts edits to the card's own script in `script` scope,
engine files in `script_engine` scope, etc. Only the base path changes.

## Verification

1. All existing tests pass
2. Create a test set run and verify fix tasks can apply edits in worktree
3. Stale worktrees cleaned up
