"""Backend-agnostic contract for ONNX model blob storage."""

from __future__ import annotations

from pathlib import Path
from typing import Optional, Protocol, runtime_checkable


@runtime_checkable
class ModelStorage(Protocol):
    """Minimal interface every model-storage backend must implement.

    `put`/`delete` are the write path (exercised by the admin router).
    `public_url` + `local_path` cover the two serve shapes we support: a
    CDN URL the client fetches directly (Spaces) or a filesystem path the
    API streams via `FileResponse` (local dev). Exactly one returns a
    non-None value for any given key.
    """

    async def put(self, key: str, source_path: Path, content_type: str) -> None:
        """Upload the file at `source_path` under the opaque storage `key`.

        `source_path` is a local temp file; the caller is responsible for
        computing its sha256 before calling and deleting it after.
        """
        ...

    async def delete(self, key: str) -> None:
        """Best-effort delete. Silently no-op if the key is already gone."""
        ...

    async def exists(self, key: str) -> bool:
        ...

    def public_url(self, key: str) -> Optional[str]:
        """Fully-qualified URL clients can GET directly, or `None` if the
        backend serves blobs only through the API (see `local_path`)."""
        ...

    def local_path(self, key: str) -> Optional[Path]:
        """Filesystem path the API should stream with `FileResponse`, or
        `None` if the backend serves blobs via `public_url` instead."""
        ...
