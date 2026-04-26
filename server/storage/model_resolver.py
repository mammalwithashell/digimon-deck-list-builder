"""Resolve DO-Spaces-hosted ONNX manifest entries to local on-disk paths.

Separated from `digimon_gym/engine/model_utils.py` so that the engine
package has no module-level dependency on SQLAlchemy or DigitalOcean
Spaces. Callers on the DB/HTTP side (e.g. `admin_models.py::prepare_model`)
use this module; engine-only callers (e.g. `games.py`) stick with
`engine.model_utils`'s filename-only resolver.
"""
from __future__ import annotations

import asyncio
import os
from pathlib import Path

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from digimon_gym.db.models import AIModel
from digimon_gym.storage import spaces


def _manifest_cache_dir() -> Path:
    """Server-side cache for manifest-fetched ONNX blobs, keyed by sha256."""
    return Path(os.environ.get("MANIFEST_MODEL_CACHE_DIR", "/tmp/digimon-models"))


async def resolve_manifest_model_path(
    db: AsyncSession, manifest_id: str
) -> tuple[str, bool]:
    """Resolve an AIModel row id to a local `.onnx` path.

    Returns `(path, downloaded)` where `downloaded` is `True` if this call
    actually fetched the blob from Spaces, `False` if it hit the cache.

    First call for a given sha256 streams the blob out of Spaces into a
    temp file in the cache directory, verifies the returned sha matches
    the DB record, and atomically renames to the final cache path. Every
    subsequent call for the same sha256 returns the cached path without
    hitting Spaces.
    """
    row = (await db.execute(
        select(AIModel).where(AIModel.id == manifest_id)
    )).scalar_one_or_none()
    if row is None or not row.published or row.state != "uploaded":
        raise FileNotFoundError(
            f"manifest model not found or not published: {manifest_id}"
        )

    cache_dir = _manifest_cache_dir()
    cache_dir.mkdir(parents=True, exist_ok=True)
    cache_path = cache_dir / f"{row.file_sha256}.onnx"
    if cache_path.exists():
        return (str(cache_path), False)

    # Download to a tempfile alongside the final path, verify sha, then
    # atomically rename. `os.replace` is atomic on POSIX and on Windows
    # when source and dest share a volume (they do — same directory).
    tmp_path = cache_path.with_suffix(
        f".onnx.partial.{os.getpid()}.{id(cache_path)}"
    )
    actual_sha, _size = await asyncio.to_thread(
        spaces.download_and_hash, row.spaces_key, str(tmp_path)
    )
    if actual_sha != row.file_sha256:
        try:
            tmp_path.unlink()
        except FileNotFoundError:
            pass
        raise FileNotFoundError(
            f"sha mismatch for manifest model {manifest_id}: "
            f"expected {row.file_sha256}, got {actual_sha}"
        )
    os.replace(tmp_path, cache_path)
    return (str(cache_path), True)
