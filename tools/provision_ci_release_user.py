"""One-shot: create (or refresh) the `ci-desktop-release` admin user and
print a long-lived JWT for use in GitHub Actions.

Usage:
    python tools/provision_ci_release_user.py --password "$(openssl rand -base64 32)"

Run against the production DB with the appropriate DATABASE_URL env var.
Safe to re-run: if the user exists, the password rotates and a fresh token
is printed to stdout. Stderr carries diagnostic messages.

The printed token can be piped directly to:
    python tools/provision_ci_release_user.py --password "$PASS" \\
      | gh secret set CI_ADMIN_TOKEN
"""
from __future__ import annotations

import argparse
import asyncio
import sys
from datetime import datetime, timedelta, timezone

import jwt
from sqlalchemy import select

from server.db.auth import (
    ALGORITHM,
    ROLE_ADMIN,
    SECRET_KEY,
    assign_role_to_user,
    hash_password,
)
from server.db.database import async_session_maker
from server.db.models import User

CI_USERNAME = "ci-desktop-release"
CI_EMAIL = "ci-desktop-release@ci.local"
TOKEN_EXPIRES = timedelta(days=365)


def _mint_long_lived_token(user_id: str, username: str) -> str:
    """Same shape as server.db.auth.create_access_token, with a
    365-day expiry instead of 30 minutes. Carries ROLE_ADMIN so
    `require_roles(ROLE_ADMIN)` passes."""
    expire = datetime.now(timezone.utc) + TOKEN_EXPIRES
    payload = {
        "sub": user_id,
        "username": username,
        "roles": [ROLE_ADMIN],
        "exp": expire,
        "type": "access",
    }
    return jwt.encode(payload, SECRET_KEY, algorithm=ALGORITHM)


async def run(password: str) -> str:
    async with async_session_maker() as db:
        user = await db.scalar(select(User).where(User.username == CI_USERNAME))
        if user is None:
            user = User(
                username=CI_USERNAME,
                email=CI_EMAIL,
                password_hash=hash_password(password),
            )
            db.add(user)
            await db.flush()
            await assign_role_to_user(db, user.id, ROLE_ADMIN)
            await db.commit()
            await db.refresh(user)
            print(f"created user {CI_USERNAME} (id={user.id})", file=sys.stderr)
        else:
            user.password_hash = hash_password(password)
            await assign_role_to_user(db, user.id, ROLE_ADMIN)
            await db.commit()
            await db.refresh(user)
            print(f"refreshed password for existing user {CI_USERNAME} (id={user.id})", file=sys.stderr)

        return _mint_long_lived_token(user.id, user.username)


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--password", required=True, help="password for CI user")
    args = p.parse_args()
    token = asyncio.run(run(args.password))
    # stdout is ONLY the token so `... | gh secret set CI_ADMIN_TOKEN` works.
    print(token)


if __name__ == "__main__":
    main()
