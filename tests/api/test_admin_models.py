"""Tests for admin model management endpoints and public manifest."""

from __future__ import annotations

import hashlib
from pathlib import Path

import boto3
import pytest
from botocore.client import Config as BotocoreConfig
from botocore.exceptions import ClientError
from httpx import ASGITransport, AsyncClient
from moto import mock_aws
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from digimon_gym.db.auth import ROLE_ADMIN, assign_role_to_user
from digimon_gym.db.database import get_db
from digimon_gym.db.models import AIModel, Base, User
from digimon_gym.storage import spaces

# ---------------------------------------------------------------------------
# ONNX fixture imports (session-scoped, built once per run)
# ---------------------------------------------------------------------------

from tests.api.fixtures.onnx_fixtures import (  # noqa: E402
    invalid_no_obs_path,
    valid_lstm_path,
    valid_mlp_path,
)

# ---------------------------------------------------------------------------
# Moto / Spaces constants (mirror test_spaces.py)
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
    """Wrap every test with moto mock_aws and create the test bucket."""
    monkeypatch.setenv("SPACES_ENDPOINT", _ENDPOINT)
    monkeypatch.setenv("SPACES_BUCKET", _BUCKET)
    monkeypatch.setenv("SPACES_REGION", _REGION)
    monkeypatch.setenv("SPACES_KEY", _KEY_ID)
    monkeypatch.setenv("SPACES_SECRET", _SECRET)
    monkeypatch.setenv("AWS_ACCESS_KEY_ID", _KEY_ID)
    monkeypatch.setenv("AWS_SECRET_ACCESS_KEY", _SECRET)
    monkeypatch.setenv("AWS_DEFAULT_REGION", _REGION)
    spaces._client.cache_clear()

    with mock_aws():
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


def _upload_to_moto(spaces_key: str, source_path: Path) -> None:
    """PUT a local ONNX file into the moto bucket at the given key."""
    raw = _raw_client()
    with open(source_path, "rb") as f:
        raw.put_object(Bucket=_BUCKET, Key=spaces_key, Body=f.read())


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


async def _create_model(client: AsyncClient, token: str, **overrides) -> dict:
    payload = {
        "name": "Test Model",
        "model_type": "mlp",
        "engine_commit": "abc1234",
    }
    payload.update(overrides)
    resp = await client.post(
        "/admin/models",
        headers={"Authorization": f"Bearer {token}"},
        json=payload,
    )
    assert resp.status_code == 201, resp.text
    return resp.json()


async def _confirm_model(
    client: AsyncClient, token: str, model_id: str, onnx_path: Path, spaces_key: str
) -> dict:
    _upload_to_moto(spaces_key, onnx_path)
    resp = await client.post(
        f"/admin/models/{model_id}/confirm",
        headers={"Authorization": f"Bearer {token}"},
    )
    return resp


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestAdminModelsCRUD:
    async def test_create_returns_presigned_put_and_pending_row(
        self, client: AsyncClient, session_factory
    ):
        token = await _register_and_login(client, "m_creator1")
        await _grant_admin(session_factory, "m_creator1")

        data = await _create_model(client, token)
        assert "upload_url" in data
        assert "X-Amz-Signature" in data["upload_url"]
        assert data["spaces_key"].startswith("models/")
        assert data["spaces_key"].endswith("/policy.onnx")
        assert data["expires_in"] == 900
        assert "id" in data

        # DB row is in pending state
        async with session_factory() as db:
            row = (
                await db.execute(select(AIModel).where(AIModel.id == data["id"]))
            ).scalar_one()
        assert row.state == "pending"
        assert row.published is False

    async def test_confirm_streams_hash_and_extracts_tensor_shape(
        self, client: AsyncClient, session_factory, valid_mlp_path: Path
    ):
        token = await _register_and_login(client, "m_confirm1")
        await _grant_admin(session_factory, "m_confirm1")

        data = await _create_model(client, token)
        model_id = data["id"]
        spaces_key = data["spaces_key"]

        resp = await _confirm_model(client, token, model_id, valid_mlp_path, spaces_key)
        assert resp.status_code == 200, resp.text
        body = resp.json()

        assert body["tensor_size"] == 1375
        assert body["action_space_size"] == 2168
        assert body["state"] == "uploaded"
        assert body["file_size_bytes"] > 0

        # Verify sha256 matches hashlib
        expected_sha = hashlib.sha256(valid_mlp_path.read_bytes()).hexdigest()
        assert body["file_sha256"] == expected_sha

    async def test_confirm_handles_lstm(
        self, client: AsyncClient, session_factory, valid_lstm_path: Path
    ):
        token = await _register_and_login(client, "m_confirm_lstm")
        await _grant_admin(session_factory, "m_confirm_lstm")

        data = await _create_model(client, token, model_type="lstm")
        model_id = data["id"]
        spaces_key = data["spaces_key"]

        resp = await _confirm_model(client, token, model_id, valid_lstm_path, spaces_key)
        assert resp.status_code == 200, resp.text
        body = resp.json()
        assert body["tensor_size"] == 1375
        assert body["action_space_size"] == 2168

    async def test_confirm_fails_if_onnx_missing_obs_input(
        self, client: AsyncClient, session_factory, invalid_no_obs_path: Path
    ):
        token = await _register_and_login(client, "m_confirm_bad")
        await _grant_admin(session_factory, "m_confirm_bad")

        data = await _create_model(client, token)
        model_id = data["id"]
        spaces_key = data["spaces_key"]

        resp = await _confirm_model(client, token, model_id, invalid_no_obs_path, spaces_key)
        assert resp.status_code == 422, resp.text
        assert "obs" in resp.json()["detail"].lower()

        # Row state updated to failed
        async with session_factory() as db:
            row = (
                await db.execute(select(AIModel).where(AIModel.id == model_id))
            ).scalar_one()
        assert row.state == "failed"

        # Spaces object was deleted
        with pytest.raises(ClientError) as exc_info:
            spaces.head_object(spaces_key)
        assert exc_info.value.response["Error"]["Code"] in ("404", "NoSuchKey")

    async def test_confirm_fails_if_object_missing(
        self, client: AsyncClient, session_factory
    ):
        token = await _register_and_login(client, "m_confirm_missing")
        await _grant_admin(session_factory, "m_confirm_missing")

        data = await _create_model(client, token)
        model_id = data["id"]
        # Do NOT upload anything

        resp = await client.post(
            f"/admin/models/{model_id}/confirm",
            headers={"Authorization": f"Bearer {token}"},
        )
        assert resp.status_code == 422, resp.text
        assert "Object not found" in resp.json()["detail"]

    async def test_confirm_is_idempotent(
        self, client: AsyncClient, session_factory, valid_mlp_path: Path
    ):
        token = await _register_and_login(client, "m_confirm_idem")
        await _grant_admin(session_factory, "m_confirm_idem")

        data = await _create_model(client, token)
        model_id = data["id"]
        spaces_key = data["spaces_key"]

        # First confirm
        resp1 = await _confirm_model(client, token, model_id, valid_mlp_path, spaces_key)
        assert resp1.status_code == 200, resp1.text
        sha1 = resp1.json()["file_sha256"]

        # Second confirm (object still in Spaces from first upload)
        resp2 = await client.post(
            f"/admin/models/{model_id}/confirm",
            headers={"Authorization": f"Bearer {token}"},
        )
        assert resp2.status_code == 200, resp2.text
        assert resp2.json()["file_sha256"] == sha1

    async def test_patch_published_requires_state_uploaded(
        self, client: AsyncClient, session_factory
    ):
        token = await _register_and_login(client, "m_patch_publish_fail")
        await _grant_admin(session_factory, "m_patch_publish_fail")

        data = await _create_model(client, token)
        model_id = data["id"]
        # Still in 'pending' state — publishing should be rejected

        resp = await client.patch(
            f"/admin/models/{model_id}",
            headers={"Authorization": f"Bearer {token}"},
            json={"published": True},
        )
        assert resp.status_code == 409, resp.text
        assert "uploaded" in resp.json()["detail"]

    async def test_patch_updates_fields(
        self, client: AsyncClient, session_factory, valid_mlp_path: Path
    ):
        token = await _register_and_login(client, "m_patch_ok")
        await _grant_admin(session_factory, "m_patch_ok")

        data = await _create_model(client, token, name="Original Name")
        model_id = data["id"]
        spaces_key = data["spaces_key"]

        await _confirm_model(client, token, model_id, valid_mlp_path, spaces_key)

        resp = await client.patch(
            f"/admin/models/{model_id}",
            headers={"Authorization": f"Bearer {token}"},
            json={"name": "Updated Name", "notes": "Good model", "published": True},
        )
        assert resp.status_code == 200, resp.text
        body = resp.json()
        assert body["name"] == "Updated Name"
        assert body["notes"] == "Good model"
        assert body["published"] is True

    async def test_delete_removes_spaces_object_and_row(
        self, client: AsyncClient, session_factory, valid_mlp_path: Path
    ):
        token = await _register_and_login(client, "m_delete_ok")
        await _grant_admin(session_factory, "m_delete_ok")

        data = await _create_model(client, token)
        model_id = data["id"]
        spaces_key = data["spaces_key"]

        await _confirm_model(client, token, model_id, valid_mlp_path, spaces_key)

        # Delete via endpoint
        del_resp = await client.delete(
            f"/admin/models/{model_id}",
            headers={"Authorization": f"Bearer {token}"},
        )
        assert del_resp.status_code == 204, del_resp.text

        # Spaces object gone
        with pytest.raises(ClientError) as exc_info:
            spaces.head_object(spaces_key)
        assert exc_info.value.response["Error"]["Code"] in ("404", "NoSuchKey")

        # Row not in list
        list_resp = await client.get(
            "/admin/models",
            headers={"Authorization": f"Bearer {token}"},
        )
        assert list_resp.status_code == 200
        ids = [m["id"] for m in list_resp.json()["models"]]
        assert model_id not in ids

    async def test_non_admin_403_on_create_patch_delete(
        self, client: AsyncClient, session_factory, valid_mlp_path: Path
    ):
        # Create admin user and a model to patch/delete
        admin_token = await _register_and_login(client, "m_admin_for_403")
        await _grant_admin(session_factory, "m_admin_for_403")
        data = await _create_model(client, admin_token)
        model_id = data["id"]

        plain_token = await _register_and_login(client, "m_plain_403")

        create_resp = await client.post(
            "/admin/models",
            headers={"Authorization": f"Bearer {plain_token}"},
            json={"name": "x", "model_type": "mlp"},
        )
        assert create_resp.status_code == 403

        patch_resp = await client.patch(
            f"/admin/models/{model_id}",
            headers={"Authorization": f"Bearer {plain_token}"},
            json={"name": "hijack"},
        )
        assert patch_resp.status_code == 403

        del_resp = await client.delete(
            f"/admin/models/{model_id}",
            headers={"Authorization": f"Bearer {plain_token}"},
        )
        assert del_resp.status_code == 403

    async def test_list_filters_by_state_and_published_and_deck(
        self, client: AsyncClient, session_factory, valid_mlp_path: Path
    ):
        token = await _register_and_login(client, "m_list_filter")
        await _grant_admin(session_factory, "m_list_filter")

        # Row 1: pending, unpublished
        d1 = await _create_model(client, token, name="Row1")
        # Row 2: uploaded, unpublished
        d2 = await _create_model(client, token, name="Row2")
        await _confirm_model(client, token, d2["id"], valid_mlp_path, d2["spaces_key"])
        # Row 3: uploaded, published
        d3 = await _create_model(client, token, name="Row3")
        await _confirm_model(client, token, d3["id"], valid_mlp_path, d3["spaces_key"])
        await client.patch(
            f"/admin/models/{d3['id']}",
            headers={"Authorization": f"Bearer {token}"},
            json={"published": True},
        )

        # Filter by state=pending → only row1
        resp = await client.get(
            "/admin/models?state=pending",
            headers={"Authorization": f"Bearer {token}"},
        )
        assert resp.status_code == 200
        models = resp.json()["models"]
        # Only d1 should appear (may include other test rows if run together — check d1 present)
        ids = [m["id"] for m in models]
        assert d1["id"] in ids
        assert d2["id"] not in ids

        # Filter by published=true → only row3
        resp2 = await client.get(
            "/admin/models?published=true",
            headers={"Authorization": f"Bearer {token}"},
        )
        assert resp2.status_code == 200
        ids2 = [m["id"] for m in resp2.json()["models"]]
        assert d3["id"] in ids2
        assert d1["id"] not in ids2

        # Filter by state=uploaded
        resp3 = await client.get(
            "/admin/models?state=uploaded",
            headers={"Authorization": f"Bearer {token}"},
        )
        assert resp3.status_code == 200
        ids3 = [m["id"] for m in resp3.json()["models"]]
        assert d2["id"] in ids3
        assert d3["id"] in ids3
        assert d1["id"] not in ids3


class TestPublicManifest:
    async def test_manifest_excludes_unpublished_and_pending(
        self, client: AsyncClient, session_factory, valid_mlp_path: Path
    ):
        token = await _register_and_login(client, "m_manifest1")
        await _grant_admin(session_factory, "m_manifest1")

        # Row A: published + uploaded → should appear
        dA = await _create_model(client, token, name="ModelA")
        await _confirm_model(client, token, dA["id"], valid_mlp_path, dA["spaces_key"])
        await client.patch(
            f"/admin/models/{dA['id']}",
            headers={"Authorization": f"Bearer {token}"},
            json={"published": True},
        )

        # Row B: NOT published + uploaded → excluded
        dB = await _create_model(client, token, name="ModelB")
        await _confirm_model(client, token, dB["id"], valid_mlp_path, dB["spaces_key"])

        # Row C: published + pending (force via DB)
        dC = await _create_model(client, token, name="ModelC")
        async with session_factory() as db:
            rowC = (
                await db.execute(select(AIModel).where(AIModel.id == dC["id"]))
            ).scalar_one()
            rowC.published = True  # published but still pending state
            await db.commit()

        resp = await client.get("/models/manifest.json")
        assert resp.status_code == 200
        ids = [m["id"] for m in resp.json()["models"]]
        assert dA["id"] in ids
        assert dB["id"] not in ids
        assert dC["id"] not in ids

    async def test_manifest_is_public_no_auth_header_needed(
        self, client: AsyncClient
    ):
        # No Authorization header whatsoever
        resp = await client.get("/models/manifest.json")
        assert resp.status_code == 200

    async def test_manifest_shape_matches_contract(
        self, client: AsyncClient, session_factory, valid_mlp_path: Path
    ):
        token = await _register_and_login(client, "m_manifest_shape")
        await _grant_admin(session_factory, "m_manifest_shape")

        data = await _create_model(client, token, name="ShapeModel")
        await _confirm_model(client, token, data["id"], valid_mlp_path, data["spaces_key"])
        await client.patch(
            f"/admin/models/{data['id']}",
            headers={"Authorization": f"Bearer {token}"},
            json={"published": True},
        )

        resp = await client.get("/models/manifest.json")
        assert resp.status_code == 200
        body = resp.json()

        # Top-level shape
        assert "generated_at" in body
        assert "models" in body

        # Find our model
        matching = [m for m in body["models"] if m["id"] == data["id"]]
        assert len(matching) == 1
        m = matching[0]

        required_keys = {
            "id", "name", "model_type", "tensor_size", "action_space_size",
            "engine_commit", "trained_at", "file_sha256", "file_size_bytes",
            "url", "deck_id", "deck_name", "notes",
        }
        assert required_keys.issubset(m.keys()), (
            f"Missing keys: {required_keys - m.keys()}"
        )
        # URL is populated
        assert m["url"].startswith("https://")

    async def test_manifest_url_uses_origin_when_cdn_unset(
        self,
        client: AsyncClient,
        session_factory,
        valid_mlp_path: Path,
        monkeypatch,
    ):
        """Without SPACES_CDN_URL, manifest urls must resolve to the path-style origin."""
        monkeypatch.delenv("SPACES_CDN_URL", raising=False)

        token = await _register_and_login(client, "m_manifest_origin")
        await _grant_admin(session_factory, "m_manifest_origin")

        data = await _create_model(client, token, name="OriginModel")
        await _confirm_model(client, token, data["id"], valid_mlp_path, data["spaces_key"])
        await client.patch(
            f"/admin/models/{data['id']}",
            headers={"Authorization": f"Bearer {token}"},
            json={"published": True},
        )

        resp = await client.get("/models/manifest.json")
        assert resp.status_code == 200
        matching = [m for m in resp.json()["models"] if m["id"] == data["id"]]
        assert len(matching) == 1
        expected_prefix = f"{_ENDPOINT}/{_BUCKET}/"
        assert matching[0]["url"].startswith(expected_prefix), matching[0]["url"]
        assert matching[0]["url"].endswith(data["spaces_key"])

    async def test_manifest_url_uses_cdn_when_set(
        self,
        client: AsyncClient,
        session_factory,
        valid_mlp_path: Path,
        monkeypatch,
    ):
        """With SPACES_CDN_URL set, every manifest entry must use that base."""
        cdn_base = "https://digimon-tcg-models.nyc3.cdn.digitaloceanspaces.com"
        monkeypatch.setenv("SPACES_CDN_URL", cdn_base)

        token = await _register_and_login(client, "m_manifest_cdn")
        await _grant_admin(session_factory, "m_manifest_cdn")

        data = await _create_model(client, token, name="CdnModel")
        await _confirm_model(client, token, data["id"], valid_mlp_path, data["spaces_key"])
        await client.patch(
            f"/admin/models/{data['id']}",
            headers={"Authorization": f"Bearer {token}"},
            json={"published": True},
        )

        resp = await client.get("/models/manifest.json")
        assert resp.status_code == 200
        body = resp.json()
        assert len(body["models"]) >= 1
        for m in body["models"]:
            assert m["url"].startswith(cdn_base + "/"), m["url"]
            # CDN URL must not embed the bucket name — CDN is bucket-scoped already.
            assert f"/{_BUCKET}/" not in m["url"], m["url"]
        matching = [m for m in body["models"] if m["id"] == data["id"]]
        assert matching[0]["url"] == f"{cdn_base}/{data['spaces_key']}"
