"""Shared ONNX model path resolution utilities.

Used by both the main server (games.py) and the desktop sidecar (desktop_main.py)
to keep filename sanitization and error messaging consistent.
"""

from __future__ import annotations

import os
from pathlib import Path
from typing import Optional


def get_models_dir() -> Path:
    """Return the configured ONNX models directory."""
    return Path(os.environ.get("ONNX_MODELS_DIR", "models"))


def get_models_cache_dir() -> Optional[Path]:
    """Return the per-user cache dir for models downloaded from the
    hosted `/models` catalog, or `None` if the sidecar was launched
    without `--models-cache-dir`."""
    raw = os.environ.get("ONNX_MODELS_CACHE_DIR", "").strip()
    return Path(raw) if raw else None


def list_onnx_models(models_dir: Path | None = None) -> list[str]:
    """List available .onnx model files in the models directory."""
    md = models_dir or get_models_dir()
    if not md.exists():
        return []
    return sorted(f.name for f in md.glob("*.onnx"))


def resolve_model_path(
    model_name: str | None, models_dir: Path | None = None
) -> Optional[str]:
    """Resolve a model reference to a full `.onnx` path.

    Supports two shapes:
    - Legacy filename ("mlp_agent.onnx"): looked up in `models_dir`
      (installer-bundled).
    - Catalog slug ("greedy-meta"): looked up in the cache dir
      populated by the `/models/download` sidecar route; newest cached
      version wins.

    Returns None if `model_name` is None/empty. Raises FileNotFoundError
    if the reference is non-empty but can't be resolved in either lane.
    """
    if not model_name:
        return None

    # Defer to the catalog resolver (handles both cached-slug and
    # bundled-filename paths with path-traversal protection).
    from digimon_gym.engine.model_catalog import resolve_model

    md = models_dir or get_models_dir()
    cache = get_models_cache_dir()
    resolved = resolve_model(model_name, bundled_dir=md, cache_dir=cache)
    if resolved is not None:
        return str(resolved)

    available = list_onnx_models(md)
    raise FileNotFoundError(
        f"ONNX model not found: {model_name}. "
        f"Available bundled models: {available}"
    )
