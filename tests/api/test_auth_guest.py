"""Guest-user endpoint + integration with matchmaking."""
from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pytest
from httpx import ASGITransport, AsyncClient
from jose import jwt
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker

from digimon_gym.db.auth import (
    ALGORITHM,
    SECRET_KEY,
    create_guest_access_token,
)
from digimon_gym.db.database import get_db
from digimon_gym.db.models import User


@pytest.fixture
async def client(db_engine):
    from digimon_gym.api import app

    session_factory = async_sessionmaker(
        db_engine, class_=AsyncSession, expire_on_commit=False
    )

    async def override_get_db():
        async with session_factory() as session:
            yield session

    app.dependency_overrides[get_db] = override_get_db
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as c:
        yield c
    app.dependency_overrides.clear()


@pytest.mark.asyncio
async def test_user_model_has_is_guest_flag(db_session) -> None:
    user = User(
        username="test_flag_user",
        email="flag@example.com",
        password_hash="dummy",
    )
    db_session.add(user)
    await db_session.flush()
    assert user.is_guest is False, "new users default to is_guest=False"


def test_create_guest_access_token_has_year_long_expiry() -> None:
    token = create_guest_access_token(user_id="guest_123", display_name="Guest-abcd")
    payload = jwt.decode(token, SECRET_KEY, algorithms=[ALGORITHM])
    assert payload["sub"] == "guest_123"
    assert payload["username"] == "Guest-abcd"
    assert payload["type"] == "access"
    assert payload["is_guest"] is True
    exp = datetime.fromtimestamp(payload["exp"], tz=timezone.utc)
    now = datetime.now(timezone.utc)
    # Between 360 and 370 days in the future.
    assert timedelta(days=360) <= (exp - now) <= timedelta(days=370)


@pytest.mark.asyncio
async def test_post_auth_guest_creates_new_guest_user(client: AsyncClient, db_session) -> None:
    resp = await client.post("/auth/guest")
    assert resp.status_code == 201, resp.text
    body = resp.json()
    assert "access_token" in body
    assert body["user_id"].startswith("guest_")
    assert body["display_name"].startswith("Guest-")
    assert len(body["display_name"]) == len("Guest-") + 4  # 4-char suffix

    # The row exists and is flagged as a guest.
    from sqlalchemy import select
    row = (await db_session.execute(
        select(User).where(User.id == body["user_id"])
    )).scalar_one()
    assert row.is_guest is True
    assert row.rating == 1500.0


@pytest.mark.asyncio
async def test_post_auth_guest_is_idempotent_per_call(client: AsyncClient) -> None:
    """Each call mints a fresh guest — no session stickiness."""
    a = (await client.post("/auth/guest")).json()
    b = (await client.post("/auth/guest")).json()
    assert a["user_id"] != b["user_id"]


@pytest.mark.asyncio
async def test_guest_username_login_fails_as_401_not_500(client: AsyncClient) -> None:
    """Guest rows have synthesized password hashes; a login attempt against
    one must return 401 (cleanly, through the normal invalid-password path)
    rather than 500 from bcrypt raising on a non-hash sentinel."""
    guest = (await client.post("/auth/guest")).json()
    resp = await client.post("/auth/login", json={
        "username": guest["user_id"],  # the guest username
        "password": "anything",
    })
    assert resp.status_code == 401, resp.text
