"""Tests for the parallel training job worker.

Covers atomic claiming, concurrency bounding, device round-robin,
stale recovery, atomic agent stats, and checkpoint path uniqueness.
"""

from __future__ import annotations

import asyncio
import os
from datetime import datetime, timedelta, timezone
from unittest.mock import MagicMock, patch
from uuid import uuid4

import pytest
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from server.db.models import Agent, Base, TrainingJob


# ── Fixtures ──────────────────────────────────────────────────────────────


@pytest.fixture
async def db_engine():
    """Create an in-memory SQLite engine for tests."""
    engine = create_async_engine(
        "sqlite+aiosqlite:///:memory:",
        echo=False,
        connect_args={"check_same_thread": False},
    )
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)
    yield engine
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.drop_all)
    await engine.dispose()


@pytest.fixture
async def session_factory(db_engine):
    """Provide an async_sessionmaker bound to the in-memory engine."""
    return async_sessionmaker(db_engine, class_=AsyncSession, expire_on_commit=False)


@pytest.fixture
async def db_session(session_factory):
    """Provide a transactional async session for direct DB setup/assertions."""
    async with session_factory() as session:
        yield session


@pytest.fixture
def patch_async_session(monkeypatch, session_factory):
    """Monkey-patch the worker module's ``async_session`` to use the test DB."""
    monkeypatch.setattr(
        "server.workers.training_worker.async_session", session_factory
    )


# ── Helpers ───────────────────────────────────────────────────────────────


def _make_job(
    *,
    status: str = "queued",
    job_type: str = "train_vs_greedy",
    worker_id: str | None = None,
    agent_id: str | None = None,
    gauntlet_id: str | None = None,
    started_at: datetime | None = None,
    created_at: datetime | None = None,
) -> TrainingJob:
    """Create a TrainingJob instance (not yet added to a session)."""
    now = datetime.now(timezone.utc)
    return TrainingJob(
        id=str(uuid4()),
        job_type=job_type,
        status=status,
        worker_id=worker_id,
        agent_id=agent_id,
        gauntlet_id=gauntlet_id,
        started_at=started_at,
        created_at=created_at or now,
    )


def _make_agent(*, total_games: int = 0, total_wins: int = 0) -> Agent:
    """Create an Agent instance (not yet added to a session)."""
    return Agent(
        id=str(uuid4()),
        name="test-agent",
        archetype="red-aggro",
        algorithm="mlp",
        total_games=total_games,
        total_wins=total_wins,
        total_losses=0,
        total_draws=0,
        total_timesteps_trained=0,
    )


# ── 1. Atomic claiming - no double claim ────────────────────────────────


class TestAtomicClaiming:
    async def test_no_double_claim(self, db_session, session_factory, patch_async_session):
        """Two workers claiming concurrently should never double-claim a job."""
        from server.workers.training_worker import TrainingJobWorker

        # Create 5 queued jobs with staggered created_at so ordering is stable
        base_time = datetime.now(timezone.utc)
        for i in range(5):
            job = _make_job(created_at=base_time + timedelta(seconds=i))
            db_session.add(job)
        await db_session.commit()

        w1 = TrainingJobWorker()
        w2 = TrainingJobWorker()

        # Both try to claim up to 3 jobs concurrently
        results = await asyncio.gather(
            w1._claim_jobs(3),
            w2._claim_jobs(3),
        )

        all_claimed_ids = [job_id for claimed in results for job_id, _ in claimed]

        # No duplicates - this is the critical invariant
        assert len(set(all_claimed_ids)) == len(all_claimed_ids), (
            "Duplicate job claims detected!"
        )

        # With SQLite, some claim iterations may fail due to locking contention
        # and the loop uses one iteration per attempt (even on failure), so the
        # total may be less than 5. But we should have claimed at least some.
        assert len(all_claimed_ids) >= 3, (
            f"Expected at least 3 claims, got {len(all_claimed_ids)}"
        )

        # Verify each claimed job has exactly one worker_id in the DB
        async with session_factory() as fresh_session:
            result = await fresh_session.execute(
                select(TrainingJob).where(TrainingJob.status == "running")
            )
            running_jobs = result.scalars().all()
            for j in running_jobs:
                assert j.worker_id in (w1._worker_id, w2._worker_id), (
                    f"Job {j.id} has unexpected worker_id {j.worker_id}"
                )


# ── 2. Concurrent capacity bounding ─────────────────────────────────────


class TestConcurrentCapacity:
    async def test_max_concurrent_limits_running_tasks(
        self, db_session, patch_async_session, monkeypatch
    ):
        """Worker with max_concurrent=2 should never have more than 2 running tasks."""
        from server.workers.training_worker import TrainingJobWorker

        # Create 4 queued jobs
        base_time = datetime.now(timezone.utc)
        for i in range(4):
            job = _make_job(created_at=base_time + timedelta(seconds=i))
            db_session.add(job)
        await db_session.commit()

        worker = TrainingJobWorker()
        monkeypatch.setattr(worker, "max_concurrent", 2)
        monkeypatch.setattr(worker, "poll_interval_seconds", 0.05)

        # Track the max observed concurrency
        max_concurrent_observed = 0
        concurrency_lock = asyncio.Lock()
        active_count = 0

        original_run_heuristic = worker._run_heuristic_training

        def slow_heuristic(job_id: str, config: dict = None) -> dict:
            nonlocal active_count, max_concurrent_observed
            # We need to track concurrency. Since this runs in a thread,
            # we use a simple integer counter (GIL protects simple ops).
            active_count += 1
            if active_count > max_concurrent_observed:
                max_concurrent_observed = active_count
            import time
            time.sleep(0.5)
            active_count -= 1
            return {"status": "completed", "note": "slow placeholder"}

        monkeypatch.setattr(worker, "_run_heuristic_training", slow_heuristic)

        # Start the worker
        await worker.start()

        # Let it run long enough to process some jobs
        await asyncio.sleep(2.0)

        await worker.stop()

        # max_concurrent_observed should be at most 2
        assert max_concurrent_observed <= 2, (
            f"Observed {max_concurrent_observed} concurrent tasks, expected <= 2"
        )
        # Should have actually run some tasks
        assert max_concurrent_observed >= 1, "No tasks ran at all"


# ── 3. Device round-robin ────────────────────────────────────────────────


class TestDeviceRoundRobin:
    def test_assign_device_cycles(self, monkeypatch):
        """Device assignment should cycle through configured devices."""
        monkeypatch.setenv("TRAINING_WORKER_DEVICES", "cpu,cuda:0,cuda:1")

        from server.workers.training_worker import TrainingJobWorker

        worker = TrainingJobWorker()

        assignments = [worker._assign_device() for _ in range(6)]

        assert assignments == [
            "cpu", "cuda:0", "cuda:1",
            "cpu", "cuda:0", "cuda:1",
        ]


# ── 4. Stale recovery - worker-aware ────────────────────────────────────


class TestStaleRecovery:
    async def test_recovers_foreign_but_not_own(
        self, db_session, session_factory, patch_async_session
    ):
        """Stale recovery should only mark foreign-worker jobs as failed."""
        from server.workers.training_worker import TrainingJobWorker

        worker = TrainingJobWorker()
        stale_time = datetime.now(timezone.utc) - timedelta(hours=3)

        # Job from a different worker (should be recovered)
        foreign_job = _make_job(
            status="running",
            worker_id="worker-foreign",
            started_at=stale_time,
        )
        db_session.add(foreign_job)

        # Job from our own worker (should NOT be recovered)
        own_job = _make_job(
            status="running",
            worker_id=worker._worker_id,
            started_at=stale_time,
        )
        db_session.add(own_job)
        await db_session.commit()

        foreign_id = foreign_job.id
        own_id = own_job.id

        await worker._recover_stale()

        # Use a fresh session to see committed changes from the worker
        async with session_factory() as fresh_session:
            result = await fresh_session.execute(
                select(TrainingJob).where(TrainingJob.id == foreign_id)
            )
            foreign_refreshed = result.scalar_one()
            assert foreign_refreshed.status == "failed", (
                f"Foreign job should be failed, got {foreign_refreshed.status}"
            )
            assert foreign_refreshed.error_text is not None

            result = await fresh_session.execute(
                select(TrainingJob).where(TrainingJob.id == own_id)
            )
            own_refreshed = result.scalar_one()
            assert own_refreshed.status == "running", (
                f"Own job should still be running, got {own_refreshed.status}"
            )


# ── 5. Atomic agent stats ───────────────────────────────────────────────


class TestAtomicAgentStats:
    async def test_concurrent_stats_updates(
        self, db_session, session_factory, patch_async_session
    ):
        """Two concurrent _update_agent_stats calls should sum correctly."""
        from server.workers.training_worker import TrainingJobWorker

        # Create an agent
        agent = _make_agent(total_games=0, total_wins=0)
        db_session.add(agent)
        await db_session.commit()
        agent_id = agent.id

        # Create two completed jobs for that agent
        job1 = _make_job(status="completed", agent_id=agent_id)
        job2 = _make_job(status="completed", agent_id=agent_id)
        db_session.add_all([job1, job2])
        await db_session.commit()

        worker = TrainingJobWorker()

        result_data_1 = {"total_games": 100, "wins": 60, "losses": 30, "draws": 10}
        result_data_2 = {"total_games": 50, "wins": 25, "losses": 20, "draws": 5}

        # Run both updates concurrently using separate sessions
        async def update_with_session(job, result_data):
            async with session_factory() as sess:
                await worker._update_agent_stats(sess, job, result_data)
                await sess.commit()

        await asyncio.gather(
            update_with_session(job1, result_data_1),
            update_with_session(job2, result_data_2),
        )

        # Use a fresh session to see committed changes
        async with session_factory() as fresh_session:
            result = await fresh_session.execute(
                select(Agent).where(Agent.id == agent_id)
            )
            refreshed = result.scalar_one()

            assert refreshed.total_games == 150, (
                f"Expected total_games=150, got {refreshed.total_games}"
            )
            assert refreshed.total_wins == 85, (
                f"Expected total_wins=85, got {refreshed.total_wins}"
            )
            assert refreshed.total_losses == 50, (
                f"Expected total_losses=50, got {refreshed.total_losses}"
            )
            assert refreshed.total_draws == 15, (
                f"Expected total_draws=15, got {refreshed.total_draws}"
            )


# ── 6. Checkpoint path uniqueness ───────────────────────────────────────


class TestCheckpointPathUniqueness:
    def test_path_with_job_id(self):
        """save_model with job_id should include that id in the filename."""
        from digimon_gym.agents.pilot_training import save_model

        mock_model = MagicMock()
        path = save_model(mock_model, models_dir="/tmp/test_models", job_id="abc123")

        assert "pilot_ppo_abc123" in path
        mock_model.save.assert_called_once()

    def test_path_without_job_id_uses_timestamp(self):
        """save_model without job_id should fall back to timestamp format."""
        from digimon_gym.agents.pilot_training import save_model

        mock_model = MagicMock()
        path = save_model(mock_model, models_dir="/tmp/test_models", job_id=None)

        # Should contain a timestamp-like pattern: YYYYMMDD_HHMMSS
        filename = os.path.basename(path)
        assert filename.startswith("pilot_ppo_")
        suffix = filename.replace("pilot_ppo_", "")
        # Verify it looks like a timestamp (8 digits _ 6 digits)
        parts = suffix.split("_")
        assert len(parts) == 2, f"Expected timestamp format YYYYMMDD_HHMMSS, got {suffix}"
        assert len(parts[0]) == 8 and parts[0].isdigit(), f"Date part invalid: {parts[0]}"
        assert len(parts[1]) == 6 and parts[1].isdigit(), f"Time part invalid: {parts[1]}"
        mock_model.save.assert_called_once()

    def test_paths_are_distinct(self):
        """Two calls with different job_ids should produce different paths."""
        from digimon_gym.agents.pilot_training import save_model

        mock_model = MagicMock()
        path1 = save_model(mock_model, models_dir="/tmp/test_models", job_id="job-aaa")
        path2 = save_model(mock_model, models_dir="/tmp/test_models", job_id="job-bbb")

        assert path1 != path2
        assert "job-aaa" in path1
        assert "job-bbb" in path2
