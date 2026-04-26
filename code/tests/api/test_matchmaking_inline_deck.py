"""Matchmaking queue with an inline deck payload (guest path)."""
from __future__ import annotations

import pytest
from httpx import ASGITransport, AsyncClient
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker

from server.db.database import get_db
from server.routers import matchmaking as mm


@pytest.fixture(autouse=True)
def _clean_mm_state():
    mm.reset_state()
    yield
    mm.reset_state()


@pytest.fixture
async def client(db_engine):
    from server.api import app
    from server.routers.lobby import code_to_game, pending_games

    # Reset in-memory lobby state between tests (matchmaking handoff
    # writes to pending_games / code_to_game).
    pending_games.clear()
    code_to_game.clear()

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
    pending_games.clear()
    code_to_game.clear()


@pytest.mark.asyncio
async def test_queue_accepts_inline_deck_for_guest(client: AsyncClient) -> None:
    # Mint a guest token.
    guest = (await client.post("/auth/guest")).json()
    headers = {"Authorization": f"Bearer {guest['access_token']}"}

    # Use no_restriction to skip the ban-list validation layer — this test
    # is exercising the inline-deck path, not format rules (covered by
    # test_matchmaking.py).
    body = {
        "queue_type": "casual",
        "main_deck": ["BT1-001"] * 50,
        "egg_deck": ["BT1-002"] * 5,
        "game_mode": "no_restriction",
    }
    resp = await client.post("/matchmaking/queue", json=body, headers=headers)
    assert resp.status_code in (200, 201), resp.text
    payload = resp.json()
    assert payload["status"] == "waiting"
    ticket_id = payload["ticket_id"]

    ticket = mm.tickets[ticket_id]
    assert ticket.deck == ["BT1-001"] * 50 + ["BT1-002"] * 5
    assert ticket.game_mode == "no_restriction"
    assert ticket.user_id == guest["user_id"]
    # Inline-deck (guest) path leaves self_tier unset so jank/sweat queues
    # aren't eligible — the alpha design explicitly defers classifier
    # support for guests.
    assert ticket.self_tier is None


@pytest.mark.asyncio
async def test_queue_rejects_request_missing_both_deck_id_and_inline(client: AsyncClient) -> None:
    guest = (await client.post("/auth/guest")).json()
    headers = {"Authorization": f"Bearer {guest['access_token']}"}
    resp = await client.post(
        "/matchmaking/queue",
        json={"queue_type": "casual"},
        headers=headers,
    )
    assert resp.status_code == 400
    assert "deck" in resp.json()["detail"].lower()


@pytest.mark.asyncio
async def test_inline_deck_cannot_queue_for_jank(client: AsyncClient) -> None:
    """Guest-inline tickets have self_tier=None, which is deliberately not
    eligible for jank or sweat queues (those require a classifier-assigned
    tier that only lives on server-side Deck rows)."""
    guest = (await client.post("/auth/guest")).json()
    headers = {"Authorization": f"Bearer {guest['access_token']}"}
    body = {
        "queue_type": "jank",
        "main_deck": ["BT1-001"] * 50,
        "egg_deck": ["BT1-002"] * 5,
        "game_mode": "no_restriction",
    }
    resp = await client.post("/matchmaking/queue", json=body, headers=headers)
    assert resp.status_code == 422
    assert "jank" in resp.json()["detail"].lower()
