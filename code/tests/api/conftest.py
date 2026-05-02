"""Shared async DB fixtures for API-layer tests."""

from __future__ import annotations

import pytest
from sqlalchemy.ext.asyncio import (
    AsyncSession,
    async_sessionmaker,
    create_async_engine,
)

from server.db.models import Base
from server.limiter import limiter


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
def _reset_api_limits():
    """Keep in-process auth limits isolated between tests."""
    from server.db.routers.auth import _reset_guest_rate_limit

    prev = limiter.enabled
    limiter.enabled = False
    _reset_guest_rate_limit()
    try:
        yield
    finally:
        _reset_guest_rate_limit()
        limiter.enabled = prev
