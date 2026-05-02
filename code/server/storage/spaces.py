"""Thin boto3 client for DigitalOcean Spaces (S3-compatible).

Keyed off env vars: SPACES_ENDPOINT, SPACES_BUCKET, SPACES_REGION,
SPACES_KEY, SPACES_SECRET. Raises RuntimeError if any are unset at call
time (fail loud, not on import) so consumers can import the module even
when credentials aren't configured.
"""
from __future__ import annotations

import hashlib
import json
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


def put_object(
    key: str,
    body: bytes,
    content_type: str = "application/octet-stream",
    cache_control: str | None = None,
    acl: str | None = None,
) -> None:
    """Upload bytes to a Spaces object. Used for manifest JSON rewrites.

    Unlike presigned PUT, this path runs server-side and sets object-level
    metadata (Content-Type, Cache-Control, ACL) in one call.
    """
    extra: dict = {
        "Bucket": _bucket(),
        "Key": key,
        "Body": body,
        "ContentType": content_type,
    }
    if cache_control is not None:
        extra["CacheControl"] = cache_control
    if acl is not None:
        extra["ACL"] = acl
    _client().put_object(**extra)


def put_json(key: str, data: dict, cache_max_age: int = 60) -> None:
    """Serialize ``data`` as JSON (sorted keys, UTF-8) and upload with a
    sensible Cache-Control header. Public-read ACL is applied so Tauri's
    updater can fetch anonymously.
    """
    body = json.dumps(data, sort_keys=True, ensure_ascii=False).encode("utf-8")
    put_object(
        key=key,
        body=body,
        content_type="application/json",
        cache_control=f"public, max-age={cache_max_age}",
        acl="public-read",
    )


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


def download_and_hash(key: str, dest_path: str) -> tuple[str, int]:
    """Stream the object once, writing to dest_path while computing sha256.

    Returns (sha256_hex, total_bytes). dest_path is opened for binary write
    and closed before return.
    """
    h = hashlib.sha256()
    total = 0
    with open(dest_path, "wb") as f:
        for chunk in iter_object_chunks(key):
            h.update(chunk)
            f.write(chunk)
            total += len(chunk)
    return h.hexdigest(), total


def public_url(key: str) -> str:
    """Public URL for a Spaces object.

    Prefers ``SPACES_CDN_URL`` (DigitalOcean Spaces CDN base, e.g.
    ``https://digimon-tcg-models.nyc3.cdn.digitaloceanspaces.com``) when set,
    producing ``{SPACES_CDN_URL}/{key}``. Otherwise falls back to the
    path-style origin form ``{SPACES_ENDPOINT}/{bucket}/{key}``.
    """
    cdn = os.environ.get("SPACES_CDN_URL", "").strip().rstrip("/")
    if cdn:
        return f"{cdn}/{key}"
    endpoint = _require_env("SPACES_ENDPOINT").rstrip("/")
    return f"{endpoint}/{_bucket()}/{key}"
