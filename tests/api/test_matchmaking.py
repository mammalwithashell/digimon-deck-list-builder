"""Matchmaking queue tests.

Covers the pure matcher logic (filter compatibility, rating window expansion)
plus the REST + WebSocket flows and the handoff to the existing /lobby/join
pipeline.
"""
from __future__ import annotations

import asyncio
import json
from datetime import datetime, timedelta, timezone

import pytest
from httpx import ASGITransport, AsyncClient
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from digimon_gym.db.database import get_db
from digimon_gym.db.models import Base


@pytest.fixture
async def db_engine():
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
async def client(db_engine):
    from digimon_gym.api import app
    from digimon_gym.routers.matchmaking import reset_state as reset_mm
    from digimon_gym.routers.lobby import pending_games, code_to_game

    # Reset all in-memory state between tests
    reset_mm()
    pending_games.clear()
    code_to_game.clear()

    session_factory = async_sessionmaker(db_engine, class_=AsyncSession, expire_on_commit=False)

    async def override_get_db():
        async with session_factory() as session:
            yield session

    app.dependency_overrides[get_db] = override_get_db
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as c:
        yield c
    app.dependency_overrides.clear()
    reset_mm()
    pending_games.clear()
    code_to_game.clear()


async def _register_login(client: AsyncClient, username: str) -> str:
    await client.post("/auth/register", json={
        "username": username,
        "email": f"{username}@example.com",
        "password": "secure-password-123",
    })
    resp = await client.post("/auth/login", json={
        "username": username,
        "password": "secure-password-123",
    })
    return resp.json()["access_token"]


# ── Pure matcher logic ──────────────────────────────────────────────────

class TestMatcherLogic:
    """Pure-function tests against matchmaking.matcher — no HTTP/DB."""

    def _ticket(self, **overrides):
        from digimon_gym.routers.matchmaking import QueueTicket
        base = dict(
            ticket_id="t-" + overrides.get("user_id", "u"),
            user_id="u",
            display_name="U",
            queue_type="casual",
            deck=["C-1"],
            deck_raw=None,
            game_mode="standard",
            self_tier="meta",
            opponent_tier_filter="any",
            rating=None,
            created_at=datetime.now(timezone.utc),
        )
        base.update(overrides)
        return QueueTicket(**base)

    def test_format_mismatch_never_compatible(self):
        from digimon_gym.routers.matchmaking import find_match
        a = self._ticket(user_id="a", game_mode="standard")
        b = self._ticket(user_id="b", game_mode="no_restriction")
        assert find_match(a, [b], now=datetime.now(timezone.utc)) is None

    def test_same_user_cannot_match_self(self):
        from digimon_gym.routers.matchmaking import find_match
        t = self._ticket(user_id="u")
        assert find_match(t, [t], now=datetime.now(timezone.utc)) is None

    def test_casual_any_filter_matches_any_tier(self):
        from digimon_gym.routers.matchmaking import find_match
        a = self._ticket(user_id="a", self_tier="meta", opponent_tier_filter="any")
        b = self._ticket(user_id="b", self_tier="jank", opponent_tier_filter="any")
        assert find_match(a, [b], now=datetime.now(timezone.utc)) is b

    def test_casual_meta_only_filter_rejects_jank(self):
        from digimon_gym.routers.matchmaking import find_match
        a = self._ticket(user_id="a", self_tier="meta", opponent_tier_filter="meta_only")
        b = self._ticket(user_id="b", self_tier="jank", opponent_tier_filter="any")
        assert find_match(a, [b], now=datetime.now(timezone.utc)) is None

    def test_casual_jank_only_filter_rejects_meta(self):
        from digimon_gym.routers.matchmaking import find_match
        a = self._ticket(user_id="a", self_tier="jank", opponent_tier_filter="jank_only")
        b = self._ticket(user_id="b", self_tier="meta", opponent_tier_filter="any")
        assert find_match(a, [b], now=datetime.now(timezone.utc)) is None

    def test_casual_same_tier_requires_equality(self):
        from digimon_gym.routers.matchmaking import find_match
        a = self._ticket(user_id="a", self_tier="jank", opponent_tier_filter="same")
        b_mismatch = self._ticket(user_id="b", self_tier="meta", opponent_tier_filter="any")
        b_match = self._ticket(user_id="c", self_tier="jank", opponent_tier_filter="any")
        assert find_match(a, [b_mismatch], now=datetime.now(timezone.utc)) is None
        assert find_match(a, [b_mismatch, b_match], now=datetime.now(timezone.utc)) is b_match

    def test_casual_filter_is_symmetric(self):
        """If A filters jank-only but B is meta, they don't match even
        if B would accept A."""
        from digimon_gym.routers.matchmaking import find_match
        a = self._ticket(user_id="a", self_tier="jank", opponent_tier_filter="jank_only")
        b = self._ticket(user_id="b", self_tier="meta", opponent_tier_filter="any")
        assert find_match(a, [b], now=datetime.now(timezone.utc)) is None

    def test_casual_fifo_ordering(self):
        from digimon_gym.routers.matchmaking import find_match
        now = datetime.now(timezone.utc)
        older = self._ticket(
            user_id="older", created_at=now - timedelta(seconds=30),
            opponent_tier_filter="any",
        )
        newer = self._ticket(
            user_id="newer", created_at=now - timedelta(seconds=5),
            opponent_tier_filter="any",
        )
        incoming = self._ticket(user_id="new", opponent_tier_filter="any")
        assert find_match(incoming, [newer, older], now=now) is older

    def test_ranked_initial_window_fifty(self):
        from digimon_gym.routers.matchmaking import find_match
        now = datetime.now(timezone.utc)
        a = self._ticket(
            user_id="a", queue_type="ranked", rating=1500.0, created_at=now,
        )
        close = self._ticket(
            user_id="b", queue_type="ranked", rating=1530.0, created_at=now,
        )
        far = self._ticket(
            user_id="c", queue_type="ranked", rating=1600.0, created_at=now,
        )
        assert find_match(a, [close], now=now) is close
        assert find_match(a, [far], now=now) is None

    def test_ranked_window_expands_over_time(self):
        from digimon_gym.routers.matchmaking import find_match, rating_window
        start = datetime.now(timezone.utc)
        a = self._ticket(user_id="a", queue_type="ranked", rating=1500.0, created_at=start)
        b = self._ticket(user_id="b", queue_type="ranked", rating=1800.0, created_at=start)

        # At t=0: gap is 300, window is 50 → no match
        assert rating_window(a, start) == pytest.approx(50.0)
        assert find_match(a, [b], now=start) is None

        # At t=30s: window has expanded by +50 every 5s = 50 + 6*50 = 350 → match
        later = start + timedelta(seconds=30)
        assert rating_window(a, later) == pytest.approx(350.0)
        assert find_match(a, [b], now=later) is b

    def test_ranked_window_capped(self):
        from digimon_gym.routers.matchmaking import rating_window
        start = datetime.now(timezone.utc)
        a = self._ticket(user_id="a", queue_type="ranked", rating=1500.0, created_at=start)
        # After 10 minutes, window should cap at 400
        assert rating_window(a, start + timedelta(minutes=10)) == pytest.approx(400.0)

    def test_cross_queue_types_never_match(self):
        from digimon_gym.routers.matchmaking import find_match
        now = datetime.now(timezone.utc)
        a = self._ticket(user_id="a", queue_type="casual")
        b = self._ticket(
            user_id="b", queue_type="ranked", rating=1500.0,
        )
        assert find_match(a, [b], now=now) is None


# ── REST + WS flow ──────────────────────────────────────────────────────

class TestQueueEndpoints:
    async def _seed_deck(self, client: AsyncClient, headers: dict, name: str,
                         card_ids: list[str]) -> str:
        resp = await client.post("/decks", json={
            "name": name,
            "game_mode": "standard",
            "main_deck": card_ids,
        }, headers=headers)
        return resp.json()["id"]

    async def test_queue_returns_ticket_id(self, client: AsyncClient):
        token = await _register_login(client, "qa1")
        headers = {"Authorization": f"Bearer {token}"}
        deck_id = await self._seed_deck(client, headers, "D", ["BT14-001"] * 50)

        resp = await client.post("/matchmaking/queue", json={
            "queue_type": "casual",
            "deck_id": deck_id,
            "opponent_tier_filter": "any",
        }, headers=headers)
        assert resp.status_code == 201
        assert "ticket_id" in resp.json()

    async def test_duplicate_queue_returns_409(self, client: AsyncClient):
        token = await _register_login(client, "qa2")
        headers = {"Authorization": f"Bearer {token}"}
        deck_id = await self._seed_deck(client, headers, "D", ["BT14-001"] * 50)

        r1 = await client.post("/matchmaking/queue", json={
            "queue_type": "casual", "deck_id": deck_id, "opponent_tier_filter": "any",
        }, headers=headers)
        assert r1.status_code == 201
        r2 = await client.post("/matchmaking/queue", json={
            "queue_type": "casual", "deck_id": deck_id, "opponent_tier_filter": "any",
        }, headers=headers)
        assert r2.status_code == 409

    async def test_unknown_deck_returns_404(self, client: AsyncClient):
        token = await _register_login(client, "qa3")
        headers = {"Authorization": f"Bearer {token}"}
        resp = await client.post("/matchmaking/queue", json={
            "queue_type": "casual",
            "deck_id": "deck-does-not-exist",
            "opponent_tier_filter": "any",
        }, headers=headers)
        assert resp.status_code == 404

    async def test_cancel_removes_ticket(self, client: AsyncClient):
        token = await _register_login(client, "qa4")
        headers = {"Authorization": f"Bearer {token}"}
        deck_id = await self._seed_deck(client, headers, "D", ["BT14-001"] * 50)

        r = await client.post("/matchmaking/queue", json={
            "queue_type": "casual", "deck_id": deck_id, "opponent_tier_filter": "any",
        }, headers=headers)
        ticket_id = r.json()["ticket_id"]

        rd = await client.delete(f"/matchmaking/queue/{ticket_id}", headers=headers)
        assert rd.status_code == 200

        # Second cancel is 404
        rd2 = await client.delete(f"/matchmaking/queue/{ticket_id}", headers=headers)
        assert rd2.status_code == 404

    async def test_two_compatible_tickets_match(self, client: AsyncClient):
        """Two players in casual 'any' queue with same format should match
        immediately; the matcher synthesizes a join code that the second
        ticket's owner can feed to /lobby/join/{code}."""
        from digimon_gym.routers.matchmaking import tickets, user_to_ticket
        from digimon_gym.routers.lobby import pending_games, code_to_game

        tok1 = await _register_login(client, "alice")
        tok2 = await _register_login(client, "bob")
        h1 = {"Authorization": f"Bearer {tok1}"}
        h2 = {"Authorization": f"Bearer {tok2}"}
        d1 = await self._seed_deck(client, h1, "DA", ["BT14-001"] * 50)
        d2 = await self._seed_deck(client, h2, "DB", ["BT14-002"] * 50)

        r1 = await client.post("/matchmaking/queue", json={
            "queue_type": "casual", "deck_id": d1, "opponent_tier_filter": "any",
        }, headers=h1)
        assert r1.status_code == 201
        # First ticket should still be waiting
        assert len(tickets) == 1

        r2 = await client.post("/matchmaking/queue", json={
            "queue_type": "casual", "deck_id": d2, "opponent_tier_filter": "any",
        }, headers=h2)
        # Second ticket triggers a match on submit: response carries join_code
        assert r2.status_code == 200
        body = r2.json()
        assert body["status"] == "matched"
        assert "join_code" in body
        assert "game_id" in body

        # Both tickets transition to "matched" but are retained briefly so
        # the other client can poll and pick up the join code.
        assert len(tickets) == 2
        assert all(t.status == "matched" for t in tickets.values())
        # PendingGame was synthesized for the handoff
        assert body["game_id"] in pending_games
        assert body["join_code"] in code_to_game

    async def test_format_mismatch_does_not_match(self, client: AsyncClient):
        from digimon_gym.routers.matchmaking import tickets
        tok1 = await _register_login(client, "std1")
        tok2 = await _register_login(client, "nr1")
        h1 = {"Authorization": f"Bearer {tok1}"}
        h2 = {"Authorization": f"Bearer {tok2}"}

        d1 = await self._seed_deck(client, h1, "Std", ["BT14-001"] * 50)
        # Second deck in no_restriction mode
        resp = await client.post("/decks", json={
            "name": "NR", "game_mode": "no_restriction",
            "main_deck": ["BT14-001"] * 50,
        }, headers=h2)
        d2 = resp.json()["id"]

        r1 = await client.post("/matchmaking/queue", json={
            "queue_type": "casual", "deck_id": d1, "opponent_tier_filter": "any",
        }, headers=h1)
        assert r1.status_code == 201
        r2 = await client.post("/matchmaking/queue", json={
            "queue_type": "casual", "deck_id": d2, "opponent_tier_filter": "any",
        }, headers=h2)
        assert r2.status_code == 201  # queued, not matched
        assert len(tickets) == 2

    async def test_first_ticket_owner_polls_and_sees_match(self, client: AsyncClient):
        """Player A queues first (no match, gets 201 waiting). Player B queues
        second and matches. Player A polls their ticket and now sees
        `status=matched` with the join code."""
        tok1 = await _register_login(client, "pollA")
        tok2 = await _register_login(client, "pollB")
        h1 = {"Authorization": f"Bearer {tok1}"}
        h2 = {"Authorization": f"Bearer {tok2}"}
        d1 = await self._seed_deck(client, h1, "DA", ["BT14-001"] * 50)
        d2 = await self._seed_deck(client, h2, "DB", ["BT14-002"] * 50)

        r1 = await client.post("/matchmaking/queue", json={
            "queue_type": "casual", "deck_id": d1, "opponent_tier_filter": "any",
        }, headers=h1)
        t1 = r1.json()["ticket_id"]

        poll_before = await client.get(f"/matchmaking/queue/{t1}", headers=h1)
        assert poll_before.json()["status"] == "waiting"
        assert poll_before.json()["join_code"] is None

        r2 = await client.post("/matchmaking/queue", json={
            "queue_type": "casual", "deck_id": d2, "opponent_tier_filter": "any",
        }, headers=h2)
        shared_code = r2.json()["join_code"]

        poll_after = await client.get(f"/matchmaking/queue/{t1}", headers=h1)
        assert poll_after.status_code == 200
        body = poll_after.json()
        assert body["status"] == "matched"
        assert body["join_code"] == shared_code

    async def test_cannot_cancel_matched_ticket(self, client: AsyncClient):
        tok1 = await _register_login(client, "noc1")
        tok2 = await _register_login(client, "noc2")
        h1 = {"Authorization": f"Bearer {tok1}"}
        h2 = {"Authorization": f"Bearer {tok2}"}
        d1 = await self._seed_deck(client, h1, "DA", ["BT14-001"] * 50)
        d2 = await self._seed_deck(client, h2, "DB", ["BT14-002"] * 50)

        r1 = await client.post("/matchmaking/queue", json={
            "queue_type": "casual", "deck_id": d1, "opponent_tier_filter": "any",
        }, headers=h1)
        t1 = r1.json()["ticket_id"]
        await client.post("/matchmaking/queue", json={
            "queue_type": "casual", "deck_id": d2, "opponent_tier_filter": "any",
        }, headers=h2)

        r_cancel = await client.delete(f"/matchmaking/queue/{t1}", headers=h1)
        assert r_cancel.status_code == 409

    async def test_matched_pair_can_join_via_lobby_code(self, client: AsyncClient):
        """After matching, the joiner hitting /lobby/join/{code} must produce a
        live InteractiveGame — same path as the friend-code flow."""
        from digimon_gym.routers.state import active_games

        tok1 = await _register_login(client, "join1")
        tok2 = await _register_login(client, "join2")
        h1 = {"Authorization": f"Bearer {tok1}"}
        h2 = {"Authorization": f"Bearer {tok2}"}
        d1 = await self._seed_deck(client, h1, "DA", ["BT14-001"] * 50)
        d2 = await self._seed_deck(client, h2, "DB", ["BT14-002"] * 50)

        await client.post("/matchmaking/queue", json={
            "queue_type": "casual", "deck_id": d1, "opponent_tier_filter": "any",
        }, headers=h1)
        r2 = await client.post("/matchmaking/queue", json={
            "queue_type": "casual", "deck_id": d2, "opponent_tier_filter": "any",
        }, headers=h2)
        body = r2.json()
        join_code = body["join_code"]
        game_id = body["game_id"]

        # The non-host matched player joins using the code
        rj = await client.post(f"/lobby/join/{join_code}", json={
            "deck": ["BT14-002"] * 50,
        }, headers=h2)
        assert rj.status_code == 200
        assert rj.json()["game_id"] == game_id
        assert game_id in active_games


# ── WebSocket flow ──────────────────────────────────────────────────────

class TestMatchmakingWebSocket:
    """Uses FastAPI's sync TestClient because httpx.AsyncClient has no WS.

    Each test builds its own in-memory SQLite, overrides get_db, and spins
    up a fresh TestClient. State across tests is isolated via reset_mm()
    and pending_games.clear() in setup/teardown.
    """

    def _setup(self):
        from fastapi.testclient import TestClient
        from sqlalchemy.ext.asyncio import (
            AsyncSession, async_sessionmaker, create_async_engine,
        )
        import asyncio as _asyncio

        from digimon_gym.api import app
        from digimon_gym.db.database import get_db
        from digimon_gym.db.models import Base
        from digimon_gym.routers.lobby import pending_games, code_to_game
        from digimon_gym.routers.matchmaking import reset_state as reset_mm

        reset_mm()
        pending_games.clear()
        code_to_game.clear()

        loop = _asyncio.new_event_loop()
        _asyncio.set_event_loop(loop)

        engine = create_async_engine(
            "sqlite+aiosqlite:///:memory:", echo=False,
            connect_args={"check_same_thread": False},
        )

        async def _init():
            async with engine.begin() as conn:
                await conn.run_sync(Base.metadata.create_all)
        loop.run_until_complete(_init())

        session_factory = async_sessionmaker(
            engine, class_=AsyncSession, expire_on_commit=False,
        )

        async def override_get_db():
            async with session_factory() as session:
                yield session

        app.dependency_overrides[get_db] = override_get_db
        client = TestClient(app)

        def teardown():
            app.dependency_overrides.clear()
            reset_mm()
            pending_games.clear()
            code_to_game.clear()
            async def _drop():
                async with engine.begin() as conn:
                    await conn.run_sync(Base.metadata.drop_all)
                await engine.dispose()
            loop.run_until_complete(_drop())
            loop.close()
        return client, teardown

    def _register_login_sync(self, client, username: str) -> str:
        client.post("/auth/register", json={
            "username": username,
            "email": f"{username}@example.com",
            "password": "secure-password-123",
        })
        resp = client.post("/auth/login", json={
            "username": username,
            "password": "secure-password-123",
        })
        return resp.json()["access_token"]

    def _seed_deck_sync(self, client, headers, cards):
        resp = client.post("/decks", json={
            "name": "D", "game_mode": "standard", "main_deck": cards,
        }, headers=headers)
        return resp.json()["id"]

    def test_rejects_invalid_token(self):
        client, teardown = self._setup()
        try:
            with pytest.raises(Exception):
                with client.websocket_connect(
                    "/ws/matchmaking/fake-ticket?token=garbage"
                ) as ws:
                    ws.receive_json()
        finally:
            teardown()

    def test_rejects_ticket_not_owned(self):
        client, teardown = self._setup()
        try:
            tok_a = self._register_login_sync(client, "owner1")
            tok_b = self._register_login_sync(client, "intruder1")
            ha = {"Authorization": f"Bearer {tok_a}"}

            deck = self._seed_deck_sync(client, ha, ["BT14-001"] * 50)
            r = client.post("/matchmaking/queue", json={
                "queue_type": "casual", "deck_id": deck, "opponent_tier_filter": "any",
            }, headers=ha)
            tid = r.json()["ticket_id"]

            with pytest.raises(Exception):
                with client.websocket_connect(
                    f"/ws/matchmaking/{tid}?token={tok_b}"
                ) as ws:
                    ws.receive_json()
        finally:
            teardown()

    def test_sends_waiting_then_match_found(self):
        """A queues, opens WS, gets 'waiting'. B queues via REST, triggers
        match. A's WS receives 'match_found' with the same join_code."""
        client, teardown = self._setup()
        try:
            tok_a = self._register_login_sync(client, "wsA")
            tok_b = self._register_login_sync(client, "wsB")
            ha = {"Authorization": f"Bearer {tok_a}"}
            hb = {"Authorization": f"Bearer {tok_b}"}
            da = self._seed_deck_sync(client, ha, ["BT14-001"] * 50)
            db_ = self._seed_deck_sync(client, hb, ["BT14-002"] * 50)

            r_a = client.post("/matchmaking/queue", json={
                "queue_type": "casual", "deck_id": da, "opponent_tier_filter": "any",
            }, headers=ha)
            assert r_a.status_code == 201
            ticket_a = r_a.json()["ticket_id"]

            with client.websocket_connect(
                f"/ws/matchmaking/{ticket_a}?token={tok_a}"
            ) as ws:
                first = ws.receive_json()
                assert first["type"] == "waiting"

                # B queues — matcher fires synchronously on POST; the WS
                # handler is awaiting an asyncio.Event that gets set.
                r_b = client.post("/matchmaking/queue", json={
                    "queue_type": "casual", "deck_id": db_, "opponent_tier_filter": "any",
                }, headers=hb)
                assert r_b.json()["status"] == "matched"
                shared_code = r_b.json()["join_code"]

                msg = ws.receive_json()
                assert msg["type"] == "match_found"
                assert msg["join_code"] == shared_code
                assert msg["game_id"] == r_b.json()["game_id"]
        finally:
            teardown()

    def test_already_matched_ticket_pushes_match_found_on_connect(self):
        """If a ticket is already matched before the WS opens, the server
        sends match_found immediately (no 'waiting' first)."""
        client, teardown = self._setup()
        try:
            tok_a = self._register_login_sync(client, "mmA")
            tok_b = self._register_login_sync(client, "mmB")
            ha = {"Authorization": f"Bearer {tok_a}"}
            hb = {"Authorization": f"Bearer {tok_b}"}
            da = self._seed_deck_sync(client, ha, ["BT14-001"] * 50)
            db_ = self._seed_deck_sync(client, hb, ["BT14-002"] * 50)

            r_a = client.post("/matchmaking/queue", json={
                "queue_type": "casual", "deck_id": da, "opponent_tier_filter": "any",
            }, headers=ha)
            ticket_a = r_a.json()["ticket_id"]
            client.post("/matchmaking/queue", json={
                "queue_type": "casual", "deck_id": db_, "opponent_tier_filter": "any",
            }, headers=hb)

            with client.websocket_connect(
                f"/ws/matchmaking/{ticket_a}?token={tok_a}"
            ) as ws:
                msg = ws.receive_json()
                assert msg["type"] == "match_found"
        finally:
            teardown()

    def test_disconnect_cancels_waiting_ticket(self):
        """Client disconnects from a waiting ticket → ticket is removed,
        preventing ghost entries in the queue."""
        client, teardown = self._setup()
        try:
            from digimon_gym.routers.matchmaking import tickets
            tok = self._register_login_sync(client, "ghost1")
            h = {"Authorization": f"Bearer {tok}"}
            d = self._seed_deck_sync(client, h, ["BT14-001"] * 50)
            r = client.post("/matchmaking/queue", json={
                "queue_type": "casual", "deck_id": d, "opponent_tier_filter": "any",
            }, headers=h)
            tid = r.json()["ticket_id"]
            assert tid in tickets

            with client.websocket_connect(
                f"/ws/matchmaking/{tid}?token={tok}"
            ) as ws:
                ws.receive_json()  # waiting
            # WS closed; ticket should be gone
            assert tid not in tickets
        finally:
            teardown()

    def test_disconnect_after_match_does_not_cancel(self):
        """Matched tickets are retained for polling; disconnecting the WS
        after match must not discard them."""
        client, teardown = self._setup()
        try:
            from digimon_gym.routers.matchmaking import tickets

            tok_a = self._register_login_sync(client, "postA")
            tok_b = self._register_login_sync(client, "postB")
            ha = {"Authorization": f"Bearer {tok_a}"}
            hb = {"Authorization": f"Bearer {tok_b}"}
            da = self._seed_deck_sync(client, ha, ["BT14-001"] * 50)
            db_ = self._seed_deck_sync(client, hb, ["BT14-002"] * 50)

            r_a = client.post("/matchmaking/queue", json={
                "queue_type": "casual", "deck_id": da, "opponent_tier_filter": "any",
            }, headers=ha)
            tid = r_a.json()["ticket_id"]
            client.post("/matchmaking/queue", json={
                "queue_type": "casual", "deck_id": db_, "opponent_tier_filter": "any",
            }, headers=hb)
            assert tickets[tid].status == "matched"

            with client.websocket_connect(
                f"/ws/matchmaking/{tid}?token={tok_a}"
            ) as ws:
                ws.receive_json()  # match_found
            assert tid in tickets
            assert tickets[tid].status == "matched"
        finally:
            teardown()
