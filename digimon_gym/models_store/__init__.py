"""Object storage for ONNX model blobs.

Callers depend on the `ModelStorage` Protocol in `storage.py` and get a
concrete backend through `get_model_storage()`. The backend is chosen by
`settings.model_storage_backend`:

- `local` — `LocalModelStorage` writes to a filesystem root and relies on
  the API's `GET /models/{slug}/{version}/blob` route to stream bytes.
- `spaces` — `SpacesModelStorage` writes to a DigitalOcean Spaces bucket
  and returns a CDN URL clients can download directly.

`get_model_storage()` is cached so handlers and background tasks share a
single instance (the Spaces variant holds an S3 client session).
"""

from __future__ import annotations

from functools import lru_cache

from digimon_gym.config import settings

from .local import LocalModelStorage
from .storage import ModelStorage


@lru_cache(maxsize=1)
def get_model_storage() -> ModelStorage:
    backend = settings.model_storage_backend
    if backend == "local":
        return LocalModelStorage(root=settings.model_storage_local_root)
    if backend == "spaces":
        # Lazy-imported so local dev doesn't require aioboto3.
        from .spaces import SpacesModelStorage

        return SpacesModelStorage(
            endpoint_url=settings.spaces_endpoint_url or "",
            region=settings.spaces_region or "",
            bucket=settings.spaces_bucket or "",
            access_key=settings.spaces_access_key or "",
            secret_key=settings.spaces_secret_key or "",
            cdn_base_url=settings.spaces_cdn_base_url,
        )
    raise ValueError(f"Unknown MODEL_STORAGE_BACKEND: {backend!r}")


__all__ = ["ModelStorage", "get_model_storage"]
