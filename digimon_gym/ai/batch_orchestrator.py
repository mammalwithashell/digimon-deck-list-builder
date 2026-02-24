"""Batch orchestration for script_autofix tasks."""

from __future__ import annotations

import asyncio
import json
import os
from datetime import datetime, timezone
from typing import Any

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from digimon_gym.ai.autofix_apply import (
    ApplyValidationError,
    apply_validated_edits,
    build_file_context_for_task,
    get_allowed_roots_for_scope,
    run_profile_checks,
    validate_edit_payload,
)
from digimon_gym.ai.dispatcher import TaskDispatcher
from digimon_gym.ai.git_adapter import GitAdapter, GitCommandError
from digimon_gym.db.database import async_session
from digimon_gym.db.models import (
    AIFixApplyAudit,
    AIFixBatch,
    AIFixBatchItem,
    AITask,
    Issue,
)


AI_APPLY_MAIN_ALLOWED_ENV = "AI_APPLY_MAIN_ALLOWED"
DEFAULT_FAILURE_MIN_SAMPLE = int(os.getenv("AI_BATCH_FAILURE_MIN_SAMPLE", "10"))

TERMINAL_BATCH_ITEM_STATUSES = {"applied", "failed", "canceled"}
ACTIVE_BATCH_ITEM_STATUSES = {"queued", "running"}


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
    normalized = card_id.strip().upper()
    module = normalized.replace("-", "_").lower()
    set_id = module.split("_", 1)[0]
    return set_id, module


class BatchOrchestrationError(RuntimeError):
    """Top-level orchestration failure."""


class AIFixBatchOrchestrator:
    def __init__(self) -> None:
        self.dispatcher = TaskDispatcher()
        self.git = GitAdapter()

    async def preview_eligible_issues(
        self,
        db: AsyncSession,
        *,
        set_id: str,
        max_tasks: int,
    ) -> list[Issue]:
        prefix = f"{set_id.upper()}-%"
        q = (
            select(Issue)
            .where(Issue.status == "approved_for_ai", Issue.card_id.like(prefix))
            .order_by(Issue.created_at.asc())
        )
        if max_tasks > 0:
            q = q.limit(max_tasks)
        rows = (await db.execute(q)).scalars().all()
        return list(rows)

    async def create_batch(
        self,
        db: AsyncSession,
        *,
        created_by: str | None,
        set_id: str,
        run_mode: str,
        scope_profile: str,
        model_name: str | None,
        concurrency: int,
        max_total_cost_usd: float,
        failure_rate_stop: float,
        max_tasks: int,
        dry_run: bool,
    ) -> tuple[AIFixBatch | None, list[str], list[str], int]:
        if run_mode == "main" and os.getenv(AI_APPLY_MAIN_ALLOWED_ENV, "0") != "1":
            raise BatchOrchestrationError(
                f"run_mode=main is disabled. Set {AI_APPLY_MAIN_ALLOWED_ENV}=1 to enable."
            )

        eligible_issues = await self.preview_eligible_issues(
            db,
            set_id=set_id,
            max_tasks=max_tasks,
        )
        eligible_count = len(eligible_issues)
        cards = [issue.card_id for issue in eligible_issues]

        if dry_run:
            return None, [], cards, eligible_count

        if eligible_count > 0:
            self.git.preflight(run_mode=run_mode)

        batch = AIFixBatch(
            set_id=set_id.lower(),
            run_mode=run_mode,
            scope_profile=scope_profile,
            status="running",
            created_by=created_by,
            model_name=model_name,
            concurrency=concurrency,
            max_total_cost_usd=max_total_cost_usd,
            failure_rate_stop=failure_rate_stop,
            max_tasks=max_tasks,
        )
        db.add(batch)
        await db.flush()

        for issue in eligible_issues:
            db.add(
                AIFixBatchItem(
                    batch_id=batch.id,
                    issue_id=issue.id,
                    card_id=issue.card_id,
                    status="pending",
                )
            )

        await db.flush()
        queued_task_ids = await self._schedule_pending_tasks(db, batch)
        await self._refresh_batch_counters(db, batch)

        if eligible_count == 0:
            batch.status = "failed_no_changes"
            batch.stopped_reason = "No approved_for_ai issues matched requested set."

        await db.commit()
        await db.refresh(batch)
        return batch, queued_task_ids, cards, eligible_count

    async def list_batches(self, db: AsyncSession, *, limit: int = 100) -> list[AIFixBatch]:
        rows = (
            await db.execute(
                select(AIFixBatch).order_by(AIFixBatch.created_at.desc()).limit(limit)
            )
        ).scalars().all()
        return list(rows)

    async def get_batch(self, db: AsyncSession, *, batch_id: str) -> AIFixBatch | None:
        return (
            await db.execute(select(AIFixBatch).where(AIFixBatch.id == batch_id))
        ).scalar_one_or_none()

    async def list_batch_items(self, db: AsyncSession, *, batch_id: str) -> list[AIFixBatchItem]:
        rows = (
            await db.execute(
                select(AIFixBatchItem)
                .where(AIFixBatchItem.batch_id == batch_id)
                .order_by(AIFixBatchItem.created_at.asc())
            )
        ).scalars().all()
        return list(rows)

    async def cancel_batch(self, db: AsyncSession, *, batch_id: str) -> AIFixBatch | None:
        batch = await self.get_batch(db, batch_id=batch_id)
        if batch is None:
            return None
        if batch.status in {"completed", "failed", "failed_no_changes", "canceled"}:
            return batch

        batch.status = "canceled"
        batch.stopped_reason = batch.stopped_reason or "Canceled by admin."

        queued_tasks = (
            await db.execute(
                select(AITask).where(
                    AITask.batch_id == batch.id,
                    AITask.status == "queued",
                )
            )
        ).scalars().all()
        for task in queued_tasks:
            task.status = "failed"
            task.error_text = "Canceled by admin before execution."
            task.completed_at = _utcnow()

        queued_items = (
            await db.execute(
                select(AIFixBatchItem).where(
                    AIFixBatchItem.batch_id == batch.id,
                    AIFixBatchItem.status.in_(["pending", "queued"]),
                )
            )
        ).scalars().all()
        for item in queued_items:
            item.status = "canceled"
            item.error_text = "Canceled by admin."

        await self._refresh_batch_counters(db, batch)
        await db.commit()
        await db.refresh(batch)
        return batch

    async def on_task_started(self, task_id: str) -> None:
        async with async_session() as db:
            task = (
                await db.execute(select(AITask).where(AITask.id == task_id))
            ).scalar_one_or_none()
            if task is None or not task.batch_id:
                return
            item = (
                await db.execute(select(AIFixBatchItem).where(AIFixBatchItem.task_id == task_id))
            ).scalar_one_or_none()
            batch = (
                await db.execute(select(AIFixBatch).where(AIFixBatch.id == task.batch_id))
            ).scalar_one_or_none()
            if batch is None or item is None:
                return
            if item.status in {"queued", "pending"}:
                item.status = "running"
            await self._refresh_batch_counters(db, batch)
            await db.commit()

    async def on_task_finished(self, task_id: str) -> None:
        async with async_session() as db:
            task = (
                await db.execute(select(AITask).where(AITask.id == task_id))
            ).scalar_one_or_none()
            if task is None or not task.batch_id:
                return

            batch = (
                await db.execute(select(AIFixBatch).where(AIFixBatch.id == task.batch_id))
            ).scalar_one_or_none()
            item = (
                await db.execute(select(AIFixBatchItem).where(AIFixBatchItem.task_id == task.id))
            ).scalar_one_or_none()
            if batch is None or item is None:
                return

            if task.status == "failed":
                item.status = "failed"
                item.error_text = task.error_text
            elif task.status == "completed":
                item.status = "completed"
                if batch.status == "running":
                    try:
                        apply_info = await asyncio.to_thread(
                            self._apply_and_commit_task,
                            batch=batch,
                            task=task,
                        )
                        item.status = "applied"
                        item.applied_at = _utcnow()
                        item.commit_sha = apply_info["commit_sha"]
                        item.error_text = None
                        db.add(
                            AIFixApplyAudit(
                                ai_task_id=task.id,
                                batch_id=batch.id,
                                card_id=item.card_id,
                                scope_profile=str(task.scope_profile or batch.scope_profile),
                                run_mode=str(task.run_mode or batch.run_mode),
                                applied_files_json=json.dumps(apply_info["applied_files"]),
                                check_outputs_json=json.dumps(apply_info["check_outputs"]),
                                commit_sha=apply_info["commit_sha"],
                                status="applied",
                            )
                        )
                    except Exception as exc:
                        item.status = "failed"
                        item.error_text = str(exc)
                        db.add(
                            AIFixApplyAudit(
                                ai_task_id=task.id,
                                batch_id=batch.id,
                                card_id=item.card_id,
                                scope_profile=str(task.scope_profile or batch.scope_profile),
                                run_mode=str(task.run_mode or batch.run_mode),
                                applied_files_json="[]",
                                check_outputs_json=json.dumps([]),
                                commit_sha=None,
                                status="failed",
                                error_text=str(exc),
                            )
                        )

            await self._refresh_batch_counters(db, batch)
            if batch.status == "running":
                await self._maybe_stop_for_guards(db, batch)
                if batch.status == "running":
                    await self._schedule_pending_tasks(db, batch)
                    await self._refresh_batch_counters(db, batch)

            await self._finalize_if_complete(db, batch)
            await db.commit()

    async def _schedule_pending_tasks(self, db: AsyncSession, batch: AIFixBatch) -> list[str]:
        if batch.status != "running":
            return []

        await self._refresh_batch_counters(db, batch)
        active_count = int(batch.queued_count or 0) + int(batch.running_count or 0)
        slots = max(0, int(batch.concurrency or 1) - active_count)
        if slots <= 0:
            return []

        pending_items = (
            await db.execute(
                select(AIFixBatchItem)
                .where(
                    AIFixBatchItem.batch_id == batch.id,
                    AIFixBatchItem.status == "pending",
                    AIFixBatchItem.task_id.is_(None),
                )
                .order_by(AIFixBatchItem.created_at.asc())
                .limit(slots)
            )
        ).scalars().all()

        queued: list[str] = []
        for item in pending_items:
            if batch.status != "running":
                break
            await self._maybe_stop_for_guards(db, batch)
            if batch.status != "running":
                break

            issue = None
            if item.issue_id:
                issue = (
                    await db.execute(select(Issue).where(Issue.id == item.issue_id))
                ).scalar_one_or_none()
            set_id, module_name = _card_to_set_module(item.card_id)
            issue_text = issue.description if issue else ""
            payload = {
                "card_id": item.card_id,
                "set_id": set_id,
                "module_name": module_name,
                "issue_id": item.issue_id,
                "issue_description": issue_text,
                "scope_profile": batch.scope_profile,
            }
            estimate = self.dispatcher.estimate_cost(
                "script_autofix",
                payload,
                model_name=batch.model_name,
            )
            task = AITask(
                task_type="script_autofix",
                payload_json=json.dumps(payload),
                status="queued",
                model_name=batch.model_name,
                cost_estimate_usd=estimate,
                max_attempts=3,
                created_by=batch.created_by,
                batch_id=batch.id,
                run_mode=batch.run_mode,
                scope_profile=batch.scope_profile,
            )
            db.add(task)
            await db.flush()
            item.task_id = task.id
            item.status = "queued"
            queued.append(task.id)

        return queued

    async def _refresh_batch_counters(self, db: AsyncSession, batch: AIFixBatch) -> None:
        items = (
            await db.execute(select(AIFixBatchItem).where(AIFixBatchItem.batch_id == batch.id))
        ).scalars().all()
        queued = 0
        running = 0
        completed = 0
        failed = 0
        applied = 0
        commits = 0
        for item in items:
            if item.status == "queued":
                queued += 1
            elif item.status == "running":
                running += 1
            elif item.status == "completed":
                completed += 1
            elif item.status == "failed":
                failed += 1
            elif item.status == "applied":
                completed += 1
                applied += 1
            if item.commit_sha:
                commits += 1

        batch.queued_count = queued
        batch.running_count = running
        batch.completed_count = completed
        batch.failed_count = failed
        batch.applied_count = applied
        batch.commit_count = commits

    async def _batch_actual_cost(self, db: AsyncSession, *, batch_id: str) -> float:
        tasks = (
            await db.execute(select(AITask).where(AITask.batch_id == batch_id))
        ).scalars().all()
        return float(sum(float(t.cost_actual_usd or 0.0) for t in tasks))

    async def _maybe_stop_for_guards(self, db: AsyncSession, batch: AIFixBatch) -> None:
        if batch.status != "running":
            return

        cost_actual = await self._batch_actual_cost(db, batch_id=batch.id)
        if float(batch.max_total_cost_usd or 0.0) > 0 and cost_actual > float(batch.max_total_cost_usd):
            batch.status = "failed"
            batch.stopped_reason = (
                f"Stopped by cost cap: ${cost_actual:.4f} > ${float(batch.max_total_cost_usd):.4f}"
            )
            return

        sample = int(batch.completed_count or 0) + int(batch.failed_count or 0)
        if sample >= DEFAULT_FAILURE_MIN_SAMPLE:
            fail_rate = float(batch.failed_count or 0) / float(sample)
            if fail_rate > float(batch.failure_rate_stop or 0.0):
                batch.status = "failed"
                batch.stopped_reason = (
                    f"Stopped by failure-rate guard: {fail_rate:.2%} > {float(batch.failure_rate_stop):.2%}"
                )

    async def _finalize_if_complete(self, db: AsyncSession, batch: AIFixBatch) -> None:
        items = (
            await db.execute(select(AIFixBatchItem).where(AIFixBatchItem.batch_id == batch.id))
        ).scalars().all()
        if any(item.status not in TERMINAL_BATCH_ITEM_STATUSES for item in items):
            return

        if int(batch.commit_count or 0) <= 0:
            batch.status = "failed_no_changes"
            batch.stopped_reason = batch.stopped_reason or "No successful card commits."
            return

        if batch.status == "canceled":
            return

        try:
            if batch.run_mode == "pr":
                ctx = self.git.prepare_worktree(
                    set_id=batch.set_id,
                    batch_id=batch.id,
                    run_mode=batch.run_mode,
                )
                title = f"AI Fix Batch {batch.set_id} ({batch.id})"
                body = (
                    f"Applied cards: {batch.applied_count}\n"
                    f"Failed cards: {batch.failed_count}\n"
                    f"Stop reason: {batch.stopped_reason or 'none'}\n"
                )
                batch.pr_url = self.git.push_pr_branch_and_open_draft_pr(
                    worktree_path=ctx.worktree_path,
                    branch_name=ctx.branch_name,
                    title=title,
                    body=body,
                )
                batch.status = "completed"
                return

            if batch.run_mode == "main":
                if os.getenv(AI_APPLY_MAIN_ALLOWED_ENV, "0") != "1":
                    raise BatchOrchestrationError(
                        f"run_mode=main disabled by {AI_APPLY_MAIN_ALLOWED_ENV}"
                    )
                ctx = self.git.prepare_worktree(
                    set_id=batch.set_id,
                    batch_id=batch.id,
                    run_mode=batch.run_mode,
                )
                self.git.push_to_main(worktree_path=ctx.worktree_path)
                batch.status = "completed"
                return
        except (GitCommandError, BatchOrchestrationError) as exc:
            batch.status = "failed"
            batch.stopped_reason = str(exc)

    def _apply_and_commit_task(self, *, batch: AIFixBatch, task: AITask) -> dict[str, Any]:
        payload = _load_json(task.payload_json, {})
        result = _load_json(task.result_json, {})

        card_id = str(payload.get("card_id", "")).strip().upper()
        set_id = str(payload.get("set_id", "")).strip().lower()
        module_name = str(payload.get("module_name", "")).strip().lower()
        scope_profile = str(task.scope_profile or batch.scope_profile)
        if not card_id or not set_id or not module_name:
            raise ApplyValidationError("script_autofix payload missing card_id/set_id/module_name")

        edits = validate_edit_payload(
            result_payload=result,
            scope_profile=scope_profile,
            set_id=set_id,
            module_name=module_name,
        )
        if not edits:
            raise ApplyValidationError("No edits returned by script_autofix task")

        self.git.preflight(run_mode=batch.run_mode)
        ctx = self.git.prepare_worktree(set_id=batch.set_id, batch_id=batch.id, run_mode=batch.run_mode)
        applied_files = apply_validated_edits(repo_root=ctx.worktree_path, edits=edits)
        check_outputs = run_profile_checks(
            repo_root=ctx.worktree_path,
            scope_profile=scope_profile,
            applied_files=applied_files,
        )
        commit_sha = self.git.commit_files(
            worktree_path=ctx.worktree_path,
            files=applied_files,
            message=f"AI autofix {card_id}",
        )
        if not commit_sha:
            raise ApplyValidationError("No git diff after applying edits; skipping commit.")
        return {
            "applied_files": applied_files,
            "check_outputs": check_outputs,
            "commit_sha": commit_sha,
        }

    def build_autofix_prompt_context(self, *, payload: dict[str, Any]) -> tuple[list[str], list[dict[str, str]]]:
        set_id = str(payload.get("set_id", "")).strip().lower()
        module_name = str(payload.get("module_name", "")).strip().lower()
        scope_profile = str(payload.get("scope_profile", "script"))
        allowed_roots = get_allowed_roots_for_scope(scope_profile)
        file_contexts = build_file_context_for_task(
            set_id=set_id,
            module_name=module_name,
            scope_profile=scope_profile,
        )
        return allowed_roots, file_contexts


batch_orchestrator = AIFixBatchOrchestrator()
