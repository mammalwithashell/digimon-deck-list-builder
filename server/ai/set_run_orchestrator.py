"""Orchestration for end-to-end per-set QA -> review -> fix runs."""

from __future__ import annotations

import asyncio
import hashlib
import json
import logging
import math
import os
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

logger = logging.getLogger(__name__)

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from server.ai.autofix_apply import (
    ApplyValidationError,
    apply_validated_edits,
    run_profile_checks,
    validate_edit_payload,
)
from server.ai.dispatcher import TaskDispatcher
from server.ai.git_adapter import GitAdapter
from server.ai.issue_resolution import (
    build_apply_resolution_note,
    resolve_open_issues_for_card,
)
from server.db.database import async_session
from server.db.models import (
    AIFixApplyAudit,
    AISetRun,
    AISetRunItem,
    AITask,
    Issue,
)


PROJECT_ROOT = Path(__file__).resolve().parents[2]
GENERATED_SCRIPTS_ROOT = PROJECT_ROOT / "digimon_gym" / "engine" / "data" / "scripts" / "generated"


def _utcnow() -> datetime:
    return datetime.now(timezone.utc)


def _load_json(raw: str | None, fallback: Any) -> Any:
    if not raw:
        return fallback
    try:
        return json.loads(raw)
    except json.JSONDecodeError:
        return fallback


def _card_to_set_module(card_id: str) -> tuple[str, str]:
    module = card_id.strip().replace("-", "_").lower()
    set_id = module.split("_", 1)[0]
    return set_id, module


def _module_to_card_id(module_name: str) -> str:
    return module_name.replace("_", "-").upper()


class SetRunOrchestrationError(RuntimeError):
    """Set-run orchestration failure."""


class AISetRunOrchestrator:
    def __init__(self) -> None:
        self.dispatcher = TaskDispatcher()
        self.git = GitAdapter()
        self._main_allowed = os.getenv("AI_APPLY_MAIN_ALLOWED", "0") == "1"

    def discover_set_cards(self, *, set_id: str) -> list[tuple[str, str, str]]:
        set_name = set_id.strip().lower()
        set_dir = GENERATED_SCRIPTS_ROOT / set_name
        if not set_dir.exists():
            raise SetRunOrchestrationError(f"Generated set directory not found: {set_dir}")
        cards: list[tuple[str, str, str]] = []
        for py_file in sorted(set_dir.glob("*.py")):
            if py_file.name == "__init__.py":
                continue
            module_name = py_file.stem
            card_id = _module_to_card_id(module_name)
            cards.append((card_id, set_name, module_name))
        if not cards:
            raise SetRunOrchestrationError(f"No generated scripts found for set: {set_name}")
        return cards

    async def create_set_run(
        self,
        db: AsyncSession,
        *,
        created_by: str | None,
        set_id: str,
        run_mode: str,
        scope_profile: str,
        model_name: str | None,
        max_total_cost_usd: float,
        failure_rate_stop: float,
        max_fix_tasks: int,
        max_fix_iterations: int = 3,
        score_threshold: float | None = None,
    ) -> AISetRun:
        if run_mode == "main" and not self._main_allowed:
            raise SetRunOrchestrationError("run_mode=main is disabled. Set AI_APPLY_MAIN_ALLOWED=1 to enable.")

        cards = self.discover_set_cards(set_id=set_id)
        run = AISetRun(
            set_id=set_id.lower(),
            status="running",
            stage="qa",
            run_mode=run_mode,
            scope_profile=scope_profile,
            model_name=model_name,
            created_by=created_by,
            max_total_cost_usd=max_total_cost_usd,
            failure_rate_stop=failure_rate_stop,
            max_fix_tasks=max_fix_tasks,
            max_fix_iterations=max_fix_iterations,
            score_threshold=score_threshold,
        )
        db.add(run)
        await db.flush()

        for card_id, set_name, module_name in cards:
            db.add(
                AISetRunItem(
                    set_run_id=run.id,
                    card_id=card_id,
                    set_id=set_name,
                    module_name=module_name,
                    fix_apply_status="pending",
                )
            )

        unresolved_issues = (
            await db.execute(
                select(Issue)
                .where(
                    Issue.card_id.like(f"{run.set_id.upper()}-%"),
                    Issue.status.in_(["new", "approved_for_ai"]),
                )
                .order_by(Issue.created_at.asc())
            )
        ).scalars().all()
        issue_by_card: dict[str, Issue] = {}
        for issue in unresolved_issues:
            issue_by_card.setdefault(issue.card_id, issue)

        items = (
            await db.execute(
                select(AISetRunItem)
                .where(AISetRunItem.set_run_id == run.id)
                .order_by(AISetRunItem.card_id.asc())
            )
        ).scalars().all()

        qa_count = 0
        for item in items:
            issue = issue_by_card.get(item.card_id)
            if issue is None:
                continue
            payload = {
                "card_id": item.card_id,
                "set_id": item.set_id,
                "module_name": item.module_name,
                "report_text": issue.description,
                "engine_version": "unknown",
                "script_text": "",
            }
            estimate = self.dispatcher.estimate_cost("qa_analysis", payload, model_name=model_name)
            task = AITask(
                task_type="qa_analysis",
                payload_json=json.dumps(payload),
                status="queued",
                model_name=model_name,
                cost_estimate_usd=estimate,
                max_attempts=3,
                created_by=created_by,
                set_run_id=run.id,
                set_id=run.set_id,
            )
            db.add(task)
            await db.flush()
            item.qa_task_id = task.id
            item.issue_id = issue.id
            qa_count += 1

        run.qa_total = qa_count
        if qa_count == 0:
            await self._queue_review_task(db, run=run)

        await db.commit()
        await db.refresh(run)
        return run

    async def list_set_runs(
        self,
        db: AsyncSession,
        *,
        set_id: str | None = None,
        status_filter: str | None = None,
        limit: int = 100,
    ) -> list[AISetRun]:
        query = select(AISetRun).order_by(AISetRun.created_at.desc())
        if set_id:
            query = query.where(AISetRun.set_id == set_id.strip().lower())
        if status_filter:
            query = query.where(AISetRun.status == status_filter)
        query = query.limit(limit)
        return list((await db.execute(query)).scalars().all())

    async def get_set_run(self, db: AsyncSession, *, set_run_id: str) -> AISetRun | None:
        return (
            await db.execute(select(AISetRun).where(AISetRun.id == set_run_id))
        ).scalar_one_or_none()

    async def list_set_run_items(self, db: AsyncSession, *, set_run_id: str) -> list[AISetRunItem]:
        return list(
            (
                await db.execute(
                    select(AISetRunItem)
                    .where(AISetRunItem.set_run_id == set_run_id)
                    .order_by(AISetRunItem.card_id.asc())
                )
            ).scalars().all()
        )

    async def cancel_set_run(self, db: AsyncSession, *, set_run_id: str) -> AISetRun | None:
        run = await self.get_set_run(db, set_run_id=set_run_id)
        if run is None:
            return None
        if run.status in {"completed", "failed", "canceled"}:
            return run

        run.status = "canceled"
        run.stage = "canceled"
        run.stopped_reason = run.stopped_reason or "Canceled by admin."
        run.completed_at = _utcnow()

        queued_tasks = (
            await db.execute(
                select(AITask).where(
                    AITask.set_run_id == run.id,
                    AITask.status == "queued",
                )
            )
        ).scalars().all()
        for task in queued_tasks:
            task.status = "failed"
            task.error_text = "Canceled by admin before execution."
            task.completed_at = _utcnow()

        items = await self.list_set_run_items(db, set_run_id=run.id)
        for item in items:
            if item.fix_task_id and item.fix_apply_status == "pending":
                item.fix_apply_status = "failed"
                item.error_text = "Canceled by admin."

        await self._refresh_run_counters(db, run)
        await db.commit()
        await db.refresh(run)
        return run

    async def on_task_started(self, task_id: str) -> None:
        async with async_session() as db:
            task = (
                await db.execute(select(AITask).where(AITask.id == task_id))
            ).scalar_one_or_none()
            if task is None or not task.set_run_id:
                return
            run = (
                await db.execute(select(AISetRun).where(AISetRun.id == task.set_run_id))
            ).scalar_one_or_none()
            if run is None:
                return
            await self._refresh_run_counters(db, run)
            await db.commit()

    async def on_task_finished(self, task_id: str) -> None:
        async with async_session() as db:
            task = (
                await db.execute(select(AITask).where(AITask.id == task_id))
            ).scalar_one_or_none()
            if task is None or not task.set_run_id:
                return
            run = (
                await db.execute(select(AISetRun).where(AISetRun.id == task.set_run_id))
            ).scalar_one_or_none()
            if run is None or run.status != "running":
                return

            if task.task_type == "llm_transpile":
                await self._handle_retranspile_task_finished(db, run=run, task=task)
            elif task.task_type == "qa_analysis":
                await self._handle_qa_task_finished(db, run=run)
            elif task.task_type == "review_batch":
                await self._handle_review_task_finished(db, run=run, task=task)
            elif task.task_type == "script_autofix":
                await self._handle_fix_task_finished(db, run=run, task=task)

            await self._refresh_run_counters(db, run)
            await self._finalize_run_if_complete(db, run)
            await db.commit()

    async def _handle_qa_task_finished(self, db: AsyncSession, *, run: AISetRun) -> None:
        qa_tasks = (
            await db.execute(
                select(AITask).where(
                    AITask.set_run_id == run.id,
                    AITask.task_type == "qa_analysis",
                )
            )
        ).scalars().all()
        if not qa_tasks:
            await self._queue_review_task(db, run=run)
            return

        if all(task.status in {"completed", "failed"} for task in qa_tasks):
            await self._queue_review_task(db, run=run)

    async def _queue_review_task(self, db: AsyncSession, *, run: AISetRun) -> None:
        # On re-review iterations, only check for queued tasks (not old completed ones)
        existing_queued = (
            await db.execute(
                select(AITask).where(
                    AITask.set_run_id == run.id,
                    AITask.task_type == "review_batch",
                    AITask.status == "queued",
                )
            )
        ).scalars().all()
        if existing_queued:
            run.stage = "review"
            run.review_total = len(existing_queued)
            return

        items = await self.list_set_run_items(db, set_run_id=run.id)
        # Only include items that need (re-)review
        items_to_review = [item for item in items if item.review_faithful is None]
        if not items_to_review:
            # All items already reviewed — advance to fix
            await self._advance_to_fix_from_review(db, run=run, items=items)
            return

        payload_cards = [
            {
                "card_id": item.card_id,
                "set_id": item.set_id,
                "module_name": item.module_name,
            }
            for item in items_to_review
        ]

        # Split into ~3 chunks to stay under rate limits
        chunk_size = max(1, math.ceil(len(payload_cards) / 3))
        chunks = [
            payload_cards[i : i + chunk_size]
            for i in range(0, len(payload_cards), chunk_size)
        ]

        item_by_card = {item.card_id: item for item in items_to_review}
        for chunk in chunks:
            payload = {"cards": chunk}
            estimate = self.dispatcher.estimate_cost("review_batch", payload, model_name=run.model_name)
            task = AITask(
                task_type="review_batch",
                payload_json=json.dumps(payload),
                status="queued",
                model_name=run.model_name,
                cost_estimate_usd=estimate,
                max_attempts=3,
                created_by=run.created_by,
                set_run_id=run.id,
                set_id=run.set_id,
            )
            db.add(task)
            await db.flush()
            for card in chunk:
                item = item_by_card.get(card["card_id"])
                if item is not None:
                    item.review_task_id = task.id

        run.stage = "review"
        run.review_total = len(chunks)

    async def _handle_review_task_finished(self, db: AsyncSession, *, run: AISetRun, task: AITask) -> None:
        # Process results from this individual review chunk
        if task.status == "completed":
            result_data = _load_json(task.result_json, {})
            cards_results = result_data.get("cards", {}) if isinstance(result_data, dict) else {}
            # Only update items that belong to this chunk
            chunk_items = (
                await db.execute(
                    select(AISetRunItem).where(
                        AISetRunItem.set_run_id == run.id,
                        AISetRunItem.review_task_id == task.id,
                    )
                )
            ).scalars().all()
            for item in chunk_items:
                review = cards_results.get(item.card_id, {}) if isinstance(cards_results, dict) else {}
                faithful = bool(review.get("faithful_to_card_text", False))
                item.review_faithful = 1 if faithful else 0
                if faithful:
                    item.fix_apply_status = "skipped"

        # Guard: if we already advanced past review, don't re-process
        if run.stage == "fix":
            return

        # Check if ALL review tasks are now done
        all_review_tasks = (
            await db.execute(
                select(AITask).where(
                    AITask.set_run_id == run.id,
                    AITask.task_type == "review_batch",
                )
            )
        ).scalars().all()
        if not all(t.status in {"completed", "failed"} for t in all_review_tasks):
            return  # Still waiting on other chunks

        # All review tasks finished — check if ALL failed
        if all(t.status == "failed" for t in all_review_tasks):
            error_msgs = [t.error_text or "Review task failed." for t in all_review_tasks]
            run.status = "failed"
            run.stopped_reason = "; ".join(error_msgs)
            run.completed_at = _utcnow()
            return

        # At least some succeeded — advance to fix stage with partial results
        items = await self.list_set_run_items(db, set_run_id=run.id)
        await self._advance_to_fix_from_review(db, run=run, items=items)

    async def _advance_to_fix_from_review(
        self, db: AsyncSession, *, run: AISetRun, items: list[AISetRunItem]
    ) -> None:
        """Transition from review to fix: create fix tasks for non-faithful items."""
        non_faithful: list[AISetRunItem] = []
        for item in items:
            if item.review_faithful is None:
                item.fix_apply_status = "skipped"
                continue
            if item.review_faithful == 1:
                continue  # already marked skipped during chunk processing
            non_faithful.append(item)
            issue = await self._upsert_non_faithful_issue(
                db, run=run, item=item, review_payload={},
            )
            item.issue_id = issue.id
            item.fix_apply_status = "pending"

        run.stage = "fix"

        if not non_faithful:
            # All items faithful — complete the run
            run.status = "completed"
            run.stage = "completed"
            run.completed_at = _utcnow()
            return

        max_fix = int(run.max_fix_tasks or 0)
        candidates = non_faithful if max_fix <= 0 else non_faithful[:max_fix]
        for item in non_faithful[max_fix:] if max_fix > 0 else []:
            item.fix_apply_status = "skipped"

        for item in candidates:
            issue = (
                await db.execute(select(Issue).where(Issue.id == item.issue_id))
            ).scalar_one_or_none()
            payload = {
                "card_id": item.card_id,
                "set_id": item.set_id,
                "module_name": item.module_name,
                "issue_id": item.issue_id,
                "issue_description": issue.description if issue else "Flagged as non-faithful in review stage.",
                "scope_profile": run.scope_profile,
            }
            estimate = self.dispatcher.estimate_cost(
                "script_autofix", payload, model_name=run.model_name,
            )
            fix_task = AITask(
                task_type="script_autofix",
                payload_json=json.dumps(payload),
                status="queued",
                model_name=run.model_name,
                cost_estimate_usd=estimate,
                max_attempts=3,
                created_by=run.created_by,
                run_mode="pr",
                scope_profile=run.scope_profile,
                set_run_id=run.id,
                set_id=item.set_id,
            )
            db.add(fix_task)
            await db.flush()
            item.fix_task_id = fix_task.id

    async def _upsert_non_faithful_issue(
        self,
        db: AsyncSession,
        *,
        run: AISetRun,
        item: AISetRunItem,
        review_payload: dict[str, Any],
    ) -> Issue:
        unresolved = (
            await db.execute(
                select(Issue)
                .where(
                    Issue.card_id == item.card_id,
                    Issue.status.in_(["new", "approved_for_ai"]),
                )
                .order_by(Issue.created_at.asc())
            )
        ).scalar_one_or_none()

        issues = review_payload.get("issues", []) if isinstance(review_payload, dict) else []
        if not isinstance(issues, list):
            issues = []
        issue_text = "; ".join(str(msg) for msg in issues if str(msg).strip())
        description = issue_text or f"{item.card_id} flagged non-faithful during set-run review."

        if unresolved is not None:
            unresolved.status = "approved_for_ai"
            unresolved.approved_by = run.created_by
            if description and description not in unresolved.triage_notes:
                prefix = unresolved.triage_notes.strip()
                unresolved.triage_notes = (
                    f"{prefix}\n[set-run {run.id}] {description}".strip()
                    if prefix
                    else f"[set-run {run.id}] {description}"
                )
            await db.flush()
            return unresolved

        created = Issue(
            card_id=item.card_id,
            description=description,
            source="system",
            severity="medium",
            status="approved_for_ai",
            created_by=run.created_by,
            approved_by=run.created_by,
            triage_notes=f"[set-run {run.id}] auto-created from review stage.",
        )
        db.add(created)
        await db.flush()
        return created

    async def _handle_fix_task_finished(self, db: AsyncSession, *, run: AISetRun, task: AITask) -> None:
        item = (
            await db.execute(
                select(AISetRunItem).where(
                    AISetRunItem.set_run_id == run.id,
                    AISetRunItem.fix_task_id == task.id,
                )
            )
        ).scalar_one_or_none()
        if item is None:
            return

        if task.status != "completed":
            item.fix_apply_status = "failed"
            item.error_text = task.error_text
            db.add(
                AIFixApplyAudit(
                    ai_task_id=task.id,
                    batch_id=None,
                    card_id=item.card_id,
                    scope_profile=str(task.scope_profile or run.scope_profile),
                    run_mode="pr",
                    applied_files_json="[]",
                    check_outputs_json=json.dumps([]),
                    commit_sha=None,
                    pr_url=None,
                    status="failed",
                    error_text=task.error_text or "Fix task failed before apply.",
                    created_by=run.created_by,
                )
            )
            return

        try:
            apply_info = await asyncio.to_thread(
                self._apply_fix_task_pr_mode,
                run=run,
                task=task,
            )
            item.fix_apply_status = "applied"
            item.commit_sha = apply_info["commit_sha"]
            item.pr_url = apply_info["pr_url"]
            item.error_text = None
            resolution_note = build_apply_resolution_note(
                task_id=task.id,
                card_id=item.card_id,
                run_mode="pr",
                commit_sha=apply_info.get("commit_sha"),
                pr_url=apply_info.get("pr_url"),
            )
            await resolve_open_issues_for_card(
                db,
                card_id=item.card_id,
                resolution_note=resolution_note,
            )
            db.add(
                AIFixApplyAudit(
                    ai_task_id=task.id,
                    batch_id=None,
                    card_id=item.card_id,
                    scope_profile=str(task.scope_profile or run.scope_profile),
                    run_mode="pr",
                    applied_files_json=json.dumps(apply_info["applied_files"]),
                    check_outputs_json=json.dumps(apply_info["check_outputs"]),
                    commit_sha=apply_info["commit_sha"],
                    pr_url=apply_info["pr_url"],
                    status="applied",
                    created_by=run.created_by,
                )
            )
        except Exception as exc:
            import traceback as _tb
            detail = f"{exc}\n{''.join(_tb.format_exception(exc))}"
            item.fix_apply_status = "failed"
            item.error_text = detail[:2000]
            db.add(
                AIFixApplyAudit(
                    ai_task_id=task.id,
                    batch_id=None,
                    card_id=item.card_id,
                    scope_profile=str(task.scope_profile or run.scope_profile),
                    run_mode="pr",
                    applied_files_json="[]",
                    check_outputs_json=json.dumps([]),
                    commit_sha=None,
                    pr_url=None,
                    status="failed",
                    error_text=detail[:2000],
                    created_by=run.created_by,
                )
            )

    def _apply_fix_task_pr_mode(self, *, run: AISetRun, task: AITask) -> dict[str, Any]:
        payload = _load_json(task.payload_json, {})
        result_payload = _load_json(task.result_json, {})
        card_id = str(payload.get("card_id", "")).strip().upper()
        set_id = str(payload.get("set_id", "")).strip().lower()
        module_name = str(payload.get("module_name", "")).strip().lower()
        scope_profile = str(task.scope_profile or run.scope_profile)
        if not card_id or not set_id or not module_name:
            raise ApplyValidationError("script_autofix payload missing card_id/set_id/module_name")

        edits = validate_edit_payload(
            result_payload=result_payload,
            scope_profile=scope_profile,
            set_id=set_id,
            module_name=module_name,
        )
        if not edits:
            raise ApplyValidationError("No edits returned by script_autofix task")

        self.git.preflight(run_mode="pr")
        ctx = self.git.prepare_worktree_for_task(task_id=task.id, run_mode="pr")
        applied_files = apply_validated_edits(repo_root=ctx.worktree_path, edits=edits)

        # Update frozen manifest hashes for any edited frozen-lane scripts
        self._update_manifest_hashes(ctx.worktree_path, card_id, applied_files)

        check_outputs = run_profile_checks(
            repo_root=ctx.worktree_path,
            scope_profile=scope_profile,
            applied_files=applied_files,
        )
        commit_files = list(applied_files)
        manifest_rel = "digimon_gym/engine/data/scripts/_frozen_manifest.json"
        if manifest_rel not in commit_files:
            commit_files.append(manifest_rel)
        commit_sha = self.git.commit_files(
            worktree_path=ctx.worktree_path,
            files=commit_files,
            message=f"AI set-run autofix {card_id}",
        )
        if commit_sha is None:
            raise ApplyValidationError("No diff produced after apply - nothing to commit.")
        pr_url = self.git.push_pr_branch_and_open_draft_pr(
            worktree_path=ctx.worktree_path,
            branch_name=ctx.branch_name,
            title=f"AI set-run autofix {card_id}",
            body=(
                f"Automated set-run fix for **{card_id}** (`{set_id}/{module_name}`).\n\n"
                f"Set run: `{run.id}`\nTask: `{task.id}`"
            ),
        )
        return {
            "applied_files": applied_files,
            "check_outputs": check_outputs,
            "commit_sha": commit_sha,
            "pr_url": pr_url,
        }

    @staticmethod
    def _update_manifest_hashes(
        worktree_path: Path, card_id: str, applied_files: list[str]
    ) -> None:
        """Update frozen manifest hashes in the worktree after editing scripts."""
        scripts_root = worktree_path / "digimon_gym" / "engine" / "data" / "scripts"
        manifest_path = scripts_root / "_frozen_manifest.json"
        if not manifest_path.exists():
            return

        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        cards = manifest.get("cards", {})
        updated = False

        for rel_path in applied_files:
            # Only update manifest for frozen-lane script edits
            if not rel_path.startswith("digimon_gym/engine/data/scripts/"):
                continue
            if "/generated/" in rel_path:
                continue
            abs_path = worktree_path / rel_path
            if not abs_path.exists():
                continue

            # Compute new hash (CRLF-normalized to match check_frozen_integrity.py)
            content = abs_path.read_bytes().replace(b"\r\n", b"\n")
            new_hash = hashlib.sha256(content).hexdigest()

            # Find matching manifest entry by card_id or relpath
            for cid, entry in cards.items():
                frozen_rel = entry.get("frozen_relpath", "").replace("\\", "/")
                script_rel = rel_path.replace(
                    "digimon_gym/engine/data/scripts/", ""
                ).replace("\\", "/")
                if frozen_rel == script_rel:
                    entry["frozen_hash"] = new_hash
                    updated = True
                    break

        if updated:
            manifest_path.write_text(
                json.dumps(manifest, indent=2, ensure_ascii=False) + "\n",
                encoding="utf-8",
            )

    def _score_cards(
        self,
        items: list[AISetRunItem],
        set_id: str,
        threshold: float,
        cs_dir: str | None = None,
    ) -> list[AISetRunItem]:
        """Compute transpile scores for all items.

        The C#->Python transpiler was removed; scoring is no longer meaningful
        here. Pipeline kept structurally for future repurposing toward user
        error reports against Rust scripts. Every item gets a zero score so
        downstream code paths that branch on the score behave deterministically.
        """
        for item in items:
            item.transpile_score = 0.0
        return items

    @staticmethod
    def _find_cs_file(cs_dir: str, module_name: str) -> Path | None:
        """Find C# source file matching a module name."""
        cs_dir_path = Path(cs_dir)
        if not cs_dir_path.exists():
            return None
        for pattern in [f"{module_name}.cs", f"{module_name.upper()}.cs"]:
            candidates = list(cs_dir_path.glob(f"**/{pattern}"))
            if candidates:
                return candidates[0]
        return None

    async def _queue_retranspile_tasks(
        self, db: AsyncSession, run: AISetRun, items: list[AISetRunItem], cs_dir: str | None
    ) -> int:
        """Create llm_transpile tasks for items below score threshold."""
        threshold = run.score_threshold or 0.7
        count = 0
        for item in items:
            if item.transpile_score is not None and item.transpile_score < threshold:
                # Load C# source if available
                cs_source = ""
                if cs_dir:
                    cs_path = self._find_cs_file(cs_dir, item.module_name)
                    if cs_path:
                        cs_source = cs_path.read_text(encoding="utf-8")

                # Load regex output
                gen_path = GENERATED_SCRIPTS_ROOT / item.set_id / f"{item.module_name}.py"
                regex_output = gen_path.read_text(encoding="utf-8") if gen_path.exists() else ""

                task = AITask(
                    task_type="llm_transpile",
                    status="queued",
                    set_run_id=run.id,
                    payload_json=json.dumps({
                        "card_id": item.card_id,
                        "set_id": item.set_id,
                        "module_name": item.module_name,
                        "cs_source": cs_source,
                        "regex_output": regex_output,
                        "regex_score": item.transpile_score,
                    }),
                    max_attempts=2,
                )
                db.add(task)
                await db.flush()
                item.retranspile_task_id = task.id
                count += 1

        run.retranspile_total = count
        run.stage = "retranspile" if count > 0 else "qa"
        await db.commit()
        return count

    async def _handle_retranspile_task_finished(
        self, db: AsyncSession, *, run: AISetRun, task: AITask
    ) -> None:
        """Write retranspiled script to disk and advance stage if all done."""
        item = (
            await db.execute(
                select(AISetRunItem).where(AISetRunItem.retranspile_task_id == task.id)
            )
        ).scalar_one_or_none()
        if not item:
            return

        if task.status == "completed" and task.result_json:
            result = _load_json(task.result_json, {})
            script_content = result.get("script_content", "")
            if script_content:
                out_path = GENERATED_SCRIPTS_ROOT / item.set_id / f"{item.module_name}.py"
                out_path.write_text(script_content, encoding="utf-8")
            run.retranspile_completed += 1
        else:
            run.retranspile_failed += 1

        # Check if all retranspile tasks done
        done = run.retranspile_completed + run.retranspile_failed
        if done >= run.retranspile_total:
            # Move to QA stage (re-queue review)
            await self._queue_review_task(db, run=run)

    async def _refresh_run_counters(self, db: AsyncSession, run: AISetRun) -> None:
        qa_tasks = (
            await db.execute(
                select(AITask).where(
                    AITask.set_run_id == run.id,
                    AITask.task_type == "qa_analysis",
                )
            )
        ).scalars().all()
        run.qa_total = len(qa_tasks)
        run.qa_completed = sum(1 for t in qa_tasks if t.status == "completed")
        run.qa_failed = sum(1 for t in qa_tasks if t.status == "failed")

        review_tasks = (
            await db.execute(
                select(AITask).where(
                    AITask.set_run_id == run.id,
                    AITask.task_type == "review_batch",
                )
            )
        ).scalars().all()
        run.review_total = len(review_tasks)
        run.review_completed = sum(1 for t in review_tasks if t.status == "completed")
        run.review_failed = sum(1 for t in review_tasks if t.status == "failed")

        items = await self.list_set_run_items(db, set_run_id=run.id)
        fix_items = [item for item in items if item.fix_task_id]
        run.fix_total = len(fix_items)
        run.fix_applied = sum(1 for item in fix_items if item.fix_apply_status == "applied")
        run.fix_failed = sum(1 for item in fix_items if item.fix_apply_status == "failed")
        run.fix_completed = run.fix_applied + run.fix_failed

        retranspile_tasks = (
            await db.execute(
                select(AITask).where(
                    AITask.set_run_id == run.id,
                    AITask.task_type == "llm_transpile",
                )
            )
        ).scalars().all()
        run.retranspile_total = len(retranspile_tasks)
        run.retranspile_completed = sum(1 for t in retranspile_tasks if t.status == "completed")
        run.retranspile_failed = sum(1 for t in retranspile_tasks if t.status == "failed")

    async def _consolidate_and_open_pr(self, db: AsyncSession, run: AISetRun) -> str | None:
        """Cherry-pick all applied fix commits into one branch, open combined PR, close individual PRs."""
        if run.run_mode != "pr":
            return None

        items = (
            await db.execute(
                select(AISetRunItem)
                .where(
                    AISetRunItem.set_run_id == run.id,
                    AISetRunItem.fix_apply_status == "applied",
                    AISetRunItem.commit_sha.isnot(None),
                )
                .order_by(AISetRunItem.card_id)
            )
        ).scalars().all()

        if not items:
            return None

        commit_shas = [item.commit_sha for item in items]
        individual_pr_urls = list({item.pr_url for item in items if item.pr_url})

        try:
            ctx, skipped = await asyncio.to_thread(
                self.git.create_consolidation_branch,
                set_id=run.set_id,
                run_id=run.id,
                commit_shas=commit_shas,
            )
        except Exception:
            logger.exception("Failed to create consolidation branch for run %s", run.id)
            return None

        picked_count = len(commit_shas) - len(skipped)
        if picked_count == 0:
            logger.warning("All cherry-picks failed for run %s, skipping consolidated PR", run.id)
            return None

        card_list = "\n".join(f"- {item.card_id}" for item in items if item.commit_sha not in skipped)
        body = (
            f"## Consolidated AI fixes for `{run.set_id}`\n\n"
            f"Set run: `{run.id}`\n"
            f"Applied: {picked_count} cards | Skipped: {len(skipped)}\n\n"
            f"### Fixed cards\n{card_list}"
        )

        try:
            pr_url = await asyncio.to_thread(
                self.git.push_pr_branch_and_open_draft_pr,
                worktree_path=ctx.worktree_path,
                branch_name=ctx.branch_name,
                title=f"AI set-run fixes: {run.set_id} ({picked_count} cards)",
                body=body,
            )
        except Exception:
            logger.exception("Failed to push consolidated PR for run %s", run.id)
            return None

        # Close individual per-card PRs
        for url in individual_pr_urls:
            try:
                await asyncio.to_thread(self.git.close_pr, url)
            except Exception:
                logger.debug("Failed to close individual PR %s", url)

        # Update all applied items and the run itself with the consolidated PR URL
        run.pr_url = pr_url
        for item in items:
            if item.commit_sha not in skipped:
                item.pr_url = pr_url

        return pr_url

    async def _finalize_run_if_complete(self, db: AsyncSession, run: AISetRun) -> None:
        if run.status != "running":
            return

        if run.stage == "qa" and run.qa_total == 0:
            await self._queue_review_task(db, run=run)
            return

        if run.stage == "review":
            review_tasks = (
                await db.execute(
                    select(AITask).where(
                        AITask.set_run_id == run.id,
                        AITask.task_type == "review_batch",
                    )
                )
            ).scalars().all()
            if review_tasks and all(t.status in {"completed", "failed"} for t in review_tasks):
                return

        if run.stage == "retranspile":
            retranspile_tasks = (
                await db.execute(
                    select(AITask).where(
                        AITask.set_run_id == run.id,
                        AITask.task_type == "llm_transpile",
                    )
                )
            ).scalars().all()
            if retranspile_tasks and all(t.status in {"completed", "failed"} for t in retranspile_tasks):
                return
            return

        if run.stage != "fix":
            return

        items = await self.list_set_run_items(db, set_run_id=run.id)
        tracked_fix_items = [item for item in items if item.fix_task_id]
        if not tracked_fix_items:
            await self._consolidate_and_open_pr(db, run=run)
            run.status = "completed"
            run.stage = "completed"
            run.completed_at = _utcnow()
            return

        if any(item.fix_apply_status == "pending" for item in tracked_fix_items):
            return

        # All fix tasks resolved — check if we should loop back for re-review
        applied_this_iteration = sum(
            1 for item in tracked_fix_items if item.fix_apply_status == "applied"
        )

        if applied_this_iteration == 0:
            # No progress — complete (convergence failure or all failed)
            await self._consolidate_and_open_pr(db, run=run)
            run.status = "completed"
            run.stage = "completed"
            run.completed_at = _utcnow()
            run.stopped_reason = f"Completed with {run.fix_failed} fix failures."
            return

        max_iters = int(run.max_fix_iterations or 3)
        if run.fix_iteration + 1 >= max_iters:
            # Safety limit reached
            still_not_faithful = sum(
                1 for item in items
                if item.review_faithful == 0 and item.fix_apply_status != "applied"
            )
            await self._consolidate_and_open_pr(db, run=run)
            run.status = "completed"
            run.stage = "completed"
            run.completed_at = _utcnow()
            run.stopped_reason = (
                f"Max iterations ({max_iters}) reached. "
                f"{still_not_faithful} items still not faithful."
            )
            return

        # Loop back: re-review items that were applied in this iteration
        run.fix_iteration += 1
        for item in tracked_fix_items:
            if item.fix_apply_status == "applied":
                item.review_faithful = None
                item.fix_apply_status = "pending"
                item.fix_task_id = None
                item.review_task_id = None

        # Reset counters for the new iteration
        run.review_total = 0
        run.review_completed = 0
        run.review_failed = 0
        run.fix_total = 0
        run.fix_completed = 0
        run.fix_failed = 0
        run.fix_applied = 0

        run.stage = "review"
        await self._queue_review_task(db, run=run)


set_run_orchestrator = AISetRunOrchestrator()
