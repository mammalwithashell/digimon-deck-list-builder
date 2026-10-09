"""`stages.base` helpers (openspec add-card-authoring-loop)."""
from __future__ import annotations

import contextlib
import subprocess
from types import SimpleNamespace

from tools.card_loop.stages import base
from tools.card_loop.stages.testing import make_ctx


def _git(cwd, *args):
    return subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t", *args], cwd=str(cwd),
                          capture_output=True, text=True, check=True).stdout.strip()


class RecordingPool:
    def __init__(self, wt):
        self.wt = wt
        self.retargets: list[str] = []

    def retarget(self, sha):
        self.retargets.append(sha)

    @contextlib.contextmanager
    def lease(self, timeout=None):
        yield SimpleNamespace(path=self.wt)


def test_a_leased_worktree_is_retargeted_to_the_run_trees_head(tmp_path):
    # Workers start from the run branch's HEAD, not the plan's base: a
    # re-authored scenario diffed against the old base conflicts on merge.
    repo = tmp_path / "repo"
    repo.mkdir()
    _git(repo, "init", "-q")
    (repo / "a.txt").write_text("a\n", encoding="utf-8")
    _git(repo, "add", "a.txt")
    _git(repo, "commit", "-q", "-m", "one")
    head = _git(repo, "rev-parse", "HEAD")
    pool = RecordingPool(str(tmp_path / "wt"))
    ctx = make_ctx(repo, pool=pool)
    with base.worktree(ctx) as wt:
        assert wt == str(tmp_path / "wt")
    assert pool.retargets == [head]


def test_without_a_pool_the_run_tree_itself_is_used(tmp_path):
    ctx = make_ctx(tmp_path, pool=None)
    with base.worktree(ctx) as wt:
        assert wt == str(tmp_path)
