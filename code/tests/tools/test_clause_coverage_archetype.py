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

# The 45 Toho Braves lists the published report was computed from, frozen. The
# live data/deck_library.json is refreshed from DCG Nexus and moves under it.
PUBLISHED_TOHO_LISTS = Path(__file__).parent / "fixtures" / "toho_braves_lists_2026-08.json"

# The report's core: every card its per-card table shows in >=33 of 45 lists.
PUBLISHED_TOHO_CORE = [
    "EX1-066", "EX12-004", "EX12-009", "EX12-011", "EX12-020", "EX12-026",
    "EX12-031", "EX12-036", "EX12-046", "EX12-047", "EX12-061", "EX12-062",
    "EX12-063", "EX12-065", "EX12-070", "EX12-074", "EX12-075", "EX12-076",
]


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


def test_published_toho_lists_reproduce_the_report_figures():
    """Guards the 0.7 default against the published report
    (qa/qa-reports/toho-braves-exam-report.md, 2026-08-26): 45 lists, a 42-card
    pool, and the 18-card core its per-card table lists.

    It reads the frozen lists the report was computed from, not the live
    library. This test used to read data/deck_library.json; the 2026-09-12 DCG
    Nexus refresh (f5e8c29b2) grew Toho Braves to 114 lists, a 50-card pool and
    a 17-card core (EX12-075 dropped out), and it failed from then on while
    saying nothing about core(). Reports are dated snapshots; the campaign
    tooling always recomputes from the live library.

    NOTE: the threshold is ceil(list_count * fraction), not a hardcoded
    literal -- ceil(45 * 0.7) = ceil(31.5) = 32, not 31. See
    code/tools/clause_coverage/archetype.py's core() docstring.
    """
    lib = load_archetypes(PUBLISHED_TOHO_LISTS)
    entry = lib[resolve(lib, "Toho Braves")]
    assert len(entry["decklists"]) == 45
    assert len(pool(entry)) == 42
    c = core(entry, DEFAULT_CORE_FRACTION)
    assert c["cards"] == PUBLISHED_TOHO_CORE, f"expected the published 18-card core, got {c['cards']}"
    assert c["threshold"] == 32
