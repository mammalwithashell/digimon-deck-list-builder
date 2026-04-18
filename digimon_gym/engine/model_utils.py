"""Shared ONNX model path resolution utilities used by the hosted API
(`games.py`) for filename sanitization and consistent error messaging.
"""

from __future__ import annotations

import asyncio
import os
import re
from pathlib import Path
from typing import Optional

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession


def get_models_dir() -> Path:
    """Return the configured ONNX models directory."""
    return Path(os.environ.get("ONNX_MODELS_DIR", "models"))


def list_onnx_models(models_dir: Path | None = None) -> list[str]:
    """List available .onnx model files in the models directory."""
    md = models_dir or get_models_dir()
    if not md.exists():
        return []
    return sorted(f.name for f in md.glob("*.onnx"))


def resolve_model_path(
    model_name: str | None, models_dir: Path | None = None
) -> Optional[str]:
    """Resolve an ONNX model filename to a full path with path-traversal protection.

    Returns None if model_name is None/empty.
    Raises FileNotFoundError if the model file doesn't exist.
    """
    if not model_name:
        return None
    md = models_dir or get_models_dir()
    safe_name = Path(model_name).name
    model_path = md / safe_name
    if not model_path.exists():
        available = list_onnx_models(md)
        raise FileNotFoundError(
            f"ONNX model not found: {safe_name}. "
            f"Available models: {available}"
        )
    return str(model_path)


_UUID_RE = re.compile(
    r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$", re.I
)


def looks_like_manifest_id(s: str) -> bool:
    """True if `s` parses as a UUID (the shape of AIModel.id)."""
    return bool(_UUID_RE.match(s))


def _manifest_cache_dir() -> Path:
    """Server-side cache for manifest-fetched ONNX blobs, keyed by sha256."""
    return Path(os.environ.get("MANIFEST_MODEL_CACHE_DIR", "/tmp/digimon-models"))


async def resolve_manifest_model_path(db: AsyncSession, manifest_id: str) -> str:
    """Resolve an AIModel row id to a local `.onnx` path.

    First call for a given sha256 streams the blob out of DO Spaces. Every
    subsequent call (same sha256) returns the cached path without hitting
    Spaces.
    """
    from digimon_gym.db.models import AIModel  # lazy: engine shouldn't hard-dep on db
    from digimon_gym.storage import spaces

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
        return str(cache_path)

    # Cold: stream out of Spaces.
    await asyncio.to_thread(
        spaces.download_and_hash, row.spaces_key, str(cache_path)
    )
    return str(cache_path)
