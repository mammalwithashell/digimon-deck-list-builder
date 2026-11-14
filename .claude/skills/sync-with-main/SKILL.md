---
name: sync-with-main
description: Pull the latest origin/main into the current branch (merge, not rebase), stash dirty changes around the merge, propose resolutions for merge conflicts (auto-handling lockfiles and additive JSON merges, asking on real semantic conflicts), and verify with the project's Rust + Python test suites. Use whenever the user wants to "pull main", "sync with main", "merge main in", "update from main", "bring in main", "rebase on main" (will use merge, not rebase, per project preference), or asks how to resolve conflicts after a partial merge with main. Also use proactively when the user starts work in a long-lived feature branch / worktree and you notice main has moved on significantly.
---

# Sync With Main

Bring the current branch up to date with `origin/main` by merging, resolving conflicts, and verifying the result with the project's test suites. The goal is to land you on a clean, tested merge commit — not to push, not to PR, not to clean up.

This skill is for *integrating* main into a feature branch, not for graduating a feature branch (use `superpowers:finishing-a-development-branch` for that).

## Why merge (and not rebase)

Project preference: merge main into the branch, preserve branch history, create a merge commit. Don't rebase — rewriting history on shared branches has caused painful re-pushes before. If the user explicitly asks to rebase, stop and confirm before doing it.

## Prerequisites — verify before doing anything

Run these checks first. Each one can short-circuit the workflow.

```bash
git status --porcelain=v1 --branch
git fetch origin main
git rev-list --left-right --count HEAD...origin/main
```

Interpret the output:

- `--branch` line tells you the branch name and any tracking divergence
- `--porcelain` lines tell you whether the tree is dirty (any output means dirty)
- `rev-list` output is `<commits-ahead>\t<commits-behind>`. If behind = 0, you're already up to date — tell the user and stop.

If you're in a git worktree (likely — this repo uses worktrees heavily under `.claude/worktrees/`), that's fine. The merge happens in the worktree on the worktree's branch. Mention which worktree/branch you're operating on so the user can confirm.

**Do not proceed** if:
- HEAD is detached (worktree on a tag/commit) — ask the user to check out a branch first
- The branch has no upstream and the user hasn't told you which branch to merge into — confirm
- There's an in-progress merge, rebase, or cherry-pick already (`.git/MERGE_HEAD`, `.git/REBASE_HEAD`, `.git/CHERRY_PICK_HEAD` exists) — ask the user how to recover; never `--abort` unilaterally

## Step 1: Handle a dirty working tree

If `git status --porcelain` returned anything, stash before merging. Always include untracked files so a brand-new file doesn't disappear under a merge:

```bash
git stash push --include-untracked -m "sync-with-main: pre-merge snapshot"
```

Record that you stashed — you'll need to pop it at the end. If the stash fails (e.g., submodule weirdness), stop and surface the error rather than barrelling on.

## Step 2: Preview what's coming

Before merging, show the user what's about to land. This catches "wait, I didn't realize main changed *that*" before any conflicts force the issue.

```bash
git log --oneline --no-merges HEAD..origin/main
git diff --stat HEAD...origin/main
```

A one-line summary is enough: "X commits, Y files changed, biggest churn in <area>." If the diff touches files the user has uncommitted edits in (now stashed), call that out — those are the most likely conflict sites.

## Step 3: Merge

```bash
git merge --no-ff origin/main -m "Merge origin/main into <branch-name>"
```

Use `--no-ff` so the merge commit always exists; this makes it obvious in history that a sync happened. The `-m` is required because hooks/editors aren't reliably available in non-interactive shells.

**If the merge succeeds cleanly**, skip to Step 5.

**If git reports conflicts**, continue to Step 4.

## Step 4: Resolve conflicts

```bash
git status
git diff --name-only --diff-filter=U
```

The `--diff-filter=U` list is your conflict file set. Categorize each file before touching it — different categories want different treatment.

### Category A: Auto-resolvable (do these first, confirm aggregate after)

Resolve these without asking, then summarize what you did at the end.

| Pattern | Resolution |
|---------|------------|
| `Cargo.lock` | Take `origin/main`'s version (`git checkout --theirs Cargo.lock`), then run `cargo check --manifest-path code/digimon-engine/Cargo.toml` to regenerate lockfile entries for new deps the branch added. |
| `package-lock.json`, `frontend/package-lock.json` | Take `origin/main`'s version, then run `npm install` in the relevant directory to reconcile. |
| `validated_cards.json`, `validated_cards_dsl.json`, `validated_cards_rust.json` | These are append-only progress trackers. Union the entries from both sides — keep every card both branches validated. If the same card appears on both sides with different verdicts, that's not auto-resolvable; bump it to Category C. |
| `qa/archetype-qa/engine-gaps.md` | Append-style log. Concatenate sections from both sides; de-duplicate identical entries. |

For each of these, after writing the resolved file, `git add <file>` so it leaves the conflict set.

### Category B: Both-added-imports / both-added-list-entries

Symmetric additions like both branches adding an import line, both adding a new entry to a registry array, both adding a new module to a `mod.rs` — these are conflicts only because the same region was edited, not because the intent disagrees. Take the union: keep both additions, sorted/ordered however the surrounding code is ordered.

Show the user the resolved file region (3-5 lines of context) and confirm before `git add`ing. Cheap to confirm, expensive to silently merge the wrong thing.

### Category C: Real semantic conflicts — ask the user

Anything that isn't Category A or B: present it to the user with full context before resolving.

For each conflict file, gather:

```bash
git log --oneline HEAD..origin/main -- <file>     # what main changed
git log --oneline origin/main..HEAD -- <file>     # what the branch changed
git diff origin/main...HEAD -- <file>             # the branch's changes
```

Then for each `<<<<<<< / ======= / >>>>>>>` block in the file, show the user:

1. The conflict block itself (both sides)
2. One or two sentences of why each side wrote what it did (from the commit log above)
3. Your proposed resolution and the reasoning

Wait for the user's call before writing. If the user wants to handle a file themselves, mark it as deferred — don't `git add` it, finish the others, and come back.

### Project-specific traps

- **`data/cards.json`** — this is regenerated from upstream API ingest. Conflicts usually mean both branches re-ingested. Take `origin/main`'s version unless the branch has hand-edits in `data/card_overrides.json` that imply intentional cards.json changes.
- **`data/card_overrides.json`** — hand-maintained corrections; merge entries by `card_id`. If both sides modified the same card_id, escalate to Category C.
- **`docs/RUST_PYTHON_PARITY.md`** — transitional cross-engine tracker. Per-section append; if both sides edited the same section, ask.
- **Python card scripts under `code/engine_py_legacy/`** — per CLAUDE.md rule 21, cards migrate one direction (Python → Rust). If a conflict adds a *new* Python script that already exists in Rust, take `origin/main`'s version (likely the deletion) and flag for the user.
- **Tensor or action spec files** (`code/digimon-engine/src/tensor.rs`, `code/digimon-engine/src/action/`) — never auto-resolve. RL action/observation shape is load-bearing for trained models. Always Category C.
- **`alembic/versions/`** — if both branches added migrations, they're likely linked to a parent revision that's now wrong. Don't auto-resolve; the user needs to renumber.

### Finalize the merge

After every conflict file is `git add`ed or deferred:

```bash
git status   # confirm no remaining unmerged paths
git diff --cached --stat   # one last sanity check on what's about to be committed
git commit --no-edit       # uses the merge message git pre-staged
```

If files are deferred, don't commit. Tell the user what's left and stop.

## Step 5: Pop the stash (if you stashed)

```bash
git stash pop
```

A pop can itself conflict — your stashed edits may overlap with what main just brought in. If it does, walk the user through the same Category A/B/C triage as Step 4, but for stashed-vs-merged conflicts. After resolution, don't auto-commit; the stash contents are working changes the user hasn't decided to commit yet.

If pop succeeds cleanly, `git stash drop` is not necessary — `pop` already drops on success.

## Step 6: Verify

Run the project's test suites. The CLAUDE.md commands:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml
python -m pytest -v
```

These take a while. Start them, watch for failures. If the engine tests fail, that's almost certainly a semantic-merge issue (git merged cleanly but the logic now contradicts). Surface the failing tests; offer to investigate before declaring done.

Skip verification only if the user explicitly tells you to (e.g., they're going to commit a WIP and run tests later).

## Step 7: Report

End-of-turn summary the user should see:

- Branch synced and merge commit SHA
- Number of conflicts resolved, categorized (auto / confirmed / deferred)
- Stash state (popped clean / popped with conflicts / no stash)
- Test results (pass / fail / not run, with reason)
- Anything still requiring user action

Do **not** push. Do **not** open a PR. The user owns those decisions; this skill stops at "your branch is synced, tested, and the working tree reflects your pre-sync edits."

## Recovery if something goes wrong mid-merge

If you get confused or the user wants to back out before the merge commit:

```bash
git merge --abort     # only if the merge hasn't been committed yet
git stash pop         # restore the pre-sync state, if you stashed
```

`git merge --abort` returns to pre-merge HEAD. Don't use `git reset --hard` as a shortcut — it can destroy stashed-but-not-popped work and uncommitted state you may have forgotten about. If the merge is already committed and the user wants out, suggest `git reset --merge HEAD~1` and ask before running.
