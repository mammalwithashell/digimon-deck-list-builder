"""Driver-managed worker worktrees (design D10, spec "Workers run in driver-managed,
pinned worktrees").

The pool creates detached worktrees at a pinned base commit with
`git worktree add --detach`, keeps at most `size` of them, and resets a worktree
to the base before every item. Neither CLI's own `--worktree` option is used, so
base pinning and cargo-target isolation are identical across vendors.

Each worktree gets its own `CARGO_TARGET_DIR = <cargo_target_base>\\<name>`
(CLAUDE.md rule 31). `CARGO_TARGET_DIR_PINNED=1` is exported too: the user's
`~/.bashrc` (sourced by every non-interactive bash through `BASH_ENV`) otherwise
re-derives `CARGO_TARGET_DIR` from the cwd — and for a worktree that is not a
direct child of `.claude/worktrees/` it would hand every pool member the SAME
target dir, re-creating the cross-worktree contamination rule 31 exists to stop.

What survives a reset: only the `keep` patterns (default: frontend
`node_modules/` and in-tree cargo `target/` dirs — pure build caches that are
expensive to rebuild; a keep pattern must name a .gitignored path, or the
post-reset cleanliness check fails and the worktree is recreated). Everything
else a worker wrote — tracked edits, commits, branch switches, untracked and
.gitignored files — is discarded, so artifacts must be captured
(`capture_artifacts`) before the next `acquire`. Branches a worker created stay
in the shared ref store (the reset detaches; it never deletes refs).

Keep the pool root SHORT on Windows: the constructor refuses a root whose members
cannot hold the base commit's longest tracked path under MAX_PATH (a scratchpad
under %TEMP% already fails for this repo), unless core.longpaths is enabled.
"""
from __future__ import annotations

import os
import re
import shutil
import subprocess
import threading
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Iterator, Mapping, Sequence

from .base import scrubbed_env

DEFAULT_KEEP: tuple[str, ...] = ("code/frontend/node_modules/", "target/", ".cargo-target-*/")
WINDOWS_MAX_PATH = 259


class PoolError(RuntimeError):
    pass


class PoolExhausted(PoolError):
    pass


def cargo_target_dir(worktree: str | os.PathLike, cargo_target_base: str) -> str:
    """`<cargo_target_base>\\<worktree dir name>` (rule 31)."""
    return os.path.join(cargo_target_base, Path(worktree).name)


def worktree_env(
    worktree: str | os.PathLike,
    cargo_target_base: str,
    sccache_dir: str | None = None,
    base_env: Mapping[str, str] | None = None,
) -> dict[str, str]:
    """Environment for a worker running in `worktree`: the parent env minus
    parent-session CLAUDE* vars, plus a pinned per-worktree cargo target."""
    env = scrubbed_env(base_env)
    env["CARGO_TARGET_DIR"] = cargo_target_dir(worktree, cargo_target_base)
    env["CARGO_TARGET_DIR_PINNED"] = "1"
    env["CARGO_TARGET_BASE"] = cargo_target_base
    if sccache_dir:
        env["SCCACHE_DIR"] = sccache_dir
    return env


@dataclass(frozen=True)
class PooledWorktree:
    path: Path
    name: str
    cargo_target_dir: str


def _norm(p: str | os.PathLike) -> str:
    return os.path.normcase(os.path.abspath(str(p)))


class WorktreePool:
    def __init__(
        self,
        repo: str | os.PathLike,
        base_sha: str,
        root: str | os.PathLike,
        size: int,
        cargo_target_base: str,
        *,
        sccache_dir: str | None = None,
        prefix: str = "card-loop",
        keep: Sequence[str] = DEFAULT_KEEP,
    ):
        if size < 1:
            raise ValueError("pool size must be >= 1")
        if not re.fullmatch(r"[A-Za-z0-9._-]+", prefix):
            raise ValueError(f"bad worktree prefix {prefix!r}")
        self.repo = Path(repo)
        self.root = Path(root)
        self.size = size
        self.cargo_target_base = cargo_target_base
        self.sccache_dir = sccache_dir
        self.prefix = prefix
        self.keep = tuple(keep)
        self.base_sha = self._git(["rev-parse", "--verify", f"{base_sha}^{{commit}}"], self.repo).strip()
        self._check_path_budget()
        self.root.mkdir(parents=True, exist_ok=True)
        self._cond = threading.Condition()
        self._admin = threading.Lock()  # serialises git worktree add/remove/prune
        self._idle: list[PooledWorktree] = []
        self._busy: dict[str, PooledWorktree] = {}
        self._closed = False
        self._adopt_existing()

    # ------------------------------------------------------------------ public

    def acquire(self, timeout: float | None = None) -> PooledWorktree:
        """A worktree reset to `base_sha`. Blocks while all `size` are busy;
        raises `PoolExhausted` if `timeout` elapses first."""
        with self._cond:
            while True:
                if self._closed:
                    raise PoolError("pool is closed")
                if self._idle:
                    wt = self._idle.pop(0)
                    break
                if self._total() < self.size:
                    # Reserve the name now; the slow `git worktree add` runs unlocked.
                    wt = self._wt(self.root / self._next_name())
                    break
                if not self._cond.wait(timeout):
                    raise PoolExhausted(f"no worktree free within {timeout}s (size {self.size})")
            self._busy[wt.name] = wt
        try:
            self._add(wt.path)
            self.reset(wt)
        except Exception:
            with self._cond:
                self._busy.pop(wt.name, None)
                self._cond.notify()
            raise
        return wt

    def release(self, wt: PooledWorktree) -> None:
        with self._cond:
            if self._busy.pop(wt.name, None) is None:
                raise PoolError(f"{wt.name} is not checked out from this pool")
            self._idle.append(wt)
            self._cond.notify()

    @contextmanager
    def lease(self, timeout: float | None = None) -> Iterator[PooledWorktree]:
        wt = self.acquire(timeout)
        try:
            yield wt
        finally:
            self.release(wt)

    def retarget(self, base_sha: str) -> None:
        """Move the base every later lease resets to -- the run branch's HEAD
        once it has advanced past the plan's base, so a worker re-authoring a
        file starts from the merged version and its diff applies cleanly."""
        sha = self._git(["rev-parse", "--verify", f"{base_sha}^{{commit}}"], self.repo).strip()
        with self._cond:
            self.base_sha = sha

    def reset(self, wt: PooledWorktree) -> None:
        """Force `wt` back to `base_sha`: detached HEAD, no edits, no untracked or
        ignored files except `keep`. Recreates the worktree if git cannot."""
        try:
            self._reset(wt.path)
        except (PoolError, OSError):
            self._remove(wt.path)
            self._add(wt.path)
            self._reset(wt.path)

    def env(self, wt: PooledWorktree, base_env: Mapping[str, str] | None = None) -> dict[str, str]:
        return worktree_env(wt.path, self.cargo_target_base, self.sccache_dir, base_env)

    def worktrees(self) -> list[PooledWorktree]:
        with self._cond:
            return sorted([*self._idle, *self._busy.values()], key=lambda w: w.name)

    def close(self, *, remove: bool = True) -> None:
        """Stop handing out worktrees; with `remove`, `git worktree remove --force`
        every member and prune. `remove=False` leaves them warm for the next run."""
        with self._cond:
            self._closed = True
            members = [*self._idle, *self._busy.values()]
            self._idle.clear()
            self._busy.clear()
            self._cond.notify_all()
        if remove:
            for wt in members:
                self._remove(wt.path)

    def __enter__(self) -> "WorktreePool":
        return self

    def __exit__(self, *exc) -> None:
        self.close()

    # ------------------------------------------------------------------ internals

    def _check_path_budget(self) -> None:
        """Windows without `core.longpaths`: refuse a root whose members cannot hold
        the base commit's longest tracked path — `git worktree add` (and later
        builds) would otherwise fail midway with a confusing error."""
        if os.name != "nt":
            return
        longpaths = self._git(["config", "--bool", "--get", "core.longpaths"], self.repo, check=False)
        if longpaths.strip() == "true":
            return
        names = self._git(["ls-tree", "-r", "--name-only", "-z", self.base_sha], self.repo)
        longest = max((len(n) for n in names.split("\0") if n), default=0)
        member = os.path.abspath(str(self.root / f"{self.prefix}-{self.size - 1}"))
        worst = len(member) + 1 + longest
        if worst > WINDOWS_MAX_PATH:
            raise PoolError(
                f"pool root {self.root} is too deep: {member} plus the longest tracked path "
                f"({longest} chars) is {worst} > {WINDOWS_MAX_PATH} (MAX_PATH). Use a shorter "
                f"root (e.g. D:\\card-loop-wt) or enable core.longpaths.")

    def _total(self) -> int:
        return len(self._idle) + len(self._busy)

    def _member_name(self, path: Path) -> str | None:
        m = re.fullmatch(re.escape(self.prefix) + r"-(\d+)", path.name)
        return path.name if m and _norm(path.parent) == _norm(self.root) else None

    def _next_name(self) -> str:
        used = {w.name for w in (*self._idle, *self._busy.values())}
        i = 0
        while f"{self.prefix}-{i}" in used:
            i += 1
        return f"{self.prefix}-{i}"

    def _wt(self, path: Path) -> PooledWorktree:
        return PooledWorktree(path=path, name=path.name,
                              cargo_target_dir=cargo_target_dir(path, self.cargo_target_base))

    def _registered(self) -> list[Path]:
        out = self._git(["worktree", "list", "--porcelain"], self.repo)
        return [Path(line[len("worktree "):]) for line in out.splitlines() if line.startswith("worktree ")]

    def _adopt_existing(self) -> None:
        """Reuse members a previous run left warm (reset happens on acquire)."""
        for path in sorted(self._registered(), key=lambda p: p.name):
            if self._member_name(path) and path.is_dir() and self._total() < self.size:
                self._idle.append(self._wt(Path(path)))

    def _add(self, path: Path) -> None:
        """Create the worktree at `path` unless it is already registered."""
        with self._admin:
            self._add_locked(path)

    def _add_locked(self, path: Path) -> None:
        if path.exists():
            if any(_norm(p) == _norm(path) for p in self._registered()):
                return
            if self._member_name(path) is None:  # never delete outside our own members
                raise PoolError(f"refusing to replace non-pool directory {path}")
            shutil.rmtree(path)
        self._git(["worktree", "prune"], self.repo, check=False)
        self._git(["worktree", "add", "--detach", str(path), self.base_sha], self.repo)

    def _remove(self, path: Path) -> None:
        with self._admin:
            self._git(["worktree", "remove", "--force", "--force", str(path)], self.repo, check=False)
            if path.exists() and self._member_name(path):
                shutil.rmtree(path, ignore_errors=True)
            self._git(["worktree", "prune"], self.repo, check=False)

    def _reset(self, path: Path) -> None:
        g = lambda *a: self._git(list(a), path)  # noqa: E731
        g("checkout", "--force", "--detach", self.base_sha)
        g("reset", "--hard", self.base_sha)
        clean = ["clean", "-ffdx"]
        for pattern in self.keep:
            clean += ["-e", pattern]
        g(*clean)
        head = g("rev-parse", "HEAD").strip()
        dirty = g("status", "--porcelain", "--untracked-files=all").strip()
        if head != self.base_sha or dirty:
            raise PoolError(f"{path} did not reset to {self.base_sha}: head={head} dirty={dirty[:200]!r}")

    @staticmethod
    def _git(args: list[str], cwd: Path, *, check: bool = True) -> str:
        cp = subprocess.run(["git", *args], cwd=str(cwd), capture_output=True, text=True, check=False)
        if check and cp.returncode != 0:
            raise PoolError(f"git {' '.join(args)} failed in {cwd}: {cp.stderr.strip()}")
        return cp.stdout
