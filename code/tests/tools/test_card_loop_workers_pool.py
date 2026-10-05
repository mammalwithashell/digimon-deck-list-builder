"""WorktreePool against a temporary git repo created in tmp_path (never the real repo)."""
from __future__ import annotations

import os
import subprocess
import threading
from pathlib import Path

import pytest

from tools.card_loop.workers import pool as poolmod
from tools.card_loop.workers.pool import PoolError, PoolExhausted, WorktreePool, cargo_target_dir, worktree_env


def git(cwd, *args, check=True):
    return subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True, check=check).stdout


@pytest.fixture
def repo(tmp_path):
    r = tmp_path / "repo"
    r.mkdir()
    git(r, "init", "-q")
    git(r, "config", "user.email", "t@example.invalid")
    git(r, "config", "user.name", "t")
    git(r, "config", "core.autocrlf", "false")    # byte-exact checkouts regardless of global config
    (r / ".gitignore").write_text("node_modules/\nbuild-out/\n")
    (r / "a.txt").write_bytes(b"base\n")
    git(r, "add", "-A")
    git(r, "commit", "-qm", "base")
    base = git(r, "rev-parse", "HEAD").strip()
    (r / "a.txt").write_bytes(b"later\n")
    git(r, "commit", "-qam", "later")
    later = git(r, "rev-parse", "HEAD").strip()
    return r, base, later


def make_pool(repo, tmp_path, size=2, **kw):
    r, base, _ = repo
    return WorktreePool(r, base, tmp_path / "wt", size, str(tmp_path / "ct"),
                        sccache_dir=str(tmp_path / "sc"), **kw)


def registered(r) -> set[str]:
    out = git(r, "worktree", "list", "--porcelain")
    return {os.path.normcase(os.path.abspath(l[9:])) for l in out.splitlines() if l.startswith("worktree ")}


def test_acquire_creates_detached_worktree_at_base(repo, tmp_path):
    r, base, later = repo
    pool = make_pool(repo, tmp_path)
    try:
        wt = pool.acquire()
        assert wt.path.parent == tmp_path / "wt" and wt.name == "card-loop-0"
        assert git(wt.path, "rev-parse", "HEAD").strip() == base       # pinned base, not repo HEAD
        assert git(wt.path, "rev-parse", "--abbrev-ref", "HEAD").strip() == "HEAD"   # detached
        assert (wt.path / "a.txt").read_bytes() == b"base\n"
        assert wt.cargo_target_dir == os.path.join(str(tmp_path / "ct"), "card-loop-0")
        env = pool.env(wt, base_env={"CLAUDECODE": "1", "PATH": "p", "CARGO_TARGET_DIR": "shared"})
        assert env["CARGO_TARGET_DIR"] == wt.cargo_target_dir
        assert env["CARGO_TARGET_DIR_PINNED"] == "1"                   # ~/.bashrc must not re-derive it
        assert env["SCCACHE_DIR"] == str(tmp_path / "sc") and env["PATH"] == "p"
        assert "CLAUDECODE" not in env
    finally:
        pool.close()


def test_concurrent_workers_get_distinct_target_dirs_and_size_is_bounded(repo, tmp_path):
    pool = make_pool(repo, tmp_path, size=2)
    try:
        a, b = pool.acquire(), pool.acquire()
        assert a.path != b.path and a.cargo_target_dir != b.cargo_target_dir
        with pytest.raises(PoolExhausted):
            pool.acquire(timeout=0.05)
        got = []
        t = threading.Thread(target=lambda: got.append(pool.acquire(timeout=10)))
        t.start()
        pool.release(a)
        t.join(10)
        assert got and got[0].path == a.path                           # reused, not a third worktree
        assert len(pool.worktrees()) == 2
        with pytest.raises(PoolError):
            pool.release(poolmod.PooledWorktree(Path("x"), "nope", "t"))    # not from this pool
    finally:
        pool.close()


def test_reset_discards_everything_a_worker_did_except_keep(repo, tmp_path):
    r, base, later = repo
    pool = make_pool(repo, tmp_path, size=1, keep=("node_modules/",))
    try:
        wt = pool.acquire()
        p = wt.path
        (p / "a.txt").write_bytes(b"edited\n")
        (p / "new.txt").write_text("untracked")
        (p / "build-out").mkdir()
        (p / "build-out" / "x.o").write_text("ignored")
        (p / "node_modules").mkdir()
        (p / "node_modules" / "dep.js").write_text("cache")
        git(p, "checkout", "-q", "-b", "worker-branch")
        (p / "c.txt").write_text("c")
        git(p, "add", "c.txt")
        git(p, "-c", "user.email=t@example.invalid", "-c", "user.name=t", "commit", "-qm", "worker commit")
        pool.release(wt)

        again = pool.acquire()
        assert again.path == p
        assert git(p, "rev-parse", "HEAD").strip() == base
        assert git(p, "status", "--porcelain", "--untracked-files=all").strip() == ""
        assert (p / "a.txt").read_bytes() == b"base\n"
        assert not (p / "new.txt").exists() and not (p / "c.txt").exists()
        assert not (p / "build-out").exists()                          # ignored outputs go too
        assert (p / "node_modules" / "dep.js").exists()                # keep patterns survive
        assert git(r, "rev-parse", "worker-branch").strip() != base    # the branch was not rewound
    finally:
        pool.close()


def test_base_drift_is_reset_before_dispatch(repo, tmp_path):
    r, base, later = repo
    pool = make_pool(repo, tmp_path, size=1)
    try:
        wt = pool.acquire()
        git(wt.path, "checkout", "-q", "--detach", later)              # HEAD drifted off the base
        pool.release(wt)
        wt = pool.acquire()
        assert git(wt.path, "rev-parse", "HEAD").strip() == base
    finally:
        pool.close()


def test_broken_worktree_is_recreated(repo, tmp_path):
    r, base, _ = repo
    pool = make_pool(repo, tmp_path, size=1)
    try:
        wt = pool.acquire()
        # a killed worker left a stale index lock: git cannot reset in place
        Path(git(wt.path, "rev-parse", "--path-format=absolute", "--git-path", "index.lock").strip()).write_text("")
        (wt.path / "a.txt").write_bytes(b"dirty\n")
        pool.release(wt)
        removed = []
        real_remove = pool._remove
        pool._remove = lambda path: (removed.append(path), real_remove(path))
        wt = pool.acquire()
        assert removed == [wt.path]                                    # recreated, not reset in place
        assert git(wt.path, "rev-parse", "HEAD").strip() == base
        assert (wt.path / "a.txt").read_bytes() == b"base\n"
    finally:
        pool.close()


def test_close_removes_worktrees(repo, tmp_path):
    r, _, _ = repo
    pool = make_pool(repo, tmp_path, size=2)
    a, b = pool.acquire(), pool.acquire()
    pool.release(a)
    pool.close()
    assert not a.path.exists() and not b.path.exists()
    assert registered(r) == {os.path.normcase(os.path.abspath(r))}
    with pytest.raises(PoolError):
        pool.acquire()


def test_warm_worktrees_are_adopted_by_the_next_pool(repo, tmp_path):
    r, _, _ = repo
    first = make_pool(repo, tmp_path, size=2)
    a = first.acquire()
    (a.path / "leftover.txt").write_text("x")
    first.release(a)
    first.close(remove=False)                                          # leave it warm
    assert a.path.exists()

    second = make_pool(repo, tmp_path, size=2)
    try:
        assert [w.name for w in second.worktrees()] == ["card-loop-0"]
        b = second.acquire()
        assert b.path == a.path and not (b.path / "leftover.txt").exists()
    finally:
        second.close()


def test_stray_directory_in_the_root_is_replaced_only_if_it_is_a_member_name(repo, tmp_path):
    pool = make_pool(repo, tmp_path, size=1)
    try:
        (tmp_path / "wt" / "card-loop-0").mkdir(parents=True)
        (tmp_path / "wt" / "card-loop-0" / "junk").write_text("x")    # not a registered worktree
        wt = pool.acquire()
        assert (wt.path / "a.txt").exists() and not (wt.path / "junk").exists()
    finally:
        pool.close()


def test_rejects_bad_arguments(repo, tmp_path):
    r, _, _ = repo
    with pytest.raises(ValueError):
        WorktreePool(r, "HEAD", tmp_path / "wt", 0, "ct")
    with pytest.raises(ValueError):
        WorktreePool(r, "HEAD", tmp_path / "wt", 1, "ct", prefix="bad/prefix")
    with pytest.raises(PoolError):
        WorktreePool(r, "0" * 40, tmp_path / "wt", 1, "ct")


@pytest.mark.skipif(os.name != "nt", reason="MAX_PATH budget is a Windows constraint")
def test_root_too_deep_for_max_path_is_refused(tmp_path):
    r = tmp_path / "r"
    r.mkdir()
    git(r, "init", "-q")
    git(r, "config", "core.longpaths", "false")
    name = "n" * 120 + ".txt"
    (r / name).write_text("x")
    git(r, "add", "-A")
    git(r, "-c", "user.email=t@example.invalid", "-c", "user.name=t", "commit", "-qm", "base")
    deep = tmp_path / ("d" * 100)
    with pytest.raises(PoolError, match="MAX_PATH"):
        WorktreePool(r, "HEAD", deep, 1, "ct")
    WorktreePool(r, "HEAD", tmp_path / "w", 1, "ct").close()           # a short root is fine


def test_cargo_target_dir_and_env_helpers(tmp_path):
    assert cargo_target_dir(tmp_path / "card-loop-3", r"D:\cargo-target") == os.path.join(
        r"D:\cargo-target", "card-loop-3")
    env = worktree_env(tmp_path / "card-loop-3", "B", None, base_env={})
    assert env == {"CARGO_TARGET_DIR": os.path.join("B", "card-loop-3"), "CARGO_TARGET_DIR_PINNED": "1",
                   "CARGO_TARGET_BASE": "B"}
