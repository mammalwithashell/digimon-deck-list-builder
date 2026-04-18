"""Snapshot parity test for the tested-cards allowlist.

Ensures ``digimon_gym/engine/data/tested_cards.json`` is in sync with the
per-card behavioral test files under ``tests/behavioral/``. If a new test
file is added without refreshing the snapshot this test fails and tells
the developer to run ``python tools/build_tested_cards.py``.
"""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
OUTPUT_JSON = REPO_ROOT / "digimon_gym" / "engine" / "data" / "tested_cards.json"
BUILD_TOOL = REPO_ROOT / "tools" / "build_tested_cards.py"


def _load_build_tool():
    spec = importlib.util.spec_from_file_location("build_tested_cards", BUILD_TOOL)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_snapshot_exists():
    assert OUTPUT_JSON.exists(), (
        f"{OUTPUT_JSON} is missing. Run `python tools/build_tested_cards.py`."
    )


def test_snapshot_is_fresh():
    """Regenerate in-memory and diff against the committed file."""
    build_tool = _load_build_tool()
    card_ids, warnings = build_tool.collect_tested_card_ids()

    assert not warnings, (
        "Unresolved behavioral test files:\n  " + "\n  ".join(warnings)
    )
    assert card_ids, "No tested cards found"

    existing = json.loads(OUTPUT_JSON.read_text())
    assert existing["card_ids"] == card_ids, (
        "tested_cards.json is stale. Run "
        "`python tools/build_tested_cards.py` to refresh."
    )
    assert existing["card_count"] == len(card_ids)


def test_loader_matches_snapshot():
    """The engine loader reads the same data the snapshot contains."""
    from digimon_gym.engine.data.tested_cards import load_tested_cards

    load_tested_cards.cache_clear()  # fresh read
    loaded = load_tested_cards()
    existing = json.loads(OUTPUT_JSON.read_text())
    assert loaded == frozenset(existing["card_ids"])


def test_out_of_set_cards_dedups_and_preserves_order():
    from digimon_gym.engine.data.tested_cards import (
        load_tested_cards,
        out_of_set_cards,
    )

    # Use synthetic IDs that cannot collide with real card IDs so the test
    # stays stable as behavioral coverage grows.
    tested = load_tested_cards()
    synthetic = ["ZZZ-001", "ZZZ-001", "ZZZ-002"]
    assert not any(c in tested for c in synthetic)

    result = out_of_set_cards(synthetic)
    assert result == ["ZZZ-001", "ZZZ-002"]


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
