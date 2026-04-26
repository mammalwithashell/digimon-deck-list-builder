"""Moto-backed unit tests for server.storage.spaces."""
from __future__ import annotations

import hashlib
import os

import boto3
import pytest
from botocore.client import Config as BotocoreConfig
from botocore.exceptions import ClientError
from moto import mock_s3

from server.storage import spaces

# ---------------------------------------------------------------------------
# Constants shared by all tests
# ---------------------------------------------------------------------------

_BUCKET = "test-digimon-bucket"
_REGION = "us-east-1"
# Use a standard AWS S3 endpoint that moto intercepts.
# moto patches the HTTP layer for *.amazonaws.com URLs but not for
# arbitrary endpoints like digitaloceanspaces.com.
_ENDPOINT = "https://s3.us-east-1.amazonaws.com"
_KEY_ID = "testing"
_SECRET = "testing"


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture(autouse=True)
def _patch_env(monkeypatch):
    """Set all SPACES_* env vars and clear the lru_cache before each test."""
    monkeypatch.setenv("SPACES_ENDPOINT", _ENDPOINT)
    monkeypatch.setenv("SPACES_BUCKET", _BUCKET)
    monkeypatch.setenv("SPACES_REGION", _REGION)
    monkeypatch.setenv("SPACES_KEY", _KEY_ID)
    monkeypatch.setenv("SPACES_SECRET", _SECRET)
    # Also set standard AWS env vars so moto accepts the credentials
    monkeypatch.setenv("AWS_ACCESS_KEY_ID", _KEY_ID)
    monkeypatch.setenv("AWS_SECRET_ACCESS_KEY", _SECRET)
    monkeypatch.setenv("AWS_DEFAULT_REGION", _REGION)
    # Clear cached client so each test gets a fresh one pointed at moto
    spaces._client.cache_clear()
    yield
    spaces._client.cache_clear()


def _raw_client():
    """Return a boto3 client configured identically to the spaces wrapper.
    Both use the same endpoint so moto sees them as the same server.
    """
    return boto3.client(
        "s3",
        endpoint_url=_ENDPOINT,
        region_name=_REGION,
        aws_access_key_id=_KEY_ID,
        aws_secret_access_key=_SECRET,
        config=BotocoreConfig(signature_version="s3v4"),
    )


def _make_bucket(s3_client):
    """Create the test bucket inside the moto context."""
    s3_client.create_bucket(Bucket=_BUCKET)


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


@mock_s3
def test_generate_presigned_put_returns_signed_url():
    """generate_presigned_put() should return a URL containing X-Amz-Signature."""
    raw = _raw_client()
    _make_bucket(raw)

    url = spaces.generate_presigned_put("models/abc/policy.onnx")
    assert "X-Amz-Signature" in url, f"Expected signed URL, got: {url}"


@mock_s3
def test_head_object_roundtrip():
    """PUT a blob via boto3 directly, then HEAD it via wrapper — ContentLength must match."""
    raw = _raw_client()
    _make_bucket(raw)
    blob = b"hello digimon" * 100
    raw.put_object(Bucket=_BUCKET, Key="models/xyz/policy.onnx", Body=blob)

    meta = spaces.head_object("models/xyz/policy.onnx")
    assert meta["ContentLength"] == len(blob)


@mock_s3
def test_stream_sha256_matches_hashlib():
    """stream_sha256() must produce the same digest as hashlib.sha256 on the same bytes."""
    raw = _raw_client()
    _make_bucket(raw)
    blob = b"digimon test bytes 12345" * 512
    raw.put_object(Bucket=_BUCKET, Key="models/sha/policy.onnx", Body=blob)

    got_hex, got_size = spaces.stream_sha256("models/sha/policy.onnx")
    expected_hex = hashlib.sha256(blob).hexdigest()
    assert got_hex == expected_hex
    assert got_size == len(blob)


@mock_s3
def test_delete_object_removes():
    """PUT an object, delete it via wrapper, then HEAD should raise ClientError 404."""
    raw = _raw_client()
    _make_bucket(raw)
    raw.put_object(Bucket=_BUCKET, Key="models/del/policy.onnx", Body=b"delete me")

    spaces.delete_object("models/del/policy.onnx")

    with pytest.raises(ClientError) as exc_info:
        spaces.head_object("models/del/policy.onnx")
    assert exc_info.value.response["Error"]["Code"] in ("404", "NoSuchKey")


@mock_s3
def test_iter_object_chunks_across_boundary():
    """Reassembled stream must equal the original and span multiple chunks."""
    raw = _raw_client()
    _make_bucket(raw)

    blob = os.urandom(40 * 1024)  # 40 KB
    raw.put_object(Bucket=_BUCKET, Key="models/large/policy.onnx", Body=blob)

    chunks = list(spaces.iter_object_chunks("models/large/policy.onnx", chunk_size=16 * 1024))
    assert b"".join(chunks) == blob
    assert len(chunks) >= 2  # confirms we actually crossed a chunk boundary


@mock_s3
def test_download_and_hash_writes_file_and_matches_hashlib(tmp_path):
    """download_and_hash writes bytes to disk AND returns sha256 matching hashlib."""
    raw = _raw_client()
    _make_bucket(raw)
    blob = b"digimon onnx blob" * 300
    raw.put_object(Bucket=_BUCKET, Key="models/dl/policy.onnx", Body=blob)

    dest = tmp_path / "out.onnx"
    sha_hex, total = spaces.download_and_hash("models/dl/policy.onnx", str(dest))

    assert dest.read_bytes() == blob
    assert total == len(blob)
    assert sha_hex == hashlib.sha256(blob).hexdigest()


@mock_s3
def test_missing_env_var_raises_at_call_time(monkeypatch):
    """If SPACES_ENDPOINT is unset, calling head_object should raise RuntimeError (not ImportError)."""
    monkeypatch.delenv("SPACES_ENDPOINT")
    spaces._client.cache_clear()  # ensure we don't use a cached client

    with pytest.raises(RuntimeError, match="SPACES_ENDPOINT"):
        spaces.head_object("models/any/policy.onnx")


def test_public_url_falls_back_to_endpoint_when_cdn_unset(monkeypatch):
    """Without SPACES_CDN_URL, public_url must return the path-style origin URL."""
    monkeypatch.delenv("SPACES_CDN_URL", raising=False)
    url = spaces.public_url("models/abc/policy.onnx")
    assert url == f"{_ENDPOINT}/{_BUCKET}/models/abc/policy.onnx"


def test_public_url_prefers_cdn_when_set(monkeypatch):
    """With SPACES_CDN_URL set, public_url must return {CDN}/{key} (no bucket path)."""
    monkeypatch.setenv(
        "SPACES_CDN_URL", "https://digimon-tcg-models.nyc3.cdn.digitaloceanspaces.com"
    )
    url = spaces.public_url("models/xyz/policy.onnx")
    assert url == (
        "https://digimon-tcg-models.nyc3.cdn.digitaloceanspaces.com/"
        "models/xyz/policy.onnx"
    )


def test_public_url_strips_trailing_slash_on_cdn(monkeypatch):
    """Trailing slash on SPACES_CDN_URL must not produce a double slash."""
    monkeypatch.setenv("SPACES_CDN_URL", "https://cdn.example.com/")
    url = spaces.public_url("models/abc/policy.onnx")
    assert url == "https://cdn.example.com/models/abc/policy.onnx"


def test_public_url_ignores_empty_cdn(monkeypatch):
    """An empty/whitespace SPACES_CDN_URL must be treated as unset (fall back)."""
    monkeypatch.setenv("SPACES_CDN_URL", "   ")
    url = spaces.public_url("models/abc/policy.onnx")
    assert url == f"{_ENDPOINT}/{_BUCKET}/models/abc/policy.onnx"
