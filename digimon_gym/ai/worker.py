"""In-process AI task worker with DB-backed queue semantics."""

from __future__ import annotations

import asyncio
import json
import logging
import os
from datetime import datetime, timedelta, timezone

from sqlalchemy import select

from digimon_gym.ai.batch_orchestrator import batch_orchestrator
from digimon_gym.ai.dispatcher import TaskDispatcher
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
        self._task: asyncio.Task | None = None
        self._stop = asyncio.Event()

    async def start(self) -> None:
        if self._task and not self._task.done():
            return
        self._stop.clear()
        self._task = asyncio.create_task(self._loop(), name="ai-task-worker")
        logger.info("AI worker started")

    async def stop(self) -> None:
        self._stop.set()
        if self._task:
            await asyncio.wait([self._task], timeout=5)
        logger.info("AI worker stopped")

    async def _loop(self) -> None:
        while not self._stop.is_set():
            try:
                await self._recover_stale_running_tasks()
                processed = await self._process_one_task()
                if not processed:
                    await asyncio.sleep(self.poll_interval_seconds)
            except Exception:
                logger.exception("Unexpected worker loop failure")
                await asyncio.sleep(self.poll_interval_seconds)

    async def _recover_stale_running_tasks(self) -> None:
        stale_before = datetime.now(timezone.utc) - timedelta(seconds=self.stale_after_seconds)
        async with async_session() as db:
            result = await db.execute(
                select(AITask).where(
                    AITask.status == "running",
                    AITask.started_at.is_not(None),
                    AITask.started_at < stale_before,
                    AITask.completed_at.is_(None),
                )
            )
            stale_tasks = result.scalars().all()
            if not stale_tasks:
                return
            for task in stale_tasks:
                task.status = "failed"
                task.error_text = "Worker recovered stale running task"
                task.completed_at = datetime.now(timezone.utc)
            await db.commit()
            for task in stale_tasks:
                try:
                    await batch_orchestrator.on_task_finished(task.id)
                except Exception:
                    logger.exception("Batch hook failed while recovering stale task %s", task.id)

    async def _process_one_task(self) -> bool:
        async with async_session() as db:
            result = await db.execute(
                select(AITask)
                .where(AITask.status == "queued")
                .order_by(AITask.created_at.asc())
                .limit(1)
            )
            task = result.scalar_one_or_none()
            if task is None:
                return False

            if float(task.cost_estimate_usd or 0.0) > self.max_task_cost_usd:
                task.status = "failed"
                task.error_text = (
                    f"Task cost estimate {task.cost_estimate_usd:.4f} exceeds hard cap {self.max_task_cost_usd:.4f}"
                )
                task.completed_at = datetime.now(timezone.utc)
                await db.commit()
                return True

            task.status = "running"
            task.attempts = int(task.attempts or 0) + 1
            task.started_at = datetime.now(timezone.utc)
            task.completed_at = None
            task.error_text = None
            await db.commit()
            await db.refresh(task)
            try:
                await batch_orchestrator.on_task_started(task.id)
            except Exception:
                logger.exception("Batch hook failed on task start %s", task.id)

            payload = {}
            try:
                payload = json.loads(task.payload_json or "{}")
            except json.JSONDecodeError:
                task.status = "failed"
                task.error_text = "Invalid payload_json"
                task.completed_at = datetime.now(timezone.utc)
                await db.commit()
                return True

            try:
                outcome = await asyncio.to_thread(
                    self.dispatcher.run,
                    task.task_type,
                    payload,
                    task.model_name,
                )
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
            except Exception as exc:
                max_att = int(task.max_attempts or 3)
                if _is_retryable_error(exc) and int(task.attempts or 0) < max_att:
                    task.status = "queued"
                    task.error_text = f"Retry {task.attempts}/{max_att}: {exc}"
                    task.started_at = None
                    task.completed_at = None
                else:
                    task.status = "failed"
                    task.error_text = str(exc)
                    task.completed_at = datetime.now(timezone.utc)
            await db.commit()
            try:
                await batch_orchestrator.on_task_finished(task.id)
            except Exception:
                logger.exception("Batch hook failed on task finish %s", task.id)
            return True


ai_task_worker = AITaskWorker()
