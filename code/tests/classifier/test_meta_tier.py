"""Tests for the deck meta-tier classifier.

The classifier consumes tournament-archetype decklists (shape of
`deck_library.json`) and, given a user's deck, returns:
    (archetype_name | None, tier, confidence)

where tier ∈ {"meta", "rogue", "jank"} based on the matched archetype's
meta_share. Tests use small in-memory libraries for determinism — no I/O.
"""
from __future__ import annotations

import json

import pytest

from server.classifier.meta_tier import (
    ClassifierLibrary,
    MetaTier,
    build_library_from_deck_library,
    classify_deck,
)


# ── Fixtures ─────────────────────────────────────────────────────────────

def _archetype(
    name: str,
    meta_share: float,
    decklists: list[list[str]],
    *,
    times_played: int | None = None,
) -> dict:
    return {
        "archetype_name": name,
        "stats": {
            "times_played": times_played if times_played is not None else len(decklists),
            "meta_share": meta_share,
            "top_cut_count": 0,
            "conversion_rate": 0.0,
            "avg_placement": 0.0,
            "sources": {},
        },
        "decklists": [
            {
                "deck_id": f"{name}-{i}",
                "source": "test",
                "decklist": json.dumps(d),
                "placement": "1",
                "is_top_cut": True,
                "event_date": "2026-01-01",
            }
            for i, d in enumerate(decklists)
        ],
    }


@pytest.fixture
def library() -> ClassifierLibrary:
    """Mini library with one clear meta deck, one rogue, one well-sampled fringe."""
    doc = {
        "version": 1,
        "generated_at": "2026-04-17T00:00:00Z",
        "archetypes": {
            # Meta: high share, very consistent staples across 4 lists
            "Rocks": _archetype(
                "Rocks",
                meta_share=0.06,
                decklists=[
                    ["BT24-001"] * 4 + ["BT24-002"] * 4 + ["BT24-003"] * 4 + ["EGG-1"] * 5,
                    ["BT24-001"] * 4 + ["BT24-002"] * 4 + ["BT24-003"] * 3 + ["EGG-1"] * 5,
                    ["BT24-001"] * 4 + ["BT24-002"] * 4 + ["BT24-003"] * 4 + ["EGG-1"] * 5,
                    ["BT24-001"] * 4 + ["BT24-002"] * 3 + ["BT24-003"] * 4 + ["EGG-1"] * 5,
                ],
            ),
            # Rogue: moderate share, 3 consistent lists
            "Sakuyamon": _archetype(
                "Sakuyamon",
                meta_share=0.012,
                decklists=[
                    ["BT20-010"] * 4 + ["BT20-011"] * 4 + ["BT20-012"] * 4 + ["EGG-2"] * 5,
                    ["BT20-010"] * 4 + ["BT20-011"] * 4 + ["BT20-012"] * 3 + ["EGG-2"] * 5,
                    ["BT20-010"] * 4 + ["BT20-011"] * 3 + ["BT20-012"] * 4 + ["EGG-2"] * 5,
                ],
            ),
            # Known fringe: below rogue threshold
            "HomebrewFringe": _archetype(
                "HomebrewFringe",
                meta_share=0.0008,
                decklists=[
                    ["BT19-100"] * 4 + ["BT19-101"] * 4 + ["EGG-3"] * 5,
                ],
            ),
        },
    }
    return build_library_from_deck_library(doc)


# ── Fingerprint construction ─────────────────────────────────────────────

class TestBuildLibrary:
    def test_skips_archetypes_with_no_decklists(self):
        doc = {
            "archetypes": {
                "Empty": {
                    "stats": {"meta_share": 0.1, "times_played": 0},
                    "decklists": [],
                },
            },
        }
        lib = build_library_from_deck_library(doc)
        assert lib.archetypes == []

    def test_staples_reflect_inclusion_rate(self, library: ClassifierLibrary):
        rocks = next(a for a in library.archetypes if a.name == "Rocks")
        # BT24-001, BT24-002, BT24-003 all appear in every list → staples
        assert "BT24-001" in rocks.staples
        assert rocks.staples["BT24-001"] == pytest.approx(1.0)
        assert rocks.staples["BT24-002"] == pytest.approx(1.0)
        assert rocks.staples["BT24-003"] == pytest.approx(1.0)

    def test_archetypes_sorted_by_meta_share_desc(self, library: ClassifierLibrary):
        names = [a.name for a in library.archetypes]
        assert names == ["Rocks", "Sakuyamon", "HomebrewFringe"]


# ── Classification ───────────────────────────────────────────────────────

class TestClassifyDeck:
    def test_known_meta_deck_returns_meta(self, library: ClassifierLibrary):
        # Exact Rocks list
        deck = ["BT24-001"] * 4 + ["BT24-002"] * 4 + ["BT24-003"] * 4 + ["EGG-1"] * 5
        arch, tier, conf = classify_deck(deck, library)
        assert arch == "Rocks"
        assert tier == "meta"
        assert conf >= 0.9

    def test_known_rogue_deck_returns_rogue(self, library: ClassifierLibrary):
        deck = ["BT20-010"] * 4 + ["BT20-011"] * 4 + ["BT20-012"] * 4 + ["EGG-2"] * 5
        arch, tier, conf = classify_deck(deck, library)
        assert arch == "Sakuyamon"
        assert tier == "rogue"

    def test_known_fringe_below_rogue_threshold_returns_jank(self, library: ClassifierLibrary):
        """A match to an archetype with meta_share < rogue threshold still tiers as jank."""
        deck = ["BT19-100"] * 4 + ["BT19-101"] * 4 + ["EGG-3"] * 5
        arch, tier, conf = classify_deck(deck, library)
        assert arch == "HomebrewFringe"
        assert tier == "jank"

    def test_completely_unknown_deck_is_jank_with_no_match(self, library: ClassifierLibrary):
        deck = ["CUSTOM-001"] * 4 + ["CUSTOM-002"] * 4 + ["CUSTOM-EGG"] * 5
        arch, tier, conf = classify_deck(deck, library)
        assert arch is None
        assert tier == "jank"
        assert conf == 0.0

    def test_partial_overlap_below_min_coverage_is_jank(self, library: ClassifierLibrary):
        """A deck that shares 1 of 3 staples with Rocks shouldn't be classified as Rocks."""
        deck = ["BT24-001"] * 2 + ["CUSTOM-X"] * 4 + ["CUSTOM-Y"] * 4 + ["CUSTOM-Z"] * 40
        arch, tier, _ = classify_deck(deck, library)
        assert arch is None
        assert tier == "jank"

    def test_empty_deck_is_jank(self, library: ClassifierLibrary):
        arch, tier, conf = classify_deck([], library)
        assert arch is None
        assert tier == "jank"

    def test_classification_is_copy_count_insensitive(self, library: ClassifierLibrary):
        """Only unique card IDs count — a deck with 1 copy of each Rocks staple still matches."""
        deck = ["BT24-001", "BT24-002", "BT24-003", "EGG-1"]
        arch, _, _ = classify_deck(deck, library)
        assert arch == "Rocks"


# ── Tier thresholds ──────────────────────────────────────────────────────

class TestTierThresholds:
    @pytest.mark.parametrize("share,expected", [
        (0.10, "meta"),
        (0.02, "meta"),       # exactly at threshold → meta
        (0.019, "rogue"),
        (0.005, "rogue"),     # exactly at threshold → rogue
        (0.0049, "jank"),
        (0.0, "jank"),
    ])
    def test_share_to_tier_mapping(self, share: float, expected: MetaTier):
        from server.classifier.meta_tier import share_to_tier
        assert share_to_tier(share) == expected

    def test_custom_thresholds_honored(self, library: ClassifierLibrary):
        # Tighten: only ≥10% is meta
        lib = ClassifierLibrary(
            archetypes=library.archetypes,
            meta_threshold=0.10,
            rogue_threshold=0.005,
        )
        deck = ["BT24-001"] * 4 + ["BT24-002"] * 4 + ["BT24-003"] * 4 + ["EGG-1"] * 5
        arch, tier, _ = classify_deck(deck, lib)
        assert arch == "Rocks"
        assert tier == "rogue"  # 6% no longer meta under tighter threshold


# ── Format scoping ───────────────────────────────────────────────────────

TOHO = ["EX12-076"] * 4 + ["EX12-047"] * 4 + ["EX12-061"] * 4 + ["EX12-004"] * 5


def _scoped_doc() -> dict:
    """A corpus where the current format is a thin slice of all-time history.

    Mirrors the real 2026-07 numbers: Toho Braves was 12.19% of EX12 but only
    1.04% of the merged corpus — under the 2% threshold, so the unscoped
    classifier called the format's most-played deck "rogue".
    """
    def lists(cards, n, fmt):
        return [
            {
                "deck_id": f"{fmt}-{i}",
                "source": "dcg_nexus",
                "decklist": json.dumps(cards),
                "format": fmt,
                "placement": "1",
                "is_top_cut": True,
                "event_date": "2026-07-18",
            }
            for i in range(n)
        ]

    return {
        "archetypes": {
            "Toho Braves": {
                "archetype_name": "Toho Braves",
                "stats": {"times_played": 3, "meta_share": 0.0104},
                "format_stats": {"EX12": {"times_played": 3, "meta_share": 0.1219}},
                "decklists": lists(TOHO, 3, "EX12"),
            },
            "Old Guard": {
                "archetype_name": "Old Guard",
                "stats": {"times_played": 95, "meta_share": 0.95},
                "format_stats": {"BT23": {"times_played": 95, "meta_share": 0.95}},
                "decklists": lists(["BT23-001"] * 4 + ["BT23-002"] * 4, 95, "BT23"),
            },
        }
    }


class TestFormatScope:
    def test_unscoped_tiers_current_format_deck_as_rogue(self):
        """The bug: all-time share buries the format's best deck."""
        lib = build_library_from_deck_library(_scoped_doc())
        arch, tier, _ = classify_deck(TOHO, lib)
        assert arch == "Toho Braves"
        assert tier == "rogue"  # 1.04% of all time, under the 2% meta threshold

    def test_scoped_tiers_same_deck_as_meta(self):
        lib = build_library_from_deck_library(_scoped_doc(), format_scope="EX12")
        arch, tier, _ = classify_deck(TOHO, lib)
        assert arch == "Toho Braves"
        assert tier == "meta"  # 12.19% of EX12

    def test_scope_excludes_archetypes_absent_from_format(self):
        lib = build_library_from_deck_library(_scoped_doc(), format_scope="EX12")
        assert [a.name for a in lib.archetypes] == ["Toho Braves"]

    def test_unknown_scope_yields_empty_library(self):
        lib = build_library_from_deck_library(_scoped_doc(), format_scope="EX99")
        assert lib.archetypes == []

    def test_default_behaviour_unchanged(self, library: ClassifierLibrary):
        """format_scope=None must reproduce the pre-existing result exactly."""
        doc = _scoped_doc()
        assert (
            [(a.name, a.meta_share)
             for a in build_library_from_deck_library(doc).archetypes]
            == [(a.name, a.meta_share)
                for a in build_library_from_deck_library(
                    doc, format_scope=None).archetypes]
        )

    def test_staples_scoped_to_format_lists(self):
        """Staples come only from the scoped format's decks, not all history."""
        doc = _scoped_doc()
        # Give Toho a stale BT23-era list that would pollute its fingerprint.
        doc["archetypes"]["Toho Braves"]["decklists"].append({
            "deck_id": "stale", "source": "dcg_nexus",
            "decklist": json.dumps(["BT23-999"] * 50),
            "format": "BT23", "placement": "1", "is_top_cut": True,
            "event_date": "2025-11-01",
        })
        lib = build_library_from_deck_library(doc, format_scope="EX12")
        toho = next(a for a in lib.archetypes if a.name == "Toho Braves")
        assert "BT23-999" not in toho.staples
        assert "EX12-076" in toho.staples
