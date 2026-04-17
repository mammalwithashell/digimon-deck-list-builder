"""Filesystem-backed `ModelStorage` for dev + single-host alpha deploys."""

from __future__ import annotations

import shutil
from pathlib import Path
from typing import Optional

import anyio


class LocalModelStorage:
    """Stores blobs under `{root}/{key}`. Never returns a public URL —
    the API's `GET /models/{slug}/{version}/blob` route streams bytes via
    `FileResponse`, which handles Range requests (resumable downloads)
    out of the box.
    """

    def __init__(self, root: str | Path) -> None:
        self.root = Path(root).resolve()
        self.root.mkdir(parents=True, exist_ok=True)

    def _path(self, key: str) -> Path:
        # Keys are URL-safe slugs joined by `/` (e.g. `greedy-meta/1.0.0/model.onnx`).
        # `resolve` guards against `..` escapes: the resolved path must sit
        # under `self.root`.
        candidate = (self.root / key).resolve()
        try:
            candidate.relative_to(self.root)
        except ValueError as exc:
            raise ValueError(f"Refusing path outside storage root: {key!r}") from exc
        return candidate

    async def put(self, key: str, source_path: Path, content_type: str) -> None:
        del content_type  # filesystem doesn't care; mime is set at serve time
        target = self._path(key)
        target.parent.mkdir(parents=True, exist_ok=True)
        await anyio.to_thread.run_sync(shutil.copyfile, str(source_path), str(target))

    async def delete(self, key: str) -> None:
        target = self._path(key)
        if target.exists():
            await anyio.to_thread.run_sync(target.unlink)

    async def exists(self, key: str) -> bool:
        return self._path(key).exists()

    def public_url(self, key: str) -> Optional[str]:
        return None

    def local_path(self, key: str) -> Optional[Path]:
        path = self._path(key)
        return path if path.exists() else None
