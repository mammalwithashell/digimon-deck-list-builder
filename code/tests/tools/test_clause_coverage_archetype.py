"""Archetype -> card pool and competitive core.

The core threshold is a FRACTION of the archetype's recorded lists, not a raw
count: a raw 33 would silently redefine the core for an archetype with a
different corpus size.
"""

import json
from pathlib import Path

import pytest

from tools.clause_coverage.archetype import (
    DEFAULT_CORE_FRACTION,
    card_frequency,
    core,
    load_archetypes,
    pool,
    resolve,
)

LIBRARY = Path("data/deck_library.json")


def _fixture_entry(lists: list[list[str]]) -> dict:
    """Build an archetype entry the way deck_library.json really stores one:
    `decklist` is a JSON-encoded STRING, not a list."""
    return {
        "archetype_name": "Fixture",
        "decklists": [{"deck_id": str(i), "decklist": json.dumps(cards)}
                      for i, cards in enumerate(lists)],
    }


def test_decklist_is_parsed_from_its_json_string():
    entry = _fixture_entry([["A-001", "A-001", "B-002"], ["A-001"]])
    freq = card_frequency(entry)
    assert freq["A-001"] == 2, "counted per LIST, not per copy"
    assert freq["B-002"] == 1


def test_pool_is_distinct_and_sorted():
    entry = _fixture_entry([["B-002", "A-001", "A-001"]])
    assert pool(entry) == ["A-001", "B-002"]


def test_core_threshold_is_a_fraction_of_the_list_count():
    # 10 lists, 0.7 -> a card must appear in >= 7 of them.
    entry = _fixture_entry([["A-001"]] * 7 + [["B-002"]] * 3)
    c = core(entry, 0.7)
    assert c["list_count"] == 10
    assert c["threshold"] == 7
    assert c["cards"] == ["A-001"]


def test_core_reports_the_threshold_it_used():
    """A report must be able to print '>=N of M lists' without recomputing it."""
    entry = _fixture_entry([["A-001"]] * 4)
    c = core(entry, DEFAULT_CORE_FRACTION)
    assert set(c) == {"cards", "threshold", "list_count", "fraction"}


def test_resolve_is_case_insensitive_and_suggests_near_misses():
    lib = {"Toho Braves": {}, "Hunters": {}}
    assert resolve(lib, "toho braves") == "Toho Braves"
    with pytest.raises(LookupError) as e:
        resolve(lib, "Toho Brave")
    assert "Toho Braves" in str(e.value), "an unknown name must suggest, not just fail"


def test_real_library_reproduces_the_published_toho_figures():
    """Guards the 0.7 default against the published report: 42-card pool,
    18-card core, 45 lists. If deck_library.json is re-scraped and these move,
    this fails loudly rather than letting a report quote stale figures.

    NOTE: the threshold is ceil(list_count * fraction), not a hardcoded
    literal -- ceil(45 * 0.7) = ceil(31.5) = 32, not 31. See
    code/tools/clause_coverage/archetype.py's core() docstring.
    """
    lib = load_archetypes(LIBRARY)
    entry = lib[resolve(lib, "Toho Braves")]
    assert len(entry["decklists"]) == 45
    assert len(pool(entry)) == 42
    c = core(entry, DEFAULT_CORE_FRACTION)
    assert len(c["cards"]) == 18, f"expected the published 18-card core, got {len(c['cards'])}"
    assert c["threshold"] == 32
