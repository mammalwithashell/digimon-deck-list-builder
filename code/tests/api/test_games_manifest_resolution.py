"""POST /games resolves player{1,2}_model as a manifest UUID by streaming
the ONNX blob out of DO Spaces into a sha256-keyed local cache."""
from __future__ import annotations

import hashlib
import uuid as _uuid
from pathlib import Path
from unittest.mock import patch

import pytest
from httpx import ASGITransport, AsyncClient
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker

from server.db.database import get_db
from server.db.models import AIModel
from server.storage.model_resolver import (
    resolve_manifest_model_path,
    _manifest_cache_dir,
)


@pytest.fixture
async def client(db_engine):
    from server.api import app

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


def _fake_onnx_bytes() -> bytes:
    # Not a real ONNX file; the test uses a mocked `download_and_hash`
    # that returns the sha256 of whatever bytes we configure.
    return b"fake-onnx-model-payload"


async def _guest_auth_headers(client: AsyncClient) -> dict[str, str]:
    resp = await client.post("/auth/guest")
    return {"Authorization": f"Bearer {resp.json()['access_token']}"}


@pytest.mark.asyncio
async def test_resolve_manifest_writes_to_sha_keyed_cache(db_session, tmp_path, monkeypatch) -> None:
    model_id = str(_uuid.uuid4())
    payload = _fake_onnx_bytes()
    expected_sha = hashlib.sha256(payload).hexdigest()
    row = AIModel(
        id=model_id,
        name="test",
        model_type="mlp",
        tensor_size=1375,
        action_space_size=2192,
        spaces_key=f"models/{model_id}/policy.onnx",
        file_sha256=expected_sha,
        file_size_bytes=len(payload),
        state="uploaded",
        published=True,
    )
    db_session.add(row)
    await db_session.commit()

    monkeypatch.setenv("MANIFEST_MODEL_CACHE_DIR", str(tmp_path))

    def fake_download(key: str, dest: str) -> tuple[str, int]:
        Path(dest).write_bytes(payload)
        return expected_sha, len(payload)

    with patch("server.storage.spaces.download_and_hash", side_effect=fake_download):
        path_str, downloaded = await resolve_manifest_model_path(db_session, model_id)

    assert downloaded is True
    cached = Path(path_str)
    assert cached.exists()
    assert cached.parent == _manifest_cache_dir()
    assert cached.name == f"{expected_sha}.onnx"
    assert cached.read_bytes() == payload


@pytest.mark.asyncio
async def test_resolve_manifest_hits_cache_on_second_call(db_session, tmp_path, monkeypatch) -> None:
    model_id = str(_uuid.uuid4())
    payload = _fake_onnx_bytes()
    expected_sha = hashlib.sha256(payload).hexdigest()
    db_session.add(AIModel(
        id=model_id, name="t", model_type="mlp",
        tensor_size=1375, action_space_size=2192,
        spaces_key=f"models/{model_id}/policy.onnx",
        file_sha256=expected_sha, file_size_bytes=len(payload),
        state="uploaded", published=True,
    ))
    await db_session.commit()
    monkeypatch.setenv("MANIFEST_MODEL_CACHE_DIR", str(tmp_path))

    calls = {"n": 0}

    def fake_download(key: str, dest: str) -> tuple[str, int]:
        calls["n"] += 1
        Path(dest).write_bytes(payload)
        return expected_sha, len(payload)

    with patch("server.storage.spaces.download_and_hash", side_effect=fake_download):
        _, dl1 = await resolve_manifest_model_path(db_session, model_id)
        _, dl2 = await resolve_manifest_model_path(db_session, model_id)

    assert calls["n"] == 1, "second call should hit the on-disk cache"
    assert dl1 is True
    assert dl2 is False


@pytest.mark.asyncio
async def test_resolve_manifest_rejects_unknown_id(db_session) -> None:
    with pytest.raises(FileNotFoundError) as excinfo:
        await resolve_manifest_model_path(db_session, "does-not-exist")
    assert "manifest" in str(excinfo.value).lower()


@pytest.mark.asyncio
async def test_prepare_model_returns_filename_and_caches(
    client: AsyncClient, db_session, tmp_path, monkeypatch
) -> None:
    model_id = str(_uuid.uuid4())
    payload = _fake_onnx_bytes()
    sha = hashlib.sha256(payload).hexdigest()
    db_session.add(AIModel(
        id=model_id, name="t", model_type="mlp",
        tensor_size=1375, action_space_size=2192,
        spaces_key=f"models/{model_id}/policy.onnx",
        file_sha256=sha, file_size_bytes=len(payload),
        state="uploaded", published=True,
    ))
    await db_session.commit()
    monkeypatch.setenv("MANIFEST_MODEL_CACHE_DIR", str(tmp_path))
    monkeypatch.setenv("ONNX_MODELS_DIR", str(tmp_path))  # so /games sees the same dir

    def fake_dl(key, dest):
        Path(dest).write_bytes(payload)
        return sha, len(payload)

    with patch("server.storage.spaces.download_and_hash", side_effect=fake_dl):
        headers = await _guest_auth_headers(client)
        resp = await client.post(f"/models/{model_id}/prepare", headers=headers)

    assert resp.status_code == 200, resp.text
    body = resp.json()
    assert body["filename"] == f"{sha}.onnx"
    assert body["downloaded"] is True
    # The file is under the configured models dir so /games can resolve it.
    assert (tmp_path / body["filename"]).exists()


@pytest.mark.asyncio
async def test_prepare_model_reports_cache_hit_on_second_call(
    client: AsyncClient, db_session, tmp_path, monkeypatch
) -> None:
    model_id = str(_uuid.uuid4())
    payload = _fake_onnx_bytes()
    sha = hashlib.sha256(payload).hexdigest()
    db_session.add(AIModel(
        id=model_id, name="t", model_type="mlp",
        tensor_size=1375, action_space_size=2192,
        spaces_key=f"models/{model_id}/policy.onnx",
        file_sha256=sha, file_size_bytes=len(payload),
        state="uploaded", published=True,
    ))
    await db_session.commit()
    monkeypatch.setenv("MANIFEST_MODEL_CACHE_DIR", str(tmp_path))
    monkeypatch.setenv("ONNX_MODELS_DIR", str(tmp_path))

    def fake_dl(key, dest):
        Path(dest).write_bytes(payload)
        return sha, len(payload)

    with patch("server.storage.spaces.download_and_hash", side_effect=fake_dl) as m:
        headers = await _guest_auth_headers(client)
        r1 = await client.post(f"/models/{model_id}/prepare", headers=headers)
        r2 = await client.post(f"/models/{model_id}/prepare", headers=headers)

    assert r1.json()["downloaded"] is True
    assert r2.json()["downloaded"] is False
    assert m.call_count == 1


@pytest.mark.asyncio
async def test_resolve_manifest_rejects_sha_mismatch(db_session, tmp_path, monkeypatch) -> None:
    model_id = str(_uuid.uuid4())
    claimed_sha = "a" * 64  # intentional mismatch
    db_session.add(AIModel(
        id=model_id, name="t", model_type="mlp",
        tensor_size=1375, action_space_size=2192,
        spaces_key=f"models/{model_id}/policy.onnx",
        file_sha256=claimed_sha, file_size_bytes=10,
        state="uploaded", published=True,
    ))
    await db_session.commit()
    monkeypatch.setenv("MANIFEST_MODEL_CACHE_DIR", str(tmp_path))

    def fake_dl(key, dest):
        Path(dest).write_bytes(b"different-bytes")
        return hashlib.sha256(b"different-bytes").hexdigest(), 15

    with patch("server.storage.spaces.download_and_hash", side_effect=fake_dl):
        with pytest.raises(FileNotFoundError) as excinfo:
            await resolve_manifest_model_path(db_session, model_id)

    # No partial file should remain at the final cache path.
    assert not (tmp_path / f"{claimed_sha}.onnx").exists()
    assert "sha" in str(excinfo.value).lower()


@pytest.mark.asyncio
async def test_prepare_model_rejects_unauthenticated(client: AsyncClient) -> None:
    # Any model_id works — auth check fires before resolution.
    resp = await client.post("/models/00000000-0000-0000-0000-000000000000/prepare")
    assert resp.status_code == 401


@pytest.mark.asyncio
async def test_prepare_model_404_on_unknown_id_when_authed(client: AsyncClient) -> None:
    headers = await _guest_auth_headers(client)
    resp = await client.post(
        "/models/00000000-0000-0000-0000-000000000000/prepare",
        headers=headers,
    )
    assert resp.status_code == 404
