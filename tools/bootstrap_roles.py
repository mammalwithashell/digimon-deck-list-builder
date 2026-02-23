#!/usr/bin/env python3
"""Bootstrap RBAC roles and grant existing users admin/judge/player."""

from __future__ import annotations

import asyncio
from pathlib import Path
import sys

PROJECT_ROOT = Path(__file__).resolve().parents[1]
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from sqlalchemy import select

from digimon_gym.db.auth import DEFAULT_ROLES, assign_role_to_user, ensure_role_exists
from digimon_gym.db.database import async_session, init_db
from digimon_gym.db.models import User


async def run() -> None:
    await init_db()
    async with async_session() as db:
        for role_name in DEFAULT_ROLES:
            await ensure_role_exists(db, role_name)

        users = (await db.execute(select(User))).scalars().all()
        for user in users:
            for role_name in DEFAULT_ROLES:
                await assign_role_to_user(db, user.id, role_name)

        await db.commit()
        print(f"Bootstrapped roles for {len(users)} users")


if __name__ == "__main__":
    asyncio.run(run())
