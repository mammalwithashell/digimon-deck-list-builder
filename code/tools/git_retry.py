"""Retry git calls that lose the race for `index.lock`, and sweep a lock a
killed process left behind.

The card loop runs several git writers against one run tree at once -- the
merger applying a worker diff, the driver committing tree writes, and the
worker CLIs themselves (`claude -p` refreshes the index with `git status`
when it starts in that tree). Git takes `index.lock` for a few milliseconds
per operation and fails immediately with "Unable to create '.../index.lock':
File exists" when another holder has it; it never waits. A driver killed
mid-operation leaves the lock file behind for good, and every later call
fails the same way (the second pilot lost nine items to one such lock).

Shared by `tools.card_loop` and `tools.author_set.merge_wave`, so it imports
neither.
"""
from __future__ import annotations

import os
import time
from pathlib import Path
from typing import Callable, TypeVar

T = TypeVar("T")

LOCK_MARKER = "index.lock"
RETRY_ATTEMPTS = 20
RETRY_DELAY_S = 0.5
#: A live git operation holds the index lock for well under this; a lock this
#: old belongs to a process that is gone.
STALE_LOCK_S = 300.0


def is_index_lock_error(stderr: str | bytes | None) -> bool:
    text = stderr.decode("utf-8", "replace") if isinstance(stderr, bytes) else (stderr or "")
    return LOCK_MARKER in text and ("File exists" in text or "Another git process" in text)


def retry_on_index_lock(run: Callable[[], T], stderr_of: Callable[[T], str | bytes | None], *,
                        attempts: int = RETRY_ATTEMPTS, delay: float = RETRY_DELAY_S,
                        sleep: Callable[[float], None] = time.sleep) -> T:
    """Call `run()` until its result's stderr is not an index-lock collision,
    at most `attempts` times with `delay` between them. The last result is
    returned either way: a lock held for `attempts * delay` is somebody
    else's problem (see `sweep_stale_index_lock`), not a reason to spin."""
    result = run()
    for _ in range(max(0, attempts - 1)):
        if not is_index_lock_error(stderr_of(result)):
            break
        sleep(delay)
        result = run()
    return result


def index_lock_path(repo: str | os.PathLike, git_path: Callable[[str], str] | None = None) -> Path | None:
    """`<gitdir>/index.lock` for `repo` (a worktree's own gitdir, not the
    common one), via `git rev-parse --git-path`, or None when git cannot say."""
    import subprocess

    if git_path is None:
        def git_path(name: str) -> str:
            cp = subprocess.run(["git", "rev-parse", "--git-path", name], cwd=str(repo), capture_output=True,
                                text=True, encoding="utf-8", errors="replace")
            return cp.stdout.strip() if cp.returncode == 0 else ""
    rel = git_path("index.lock")
    if not rel:
        return None
    p = Path(rel)
    return p if p.is_absolute() else Path(repo) / p


def sweep_stale_index_lock(repo: str | os.PathLike, *, max_age_s: float = STALE_LOCK_S,
                           now: Callable[[], float] = time.time, lock_path: Path | None = None) -> Path | None:
    """Remove `index.lock` when it is older than `max_age_s` (a holder that
    died); return the removed path, else None. A fresh lock is left alone: it
    may be a live operation."""
    path = lock_path if lock_path is not None else index_lock_path(repo)
    if path is None or not path.exists():
        return None
    try:
        age = now() - path.stat().st_mtime
    except OSError:
        return None
    if age < max_age_s:
        return None
    try:
        path.unlink()
    except OSError:
        return None
    return path
