"""Shared fixtures for tests under `tests/api/`.

Rate limiting is disabled here because per-IP limits collide with the
in-process AsyncClient (all test requests share the same loopback address)
and would cause cascading 429s between unrelated test cases.
"""

from __future__ import annotations

import pytest

from digimon_gym.limiter import limiter


@pytest.fixture(autouse=True)
def _disable_rate_limiting():
    prev = limiter.enabled
    limiter.enabled = False
    try:
        yield
    finally:
        limiter.enabled = prev
