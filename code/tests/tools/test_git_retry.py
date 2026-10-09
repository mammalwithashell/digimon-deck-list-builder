"""`tools.git_retry`: index.lock collisions are retried, stale locks swept."""
from __future__ import annotations

import os
import time

from tools import git_retry as gr

LOCK_ERR = ("fatal: Unable to create 'C:/x/.git/worktrees/run/index.lock': File exists.\n\n"
            "Another git process seems to be running in this repository")


def test_lock_errors_are_recognised_and_others_are_not():
    assert gr.is_index_lock_error(LOCK_ERR)
    assert gr.is_index_lock_error(LOCK_ERR.encode())
    assert not gr.is_index_lock_error("error: patch failed: a.yaml:3")
    assert not gr.is_index_lock_error(None)


def test_a_collision_is_retried_until_the_lock_clears():
    results = iter([(128, "", LOCK_ERR), (128, "", LOCK_ERR), (0, "ok", "")])
    slept = []
    out = gr.retry_on_index_lock(lambda: next(results), lambda r: r[2], attempts=5, delay=0.25,
                                 sleep=slept.append)
    assert out == (0, "ok", "") and slept == [0.25, 0.25]


def test_a_lock_that_never_clears_returns_the_last_failure():
    calls = []
    out = gr.retry_on_index_lock(lambda: (calls.append(1), (128, "", LOCK_ERR))[1], lambda r: r[2],
                                 attempts=4, delay=0, sleep=lambda s: None)
    assert out[0] == 128 and len(calls) == 4


def test_a_real_error_is_not_retried():
    calls = []
    gr.retry_on_index_lock(lambda: (calls.append(1), (1, "", "error: patch failed"))[1], lambda r: r[2],
                           attempts=5, delay=0, sleep=lambda s: None)
    assert len(calls) == 1


def test_a_stale_lock_is_swept_and_a_fresh_one_kept(tmp_path):
    lock = tmp_path / "index.lock"
    lock.write_text("")
    t = time.time()
    os.utime(lock, (t - 10, t - 10))
    assert gr.sweep_stale_index_lock(tmp_path, lock_path=lock, now=lambda: t) is None
    assert lock.exists(), "a ten-second-old lock may be a live operation"
    os.utime(lock, (t - 600, t - 600))
    assert gr.sweep_stale_index_lock(tmp_path, lock_path=lock, now=lambda: t) == lock
    assert not lock.exists()
    assert gr.sweep_stale_index_lock(tmp_path, lock_path=lock, now=lambda: t) is None


def test_the_lock_path_comes_from_the_worktrees_own_gitdir(tmp_path):
    p = gr.index_lock_path(tmp_path, git_path=lambda name: "C:/repo/.git/worktrees/run/" + name)
    assert str(p).replace("\\", "/").endswith(".git/worktrees/run/index.lock")
    assert gr.index_lock_path(tmp_path, git_path=lambda name: "") is None
