from __future__ import annotations

from typing import Any

MAX_U64 = (1 << 64) - 1


def parse_optional_seed(value: Any) -> int | None:
    """Normalize a frontend-facing seed value to an engine u64.

    Browser and Tauri-facing clients represent seeds as decimal strings so
    JavaScript never rounds large u64 values. Safe integer callers are still
    accepted for older debug tooling.
    """
    if value is None:
        return None
    if isinstance(value, str):
        stripped = value.strip()
        if not stripped:
            return None
        if not stripped.isdecimal():
            raise ValueError("seed must be a base-10 integer in the u64 range")
        parsed = int(stripped, 10)
    elif isinstance(value, int) and not isinstance(value, bool):
        parsed = value
    else:
        raise ValueError("seed must be a base-10 integer in the u64 range")

    if parsed < 0 or parsed > MAX_U64:
        raise ValueError("seed must be in the u64 range")
    return parsed


def seed_to_wire(seed: int) -> str:
    return str(seed)
