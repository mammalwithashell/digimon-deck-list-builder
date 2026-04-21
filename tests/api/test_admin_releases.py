"""Tests for admin release management endpoints (Tauri auto-updater)."""

from __future__ import annotations

import boto3
import pytest
from botocore.client import Config as BotocoreConfig
from httpx import ASGITransport, AsyncClient
from moto import mock_s3
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from digimon_gym.db.auth import ROLE_ADMIN, assign_role_to_user
from digimon_gym.db.database import get_db
from digimon_gym.db.models import AppRelease, AppReleaseArtifact, Base, User
from digimon_gym.storage import spaces

# ---------------------------------------------------------------------------
# Moto / Spaces constants (mirror test_admin_models.py)
# ---------------------------------------------------------------------------

_BUCKET = "test-digimon-bucket"
_REGION = "us-east-1"
_ENDPOINT = "https://s3.us-east-1.amazonaws.com"
_KEY_ID = "testing"
_SECRET = "testing"


# ---------------------------------------------------------------------------
# DB + app fixtures
# ---------------------------------------------------------------------------


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
async def session_factory(db_engine):
    return async_sessionmaker(db_engine, class_=AsyncSession, expire_on_commit=False)


@pytest.fixture
async def client(session_factory, monkeypatch):
    from digimon_gym.api import app, ai_task_worker, training_job_worker

    async def override_get_db():
        async with session_factory() as session:
            yield session

    async def _noop():
        return None

    monkeypatch.setattr(ai_task_worker, "start", _noop)
    monkeypatch.setattr(ai_task_worker, "stop", _noop)
    monkeypatch.setattr(training_job_worker, "start", _noop)
    monkeypatch.setattr(training_job_worker, "stop", _noop)

    app.dependency_overrides[get_db] = override_get_db
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as c:
        yield c
    app.dependency_overrides.clear()


# ---------------------------------------------------------------------------
# Moto S3 fixture (function-scoped, wraps each test)
# ---------------------------------------------------------------------------


@pytest.fixture(autouse=True)
def _mock_spaces(monkeypatch):
    """Wrap every test with moto mock_s3 and create the test bucket."""
    monkeypatch.setenv("SPACES_ENDPOINT", _ENDPOINT)
    monkeypatch.setenv("SPACES_BUCKET", _BUCKET)
    monkeypatch.setenv("SPACES_REGION", _REGION)
    monkeypatch.setenv("SPACES_KEY", _KEY_ID)
    monkeypatch.setenv("SPACES_SECRET", _SECRET)
    monkeypatch.setenv("AWS_ACCESS_KEY_ID", _KEY_ID)
    monkeypatch.setenv("AWS_SECRET_ACCESS_KEY", _SECRET)
    monkeypatch.setenv("AWS_DEFAULT_REGION", _REGION)
    spaces._client.cache_clear()

    with mock_s3():
        raw = _raw_client()
        raw.create_bucket(Bucket=_BUCKET)
        yield

    spaces._client.cache_clear()


def _raw_client():
    return boto3.client(
        "s3",
        endpoint_url=_ENDPOINT,
        region_name=_REGION,
        aws_access_key_id=_KEY_ID,
        aws_secret_access_key=_SECRET,
        config=BotocoreConfig(signature_version="s3v4"),
    )


# ---------------------------------------------------------------------------
# Auth helpers
# ---------------------------------------------------------------------------


async def _register_and_login(client: AsyncClient, username: str) -> str:
    await client.post(
        "/auth/register",
        json={
            "username": username,
            "email": f"{username}@example.com",
            "password": "secure-password-123",
        },
    )
    login = await client.post(
        "/auth/login",
        json={"username": username, "password": "secure-password-123"},
    )
    assert login.status_code == 200
    return login.json()["access_token"]


async def _grant_admin(session_factory, username: str) -> None:
    async with session_factory() as db:
        user = (
            await db.execute(select(User).where(User.username == username))
        ).scalar_one()
        await assign_role_to_user(db, user.id, ROLE_ADMIN)
        await db.commit()


async def _create_release(client: AsyncClient, token: str, **overrides) -> dict:
    payload = {
        "version": "0.2.0-alpha.3",
        "channel": "alpha",
        "engine_commit": "abc1234",
        "min_version": "0.1.0",
        "release_notes": "Test release.",
        "targets": ["windows-x86_64", "linux-x86_64"],
    }
    payload.update(overrides)
    resp = await client.post(
        "/admin/releases",
        headers={"Authorization": f"Bearer {token}"},
        json=payload,
    )
    return resp


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestAdminReleasesCreate:
    async def test_create_release_returns_presigned_urls_per_target(
        self, client: AsyncClient, session_factory
    ):
        token = await _register_and_login(client, "r_creator1")
        await _grant_admin(session_factory, "r_creator1")

        resp = await _create_release(client, token)
        assert resp.status_code == 201, resp.text
        data = resp.json()

        assert "release_id" in data
        assert data["version"] == "0.2.0-alpha.3"
        assert data["channel"] == "alpha"

        artifacts = data["artifacts"]
        assert len(artifacts) == 2
        targets = {a["target"] for a in artifacts}
        assert targets == {"windows-x86_64", "linux-x86_64"}

        for art in artifacts:
            assert "upload_url" in art
            assert "X-Amz-Signature" in art["upload_url"]
            assert art["spaces_key"].startswith(f"releases/{data['release_id']}/")
            assert art["expires_in"] == 900
            assert art["filename"]
            # Filename matches target extension convention
            if art["target"] == "windows-x86_64":
                assert art["filename"].endswith("x86_64-setup.exe")
            else:
                assert art["filename"].endswith("x86_64.AppImage")

        # DB rows exist
        async with session_factory() as db:
            release = (
                await db.execute(
                    select(AppRelease).where(AppRelease.id == data["release_id"])
                )
            ).scalar_one()
            assert release.state == "pending"
            assert release.published is False
            assert release.version == "0.2.0-alpha.3"
            assert release.channel == "alpha"
            assert release.engine_commit == "abc1234"
            assert release.min_version == "0.1.0"

            artifact_rows = (
                await db.execute(
                    select(AppReleaseArtifact).where(
                        AppReleaseArtifact.release_id == data["release_id"]
                    )
                )
            ).scalars().all()
            assert len(artifact_rows) == 2
            db_targets = {a.target for a in artifact_rows}
            assert db_targets == {"windows-x86_64", "linux-x86_64"}

    async def test_create_release_rejects_non_admin(
        self, client: AsyncClient, session_factory
    ):
        plain_token = await _register_and_login(client, "r_plain_403")

        resp = await _create_release(client, plain_token)
        assert resp.status_code == 403
