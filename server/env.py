"""Project-level environment loading helpers."""

from __future__ import annotations

from pathlib import Path

_ENV_LOADED = False


def load_project_env() -> bool:
    """Load project `.env` once if python-dotenv is available.

    Returns True when dotenv loading was attempted (file may or may not exist),
    False when python-dotenv is unavailable.
    """
    global _ENV_LOADED
    if _ENV_LOADED:
        return True
    _ENV_LOADED = True

    try:
        from dotenv import load_dotenv
    except Exception:
        return False

    project_root = Path(__file__).resolve().parents[1]
    env_path = project_root / ".env"
    # Do not override environment variables already set in the process.
    load_dotenv(env_path, override=False)
    return True
