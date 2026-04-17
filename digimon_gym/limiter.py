"""Shared slowapi rate limiter.

Import `limiter` anywhere an endpoint needs a `@limiter.limit(...)` decorator.
The instance is wired into the FastAPI app in `digimon_gym.api`.
"""

from __future__ import annotations

from slowapi import Limiter
from slowapi.util import get_remote_address

from digimon_gym.config import settings


def _storage_uri() -> str:
    if settings.rate_limit_storage_uri:
        return settings.rate_limit_storage_uri
    return "memory://"


limiter = Limiter(
    key_func=get_remote_address,
    storage_uri=_storage_uri(),
    default_limits=[],
    enabled=settings.rate_limits_enabled,
)
