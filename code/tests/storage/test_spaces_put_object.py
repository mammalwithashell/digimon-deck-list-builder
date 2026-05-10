"""Moto-backed tests for put_object / put_json in server.storage.spaces."""
from __future__ import annotations

import json

import boto3
import pytest
from botocore.client import Config as BotocoreConfig

from server.storage import spaces
from tests.moto_compat import mock_aws

# ---------------------------------------------------------------------------
# Constants shared by all tests (mirror test_spaces.py conventions)
# ---------------------------------------------------------------------------

_BUCKET = "test-digimon-bucket"
_REGION = "us-east-1"
# moto patches the HTTP layer for *.amazonaws.com URLs but not for
# arbitrary endpoints like digitaloceanspaces.com.
_ENDPOINT = "https://s3.us-east-1.amazonaws.com"
_KEY_ID = "testing"
_SECRET = "testing"


@pytest.fixture(autouse=True)
def _patch_env(monkeypatch):
    """Set all SPACES_* env vars and clear the lru_cache before each test."""
    monkeypatch.setenv("SPACES_ENDPOINT", _ENDPOINT)
    monkeypatch.setenv("SPACES_BUCKET", _BUCKET)
    monkeypatch.setenv("SPACES_REGION", _REGION)
    monkeypatch.setenv("SPACES_KEY", _KEY_ID)
    monkeypatch.setenv("SPACES_SECRET", _SECRET)
    monkeypatch.setenv("AWS_ACCESS_KEY_ID", _KEY_ID)
    monkeypatch.setenv("AWS_SECRET_ACCESS_KEY", _SECRET)
    monkeypatch.setenv("AWS_DEFAULT_REGION", _REGION)
    spaces._client.cache_clear()
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


def _make_bucket(s3_client):
    s3_client.create_bucket(Bucket=_BUCKET)


@mock_aws
def test_put_object_stores_bytes_with_headers():
    raw = _raw_client()
    _make_bucket(raw)

    spaces.put_object(
        key="updates/alpha/latest.json",
        body=b'{"hello": "world"}',
        content_type="application/json",
        cache_control="public, max-age=60",
        acl="public-read",
    )
    head = spaces.head_object("updates/alpha/latest.json")
    assert head["ContentLength"] == 18
    assert head["ContentType"] == "application/json"
    assert head["CacheControl"] == "public, max-age=60"


@mock_aws
def test_put_json_serializes_dict():
    raw = _raw_client()
    _make_bucket(raw)

    spaces.put_json(
        key="updates/alpha/latest.json",
        data={"version": "0.2.0"},
        cache_max_age=60,
    )
    body = b"".join(spaces.iter_object_chunks("updates/alpha/latest.json"))
    assert json.loads(body) == {"version": "0.2.0"}
