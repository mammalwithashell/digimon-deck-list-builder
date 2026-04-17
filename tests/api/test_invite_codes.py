"""Tests for alpha invite-code gating on /auth/register and the
/admin/invite-codes admin endpoints."""

import pytest
from httpx import ASGITransport, AsyncClient
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from digimon_gym.config import settings
from digimon_gym.db.auth import ROLE_ADMIN, assign_role_to_user, hash_password
from digimon_gym.db.database import get_db
from digimon_gym.db.models import Base, InviteCode, User


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

    session_factory = async_sessionmaker(db_engine, class_=AsyncSession, expire_on_commit=False)

    async def override_get_db():
        async with session_factory() as session:
            yield session

    app.dependency_overrides[get_db] = override_get_db
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as c:
        yield c
    app.dependency_overrides.clear()


@pytest.fixture
async def admin_token(db_engine, client):
    session_factory = async_sessionmaker(db_engine, class_=AsyncSession, expire_on_commit=False)
    async with session_factory() as session:
        admin = User(
            username="admin",
            email="admin@example.com",
            password_hash=hash_password("admin-password-xyz"),
        )
        session.add(admin)
        await session.flush()
        await assign_role_to_user(session, admin.id, ROLE_ADMIN)
        await session.commit()

    resp = await client.post(
        "/auth/login", json={"username": "admin", "password": "admin-password-xyz"}
    )
    assert resp.status_code == 200
    return resp.json()["access_token"]


class TestInviteGateDisabled:
    """When INVITE_CODES_REQUIRED is false, registration is open."""

    async def test_register_without_code_succeeds(self, client, monkeypatch):
        monkeypatch.setattr(settings, "invite_codes_required", False)
        resp = await client.post(
            "/auth/register",
            json={
                "username": "openuser",
                "email": "open@example.com",
                "password": "a-long-enough-password",
            },
        )
        assert resp.status_code == 201


class TestInviteGateEnabled:
    """When INVITE_CODES_REQUIRED is true, a valid single-use code is required."""

    async def test_missing_code_is_rejected(self, client, monkeypatch):
        monkeypatch.setattr(settings, "invite_codes_required", True)
        resp = await client.post(
            "/auth/register",
            json={
                "username": "user1",
                "email": "u1@example.com",
                "password": "a-long-enough-password",
            },
        )
        assert resp.status_code == 400
        assert "invite code" in resp.json()["detail"].lower()

    async def test_unknown_code_is_rejected(self, client, monkeypatch):
        monkeypatch.setattr(settings, "invite_codes_required", True)
        resp = await client.post(
            "/auth/register",
            json={
                "username": "user2",
                "email": "u2@example.com",
                "password": "a-long-enough-password",
                "invite_code": "does-not-exist",
            },
        )
        assert resp.status_code == 400

    async def test_valid_code_is_redeemed_once(self, client, db_engine, monkeypatch):
        monkeypatch.setattr(settings, "invite_codes_required", True)
        session_factory = async_sessionmaker(
            db_engine, class_=AsyncSession, expire_on_commit=False
        )
        async with session_factory() as s:
            s.add(InviteCode(code="GOOD-CODE-1"))
            await s.commit()

        ok = await client.post(
            "/auth/register",
            json={
                "username": "user3",
                "email": "u3@example.com",
                "password": "a-long-enough-password",
                "invite_code": "GOOD-CODE-1",
            },
        )
        assert ok.status_code == 201

        reuse = await client.post(
            "/auth/register",
            json={
                "username": "user4",
                "email": "u4@example.com",
                "password": "a-long-enough-password",
                "invite_code": "GOOD-CODE-1",
            },
        )
        assert reuse.status_code == 400


class TestAdminInviteCodeEndpoints:
    async def test_mint_and_list(self, client, admin_token):
        headers = {"Authorization": f"Bearer {admin_token}"}

        mint = await client.post(
            "/admin/invite-codes",
            headers=headers,
            json={"count": 3, "note": "alpha wave 1"},
        )
        assert mint.status_code == 201
        minted = mint.json()
        assert len(minted) == 3
        codes = {row["code"] for row in minted}
        assert len(codes) == 3  # all unique
        assert all(row["redeemed_at"] is None for row in minted)

        listed = await client.get("/admin/invite-codes", headers=headers)
        assert listed.status_code == 200
        listed_rows = listed.json()
        assert {row["code"] for row in listed_rows} >= codes

    async def test_non_admin_is_forbidden(self, client):
        reg = await client.post(
            "/auth/register",
            json={
                "username": "regular",
                "email": "reg@example.com",
                "password": "a-long-enough-password",
            },
        )
        assert reg.status_code == 201
        login = await client.post(
            "/auth/login",
            json={"username": "regular", "password": "a-long-enough-password"},
        )
        token = login.json()["access_token"]

        resp = await client.post(
            "/admin/invite-codes",
            headers={"Authorization": f"Bearer {token}"},
            json={"count": 1},
        )
        assert resp.status_code == 403
