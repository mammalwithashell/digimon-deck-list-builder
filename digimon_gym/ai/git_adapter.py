"""Constrained git/gh command adapter for AI batch and manual apply flows."""

from __future__ import annotations

import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parents[2]
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
        return self.repo_root / "data" / "run" / "ai_worktrees" / batch_id

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
        return self.repo_root / "data" / "run" / "ai_worktrees" / f"task-{task_id}"

    def prepare_worktree_for_task(
        self, *, task_id: str, run_mode: str
    ) -> WorktreeContext:
        wt = self.worktree_path_for_task(task_id=task_id)
        wt.parent.mkdir(parents=True, exist_ok=True)
        branch = self.branch_name_for_task(task_id=task_id)

        if wt.exists() and (wt / ".git").exists():
            target_branch = branch if run_mode == "pr" else DEFAULT_BRANCH
            try:
                _run(["git", "checkout", target_branch], cwd=wt)
            except GitCommandError:
                _run(["git", "fetch", "origin", DEFAULT_BRANCH], cwd=self.repo_root)
                _run(
                    ["git", "checkout", "-B", target_branch, f"origin/{DEFAULT_BRANCH}"],
                    cwd=wt,
                )
            self.restore_worktree_to_head(worktree_path=wt)
            return WorktreeContext(
                worktree_path=wt,
                branch_name=target_branch,
            )

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
