"""Async SQLAlchemy engine and session factory for SQLite."""

from __future__ import annotations

import logging
import os

from sqlalchemy import text
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from digimon_gym.db.models import Base

logger = logging.getLogger(__name__)

DATABASE_URL = os.getenv("DATABASE_URL", "sqlite+aiosqlite:///./data/app.db")

connect_args = {}
if DATABASE_URL.startswith("sqlite"):
    connect_args["check_same_thread"] = False

engine = create_async_engine(
    DATABASE_URL,
    echo=False,
    connect_args=connect_args,
)

async_session = async_sessionmaker(engine, class_=AsyncSession, expire_on_commit=False)


async def init_db() -> None:
    """Create all tables. Call once at application startup."""
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)
    await migrate_training_jobs_worker_columns()


async def migrate_training_jobs_worker_columns() -> None:
    """Add worker_id, claimed_at, and device columns to training_jobs if missing.

    Idempotent: safe to call on every startup.  New databases already have
    the columns via ``create_all``; this handles pre-existing databases.
    """
    new_columns = {
        "worker_id": "VARCHAR",
        "claimed_at": "TIMESTAMP",
        "device": "VARCHAR",
    }

    async with engine.begin() as conn:
        # Fetch existing column names for the table.
        result = await conn.execute(text("PRAGMA table_info('training_jobs')"))
        existing = {row[1] for row in result.fetchall()}

        for col_name, col_type in new_columns.items():
            if col_name not in existing:
                await conn.execute(
                    text(
                        f"ALTER TABLE training_jobs ADD COLUMN {col_name} {col_type}"
                    )
                )
                logger.info("Added column training_jobs.%s", col_name)

        # Composite index — CREATE INDEX IF NOT EXISTS is safe to repeat.
        await conn.execute(
            text(
                "CREATE INDEX IF NOT EXISTS idx_training_jobs_status_created "
                "ON training_jobs (status, created_at)"
            )
        )

        # Individual index on worker_id for claim lookups.
        await conn.execute(
            text(
                "CREATE INDEX IF NOT EXISTS ix_training_jobs_worker_id "
                "ON training_jobs (worker_id)"
            )
        )


async def get_db():
    """FastAPI dependency that yields an async database session."""
    async with async_session() as session:
        try:
            yield session
        finally:
            await session.close()
