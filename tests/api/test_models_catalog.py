"""Public `/models` catalog + admin upload round-trip.

Uses `LocalModelStorage` pointed at a tmp dir. ONNX shape extraction is
exercised only when the `onnx` package is installed; these tests still
run (and pass) when it's missing.
"""

import hashlib
import json
from pathlib import Path

import pytest
from httpx import ASGITransport, AsyncClient
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from digimon_gym.config import settings
from digimon_gym.db.auth import ROLE_ADMIN, assign_role_to_user, hash_password
from digimon_gym.db.database import get_db
from digimon_gym.db.models import Base, User
from digimon_gym.models_store import get_model_storage


@pytest.fixture(autouse=True)
def _local_storage(tmp_path, monkeypatch):
    # Rebase the model storage at a per-test tmp dir so blobs don't
    # leak between runs and the lru_cache is refreshed.
    monkeypatch.setattr(settings, "model_storage_local_root", str(tmp_path / "models"))
    get_model_storage.cache_clear()
    yield
    get_model_storage.cache_clear()


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


def _fake_onnx(tmp_path: Path, payload: bytes = b"fake-onnx-bytes") -> tuple[Path, str]:
    tmp_path.mkdir(parents=True, exist_ok=True)
    p = tmp_path / "model.onnx"
    p.write_bytes(payload)
    return p, hashlib.sha256(payload).hexdigest()


class TestEmptyCatalog:
    async def test_list_models_empty(self, client):
        resp = await client.get("/models")
        assert resp.status_code == 200
        assert resp.json() == []

    async def test_unknown_slug_404(self, client):
        resp = await client.get("/models/nonexistent")
        assert resp.status_code == 404


class TestAdminCreate:
    async def test_create_requires_admin(self, client):
        resp = await client.post(
            "/admin/models",
            json={"slug": "greedy-meta", "name": "Greedy Meta", "arch": "mlp"},
        )
        # unauthenticated → 401 via require_roles dependency chain
        assert resp.status_code == 401

    async def test_admin_can_create_record(self, client, admin_token):
        resp = await client.post(
            "/admin/models",
            headers={"Authorization": f"Bearer {admin_token}"},
            json={
                "slug": "greedy-meta",
                "name": "Greedy Meta Gauntlet",
                "arch": "mlp",
                "description": "Baseline opponent for alpha",
                "min_engine_version": "0.1.0",
            },
        )
        assert resp.status_code == 201, resp.text
        body = resp.json()
        assert body["slug"] == "greedy-meta"
        assert body["arch"] == "mlp"
        assert body["versions"] == []
        assert body["latest"] is None

    async def test_duplicate_slug_rejected(self, client, admin_token):
        payload = {"slug": "dup", "name": "dup", "arch": "mlp"}
        first = await client.post(
            "/admin/models",
            headers={"Authorization": f"Bearer {admin_token}"},
            json=payload,
        )
        assert first.status_code == 201
        second = await client.post(
            "/admin/models",
            headers={"Authorization": f"Bearer {admin_token}"},
            json=payload,
        )
        assert second.status_code == 409


class TestUploadRoundTrip:
    async def test_upload_and_download(self, client, admin_token, tmp_path):
        await client.post(
            "/admin/models",
            headers={"Authorization": f"Bearer {admin_token}"},
            json={"slug": "g1", "name": "G1", "arch": "mlp"},
        )

        blob_path, expected_sha = _fake_onnx(tmp_path)
        meta = json.dumps({"version": "1.0.0", "changelog": "first upload"})
        with blob_path.open("rb") as fh:
            upload = await client.post(
                "/admin/models/g1/versions",
                headers={"Authorization": f"Bearer {admin_token}"},
                files={"file": ("model.onnx", fh, "application/octet-stream")},
                data={"meta": meta},
            )
        assert upload.status_code == 201, upload.text
        version = upload.json()
        assert version["version"] == "1.0.0"
        assert version["sha256"] == expected_sha
        assert version["size_bytes"] == blob_path.stat().st_size

        # Manifest now includes the uploaded version
        listing = await client.get("/models")
        assert listing.status_code == 200
        rows = listing.json()
        assert len(rows) == 1
        assert rows[0]["slug"] == "g1"
        assert rows[0]["latest"]["sha256"] == expected_sha

        # `url` points back to the local-backend blob endpoint; fetch it
        blob_url = version["url"]
        # Strip the host prefix (AsyncClient base_url) so we can re-hit it.
        blob_path_only = blob_url.split("http://test", 1)[-1]
        dl = await client.get(blob_path_only)
        assert dl.status_code == 200
        assert hashlib.sha256(dl.content).hexdigest() == expected_sha

    async def test_invalid_semver_rejected(self, client, admin_token, tmp_path):
        await client.post(
            "/admin/models",
            headers={"Authorization": f"Bearer {admin_token}"},
            json={"slug": "g2", "name": "G2", "arch": "mlp"},
        )
        blob_path, _ = _fake_onnx(tmp_path)
        meta = json.dumps({"version": "not-a-version"})
        with blob_path.open("rb") as fh:
            resp = await client.post(
                "/admin/models/g2/versions",
                headers={"Authorization": f"Bearer {admin_token}"},
                files={"file": ("model.onnx", fh, "application/octet-stream")},
                data={"meta": meta},
            )
        assert resp.status_code == 400

    async def test_duplicate_version_rejected(self, client, admin_token, tmp_path):
        await client.post(
            "/admin/models",
            headers={"Authorization": f"Bearer {admin_token}"},
            json={"slug": "g3", "name": "G3", "arch": "mlp"},
        )
        for expected_status in (201, 409):
            blob_path, _ = _fake_onnx(tmp_path / f"{expected_status}")
            meta = json.dumps({"version": "1.0.0"})
            with blob_path.open("rb") as fh:
                resp = await client.post(
                    "/admin/models/g3/versions",
                    headers={"Authorization": f"Bearer {admin_token}"},
                    files={"file": ("model.onnx", fh, "application/octet-stream")},
                    data={"meta": meta},
                )
            assert resp.status_code == expected_status

    async def test_deprecate_marks_version_hidden_from_public_latest(
        self, client, admin_token, tmp_path
    ):
        await client.post(
            "/admin/models",
            headers={"Authorization": f"Bearer {admin_token}"},
            json={"slug": "g4", "name": "G4", "arch": "mlp"},
        )
        for ver in ("1.0.0", "1.1.0"):
            blob_path, _ = _fake_onnx(tmp_path / ver)
            meta = json.dumps({"version": ver})
            with blob_path.open("rb") as fh:
                await client.post(
                    "/admin/models/g4/versions",
                    headers={"Authorization": f"Bearer {admin_token}"},
                    files={"file": ("model.onnx", fh, "application/octet-stream")},
                    data={"meta": meta},
                )

        dep = await client.delete(
            "/admin/models/g4/versions/1.1.0",
            headers={"Authorization": f"Bearer {admin_token}"},
        )
        assert dep.status_code == 204

        listing = await client.get("/models/g4")
        assert listing.status_code == 200
        detail = listing.json()
        # `latest` skips deprecated versions; both versions still listed.
        assert detail["latest"]["version"] == "1.0.0"
        assert len(detail["versions"]) == 2
        deprecated = next(v for v in detail["versions"] if v["version"] == "1.1.0")
        assert deprecated["is_deprecated"] is True
