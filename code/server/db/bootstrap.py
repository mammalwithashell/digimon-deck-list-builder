"""Container-start DB bootstrap: fresh vs. existing database.

The schema's source of truth is ``Base.metadata`` — historically created by
``init_db()`` (``create_all``) at app startup, with alembic migrations only
additive on top of the 20260222_0001 no-op baseline. That means a FRESH
database cannot replay ``alembic upgrade head`` (the early migrations assume
the create_all tables already exist). The correct bootstrap is:

- fresh database (no ``alembic_version`` table): ``create_all`` then
  ``alembic stamp head`` — the models already reflect every migration.
- existing database: ``alembic upgrade head`` as usual.

Run as ``python -m server.db.bootstrap`` before starting uvicorn.
"""
from __future__ import annotations

import asyncio
import subprocess
import sys

from sqlalchemy import inspect

from server.db.database import engine, init_db


async def _has_alembic_version() -> bool:
    async with engine.connect() as conn:
        return await conn.run_sync(
            lambda sync_conn: inspect(sync_conn).has_table("alembic_version")
        )


def _alembic(*args: str) -> None:
    subprocess.run([sys.executable, "-m", "alembic", *args], check=True)


async def main() -> None:
    if await _has_alembic_version():
        print("bootstrap: existing database — alembic upgrade head", flush=True)
        _alembic("upgrade", "head")
    else:
        print("bootstrap: fresh database — create_all + alembic stamp head", flush=True)
        await init_db()
        _alembic("stamp", "head")
    await engine.dispose()


if __name__ == "__main__":
    asyncio.run(main())
