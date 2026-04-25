"""Surface tests for `digimon-engine-py` PyO3 bindings.

Each export added in Phase 2 of the server split gets a smoke test here
before any caller is migrated. The bindings module is `digimon_engine`
(crate `digimon-engine-py`, lib name `digimon_engine`).
"""

from __future__ import annotations

import pytest


def test_module_imports():
    import digimon_engine  # noqa: F401


def test_rust_headless_game_still_exported():
    """Phase 2 must not regress the existing RustHeadlessGame surface."""
    from digimon_engine import RustHeadlessGame  # noqa: F401
