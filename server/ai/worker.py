"""In-process AI task worker with DB-backed queue semantics."""

from __future__ import annotations

import asyncio
import json
import logging
import os
from datetime import datetime, timedelta, timezone
from uuid import uuid4

from sqlalchemy import or_, select, update

from digimon_gym.ai.batch_orchestrator import batch_orchestrator
from digimon_gym.ai.dispatcher import TaskDispatcher
from digimon_gym.ai.set_run_orchestrator import set_run_orchestrator
from digimon_gym.db.database import async_session
from digimon_gym.db.models import AITask
from digimon_gym.env import load_project_env

load_project_env()

logger = logging.getLogger(__name__)

_RETRYABLE_ERROR_NAMES = frozenset({
    "RateLimitError",
    "APITimeoutError",
    "InternalServerError",
    "APIConnectionError",
})


def _is_tpm_request_too_large_error(exc: Exception) -> bool:
    text = str(exc).lower()
    return (
        "request too large" in text
        and "tokens per min" in text
        and "rate_limit_exceeded" in text
    )


def _is_retryable_error(exc: Exception) -> bool:
    """Return True for transient LLM API errors that may succeed on retry.

    Uses string-based type checking to avoid a hard import dependency on the
    openai package.
    """
    if isinstance(exc, (ConnectionError, TimeoutError)):
        return True
    return type(exc).__name__ in _RETRYABLE_ERROR_NAMES


class AITaskWorker:
    def __init__(self) -> None:
        self.dispatcher = TaskDispatcher()
        self.poll_interval_seconds = float(os.getenv("AI_WORKER_POLL_SECONDS", "2.0"))
        self.stale_after_seconds = int(os.getenv("AI_WORKER_STALE_SECONDS", "1800"))
        self.max_task_cost_usd = float(os.getenv("AI_TASK_MAX_COST_USD", "5.0"))
        self.max_concurrent = max(1, int(os.getenv("AI_WORKER_MAX_CONCURRENT", "1")))
        self._worker_id = f"ai-worker-{uuid4().hex[:8]}"
        self._task: asyncio.Task | None = None
        self._stop = asyncio.Event()
        self._running_tasks: set[asyncio.Task] = set()

    async def start(self) -> None:
        if self._task and not self._task.done():
            return
        self._stop.clear()
        self._task = asyncio.create_task(self._loop(), name="ai-task-worker")
        logger.info(
            "AI worker started (id=%s, max_concurrent=%d)",
            self._worker_id,
            self.max_concurrent,
        )

    async def stop(self) -> None:
        self._stop.set()
        if self._running_tasks:
            done, pending = await asyncio.wait(self._running_tasks, timeout=30)
            for task in done:
                if task.cancelled():
                    logger.warning("AI task execution cancelled: %s", task.get_name())
                elif task.exception() is not None:
                    logger.error("AI task execution error in %s: %s", task.get_name(), task.exception())
            for task in pending:
                task.cancel()
            self._running_tasks.clear()
        if self._task:
            await asyncio.wait([self._task], timeout=5)
        logger.info("AI worker stopped (id=%s)", self._worker_id)

    async def _loop(self) -> None:
        while not self._stop.is_set():
            try:
                await self._recover_stale_running_tasks()
                self._prune_finished_tasks()

                capacity = self.max_concurrent - len(self._running_tasks)
                if capacity <= 0:
                    await asyncio.sleep(self.poll_interval_seconds)
                    continue

                claimed_task_ids = await self._claim_tasks(capacity)
                if not claimed_task_ids:
                    await asyncio.sleep(self.poll_interval_seconds)
                    continue

                for task_id in claimed_task_ids:
                    run_task = asyncio.create_task(
                        self._run_task(task_id),
                        name=f"ai-task-{task_id[:8]}",
                    )
                    self._running_tasks.add(run_task)
            except Exception:
                logger.exception("Unexpected worker loop failure")
                await asyncio.sleep(self.poll_interval_seconds)

    def _prune_finished_tasks(self) -> None:
        done_tasks = {task for task in self._running_tasks if task.done()}
        for task in done_tasks:
            if task.cancelled():
                logger.warning("AI task execution cancelled: %s", task.get_name())
            elif task.exception() is not None:
                logger.error("AI task execution error in %s: %s", task.get_name(), task.exception())
        self._running_tasks -= done_tasks

    async def _recover_stale_running_tasks(self) -> None:
        stale_before = datetime.now(timezone.utc) - timedelta(seconds=self.stale_after_seconds)
        async with async_session() as db:
            result = await db.execute(
                select(AITask).where(
                    AITask.status == "running",
                    AITask.started_at.is_not(None),
                    AITask.started_at < stale_before,
                    AITask.completed_at.is_(None),
                    or_(
                        AITask.worker_id.is_(None),
                        AITask.worker_id != self._worker_id,
                    ),
                )
            )
            stale_tasks = result.scalars().all()
            if not stale_tasks:
                return
            for task in stale_tasks:
                task.status = "failed"
                task.error_text = "Worker recovered stale running task"
                task.completed_at = datetime.now(timezone.utc)
                task.worker_id = None
                task.claimed_at = None
            await db.commit()

            for task in stale_tasks:
                await self._notify_task_finished(task.id)

    async def _claim_tasks(self, count: int) -> list[str]:
        claimed: list[str] = []

        for _ in range(max(0, count)):
            now = datetime.now(timezone.utc)
            async with async_session() as db:
                oldest = await db.execute(
                    select(AITask.id)
                    .where(AITask.status == "queued")
                    .order_by(AITask.created_at.asc())
                    .limit(1)
                )
                row = oldest.one_or_none()
                if row is None:
                    break

                task_id = str(row[0])
                result = await db.execute(
                    update(AITask)
                    .where(
                        AITask.id == task_id,
                        AITask.status == "queued",
                    )
                    .values(
                        status="running",
                        attempts=AITask.attempts + 1,
                        started_at=now,
                        completed_at=None,
                        error_text=None,
                        worker_id=self._worker_id,
                        claimed_at=now,
                    )
                )
                if int(result.rowcount or 0) == 0:
                    await db.commit()
                    continue

                await db.commit()
                claimed.append(task_id)

        return claimed

    async def _process_one_task(self) -> bool:
        """Compatibility helper used by existing tests."""
        claimed = await self._claim_tasks(1)
        if not claimed:
            return False
        await self._run_task(claimed[0])
        return True

    async def _notify_task_started(self, task_id: str) -> None:
        try:
            await batch_orchestrator.on_task_started(task_id)
        except Exception:
            logger.exception("Batch hook failed on task start %s", task_id)
        try:
            await set_run_orchestrator.on_task_started(task_id)
        except Exception:
            logger.exception("Set-run hook failed on task start %s", task_id)

    async def _notify_task_finished(self, task_id: str) -> None:
        try:
            await batch_orchestrator.on_task_finished(task_id)
        except Exception:
            logger.exception("Batch hook failed on task finish %s", task_id)
        try:
            await set_run_orchestrator.on_task_finished(task_id)
        except Exception:
            logger.exception("Set-run hook failed on task finish %s", task_id)

    async def _run_task(self, task_id: str) -> None:
        await self._notify_task_started(task_id)

        payload: dict[str, object] = {}
        task_type = ""
        model_name: str | None = None
        attempts = 0
        max_attempts = 0

        async with async_session() as db:
            task = (await db.execute(select(AITask).where(AITask.id == task_id))).scalar_one_or_none()
            if task is None or task.status != "running":
                return

            if float(task.cost_estimate_usd or 0.0) > self.max_task_cost_usd:
                task.status = "failed"
                task.error_text = (
                    f"Task cost estimate {task.cost_estimate_usd:.4f} exceeds hard cap {self.max_task_cost_usd:.4f}"
                )
                task.completed_at = datetime.now(timezone.utc)
                task.worker_id = None
                task.claimed_at = None
                await db.commit()
                await self._notify_task_finished(task.id)
                return

            try:
                payload = json.loads(task.payload_json or "{}")
            except json.JSONDecodeError:
                task.status = "failed"
                task.error_text = "Invalid payload_json"
                task.completed_at = datetime.now(timezone.utc)
                task.worker_id = None
                task.claimed_at = None
                await db.commit()
                await self._notify_task_finished(task.id)
                return

            task_type = task.task_type
            model_name = task.model_name
            attempts = int(task.attempts or 0)
            max_attempts = int(task.max_attempts or 3)

        try:
            outcome = await asyncio.to_thread(
                self.dispatcher.run,
                task_type,
                payload,
                model_name,
            )
            status_after = "completed"
            error_text = None
        except Exception as exc:
            outcome = None
            error_text = str(exc)
            if (
                _is_tpm_request_too_large_error(exc)
                and str(model_name or "").strip() in {"", "gpt-4.1"}
                and attempts < max_attempts
            ):
                status_after = "queued"
                model_name = "gpt-4.1-mini"
                error_text = f"Downgraded model due to TPM overflow: {exc}"
                logger.warning(
                    "TPM overflow on task %s, retrying on gpt-4.1-mini (attempt %s/%s)",
                    task_id,
                    attempts,
                    max_attempts,
                )
            elif _is_retryable_error(exc) and attempts < max_attempts:
                status_after = "queued"
                error_text = f"Retry {attempts}/{max_attempts}: {exc}"
                logger.warning(
                    "Retryable error on task %s (attempt %s/%s): %s",
                    task_id,
                    attempts,
                    max_attempts,
                    exc,
                )
            else:
                status_after = "failed"

        async with async_session() as db:
            task = (await db.execute(select(AITask).where(AITask.id == task_id))).scalar_one_or_none()
            if task is None:
                return
            if task.status != "running":
                if task.worker_id == self._worker_id:
                    task.worker_id = None
                    task.claimed_at = None
                    await db.commit()
                return

            if status_after == "completed" and outcome is not None:
                task.status = "completed"
                task.result_json = json.dumps(outcome.result)
                task.sanitized_input_json = json.dumps(outcome.sanitized_input)
                task.retrieval_refs_json = json.dumps(outcome.retrieval_refs)
                task.model_name = outcome.model_name
                task.input_tokens = outcome.input_tokens
                task.output_tokens = outcome.output_tokens
                task.cost_actual_usd = outcome.cost_actual
                task.completed_at = datetime.now(timezone.utc)
                task.error_text = None
                task.worker_id = None
                task.claimed_at = None
            elif status_after == "queued":
                task.status = "queued"
                task.model_name = model_name
                task.error_text = error_text
                task.started_at = None
                task.completed_at = None
                task.worker_id = None
                task.claimed_at = None
            else:
                task.status = "failed"
                task.error_text = error_text
                task.completed_at = datetime.now(timezone.utc)
                task.worker_id = None
                task.claimed_at = None

            final_status = task.status
            await db.commit()

        if final_status in {"completed", "failed"}:
            await self._notify_task_finished(task_id)


ai_task_worker = AITaskWorker()
