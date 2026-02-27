"""Constrained git/gh command adapter for AI batch and manual apply flows."""

from __future__ import annotations

import logging
import os
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path

logger = logging.getLogger(__name__)


PROJECT_ROOT = Path(__file__).resolve().parents[2]
_WORKTREE_BASE = Path(
    os.environ.get("AI_WORKTREE_DIR", "")
) if os.environ.get("AI_WORKTREE_DIR") else Path(tempfile.gettempdir()) / "ai_worktrees"
DEFAULT_BRANCH = "main"


class GitCommandError(RuntimeError):
    """Raised when a git/gh command fails."""


@dataclass
class WorktreeContext:
    worktree_path: Path
    branch_name: str


def _run(args: list[str], *, cwd: Path) -> str:
    proc = subprocess.run(
        args,
        cwd=str(cwd),
        capture_output=True,
        text=True,
        check=False,
    )
    out = ((proc.stdout or "") + ("\n" + proc.stderr if proc.stderr else "")).strip()
    if proc.returncode != 0:
        raise GitCommandError(f"Command failed: {' '.join(args)}\n{out}")
    return out


def _ensure_command(name: str) -> None:
    if shutil.which(name) is None:
        raise GitCommandError(f"Required command is missing: {name}")


class GitAdapter:
    def __init__(self, repo_root: Path | None = None) -> None:
        self.repo_root = repo_root or PROJECT_ROOT

    def preflight(self, *, run_mode: str) -> None:
        _ensure_command("git")
        _run(["git", "rev-parse", "--is-inside-work-tree"], cwd=self.repo_root)
        _run(["git", "remote", "get-url", "origin"], cwd=self.repo_root)
        if run_mode == "pr":
            _ensure_command("gh")
            _run(["gh", "auth", "status"], cwd=self.repo_root)

    def branch_name_for_batch(self, *, set_id: str, batch_id: str) -> str:
        prefix = batch_id.split("-", 1)[0]
        return f"ai/fix-{set_id.lower()}-{prefix}"

    def worktree_path_for_batch(self, *, batch_id: str) -> Path:
        return _WORKTREE_BASE / batch_id

    def prepare_worktree(self, *, set_id: str, batch_id: str, run_mode: str) -> WorktreeContext:
        wt = self.worktree_path_for_batch(batch_id=batch_id)
        wt.parent.mkdir(parents=True, exist_ok=True)
        branch = self.branch_name_for_batch(set_id=set_id, batch_id=batch_id)

        if wt.exists() and (wt / ".git").exists():
            return WorktreeContext(worktree_path=wt, branch_name=branch if run_mode == "pr" else DEFAULT_BRANCH)

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

    # ── Task-oriented helpers (single manual apply-fix) ──────────────

    def branch_name_for_task(self, *, task_id: str) -> str:
        prefix = task_id.split("-", 1)[0]
        return f"ai/fix-manual-{prefix}"

    def worktree_path_for_task(self, *, task_id: str) -> Path:
        return _WORKTREE_BASE / f"task-{task_id}"

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

    def restore_worktree_to_head(self, *, worktree_path: Path) -> None:
        _run(["git", "reset", "--hard", "HEAD"], cwd=worktree_path)
        _run(["git", "clean", "-fd"], cwd=worktree_path)

    # ── Shared git operations ──────────────────────────────────────

    def commit_files(self, *, worktree_path: Path, files: list[str], message: str) -> str | None:
        if not files:
            return None
        _run(["git", "add", "--", *files], cwd=worktree_path)

        diff = subprocess.run(
            ["git", "diff", "--cached", "--quiet"],
            cwd=str(worktree_path),
            capture_output=True,
            text=True,
            check=False,
        )
        if diff.returncode == 0:
            return None
        if diff.returncode not in {0, 1}:
            raise GitCommandError("git diff --cached --quiet failed")

        _run(["git", "commit", "-m", message], cwd=worktree_path)
        sha = _run(["git", "rev-parse", "HEAD"], cwd=worktree_path).strip()
        return sha

    def push_pr_branch_and_open_draft_pr(
        self,
        *,
        worktree_path: Path,
        branch_name: str,
        title: str,
        body: str,
    ) -> str:
        _run(["git", "push", "-u", "origin", branch_name], cwd=worktree_path)
        pr_url = _run(
            [
                "gh",
                "pr",
                "create",
                "--draft",
                "--base",
                DEFAULT_BRANCH,
                "--head",
                branch_name,
                "--title",
                title,
                "--body",
                body,
            ],
            cwd=worktree_path,
        ).strip()
        return pr_url

    def push_to_main(self, *, worktree_path: Path) -> None:
        _run(["git", "push", "origin", "HEAD:main"], cwd=worktree_path)

    # ── Set-run consolidation ────────────────────────────────────────

    def branch_name_for_set_run(self, *, set_id: str, run_id: str) -> str:
        prefix = run_id.split("-", 1)[0]
        return f"ai/set-run-{set_id.lower()}-{prefix}"

    def create_consolidation_branch(
        self,
        *,
        set_id: str,
        run_id: str,
        commit_shas: list[str],
    ) -> tuple[WorktreeContext, list[str]]:
        """Cherry-pick applied fix commits onto a fresh consolidation branch.

        Returns (WorktreeContext, skipped_shas) where skipped_shas are commits
        that could not be cherry-picked (conflict).
        """
        branch = self.branch_name_for_set_run(set_id=set_id, run_id=run_id)
        wt = _WORKTREE_BASE / f"consolidate-{run_id}"

        # Clean up any existing worktree at this path
        if wt.exists():
            shutil.rmtree(wt, ignore_errors=True)
            _run(["git", "worktree", "prune"], cwd=self.repo_root)

        _run(["git", "fetch", "origin", DEFAULT_BRANCH], cwd=self.repo_root)
        wt.parent.mkdir(parents=True, exist_ok=True)
        _run(
            ["git", "worktree", "add", "-B", branch, str(wt), f"origin/{DEFAULT_BRANCH}"],
            cwd=self.repo_root,
        )

        skipped: list[str] = []
        for sha in commit_shas:
            try:
                _run(["git", "cherry-pick", sha], cwd=wt)
            except GitCommandError:
                logger.warning("Cherry-pick failed for %s, skipping", sha)
                # Abort the failed cherry-pick so the worktree is clean
                subprocess.run(
                    ["git", "cherry-pick", "--abort"],
                    cwd=str(wt),
                    capture_output=True,
                    check=False,
                )
                skipped.append(sha)

        return WorktreeContext(worktree_path=wt, branch_name=branch), skipped

    def close_pr(self, pr_url: str) -> None:
        """Close a PR by URL. Ignores errors (already closed/merged)."""
        try:
            _run(["gh", "pr", "close", pr_url], cwd=self.repo_root)
        except GitCommandError:
            logger.debug("Could not close PR %s (may already be closed)", pr_url)
