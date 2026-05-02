"""Shared slowapi rate limiter."""

from __future__ import annotations

from slowapi import Limiter
from slowapi.util import get_remote_address

from server.config import settings


limiter = Limiter(
    key_func=get_remote_address,
    storage_uri=settings.rate_limit_storage_uri or "memory://",
    default_limits=[],
    enabled=settings.rate_limits_enabled,
)
