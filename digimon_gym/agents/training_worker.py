"""In-process training job worker with DB-backed queue semantics.

Polls for queued TrainingJob rows and dispatches training/evaluation
jobs to background threads.  Follows the same lifecycle pattern as
``digimon_gym.ai.worker.AITaskWorker``.
"""

from __future__ import annotations

import asyncio
import json
import logging
import os
from datetime import datetime, timedelta, timezone

from sqlalchemy import select

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
        self._task: asyncio.Task | None = None
        self._stop = asyncio.Event()

    # ── Lifecycle ────────────────────────────────────────────────────

    async def start(self) -> None:
        if self._task and not self._task.done():
            return
        self._stop.clear()
        self._task = asyncio.create_task(self._loop(), name="training-job-worker")
        logger.info("Training job worker started")

    async def stop(self) -> None:
        self._stop.set()
        if self._task:
            await asyncio.wait([self._task], timeout=5)
        logger.info("Training job worker stopped")

    # ── Main loop ────────────────────────────────────────────────────

    async def _loop(self) -> None:
        while not self._stop.is_set():
            try:
                await self._recover_stale()
                processed = await self._process_one_job()
                if not processed:
                    await asyncio.sleep(self.poll_interval_seconds)
            except Exception:
                logger.exception("Unexpected training worker loop failure")
                await asyncio.sleep(self.poll_interval_seconds)

    # ── Stale recovery ───────────────────────────────────────────────

    async def _recover_stale(self) -> None:
        """Mark long-running jobs as failed so they don't block the queue."""
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

    # ── Job processing ───────────────────────────────────────────────

    async def _process_one_job(self) -> bool:
        """Claim the oldest queued job, run it, and update results.

        Returns True if a job was processed (success or failure),
        False if the queue was empty.
        """
        # --- Claim ---
        async with async_session() as db:
            result = await db.execute(
                select(TrainingJob)
                .where(TrainingJob.status == "queued")
                .order_by(TrainingJob.created_at.asc())
                .limit(1)
            )
            job = result.scalar_one_or_none()
            if job is None:
                return False

            job.status = "running"
            job.started_at = datetime.now(timezone.utc)
            await db.commit()
            job_id = job.id
            job_type = job.job_type

        logger.info("Claimed training job %s (type=%s)", job_id, job_type)

        # --- Execute in thread ---
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

        # --- Persist outcome ---
        async with async_session() as db:
            result = await db.execute(
                select(TrainingJob).where(TrainingJob.id == job_id)
            )
            job = result.scalar_one_or_none()
            if job is None:
                logger.error("Training job %s vanished after execution", job_id)
                return True

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

        return True

    # ── Agent stats update ───────────────────────────────────────────

    async def _update_agent_stats(
        self, db, job: TrainingJob, result_data: dict  # noqa: ANN001
    ) -> None:
        """Push training/eval results back to the Agent row."""
        if not job.agent_id:
            return

        agent_result = await db.execute(
            select(Agent).where(Agent.id == job.agent_id)
        )
        agent = agent_result.scalar_one_or_none()
        if agent is None:
            return

        if "total_games" in result_data:
            agent.total_games += int(result_data["total_games"])
        if "wins" in result_data:
            agent.total_wins += int(result_data["wins"])
        if "losses" in result_data:
            agent.total_losses += int(result_data["losses"])
        if "draws" in result_data:
            agent.total_draws += int(result_data["draws"])

        total = agent.total_wins + agent.total_losses + agent.total_draws
        agent.win_rate = agent.total_wins / total if total > 0 else 0.0

        if "weights_path" in result_data:
            agent.weights_path = result_data["weights_path"]
        if "total_timesteps" in result_data:
            agent.total_timesteps_trained += int(result_data["total_timesteps"])

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
