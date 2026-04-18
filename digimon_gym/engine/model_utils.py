"""Shared ONNX model path resolution utilities used by the hosted API
(`games.py`) for filename sanitization and consistent error messaging.
"""

from __future__ import annotations

import os
import re
from pathlib import Path
from typing import Optional


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
