"""The driver's merger (design D13; tasks 6.5, 6.7): one worker diff at a time,
through `author_set.merge_wave` semantics -- never a fork of them.

`LoopMerger.merge(ctx, request)`:

1. Reads `request.artifacts` (`{"diff", "manifest"}`, the D1 transport made by
   `workers.base.capture_artifacts`). Refuses a diff that edits a gap tracker
   (tracker writes are orchestrator-only, `gaps.RunGapLane.record_gap`) and a
   diff that touches engine code (`is_engine_path`) without `engine=True`.
2. Picks the tree:
   * `engine=False`: `ctx.repo`, on its current run branch. Refused on `main` /
     `master` / a detached HEAD, or when a manifest path has uncommitted edits
     there. Other dirty files in the run tree are left alone and never committed.
   * `engine=True`: branch `card-loop/<run_id>/engine-<gap_id or attempt_id>`,
     created from `ctx.base_sha` (or reused, so a second fix for the same gap
     stacks on the first) in a separate worktree `<root>/cl-<run>-eng`. The run
     branch is never checked out, written or moved. The worktree is detached
     after the merge so a human can check the branch out (D13: humans merge it).
     NOTE: a run branch named exactly `card-loop/<run_id>` makes every engine
     branch impossible (a git ref cannot be both a branch and a directory);
     name it `card-loop/<run_id>/run`.
3. `merge_wave.apply_manifest_diff` (path policy, 3-way, idempotent), then
   `merge_wave.assert_registration`, then the pack build when a card YAML
   changed, then `merge_wave.compute_scope` (impact_scope) over the touched
   paths and `merge_wave.run_scoped_suite` (full binary when
   `full_suite_required`), all through `ctx.run_command`.
4. On green, commits only the manifest's paths with
   `provenance.loop_commit_message` (`Loop-Attempt`, `Loop-Model`, plus
   `Loop-Run` and `Loop-Gap`), and for an engine merge notes the branch on the
   gap lane (`note_branch(gap_id, branch, sha)`).

Any failure restores the touched paths (`merge_wave.rollback`); an engine
branch created by the failed call is deleted again. Re-merging an attempt whose
content is already in the tree is a no-op returning the existing commit.

`MergeResult.scope` is impact_scope's JSON plus `scope["suite"] =
{"green", "ran", "commands": [{command, rc, tail, ...}], "impact_scope": {...}}`
so the fix gate can reuse the scoped run instead of repeating it.
Cargo runs get `RUST_MIN_STACK` (rule 33); a scratch worktree also gets its own
`CARGO_TARGET_DIR = <config.cargo_target_base>\\<worktree name>` (rule 31), a
stable name so its build stays warm across merges.
"""
from __future__ import annotations

import json

import os
import re
import subprocess
from pathlib import Path
from typing import Any, Sequence

from ..author_set import merge_wave as mw
from .driver_contracts import MergeRequest, MergeResult, RunContext
from .provenance import loop_commit_message
from .workers.pool import cargo_target_dir

ENGINE_PATH_PREFIXES = ("code/digimon-engine/src/", "code/digimon-dsl/")
TRACKER_PATHS = ("qa/dsl-vocab-gaps.md", "docs/RUST_ENGINE_GAPS.md")
PROTECTED_BRANCHES = ("main", "master")


def is_engine_path(path: str) -> bool:
    return mw.canonical(path).startswith(ENGINE_PATH_PREFIXES)


def _slug(text: str, limit: int = 60) -> str:
    s = re.sub(r"[^A-Za-z0-9_-]+", "-", text).strip("-")
    s = re.sub(r"-{2,}", "-", s)
    return s[:limit].rstrip("-") or "x"


def engine_branch_name(run_id: str, gap_or_attempt: str) -> str:
    return f"card-loop/{_slug(run_id, 80)}/engine-{_slug(gap_or_attempt)}"


# ---------------------------------------------------------------------------
# shared helpers (the fix gate uses them too)
# ---------------------------------------------------------------------------


def git(repo: str | os.PathLike, *args: str, check: bool = False) -> subprocess.CompletedProcess:
    cp = subprocess.run(["git", *args], cwd=str(repo), capture_output=True, text=True,
                        encoding="utf-8", errors="replace")
    if check and cp.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)} failed in {repo}: {cp.stderr.strip()}")
    return cp


def _same_path(a: str | os.PathLike, b: str | os.PathLike) -> bool:
    return os.path.normcase(os.path.abspath(a)) == os.path.normcase(os.path.abspath(b))


def worktree_root(ctx: Any, override: str | os.PathLike | None = None) -> Path:
    """Where scratch worktrees live: the explicit override, the worker pool's
    root, `config.worktree_root`, else `<run_dir>/wt`. Keep it short on
    Windows (MAX_PATH, see workers/pool.py)."""
    for cand in (override, getattr(getattr(ctx, "pool", None), "root", None),
                 getattr(getattr(ctx, "config", None), "worktree_root", None)):
        if cand:
            return Path(cand)
    return Path(ctx.run_dir) / "wt"


def scratch_name(ctx: Any, role: str) -> str:
    return f"cl-{_slug(str(ctx.run_id), 32)}-{role}"


def ensure_worktree(repo: str | os.PathLike, path: Path, ref: str) -> None:
    """A worktree of `repo` at `path` (created detached at `ref`, or reused)."""
    listed = git(repo, "worktree", "list", "--porcelain", check=True).stdout
    registered = [l[len("worktree "):] for l in listed.splitlines() if l.startswith("worktree ")]
    if path.is_dir() and any(_same_path(r, path) for r in registered):
        return
    if path.exists() and any(path.iterdir()):
        raise RuntimeError(f"refusing to replace {path}: it exists and is not a worktree of {repo}")
    git(repo, "worktree", "prune")
    path.parent.mkdir(parents=True, exist_ok=True)
    git(repo, "worktree", "add", "-q", "--detach", str(path), ref, check=True)


def reset_worktree(path: Path, ref: str) -> None:
    """Detach `path` at `ref` with no tracked or untracked leftovers."""
    git(path, "checkout", "-q", "--force", "--detach", ref, check=True)
    git(path, "reset", "-q", "--hard", ref, check=True)
    git(path, "clean", "-q", "-ffd", check=True)


def cargo_env_for(ctx: Any, tree: str | os.PathLike) -> dict:
    """RUST_MIN_STACK, plus a per-worktree CARGO_TARGET_DIR for any tree that
    is not the run tree itself (the run tree keeps the driver's own env)."""
    base = getattr(getattr(ctx, "config", None), "cargo_target_base", None)
    if base and not _same_path(tree, ctx.repo):
        return mw.cargo_env(cargo_target_dir=cargo_target_dir(tree, base))
    return mw.cargo_env()


def current_branch(repo: str | os.PathLike) -> str | None:
    cp = git(repo, "symbolic-ref", "-q", "--short", "HEAD")
    return cp.stdout.strip() if cp.returncode == 0 and cp.stdout.strip() else None


def rev(repo: str | os.PathLike, ref: str) -> str | None:
    cp = git(repo, "rev-parse", "--verify", "-q", f"{ref}^{{commit}}")
    return cp.stdout.strip() if cp.returncode == 0 and cp.stdout.strip() else None


# ---------------------------------------------------------------------------
# the merger
# ---------------------------------------------------------------------------


def manifest_base(manifest_path) -> str | None:
    """The `base_sha` a worker's manifest was captured against, or None."""
    try:
        with open(manifest_path, encoding="utf-8") as f:
            v = json.load(f).get("base_sha")
        return v if isinstance(v, str) and v else None
    except (OSError, ValueError):
        return None


class LoopMerger:
    """`driver_contracts.Merger`. Merges are serialised by the driver (D13)."""

    def __init__(self, *, gap_lane: Any = None, worktree_root: str | os.PathLike | None = None,
                 test_threads: int = mw.DEFAULT_TEST_THREADS, suite_timeout: float = 3600.0,
                 build_timeout: float = 3600.0, scope_timeout: float = 300.0,
                 check_pack: bool = True, allowed_roots: Sequence[str] = mw.ALLOWED_ROOTS):
        self.gap_lane = gap_lane
        self.worktree_root = worktree_root
        self.test_threads = test_threads
        self.suite_timeout = suite_timeout
        self.build_timeout = build_timeout
        self.scope_timeout = scope_timeout
        self.check_pack = check_pack
        self.allowed_roots = tuple(allowed_roots)

    # ------------------------------------------------------------------ public

    def merge(self, ctx: RunContext, request: MergeRequest) -> MergeResult:
        art = request.artifacts or {}
        diff, manifest_path = art.get("diff"), art.get("manifest")
        if not diff or not manifest_path:
            return MergeResult(ok=False, errors=[
                "MergeRequest.artifacts needs {'diff', 'manifest'} paths (capture_artifacts)"])
        try:
            manifest, _ = mw.load_manifest(manifest_path)
        except (OSError, ValueError, mw.MergeWaveError) as e:
            return MergeResult(ok=False, errors=[f"manifest unreadable: {e}"])
        paths = [f["path"] for f in manifest["files"]]
        trackers = sorted(p for p in paths if p in TRACKER_PATHS)
        if trackers:
            return MergeResult(ok=False, errors=[
                f"{', '.join(trackers)}: gap-tracker writes are orchestrator-only -- the worker "
                f"must report gaps in its result's `gaps`, and the driver records them"])
        engine_paths = sorted(p for p in paths if is_engine_path(p))
        if engine_paths and not request.engine:
            return MergeResult(ok=False, errors=[
                f"the diff touches engine code ({', '.join(engine_paths)}); engine fixes land on "
                f"their own branch for human review (D13) -- resubmit with engine=True"])
        if request.engine:
            return self._merge_engine(ctx, request, diff, manifest_path)
        return self._merge_run(ctx, request, diff, manifest_path, paths)

    def engine_branch(self, ctx: RunContext, request: MergeRequest) -> str:
        return engine_branch_name(ctx.run_id, request.gap_id or request.attempt_id)

    def close(self, ctx: RunContext) -> None:
        """Remove the engine worktree (its branches stay)."""
        wt = worktree_root(ctx, self.worktree_root) / scratch_name(ctx, "eng")
        git(ctx.repo, "worktree", "remove", "--force", "--force", str(wt))
        git(ctx.repo, "worktree", "prune")

    # ------------------------------------------------------------------ run branch

    def _merge_run(self, ctx, request, diff, manifest_path, paths) -> MergeResult:
        repo = str(ctx.repo)
        branch = current_branch(repo)
        if branch is None or branch in PROTECTED_BRANCHES:
            return MergeResult(ok=False, errors=[
                f"the run tree {repo} is on {branch or 'a detached HEAD'}; the loop commits only "
                f"onto a run branch (e.g. card-loop/{ctx.run_id}/run), never main"])
        safe = [p for p in paths if mw.path_problem(p, None) is None]
        dirty = git(repo, "status", "--porcelain", "--untracked-files=all", "--", *safe).stdout \
            if safe else ""
        if dirty.strip():
            return MergeResult(ok=False, branch=branch, errors=[
                f"manifest paths have uncommitted changes in the run tree: "
                f"{', '.join(l[3:] for l in dirty.splitlines())} -- commit or discard them first"])
        return self._apply_and_commit(ctx, request, repo, branch, diff, manifest_path)

    # ------------------------------------------------------------------ engine branch

    def _merge_engine(self, ctx, request, diff, manifest_path) -> MergeResult:
        repo = str(ctx.repo)
        branch = self.engine_branch(ctx, request)
        run_head = rev(repo, "HEAD")
        run_branch = current_branch(repo)
        if git(repo, "check-ref-format", "--branch", branch).returncode != 0:
            return MergeResult(ok=False, errors=[f"{branch!r} is not a valid branch name"])
        wt = worktree_root(ctx, self.worktree_root) / scratch_name(ctx, "eng")
        base = manifest_base(manifest_path) or ctx.base_sha
        try:
            ensure_worktree(repo, wt, base)
            reset_worktree(wt, base)
        except RuntimeError as e:
            return MergeResult(ok=False, branch=branch, errors=[f"engine worktree: {e}"])
        created = rev(repo, f"refs/heads/{branch}") is None
        if created:
            cp = git(wt, "checkout", "-q", "-b", branch, base)
        else:
            cp = git(wt, "checkout", "-q", "--force", branch)
        if cp.returncode != 0:
            err = cp.stderr.strip()
            if "cannot lock ref" in err or "exists" in err:
                err += (f" -- a ref on the path of {branch} already exists; if the run branch is "
                        f"named card-loop/{ctx.run_id}, rename it card-loop/{ctx.run_id}/run")
            return MergeResult(ok=False, branch=branch, errors=[f"cannot check out {branch}: {err}"])
        try:
            res = self._apply_and_commit(ctx, request, str(wt), branch, diff, manifest_path)
        finally:
            git(wt, "checkout", "-q", "--force", "--detach")
        if not res.ok and created and rev(repo, f"refs/heads/{branch}") in (base, None):
            git(repo, "branch", "-q", "-D", branch)
        if rev(repo, "HEAD") != run_head or current_branch(repo) != run_branch:
            res.ok = False
            res.errors.append("internal: the run branch moved during an engine merge")
        if res.ok and request.gap_id and self.gap_lane is not None:
            self.gap_lane.note_branch(request.gap_id, branch, res.sha)
        return res

    # ------------------------------------------------------------------ shared

    def _apply_and_commit(self, ctx, request, tree, branch, diff, manifest_path) -> MergeResult:
        runner = ctx.run_command
        # The worker's own base: the run branch's HEAD when it was leased, which
        # may be newer than the plan's base (re-authoring after an earlier
        # merge). The apply checks that it is an ancestor of the target HEAD.
        applied = mw.apply_manifest_diff(tree, diff, manifest_path,
                                         manifest_base(manifest_path) or ctx.base_sha,
                                         allowed_roots=self.allowed_roots)
        result = MergeResult(ok=False, branch=branch, touched=applied.touched)
        if not applied.ok:
            result.errors = list(applied.errors)
            return result
        if applied.noop:
            sha = self._commit_of(tree, request.attempt_id) or rev(tree, "HEAD")
            return MergeResult(ok=True, sha=sha, branch=branch, touched=applied.touched,
                               scope={"noop": True})

        def fail(*errors: str) -> MergeResult:
            mw.rollback(applied)
            result.errors = [*errors]
            return result

        problems = mw.assert_registration(tree, applied)
        if problems:
            return fail(*problems)
        env = cargo_env_for(ctx, tree)
        if self.check_pack and any(mw.is_card_yaml(p) for p in applied.changed):
            pack = mw.pack_check(runner, tree, env=env, timeout=self.build_timeout)
            if not pack["ok"]:
                return fail(f"pack build failed ({pack['command']}, rc {pack['rc']}): "
                            f"{pack['tail']}")
        scope, scope_rec = mw.compute_scope(runner, tree, applied.touched,
                                            timeout=self.scope_timeout)
        suite = mw.run_scoped_suite(runner, tree, scope, applied.touched, env=env,
                                    timeout=self.suite_timeout, test_threads=self.test_threads)
        suite["impact_scope"] = scope_rec
        scope = {**scope, "suite": suite}
        result.scope = scope
        if not suite["green"]:
            last = suite["commands"][-1]
            return fail(f"scoped suite failed ({last['command']}, rc {last['rc']}): {last['tail']}")
        msg = loop_commit_message(
            request.subject or self._subject(request), self._body(applied, scope),
            attempt_id=request.attempt_id, family=request.family, model=request.model,
            extra_trailers=[f"Loop-Run: {ctx.run_id}",
                            *([f"Loop-Gap: {request.gap_id}"] if request.gap_id else [])])
        cp = git(tree, "commit", "-q", "-m", msg, "--", *applied.changed)
        if cp.returncode != 0:
            return fail(f"commit failed: {(cp.stderr or cp.stdout).strip()}")
        result.ok = True
        result.sha = rev(tree, "HEAD")
        return result

    @staticmethod
    def _commit_of(tree: str, attempt_id: str) -> str | None:
        cp = git(tree, "log", "-1", "--format=%H", "--fixed-strings",
                 f"--grep=Loop-Attempt: {attempt_id}")
        return cp.stdout.strip() or None

    @staticmethod
    def _subject(request: MergeRequest) -> str:
        what = "engine fix" if request.engine else "merge"
        return f"card-loop: {what} {request.gap_id or request.attempt_id}"

    @staticmethod
    def _body(applied: mw.AppliedDiff, scope: dict) -> str:
        lines = ["Touched:"]
        lines += [f"- {p} ({applied.statuses.get(p) or '?'})" for p in applied.touched]
        if scope.get("full_suite_required"):
            what = "full cards_behavioral"
        else:
            what = "cards: " + (", ".join(scope.get("cards") or []) or "-")
        side = ", ".join(scope.get("side_binaries") or []) or "-"
        lines += ["", f"Scope: {what}; side binaries: {side}"]
        for c in scope.get("suite", {}).get("commands", []):
            lines.append(f"Suite: {c['command']} -> rc {c['rc']}")
        return "\n".join(lines)
