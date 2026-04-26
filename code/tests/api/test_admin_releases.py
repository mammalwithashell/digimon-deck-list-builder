"""Tests for admin release management endpoints (Tauri auto-updater)."""

from __future__ import annotations

import hashlib
import json
import os

import boto3
import pytest
from botocore.client import Config as BotocoreConfig
from httpx import ASGITransport, AsyncClient
from moto import mock_s3
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from server.db.auth import ROLE_ADMIN, assign_role_to_user
from server.db.database import get_db
from server.db.models import AppRelease, AppReleaseArtifact, Base, User
from server.storage import spaces

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
    from server.api import app, ai_task_worker, training_job_worker

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


class TestAdminReleasesConfirm:
    """confirm endpoint tests"""

    async def test_confirm_hashes_and_stores_signature(
        self, client: AsyncClient, session_factory
    ):
        token = await _register_and_login(client, "c_confirmer1")
        await _grant_admin(session_factory, "c_confirmer1")

        create = await _create_release(
            client,
            token,
            version="0.2.0",
            targets=["windows-x86_64"],
        )
        assert create.status_code == 201
        body = create.json()
        rid = body["release_id"]
        win_slot = next(a for a in body["artifacts"] if a["target"] == "windows-x86_64")
        win_key = win_slot["spaces_key"]

        # Simulate CI uploading the installer bytes via moto
        payload = b"fake-installer-bytes"
        _raw_client().put_object(
            Bucket=_BUCKET,
            Key=win_key,
            Body=payload,
        )

        confirm = await client.post(
            f"/admin/releases/{rid}/artifacts/windows-x86_64/confirm",
            headers={"Authorization": f"Bearer {token}"},
            json={"signature_b64": "c29tZS1zaWduYXR1cmU="},
        )
        assert confirm.status_code == 200, confirm.text
        cbody = confirm.json()
        assert cbody["release_id"] == rid
        assert cbody["target"] == "windows-x86_64"
        assert cbody["file_size_bytes"] == len(payload)
        assert cbody["file_sha256"] == hashlib.sha256(payload).hexdigest()

        # DB row updated with hash, size, signature
        async with session_factory() as db:
            art = (
                await db.execute(
                    select(AppReleaseArtifact).where(
                        AppReleaseArtifact.release_id == rid,
                        AppReleaseArtifact.target == "windows-x86_64",
                    )
                )
            ).scalar_one()
            assert art.file_sha256 == hashlib.sha256(payload).hexdigest()
            assert art.file_size_bytes == len(payload)
            assert art.signature_b64 == "c29tZS1zaWduYXR1cmU="

    async def test_confirm_unknown_target_returns_404(
        self, client: AsyncClient, session_factory
    ):
        token = await _register_and_login(client, "c_confirmer2")
        await _grant_admin(session_factory, "c_confirmer2")

        create = await _create_release(
            client,
            token,
            version="0.3.0",
            targets=["windows-x86_64"],
        )
        assert create.status_code == 201
        rid = create.json()["release_id"]

        resp = await client.post(
            f"/admin/releases/{rid}/artifacts/linux-x86_64/confirm",
            headers={"Authorization": f"Bearer {token}"},
            json={"signature_b64": "xx"},
        )
        assert resp.status_code == 404


class TestAdminReleasesPublish:
    """publish endpoint + manifest rewrite + list tests"""

    async def test_publish_writes_manifest_to_spaces(
        self, client: AsyncClient, session_factory
    ):
        token = await _register_and_login(client, "p_publisher1")
        await _grant_admin(session_factory, "p_publisher1")

        # Create + upload + confirm both targets
        create_resp = await _create_release(
            client,
            token,
            version="0.2.0",
            engine_commit="abc1234",
            min_version="0.1.0",
            release_notes="first alpha",
            targets=["windows-x86_64", "linux-x86_64"],
        )
        assert create_resp.status_code == 201, create_resp.text
        create = create_resp.json()
        rid = create["release_id"]

        for art in create["artifacts"]:
            _raw_client().put_object(
                Bucket=os.environ["SPACES_BUCKET"],
                Key=art["spaces_key"],
                Body=b"x" * 1024,
            )
            await client.post(
                f"/admin/releases/{rid}/artifacts/{art['target']}/confirm",
                headers={"Authorization": f"Bearer {token}"},
                json={"signature_b64": f"sig-{art['target']}"},
            )

        pub = await client.post(
            f"/admin/releases/{rid}/publish",
            headers={"Authorization": f"Bearer {token}"},
        )
        assert pub.status_code == 200, pub.text

        manifest_obj = _raw_client().get_object(
            Bucket=os.environ["SPACES_BUCKET"],
            Key="updates/alpha/latest.json",
        )
        manifest = json.loads(manifest_obj["Body"].read())

        assert manifest["version"] == "0.2.0"
        assert manifest["channel"] == "alpha"
        assert manifest["release_id"] == rid
        assert manifest["engine_commit"] == "abc1234"
        assert manifest["min_version"] == "0.1.0"
        assert manifest["notes"] == "first alpha"
        assert set(manifest["platforms"].keys()) == {"windows-x86_64", "linux-x86_64"}
        for target, plat in manifest["platforms"].items():
            assert plat["signature"] == f"sig-{target}"
            assert f"releases/{rid}" in plat["url"]

    async def test_publish_refuses_if_artifact_unconfirmed(
        self, client: AsyncClient, session_factory
    ):
        token = await _register_and_login(client, "p_publisher2")
        await _grant_admin(session_factory, "p_publisher2")
        create_resp = await _create_release(
            client,
            token,
            version="0.2.1",
            engine_commit="x",
            min_version="0.1.0",
            targets=["windows-x86_64"],
        )
        assert create_resp.status_code == 201
        rid = create_resp.json()["release_id"]
        pub = await client.post(
            f"/admin/releases/{rid}/publish",
            headers={"Authorization": f"Bearer {token}"},
        )
        assert pub.status_code == 409

    async def test_publish_unpublishes_previous_on_same_channel(
        self, client: AsyncClient, session_factory
    ):
        token = await _register_and_login(client, "p_publisher3")
        await _grant_admin(session_factory, "p_publisher3")

        async def _cut(version):
            create_resp = await _create_release(
                client,
                token,
                version=version,
                engine_commit="e",
                min_version="0.1.0",
                targets=["windows-x86_64"],
            )
            assert create_resp.status_code == 201, create_resp.text
            create = create_resp.json()
            rid = create["release_id"]
            for art in create["artifacts"]:
                _raw_client().put_object(
                    Bucket=os.environ["SPACES_BUCKET"],
                    Key=art["spaces_key"],
                    Body=b"x",
                )
                await client.post(
                    f"/admin/releases/{rid}/artifacts/{art['target']}/confirm",
                    headers={"Authorization": f"Bearer {token}"},
                    json={"signature_b64": "sig"},
                )
            pub = await client.post(
                f"/admin/releases/{rid}/publish",
                headers={"Authorization": f"Bearer {token}"},
            )
            assert pub.status_code == 200, pub.text
            return rid

        first = await _cut("0.2.0")
        second = await _cut("0.2.1")

        listing = await client.get(
            "/admin/releases?channel=alpha",
            headers={"Authorization": f"Bearer {token}"},
        )
        assert listing.status_code == 200, listing.text
        rows = listing.json()["releases"]
        by_id = {r["id"]: r for r in rows}
        assert by_id[first]["published"] is False
        assert by_id[second]["published"] is True


class TestAdminReleasesLifecycle:

    @pytest.mark.asyncio
    async def test_unpublish_rewrites_manifest_to_previous(
        self, client: AsyncClient, session_factory
    ):
        import json
        token = await _register_and_login(client, "l_lifecycle1")
        await _grant_admin(session_factory, "l_lifecycle1")

        async def _cut(version):
            create = await _create_release(
                client, token,
                version=version, engine_commit="e", min_version="0.1.0",
                targets=["windows-x86_64"],
            )
            body = create.json() if hasattr(create, "json") else create
            rid = body["release_id"]
            for art in body["artifacts"]:
                _raw_client().put_object(
                    Bucket=os.environ["SPACES_BUCKET"],
                    Key=art["spaces_key"], Body=b"x",
                )
                await client.post(
                    f"/admin/releases/{rid}/artifacts/{art['target']}/confirm",
                    headers={"Authorization": f"Bearer {token}"},
                    json={"signature_b64": f"sig-{version}"},
                )
            await client.post(
                f"/admin/releases/{rid}/publish",
                headers={"Authorization": f"Bearer {token}"},
            )
            return rid

        first = await _cut("0.2.0")
        second = await _cut("0.2.1")

        unpub = await client.post(
            f"/admin/releases/{second}/unpublish",
            headers={"Authorization": f"Bearer {token}"},
        )
        assert unpub.status_code == 200
        assert unpub.json()["current_version"] == "0.2.0"

        manifest = json.loads(
            _raw_client().get_object(
                Bucket=os.environ["SPACES_BUCKET"],
                Key="updates/alpha/latest.json",
            )["Body"].read()
        )
        assert manifest["version"] == "0.2.0"

    @pytest.mark.asyncio
    async def test_unpublish_only_release_deletes_manifest(
        self, client: AsyncClient, session_factory
    ):
        import botocore.exceptions
        token = await _register_and_login(client, "l_lifecycle2")
        await _grant_admin(session_factory, "l_lifecycle2")

        create = await _create_release(
            client, token,
            version="0.2.0", engine_commit="e", min_version="0.1.0",
            targets=["windows-x86_64"],
        )
        body = create.json() if hasattr(create, "json") else create
        rid = body["release_id"]
        for art in body["artifacts"]:
            _raw_client().put_object(
                Bucket=os.environ["SPACES_BUCKET"],
                Key=art["spaces_key"], Body=b"x",
            )
            await client.post(
                f"/admin/releases/{rid}/artifacts/{art['target']}/confirm",
                headers={"Authorization": f"Bearer {token}"},
                json={"signature_b64": "s"},
            )
        await client.post(
            f"/admin/releases/{rid}/publish",
            headers={"Authorization": f"Bearer {token}"},
        )

        unpub = await client.post(
            f"/admin/releases/{rid}/unpublish",
            headers={"Authorization": f"Bearer {token}"},
        )
        assert unpub.status_code == 200
        assert unpub.json()["current_version"] is None

        with pytest.raises(botocore.exceptions.ClientError):
            _raw_client().get_object(
                Bucket=os.environ["SPACES_BUCKET"],
                Key="updates/alpha/latest.json",
            )

    @pytest.mark.asyncio
    async def test_patch_release_notes_rewrites_manifest_if_published(
        self, client: AsyncClient, session_factory
    ):
        import json
        token = await _register_and_login(client, "l_lifecycle3")
        await _grant_admin(session_factory, "l_lifecycle3")

        create = await _create_release(
            client, token,
            version="0.2.0", engine_commit="e", min_version="0.1.0",
            release_notes="typo",
            targets=["windows-x86_64"],
        )
        body = create.json() if hasattr(create, "json") else create
        rid = body["release_id"]
        for art in body["artifacts"]:
            _raw_client().put_object(
                Bucket=os.environ["SPACES_BUCKET"],
                Key=art["spaces_key"], Body=b"x",
            )
            await client.post(
                f"/admin/releases/{rid}/artifacts/{art['target']}/confirm",
                headers={"Authorization": f"Bearer {token}"},
                json={"signature_b64": "s"},
            )
        await client.post(
            f"/admin/releases/{rid}/publish",
            headers={"Authorization": f"Bearer {token}"},
        )

        patch = await client.patch(
            f"/admin/releases/{rid}",
            headers={"Authorization": f"Bearer {token}"},
            json={"release_notes": "fixed typo"},
        )
        assert patch.status_code == 200

        manifest = json.loads(
            _raw_client().get_object(
                Bucket=os.environ["SPACES_BUCKET"],
                Key="updates/alpha/latest.json",
            )["Body"].read()
        )
        assert manifest["notes"] == "fixed typo"

    @pytest.mark.asyncio
    async def test_delete_refuses_published(
        self, client: AsyncClient, session_factory
    ):
        token = await _register_and_login(client, "l_lifecycle4")
        await _grant_admin(session_factory, "l_lifecycle4")

        create = await _create_release(
            client, token,
            version="0.2.0", engine_commit="e", min_version="0.1.0",
            targets=["windows-x86_64"],
        )
        body = create.json() if hasattr(create, "json") else create
        rid = body["release_id"]
        for art in body["artifacts"]:
            _raw_client().put_object(
                Bucket=os.environ["SPACES_BUCKET"],
                Key=art["spaces_key"], Body=b"x",
            )
            await client.post(
                f"/admin/releases/{rid}/artifacts/{art['target']}/confirm",
                headers={"Authorization": f"Bearer {token}"},
                json={"signature_b64": "s"},
            )
        await client.post(
            f"/admin/releases/{rid}/publish",
            headers={"Authorization": f"Bearer {token}"},
        )
        del_resp = await client.delete(
            f"/admin/releases/{rid}",
            headers={"Authorization": f"Bearer {token}"},
        )
        assert del_resp.status_code == 409

    @pytest.mark.asyncio
    async def test_regenerate_manifest_matches_current_published(
        self, client: AsyncClient, session_factory
    ):
        token = await _register_and_login(client, "l_lifecycle5")
        await _grant_admin(session_factory, "l_lifecycle5")

        create = await _create_release(
            client, token,
            version="0.2.0", engine_commit="e", min_version="0.1.0",
            targets=["windows-x86_64"],
        )
        body = create.json() if hasattr(create, "json") else create
        rid = body["release_id"]
        for art in body["artifacts"]:
            _raw_client().put_object(
                Bucket=os.environ["SPACES_BUCKET"],
                Key=art["spaces_key"], Body=b"x",
            )
            await client.post(
                f"/admin/releases/{rid}/artifacts/{art['target']}/confirm",
                headers={"Authorization": f"Bearer {token}"},
                json={"signature_b64": "s"},
            )
        await client.post(
            f"/admin/releases/{rid}/publish",
            headers={"Authorization": f"Bearer {token}"},
        )

        _raw_client().delete_object(
            Bucket=os.environ["SPACES_BUCKET"],
            Key="updates/alpha/latest.json",
        )
        regen = await client.post(
            "/admin/releases/regenerate-manifest?channel=alpha",
            headers={"Authorization": f"Bearer {token}"},
        )
        assert regen.status_code == 200
        assert regen.json()["current_version"] == "0.2.0"
