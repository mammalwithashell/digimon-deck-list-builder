"""Fingerprint loaded native bindings, including packaged extension modules."""

from __future__ import annotations
import hashlib
import importlib.machinery
import sys
from pathlib import Path


def native_binding_fingerprints() -> dict[str, str]:
    # Maturin normally exposes a Python __init__.py shim. Hashing only that
    # shim cannot detect a rebuilt/stale PyO3 wheel with the same package code.
    import digimon_engine  # noqa: F401

    files = {
        name: Path(module.__file__)
        for name, module in list(sys.modules.items())
        if (name == "digimon_engine" or name.startswith("digimon_engine."))
        and getattr(module, "__file__", None)
        and any(
            str(module.__file__).endswith(suffix)
            for suffix in importlib.machinery.EXTENSION_SUFFIXES
        )
    }
    if not files:
        raise RuntimeError("no loaded native digimon_engine binding to fingerprint")
    return {
        name: hashlib.sha256(path.read_bytes()).hexdigest()
        for name, path in sorted(files.items())
    }
