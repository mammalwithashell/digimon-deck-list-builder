"""Tests for the database layer: models, auth, and API endpoints."""

from __future__ import annotations

import json

import pytest
from httpx import ASGITransport, AsyncClient
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from digimon_gym.db.auth import (
    create_access_token,
    create_refresh_token_value,
    decode_access_token,
    hash_password,
    hash_token,
    verify_password,
)
from digimon_gym.db.database import get_db
from digimon_gym.db.models import (
    Base,
    Deck,
    Friendship,
    GameParticipant,
    GameSession,
    RefreshToken,
    User,
    UserAsset,
    UserPreferences,
)


# ── Test database setup ─────────────────────────────────────────────────

@pytest.fixture
async def db_engine():
    """Create an in-memory SQLite engine for tests."""
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
async def db_session(db_engine):
    """Provide a transactional async session for tests."""
    session_factory = async_sessionmaker(db_engine, class_=AsyncSession, expire_on_commit=False)
    async with session_factory() as session:
        yield session


@pytest.fixture
async def client(db_engine):
    """AsyncClient wired to the FastAPI app with test DB."""
    from digimon_gym.api import app

    session_factory = async_sessionmaker(db_engine, class_=AsyncSession, expire_on_commit=False)

    async def override_get_db():
        async with session_factory() as session:
            yield session

    app.dependency_overrides[get_db] = override_get_db
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as c:
        yield c
    app.dependency_overrides.clear()


# ── Auth utility tests ──────────────────────────────────────────────────

class TestPasswordHashing:
    def test_hash_and_verify(self):
        pw = "my-secure-password-123"
        hashed = hash_password(pw)
        assert hashed != pw
        assert verify_password(pw, hashed)

    def test_wrong_password_fails(self):
        hashed = hash_password("correct")
        assert not verify_password("wrong", hashed)


class TestJWT:
    def test_create_and_decode_access_token(self):
        token = create_access_token("user-123", "testuser")
        payload = decode_access_token(token)
        assert payload["sub"] == "user-123"
        assert payload["username"] == "testuser"
        assert payload["type"] == "access"

    def test_refresh_token_hashing(self):
        value = create_refresh_token_value()
        h = hash_token(value)
        assert len(h) == 64  # SHA-256 hex digest
        assert hash_token(value) == h  # Deterministic


# ── Model CRUD tests ───────────────────────────────────────────────────

class TestUserModel:
    async def test_create_user(self, db_session: AsyncSession):
        user = User(
            username="testplayer",
            email="test@example.com",
            password_hash=hash_password("pass123!"),
        )
        db_session.add(user)
        await db_session.commit()
        await db_session.refresh(user)

        assert user.id is not None
        assert user.username == "testplayer"
        assert user.is_active == 1
        assert user.created_at is not None


class TestDeckModel:
    async def test_create_standard_deck(self, db_session: AsyncSession):
        user = User(username="deckbuilder", email="db@test.com", password_hash="hash")
        db_session.add(user)
        await db_session.commit()

        deck = Deck(
            owner_id=user.id,
            name="My Red Deck",
            game_mode="standard",
            main_deck=json.dumps(["BT20-001"] * 50),
            egg_deck=json.dumps(["BT20-002"] * 5),
        )
        db_session.add(deck)
        await db_session.commit()
        await db_session.refresh(deck)

        assert deck.id is not None
        assert deck.game_mode == "standard"
        assert deck.titan_role is None
        assert deck.commander_id is None
        cards = json.loads(deck.main_deck)
        assert len(cards) == 50

    async def test_create_titan_deck(self, db_session: AsyncSession):
        user = User(username="titanplayer", email="titan@test.com", password_hash="hash")
        db_session.add(user)
        await db_session.commit()

        deck = Deck(
            owner_id=user.id,
            name="Titan Boss Deck",
            game_mode="titan",
            titan_role="titan",
            main_deck=json.dumps(["BT20-001"] * 80),
        )
        db_session.add(deck)
        await db_session.commit()
        assert deck.titan_role == "titan"

    async def test_create_edh_deck(self, db_session: AsyncSession):
        user = User(username="edhplayer", email="edh@test.com", password_hash="hash")
        db_session.add(user)
        await db_session.commit()

        deck = Deck(
            owner_id=user.id,
            name="Commander Deck",
            game_mode="edh_commander",
            commander_id="BT20-050",
            main_deck=json.dumps(["BT20-001"] * 70),
        )
        db_session.add(deck)
        await db_session.commit()
        assert deck.commander_id == "BT20-050"


class TestFriendshipModel:
    async def test_create_friendship(self, db_session: AsyncSession):
        user_a = User(username="alice", email="a@test.com", password_hash="hash")
        user_b = User(username="bob", email="b@test.com", password_hash="hash")
        db_session.add_all([user_a, user_b])
        await db_session.commit()

        friendship = Friendship(
            user_id=user_a.id,
            friend_id=user_b.id,
            status="pending",
        )
        db_session.add(friendship)
        await db_session.commit()
        assert friendship.status == "pending"


# ── API endpoint tests ──────────────────────────────────────────────────

class TestAuthEndpoints:
    async def test_register(self, client: AsyncClient):
        resp = await client.post("/auth/register", json={
            "username": "newuser",
            "email": "new@example.com",
            "password": "secure-password-123",
        })
        assert resp.status_code == 201
        data = resp.json()
        assert data["username"] == "newuser"
        assert data["email"] == "new@example.com"
        assert "password_hash" not in data

    async def test_register_duplicate_username(self, client: AsyncClient):
        await client.post("/auth/register", json={
            "username": "dupe",
            "email": "a@example.com",
            "password": "secure-password-123",
        })
        resp = await client.post("/auth/register", json={
            "username": "dupe",
            "email": "b@example.com",
            "password": "secure-password-123",
        })
        assert resp.status_code == 409

    async def test_login_and_access(self, client: AsyncClient):
        # Register
        await client.post("/auth/register", json={
            "username": "logintest",
            "email": "login@example.com",
            "password": "secure-password-123",
        })
        # Login
        resp = await client.post("/auth/login", json={
            "username": "logintest",
            "password": "secure-password-123",
        })
        assert resp.status_code == 200
        tokens = resp.json()
        assert "access_token" in tokens
        assert "refresh_token" in tokens

        # Access protected endpoint
        resp = await client.get("/users/me", headers={
            "Authorization": f"Bearer {tokens['access_token']}"
        })
        assert resp.status_code == 200
        assert resp.json()["username"] == "logintest"

    async def test_login_wrong_password(self, client: AsyncClient):
        await client.post("/auth/register", json={
            "username": "wrongpw",
            "email": "wp@example.com",
            "password": "correct-password-123",
        })
        resp = await client.post("/auth/login", json={
            "username": "wrongpw",
            "password": "wrong-password-456",
        })
        assert resp.status_code == 401

    async def test_refresh_token_flow(self, client: AsyncClient):
        await client.post("/auth/register", json={
            "username": "refreshtest",
            "email": "refresh@example.com",
            "password": "secure-password-123",
        })
        login_resp = await client.post("/auth/login", json={
            "username": "refreshtest",
            "password": "secure-password-123",
        })
        tokens = login_resp.json()

        # Refresh
        resp = await client.post("/auth/refresh", json={
            "refresh_token": tokens["refresh_token"],
        })
        assert resp.status_code == 200
        new_tokens = resp.json()
        assert "access_token" in new_tokens
        # New refresh token is always different (random)
        assert new_tokens["refresh_token"] != tokens["refresh_token"]

        # Old refresh token should be revoked
        resp = await client.post("/auth/refresh", json={
            "refresh_token": tokens["refresh_token"],
        })
        assert resp.status_code == 401


class TestDeckEndpoints:
    async def _register_and_login(self, client: AsyncClient, username: str = "deckuser"):
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

    async def test_create_and_get_deck(self, client: AsyncClient):
        token = await self._register_and_login(client)
        headers = {"Authorization": f"Bearer {token}"}

        resp = await client.post("/decks", json={
            "name": "Test Standard Deck",
            "game_mode": "standard",
            "main_deck": ["BT14-001"] * 50,
            "egg_deck": ["BT14-002"] * 5,
        }, headers=headers)
        assert resp.status_code == 201
        deck = resp.json()
        assert deck["name"] == "Test Standard Deck"
        assert deck["game_mode"] == "standard"
        assert len(deck["main_deck"]) == 50

        # Get it back
        resp = await client.get(f"/decks/{deck['id']}", headers=headers)
        assert resp.status_code == 200
        assert resp.json()["name"] == "Test Standard Deck"

    async def test_list_my_decks(self, client: AsyncClient):
        token = await self._register_and_login(client, "listuser")
        headers = {"Authorization": f"Bearer {token}"}

        for i in range(3):
            await client.post("/decks", json={
                "name": f"Deck {i}",
                "game_mode": "standard",
                "main_deck": ["BT14-001"] * 50,
            }, headers=headers)

        resp = await client.get("/decks", headers=headers)
        assert resp.status_code == 200
        assert len(resp.json()) == 3

    async def test_update_deck(self, client: AsyncClient):
        token = await self._register_and_login(client, "updateuser")
        headers = {"Authorization": f"Bearer {token}"}

        resp = await client.post("/decks", json={
            "name": "Original Name",
            "game_mode": "standard",
            "main_deck": ["BT14-001"] * 50,
        }, headers=headers)
        deck_id = resp.json()["id"]

        resp = await client.put(f"/decks/{deck_id}", json={
            "name": "Updated Name",
        }, headers=headers)
        assert resp.status_code == 200
        assert resp.json()["name"] == "Updated Name"

    async def test_delete_deck(self, client: AsyncClient):
        token = await self._register_and_login(client, "deleteuser")
        headers = {"Authorization": f"Bearer {token}"}

        resp = await client.post("/decks", json={
            "name": "To Delete",
            "game_mode": "standard",
            "main_deck": ["BT14-001"] * 50,
        }, headers=headers)
        deck_id = resp.json()["id"]

        resp = await client.delete(f"/decks/{deck_id}", headers=headers)
        assert resp.status_code == 200

        resp = await client.get(f"/decks/{deck_id}", headers=headers)
        assert resp.status_code == 404

    async def test_edh_requires_commander(self, client: AsyncClient):
        token = await self._register_and_login(client, "edhuser")
        headers = {"Authorization": f"Bearer {token}"}

        resp = await client.post("/decks", json={
            "name": "Bad EDH Deck",
            "game_mode": "edh_commander",
            "main_deck": ["BT14-001"] * 70,
        }, headers=headers)
        assert resp.status_code == 400
        assert "commander_id" in resp.json()["detail"]

    async def test_titan_requires_role(self, client: AsyncClient):
        token = await self._register_and_login(client, "titanuser")
        headers = {"Authorization": f"Bearer {token}"}

        resp = await client.post("/decks", json={
            "name": "Bad Titan Deck",
            "game_mode": "titan",
            "main_deck": ["BT14-001"] * 80,
        }, headers=headers)
        assert resp.status_code == 400
        assert "titan_role" in resp.json()["detail"]


class TestFriendEndpoints:
    async def _make_user(self, client: AsyncClient, name: str) -> str:
        await client.post("/auth/register", json={
            "username": name,
            "email": f"{name}@example.com",
            "password": "secure-password-123",
        })
        resp = await client.post("/auth/login", json={
            "username": name,
            "password": "secure-password-123",
        })
        return resp.json()["access_token"]

    async def _get_user_id(self, client: AsyncClient, token: str) -> str:
        resp = await client.get("/users/me", headers={"Authorization": f"Bearer {token}"})
        return resp.json()["id"]

    async def test_friend_request_and_accept(self, client: AsyncClient):
        token_a = await self._make_user(client, "friendA")
        token_b = await self._make_user(client, "friendB")
        id_a = await self._get_user_id(client, token_a)
        id_b = await self._get_user_id(client, token_b)

        # A sends request to B
        resp = await client.post(f"/friends/request/{id_b}", headers={"Authorization": f"Bearer {token_a}"})
        assert resp.status_code == 200
        assert resp.json()["status"] == "pending"

        # B sees pending request
        resp = await client.get("/friends/requests", headers={"Authorization": f"Bearer {token_b}"})
        assert resp.status_code == 200
        assert len(resp.json()) == 1

        # B accepts
        resp = await client.post(f"/friends/accept/{id_a}", headers={"Authorization": f"Bearer {token_b}"})
        assert resp.status_code == 200

        # Both see each other as friends
        resp = await client.get("/friends", headers={"Authorization": f"Bearer {token_a}"})
        assert len(resp.json()) == 1
        resp = await client.get("/friends", headers={"Authorization": f"Bearer {token_b}"})
        assert len(resp.json()) == 1

    async def test_block_user(self, client: AsyncClient):
        token_a = await self._make_user(client, "blockerA")
        token_b = await self._make_user(client, "blockerB")
        id_b = await self._get_user_id(client, token_b)
        id_a = await self._get_user_id(client, token_a)

        # A blocks B
        resp = await client.post(f"/friends/block/{id_b}", headers={"Authorization": f"Bearer {token_a}"})
        assert resp.status_code == 200

        # B cannot send request to A
        resp = await client.post(f"/friends/request/{id_a}", headers={"Authorization": f"Bearer {token_b}"})
        assert resp.status_code == 403


class TestChangePassword:
    async def _register_and_login(self, client: AsyncClient, username: str = "pwuser"):
        await client.post("/auth/register", json={
            "username": username,
            "email": f"{username}@example.com",
            "password": "old-password-123",
        })
        resp = await client.post("/auth/login", json={
            "username": username,
            "password": "old-password-123",
        })
        return resp.json()

    async def test_change_password_success(self, client: AsyncClient):
        tokens = await self._register_and_login(client, "changepw1")
        headers = {"Authorization": f"Bearer {tokens['access_token']}"}

        resp = await client.post("/auth/change-password", json={
            "current_password": "old-password-123",
            "new_password": "new-password-456",
        }, headers=headers)
        assert resp.status_code == 200
        assert resp.json()["status"] == "password_changed"

        # Old password no longer works
        resp = await client.post("/auth/login", json={
            "username": "changepw1",
            "password": "old-password-123",
        })
        assert resp.status_code == 401

        # New password works
        resp = await client.post("/auth/login", json={
            "username": "changepw1",
            "password": "new-password-456",
        })
        assert resp.status_code == 200

    async def test_change_password_wrong_current(self, client: AsyncClient):
        tokens = await self._register_and_login(client, "changepw2")
        headers = {"Authorization": f"Bearer {tokens['access_token']}"}

        resp = await client.post("/auth/change-password", json={
            "current_password": "wrong-password-999",
            "new_password": "new-password-456",
        }, headers=headers)
        assert resp.status_code == 400
        assert "incorrect" in resp.json()["detail"].lower()

    async def test_change_password_revokes_refresh_tokens(self, client: AsyncClient):
        tokens = await self._register_and_login(client, "changepw3")
        headers = {"Authorization": f"Bearer {tokens['access_token']}"}

        # Change password
        await client.post("/auth/change-password", json={
            "current_password": "old-password-123",
            "new_password": "new-password-456",
        }, headers=headers)

        # Old refresh token should be revoked
        resp = await client.post("/auth/refresh", json={
            "refresh_token": tokens["refresh_token"],
        })
        assert resp.status_code == 401

    async def test_change_password_requires_auth(self, client: AsyncClient):
        resp = await client.post("/auth/change-password", json={
            "current_password": "old-password-123",
            "new_password": "new-password-456",
        })
        assert resp.status_code == 401

    async def test_change_password_too_short(self, client: AsyncClient):
        tokens = await self._register_and_login(client, "changepw4")
        headers = {"Authorization": f"Bearer {tokens['access_token']}"}

        resp = await client.post("/auth/change-password", json={
            "current_password": "old-password-123",
            "new_password": "short",
        }, headers=headers)
        assert resp.status_code == 422  # Pydantic validation error


class TestAccountDeactivation:
    async def _register_and_login(self, client: AsyncClient, username: str):
        await client.post("/auth/register", json={
            "username": username,
            "email": f"{username}@example.com",
            "password": "secure-password-123",
        })
        resp = await client.post("/auth/login", json={
            "username": username,
            "password": "secure-password-123",
        })
        return resp.json()

    async def test_deactivate_account(self, client: AsyncClient):
        tokens = await self._register_and_login(client, "deactivate1")
        headers = {"Authorization": f"Bearer {tokens['access_token']}"}

        resp = await client.request("DELETE", "/users/me", json={
            "password": "secure-password-123",
        }, headers=headers)
        assert resp.status_code == 200
        assert resp.json()["status"] == "account_deactivated"

        # Can no longer login
        resp = await client.post("/auth/login", json={
            "username": "deactivate1",
            "password": "secure-password-123",
        })
        # Login succeeds but account is disabled
        assert resp.status_code == 403

    async def test_deactivate_wrong_password(self, client: AsyncClient):
        tokens = await self._register_and_login(client, "deactivate2")
        headers = {"Authorization": f"Bearer {tokens['access_token']}"}

        resp = await client.request("DELETE", "/users/me", json={
            "password": "wrong-password",
        }, headers=headers)
        assert resp.status_code == 400

    async def test_deactivated_user_not_in_search(self, client: AsyncClient):
        tokens = await self._register_and_login(client, "deactivate3")
        headers = {"Authorization": f"Bearer {tokens['access_token']}"}

        # Deactivate
        await client.request("DELETE", "/users/me", json={
            "password": "secure-password-123",
        }, headers=headers)

        # Search should not find deactivated user
        resp = await client.get("/users/search?q=deactivate3")
        assert resp.status_code == 200
        assert len(resp.json()) == 0

    async def test_deactivate_revokes_refresh_tokens(self, client: AsyncClient):
        tokens = await self._register_and_login(client, "deactivate4")
        headers = {"Authorization": f"Bearer {tokens['access_token']}"}

        await client.request("DELETE", "/users/me", json={
            "password": "secure-password-123",
        }, headers=headers)

        # Refresh token should be revoked
        resp = await client.post("/auth/refresh", json={
            "refresh_token": tokens["refresh_token"],
        })
        assert resp.status_code == 401


class TestGameSessionModel:
    async def test_create_game_session(self, db_session: AsyncSession):
        user = User(username="gamer", email="gamer@test.com", password_hash="hash")
        db_session.add(user)
        await db_session.commit()

        session = GameSession(game_mode="standard")
        db_session.add(session)
        await db_session.flush()

        p1 = GameParticipant(
            game_id=session.id, player_slot=0, user_id=user.id, player_type="human"
        )
        p2 = GameParticipant(
            game_id=session.id, player_slot=1, player_type="agent"
        )
        db_session.add_all([p1, p2])
        await db_session.commit()

        assert session.id is not None
        assert session.game_mode == "standard"


class TestGameHistoryEndpoints:
    async def _register_and_login(self, client: AsyncClient, username: str):
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

    async def _get_user_id(self, client: AsyncClient, token: str) -> str:
        resp = await client.get("/users/me", headers={"Authorization": f"Bearer {token}"})
        return resp.json()["id"]

    async def test_create_game_session(self, client: AsyncClient):
        token = await self._register_and_login(client, "gamesession1")
        headers = {"Authorization": f"Bearer {token}"}

        resp = await client.post("/games/sessions?game_mode=standard", headers=headers)
        assert resp.status_code == 201
        data = resp.json()
        assert data["game_mode"] == "standard"
        assert len(data["participants"]) == 1
        assert data["participants"][0]["player_type"] == "human"

    async def test_complete_game_session(self, client: AsyncClient):
        token = await self._register_and_login(client, "gamesession2")
        headers = {"Authorization": f"Bearer {token}"}

        # Create session
        resp = await client.post("/games/sessions?game_mode=standard", headers=headers)
        session_id = resp.json()["id"]

        # Complete it
        resp = await client.post(
            f"/games/sessions/{session_id}/complete?result_type=completed&winner_slot=0&total_turns=15",
            headers=headers,
        )
        assert resp.status_code == 200
        assert resp.json()["status"] == "completed"

    async def test_complete_already_completed(self, client: AsyncClient):
        token = await self._register_and_login(client, "gamesession3")
        headers = {"Authorization": f"Bearer {token}"}

        resp = await client.post("/games/sessions?game_mode=standard", headers=headers)
        session_id = resp.json()["id"]

        # Complete once
        await client.post(
            f"/games/sessions/{session_id}/complete?result_type=completed&winner_slot=0",
            headers=headers,
        )

        # Try to complete again
        resp = await client.post(
            f"/games/sessions/{session_id}/complete?result_type=completed&winner_slot=0",
            headers=headers,
        )
        assert resp.status_code == 400

    async def test_get_game_session_detail(self, client: AsyncClient):
        token = await self._register_and_login(client, "gamesession4")
        headers = {"Authorization": f"Bearer {token}"}

        resp = await client.post("/games/sessions?game_mode=standard", headers=headers)
        session_id = resp.json()["id"]

        resp = await client.get(f"/games/sessions/{session_id}", headers=headers)
        assert resp.status_code == 200
        assert resp.json()["id"] == session_id

    async def test_get_session_not_participant(self, client: AsyncClient):
        token_a = await self._register_and_login(client, "gamesession5a")
        token_b = await self._register_and_login(client, "gamesession5b")
        headers_a = {"Authorization": f"Bearer {token_a}"}
        headers_b = {"Authorization": f"Bearer {token_b}"}

        # A creates a session
        resp = await client.post("/games/sessions?game_mode=standard", headers=headers_a)
        session_id = resp.json()["id"]

        # B tries to view it
        resp = await client.get(f"/games/sessions/{session_id}", headers=headers_b)
        assert resp.status_code == 403

    async def test_game_history_empty(self, client: AsyncClient):
        token = await self._register_and_login(client, "gamesession6")
        headers = {"Authorization": f"Bearer {token}"}

        resp = await client.get("/games/history", headers=headers)
        assert resp.status_code == 200
        assert resp.json() == []

    async def test_game_history_with_sessions(self, client: AsyncClient):
        token = await self._register_and_login(client, "gamesession7")
        headers = {"Authorization": f"Bearer {token}"}

        # Create 3 sessions
        for _ in range(3):
            await client.post("/games/sessions?game_mode=standard", headers=headers)

        resp = await client.get("/games/history", headers=headers)
        assert resp.status_code == 200
        assert len(resp.json()) == 3

    async def test_game_stats_empty(self, client: AsyncClient):
        token = await self._register_and_login(client, "gamesession8")
        headers = {"Authorization": f"Bearer {token}"}

        resp = await client.get("/games/history/stats", headers=headers)
        assert resp.status_code == 200
        data = resp.json()
        assert data["total_games"] == 0
        assert data["win_rate"] == 0.0

    async def test_game_stats_with_results(self, client: AsyncClient):
        token = await self._register_and_login(client, "gamesession9")
        headers = {"Authorization": f"Bearer {token}"}

        # Create and complete sessions with different results
        for i in range(3):
            resp = await client.post("/games/sessions?game_mode=standard", headers=headers)
            session_id = resp.json()["id"]
            winner = 0 if i < 2 else None  # 2 wins, 1 draw
            result_type = "completed" if i < 2 else "abandoned"
            params = f"result_type={result_type}"
            if winner is not None:
                params += f"&winner_slot={winner}"
            await client.post(
                f"/games/sessions/{session_id}/complete?{params}",
                headers=headers,
            )

        resp = await client.get("/games/history/stats", headers=headers)
        assert resp.status_code == 200
        data = resp.json()
        assert data["wins"] == 2
        assert data["total_games"] >= 2

    async def test_game_history_requires_auth(self, client: AsyncClient):
        resp = await client.get("/games/history")
        assert resp.status_code == 401

    async def test_game_stats_requires_auth(self, client: AsyncClient):
        resp = await client.get("/games/history/stats")
        assert resp.status_code == 401


class TestExistingEndpointsStillWork:
    """Verify backward compatibility with existing game/deck endpoints."""

    async def test_health_check(self, client: AsyncClient):
        resp = await client.get("/")
        assert resp.status_code == 200
        assert resp.json()["status"] == "ok"
