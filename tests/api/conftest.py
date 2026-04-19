"""Shared async DB fixtures for API-layer tests.

Per-test ephemeral in-memory SQLite instances — each test gets a clean
schema. Do not use for tests that also need the full FastAPI app wired
up (those files still set up their own `client` fixture with router
resets, e.g. test_matchmaking.py / test_admin_models.py).
"""
from __future__ import annotations

import pytest
from sqlalchemy.ext.asyncio import (
    AsyncSession,
    async_sessionmaker,
    create_async_engine,
)

from digimon_gym.db.models import Base


@pytest.fixture
async def db_engine():
    engine = create_async_engine("sqlite+aiosqlite:///:memory:", echo=False)
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)
    yield engine
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.drop_all)
    await engine.dispose()


@pytest.fixture
async def db_session(db_engine):
    session_factory = async_sessionmaker(
        db_engine, class_=AsyncSession, expire_on_commit=False
    )
    async with session_factory() as session:
        yield session


@pytest.fixture(autouse=True)
def _reset_rate_limit():
    """Wipe /auth/guest rate-limit state before and after every API test so
    earlier tests that mint several guests don't trip the limit in later tests.
    """
    from digimon_gym.db.routers.auth import _reset_guest_rate_limit

    _reset_guest_rate_limit()
    yield
    _reset_guest_rate_limit()
