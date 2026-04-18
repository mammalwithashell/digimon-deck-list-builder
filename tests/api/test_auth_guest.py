"""Guest-user endpoint + integration with matchmaking."""
from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pytest
from jose import jwt

from digimon_gym.db.auth import (
    ALGORITHM,
    SECRET_KEY,
    create_guest_access_token,
)
from digimon_gym.db.models import User


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
