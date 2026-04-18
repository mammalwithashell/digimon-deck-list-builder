"""Thin boto3 client for DigitalOcean Spaces (S3-compatible).

Keyed off env vars: SPACES_ENDPOINT, SPACES_BUCKET, SPACES_REGION,
SPACES_KEY, SPACES_SECRET. Raises RuntimeError if any are unset at call time
(fail loud, not on import, so desktop_main.py can still import the module
transitively via db.routers — though in practice the router never imports it).
"""
from __future__ import annotations

import hashlib
import os
from functools import lru_cache
from typing import Iterator

import boto3
from botocore.client import Config


def _require_env(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise RuntimeError(f"{name} is not set")
    return value


@lru_cache(maxsize=1)
def _client():
    return boto3.client(
        "s3",
        endpoint_url=_require_env("SPACES_ENDPOINT"),
        region_name=_require_env("SPACES_REGION"),
        aws_access_key_id=_require_env("SPACES_KEY"),
        aws_secret_access_key=_require_env("SPACES_SECRET"),
        config=Config(signature_version="s3v4"),
    )


def _bucket() -> str:
    return _require_env("SPACES_BUCKET")


def generate_presigned_put(
    key: str,
    expires_in: int = 900,
    content_type: str = "application/octet-stream",
) -> str:
    return _client().generate_presigned_url(
        "put_object",
        Params={"Bucket": _bucket(), "Key": key, "ContentType": content_type, "ACL": "public-read"},
        ExpiresIn=expires_in,
        HttpMethod="PUT",
    )


def generate_presigned_get(key: str, expires_in: int = 900) -> str:
    return _client().generate_presigned_url(
        "get_object",
        Params={"Bucket": _bucket(), "Key": key},
        ExpiresIn=expires_in,
        HttpMethod="GET",
    )


def head_object(key: str) -> dict:
    """Returns {'ContentLength': int, 'ETag': str, ...} or raises ClientError 404."""
    return _client().head_object(Bucket=_bucket(), Key=key)


def delete_object(key: str) -> None:
    _client().delete_object(Bucket=_bucket(), Key=key)


def iter_object_chunks(key: str, chunk_size: int = 8 * 1024 * 1024) -> Iterator[bytes]:
    """Stream an object body in chunks. Used by confirm endpoint for sha256."""
    body = _client().get_object(Bucket=_bucket(), Key=key)["Body"]
    try:
        while True:
            chunk = body.read(chunk_size)
            if not chunk:
                return
            yield chunk
    finally:
        body.close()


def stream_sha256(key: str) -> tuple[str, int]:
    """Return (sha256_hex, total_bytes) by streaming the object once."""
    h = hashlib.sha256()
    total = 0
    for chunk in iter_object_chunks(key):
        h.update(chunk)
        total += len(chunk)
    return h.hexdigest(), total


def public_url(key: str) -> str:
    """Stable public URL — <bucket>.<region>.digitaloceanspaces.com/<key>.
    The presence of SPACES_ENDPOINT (with scheme+host) gives us the origin.
    """
    endpoint = _require_env("SPACES_ENDPOINT").rstrip("/")
    return f"{endpoint}/{_bucket()}/{key}"
