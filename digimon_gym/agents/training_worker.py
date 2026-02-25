"""In-process training job worker with DB-backed queue semantics.

Polls for queued TrainingJob rows and dispatches training/evaluation
jobs to background threads.  Supports concurrent execution bounded by
a capacity check (controlled via ``TRAINING_WORKER_MAX_CONCURRENT``).

Follows the same lifecycle pattern as
``digimon_gym.ai.worker.AITaskWorker``.
"""

from __future__ import annotations

import asyncio
import json
import logging
import os
from datetime import datetime, timedelta, timezone
from uuid import uuid4

from sqlalchemy import case, cast, select, update, Float as SAFloat

from digimon_gym.db.database import async_session
from digimon_gym.db.models import Agent, TrainingJob

logger = logging.getLogger(__name__)


class TrainingJobWorker:
    """Async worker that claims and executes TrainingJob rows."""

    def __init__(self) -> None:
        self.poll_interval_seconds = float(
            os.getenv("TRAINING_WORKER_POLL_SECONDS", "5.0")
        )
        self.stale_after_seconds = int(
            os.getenv("TRAINING_WORKER_STALE_SECONDS", "7200")
        )
        self.max_concurrent = int(
            os.getenv("TRAINING_WORKER_MAX_CONCURRENT", "1")
        )
        self._worker_id = f"worker-{uuid4().hex[:8]}"
        self._task: asyncio.Task | None = None
        self._stop = asyncio.Event()
        self._running_tasks: set[asyncio.Task] = set()
        self._devices: list[str] = self._discover_devices()
        self._device_index: int = 0

    # ── Device management ─────────────────────────────────────────────

    def _discover_devices(self) -> list[str]:
        """Discover available training devices.

        Checks (in order):
        1. ``TRAINING_WORKER_DEVICES`` env var (comma-separated, e.g. ``cuda:0,cuda:1``)
        2. Auto-detect CUDA GPUs via ``torch.cuda``
        3. Fall back to ``["cpu"]``
        """
        env_devices = os.getenv("TRAINING_WORKER_DEVICES", "").strip()
        if env_devices:
            return [d.strip() for d in env_devices.split(",") if d.strip()]

        try:
            import torch

            if torch.cuda.is_available():
                count = torch.cuda.device_count()
                if count > 0:
                    return [f"cuda:{i}" for i in range(count)]
        except ImportError:
            pass

        return ["cpu"]

    def _assign_device(self) -> str:
        """Round-robin assignment across discovered devices."""
        device = self._devices[self._device_index % len(self._devices)]
        self._device_index += 1
        return device

    # ── Lifecycle ────────────────────────────────────────────────────

    async def start(self) -> None:
        if self._task and not self._task.done():
            return
        self._stop.clear()
        self._task = asyncio.create_task(self._loop(), name="training-job-worker")
        logger.info(
            "Training job worker started (id=%s, max_concurrent=%d, devices=%s)",
            self._worker_id,
            self.max_concurrent,
            self._devices,
        )

    async def stop(self) -> None:
        self._stop.set()
        if self._running_tasks:
            logger.info(
                "Waiting for %d running training tasks to finish...",
                len(self._running_tasks),
            )
            done, pending = await asyncio.wait(self._running_tasks, timeout=30)
            if pending:
                logger.warning(
                    "%d training tasks did not finish within shutdown timeout, cancelling",
                    len(pending),
                )
                for t in pending:
                    t.cancel()
        if self._task:
            await asyncio.wait([self._task], timeout=5)
        logger.info("Training job worker stopped (id=%s)", self._worker_id)

    # ── Main loop ────────────────────────────────────────────────────

    async def _loop(self) -> None:
        while not self._stop.is_set():
            try:
                await self._recover_stale()

                # Clean up finished tasks
                done_tasks = {t for t in self._running_tasks if t.done()}
                for t in done_tasks:
                    if t.cancelled():
                        logger.warning("Training task %s was cancelled", t.get_name())
                    elif t.exception() is not None:
                        logger.error(
                            "Training task %s raised: %s", t.get_name(), t.exception()
                        )
                self._running_tasks -= done_tasks

                # Check if we have capacity for more jobs
                capacity = self.max_concurrent - len(self._running_tasks)
                if capacity <= 0:
                    await asyncio.sleep(self.poll_interval_seconds)
                    continue

                # Try to claim jobs up to available capacity
                claimed = await self._claim_jobs(capacity)
                if not claimed:
                    await asyncio.sleep(self.poll_interval_seconds)
                    continue

                # Dispatch each claimed job as a concurrent task
                for job_id, job_type in claimed:
                    task = asyncio.create_task(
                        self._run_job(job_id, job_type),
                        name=f"training-{job_id[:8]}",
                    )
                    self._running_tasks.add(task)

                # Continue immediately to potentially claim more if capacity remains
            except Exception:
                logger.exception("Unexpected training worker loop failure")
                await asyncio.sleep(self.poll_interval_seconds)

    # ── Stale recovery ───────────────────────────────────────────────

    async def _recover_stale(self) -> None:
        """Mark long-running jobs as failed so they don't block the queue.

        Only recovers jobs belonging to other workers (or with no worker_id)
        to avoid recovering our own legitimately-running parallel jobs.
        """
        stale_before = datetime.now(timezone.utc) - timedelta(
            seconds=self.stale_after_seconds
        )
        async with async_session() as db:
            result = await db.execute(
                select(TrainingJob).where(
                    TrainingJob.status == "running",
                    TrainingJob.started_at.is_not(None),
                    TrainingJob.started_at < stale_before,
                    TrainingJob.completed_at.is_(None),
                    # Only recover jobs from other workers or with no worker_id
                    (TrainingJob.worker_id != self._worker_id)
                    | TrainingJob.worker_id.is_(None),
                )
            )
            stale_jobs = result.scalars().all()
            if not stale_jobs:
                return
            for job in stale_jobs:
                job.status = "failed"
                job.error_text = "Worker recovered stale running job"
                job.completed_at = datetime.now(timezone.utc)
            await db.commit()

            for job in stale_jobs:
                logger.warning("Recovered stale training job %s", job.id)
                await self._gauntlet_hook(job)

    # ── Atomic job claiming ──────────────────────────────────────────

    async def _claim_jobs(self, n: int) -> list[tuple[str, str]]:
        """Claim up to n queued jobs atomically. Returns [(job_id, job_type), ...]."""
        claimed: list[tuple[str, str]] = []

        for _ in range(n):
            now = datetime.now(timezone.utc)
            async with async_session() as db:
                # Find the oldest queued job (fetch id and job_type together)
                oldest = await db.execute(
                    select(TrainingJob.id, TrainingJob.job_type)
                    .where(TrainingJob.status == "queued")
                    .order_by(TrainingJob.created_at.asc())
                    .limit(1)
                )
                row = oldest.one_or_none()
                if row is None:
                    break  # No more queued jobs

                job_id, job_type = row[0], row[1]

                # Atomic UPDATE: only claim if still queued
                result = await db.execute(
                    update(TrainingJob)
                    .where(
                        TrainingJob.id == job_id,
                        TrainingJob.status == "queued",
                    )
                    .values(
                        status="running",
                        worker_id=self._worker_id,
                        started_at=now,
                        claimed_at=now,
                    )
                )
                if result.rowcount == 0:  # type: ignore[union-attr]
                    # Another worker got it first, try next
                    await db.commit()
                    continue

                await db.commit()

                claimed.append((job_id, job_type))
                logger.info(
                    "Claimed training job %s (type=%s, worker=%s)",
                    job_id,
                    job_type,
                    self._worker_id,
                )

        return claimed

    # ── Job execution ────────────────────────────────────────────────

    async def _run_job(self, job_id: str, job_type: str) -> None:
        """Execute a single training job."""
        device = self._assign_device()
        async with async_session() as db:
            await db.execute(
                update(TrainingJob)
                .where(TrainingJob.id == job_id)
                .values(device=device)
            )
            await db.commit()

        logger.info(
            "Running training job %s (type=%s, device=%s, worker=%s)",
            job_id,
            job_type,
            device,
            self._worker_id,
        )

        # Execute in thread
        result_data: dict | None = None
        error_text = ""
        try:
            if job_type in ("train_vs_greedy", "train_vs_random"):
                result_data = await asyncio.to_thread(
                    self._run_heuristic_training, job_id
                )
            elif job_type == "train_vs_agent":
                result_data = await asyncio.to_thread(
                    self._run_agent_training, job_id
                )
            elif job_type == "evaluate":
                result_data = await asyncio.to_thread(
                    self._run_evaluation, job_id
                )
            else:
                result_data = {"error": f"Unknown job type: {job_type}"}
        except Exception as exc:
            logger.exception("Training job %s failed", job_id)
            result_data = None
            error_text = str(exc)

        # Persist outcome
        await self._persist_outcome(job_id, result_data, error_text)

    # ── Persist outcome ──────────────────────────────────────────────

    async def _persist_outcome(
        self,
        job_id: str,
        result_data: dict | None,
        error_text: str,
    ) -> None:
        """Write training result or error back to the TrainingJob row."""
        async with async_session() as db:
            result = await db.execute(
                select(TrainingJob).where(TrainingJob.id == job_id)
            )
            job = result.scalar_one_or_none()
            if job is None:
                logger.error("Training job %s vanished after execution", job_id)
                return

            now = datetime.now(timezone.utc)
            if result_data is not None:
                job.status = "completed"
                job.result_json = json.dumps(result_data)
                job.completed_at = now

                # Update agent stats when training results are available
                await self._update_agent_stats(db, job, result_data)
            else:
                job.status = "failed"
                job.error_text = error_text
                job.completed_at = now

            await db.commit()
            logger.info(
                "Training job %s finished with status=%s", job_id, job.status
            )

            await self._gauntlet_hook(job)

    # ── Agent stats update ───────────────────────────────────────────

    async def _update_agent_stats(
        self, db, job: TrainingJob, result_data: dict  # noqa: ANN001
    ) -> None:
        """Push training/eval results back to the Agent row using atomic SQL increments.

        Uses SQL-level ``col = col + N`` expressions so concurrent jobs
        finishing for the same agent do not suffer lost-update anomalies.
        """
        if not job.agent_id:
            return

        # Build atomic increment values
        values: dict = {}
        if "total_games" in result_data:
            values[Agent.total_games] = Agent.total_games + int(result_data["total_games"])
        if "wins" in result_data:
            values[Agent.total_wins] = Agent.total_wins + int(result_data["wins"])
        if "losses" in result_data:
            values[Agent.total_losses] = Agent.total_losses + int(result_data["losses"])
        if "draws" in result_data:
            values[Agent.total_draws] = Agent.total_draws + int(result_data["draws"])
        if "total_timesteps" in result_data:
            values[Agent.total_timesteps_trained] = (
                Agent.total_timesteps_trained + int(result_data["total_timesteps"])
            )
        if "weights_path" in result_data:
            values[Agent.weights_path] = result_data["weights_path"]

        if values:
            await db.execute(
                update(Agent).where(Agent.id == job.agent_id).values(values)
            )

        # Recompute win_rate atomically from the freshly-incremented columns
        if any(k in result_data for k in ("wins", "losses", "draws")):
            total_expr = Agent.total_wins + Agent.total_losses + Agent.total_draws
            await db.execute(
                update(Agent)
                .where(Agent.id == job.agent_id)
                .values(
                    win_rate=case(
                        (total_expr > 0, cast(Agent.total_wins, SAFloat) / cast(total_expr, SAFloat)),
                        else_=0.0,
                    )
                )
            )

    # ── Gauntlet hook ────────────────────────────────────────────────

    async def _gauntlet_hook(self, job: TrainingJob) -> None:
        """Notify the gauntlet orchestrator when a gauntlet-linked job finishes."""
        if not job.gauntlet_id:
            return
        try:
            from digimon_gym.agents.gauntlet_orchestrator import (
                gauntlet_orchestrator,
            )

            await gauntlet_orchestrator.on_job_finished(job.id)
        except Exception:
            logger.exception("Gauntlet hook failed for job %s", job.id)

    # ── Training dispatch (placeholders) ─────────────────────────────

    def _run_heuristic_training(self, job_id: str) -> dict:
        """Train agent vs greedy/random heuristic. Placeholder for now."""
        # TODO: Reuse pilot_training.train() with agent's deck and weights
        logger.info("Would run heuristic training for job %s", job_id)
        return {"status": "completed", "note": "placeholder"}

    def _run_agent_training(self, job_id: str) -> dict:
        """Train agent vs another agent. Placeholder for now."""
        # TODO: Set up two-agent training loop
        logger.info("Would run agent-vs-agent training for job %s", job_id)
        return {"status": "completed", "note": "placeholder"}

    def _run_evaluation(self, job_id: str) -> dict:
        """Evaluate two agents head-to-head. Placeholder for now."""
        # TODO: Run N games and return win/loss/draw counts
        logger.info("Would run evaluation for job %s", job_id)
        return {"status": "completed", "note": "placeholder"}


training_job_worker = TrainingJobWorker()
