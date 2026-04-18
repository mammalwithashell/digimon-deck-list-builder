"""Tests for the decklist analysis module (card frequency, differentials, trends)."""

from dataclasses import dataclass, field
from typing import Any, Dict, Optional

import pytest

from tools.decklist_analysis import (
    CardDifferential,
    CardFrequency,
    CardTrend,
    compute_card_frequencies,
    compute_card_trends,
    compute_winning_differentials,
    digilab_json_to_card_counts,
)


@dataclass
class MockDecklistRecord:
    """Minimal mock for DecklistRecord with required attributes."""
    result_id: int
    archetype_name: str
    decklist_json: Dict[str, Any]
    placement: Optional[int] = None
    wins: int = 0
    losses: int = 0
    event_date: Optional[str] = None


# ---------------------------------------------------------------------------
# JSON parsing
# ---------------------------------------------------------------------------

class TestDigilabJsonParsing:
    def test_basic_conversion(self):
        dl = {
            "egg": [{"count": 4, "name": "Egg1", "set": "ST14", "number": "01"}],
            "digimon": [
                {"count": 4, "name": "Digimon1", "set": "BT12", "number": "073"},
                {"count": 2, "name": "Digimon2", "set": "EX10", "number": "037"},
            ],
            "tamer": [{"count": 3, "name": "Tamer1", "set": "BT24", "number": "099"}],
            "option": [{"count": 2, "name": "Option1", "set": "EX2", "number": "071"}],
        }
        counts = digilab_json_to_card_counts(dl)
        assert counts["ST14-01"] == 4
        assert counts["BT12-073"] == 4
        assert counts["EX10-037"] == 2
        assert counts["BT24-099"] == 3
        assert counts["EX2-071"] == 2

    def test_empty_json(self):
        assert digilab_json_to_card_counts({}) == {}

    def test_missing_fields_skipped(self):
        dl = {
            "digimon": [
                {"count": 4, "name": "Good", "set": "BT1", "number": "010"},
                {"count": 0, "name": "Zero", "set": "BT1", "number": "999"},
                {"name": "NoCount", "set": "BT1", "number": "888"},
                {"count": 2, "name": "NoSet", "number": "777"},
            ],
        }
        counts = digilab_json_to_card_counts(dl)
        assert counts == {"BT1-010": 4}


# ---------------------------------------------------------------------------
# Card frequency
# ---------------------------------------------------------------------------

def _make_decklists():
    """Create 4 decklists with overlapping cards for testing."""
    base_cards = {
        "digimon": [
            {"count": 4, "name": "Core1", "set": "BT1", "number": "001"},
            {"count": 4, "name": "Core2", "set": "BT1", "number": "002"},
        ],
    }
    # All 4 lists share Core1 and Core2
    d1 = MockDecklistRecord(1, "Rocks", {
        **base_cards,
        "tamer": [{"count": 3, "name": "TechA", "set": "BT2", "number": "010"}],
    }, placement=1, wins=4, losses=0, event_date="2026-01-01")

    d2 = MockDecklistRecord(2, "Rocks", {
        **base_cards,
        "tamer": [{"count": 2, "name": "TechA", "set": "BT2", "number": "010"}],
    }, placement=2, wins=3, losses=1, event_date="2026-01-15")

    d3 = MockDecklistRecord(3, "Rocks", {
        **base_cards,
        "option": [{"count": 3, "name": "TechB", "set": "BT3", "number": "020"}],
    }, placement=5, wins=2, losses=2, event_date="2026-02-01")

    d4 = MockDecklistRecord(4, "Rocks", {
        **base_cards,
        "option": [
            {"count": 2, "name": "TechB", "set": "BT3", "number": "020"},
            {"count": 2, "name": "TechC", "set": "BT3", "number": "030"},
        ],
    }, placement=6, wins=1, losses=3, event_date="2026-02-15")

    return [d1, d2, d3, d4]


class TestCardFrequency:
    def test_basic_frequencies(self):
        decklists = _make_decklists()
        freqs = compute_card_frequencies(decklists)

        by_id = {f.card_id: f for f in freqs}

        # Core1 and Core2 appear in all 4 lists
        assert by_id["BT1-001"].inclusion_rate == 1.0
        assert by_id["BT1-002"].inclusion_rate == 1.0

        # TechA appears in 2 of 4 lists
        assert by_id["BT2-010"].inclusion_rate == 0.5

        # TechB appears in 2 of 4 lists
        assert by_id["BT3-020"].inclusion_rate == 0.5

        # TechC appears in 1 of 4 lists
        assert by_id["BT3-030"].inclusion_rate == 0.25

    def test_avg_copies(self):
        decklists = _make_decklists()
        freqs = compute_card_frequencies(decklists)
        by_id = {f.card_id: f for f in freqs}

        # TechA: 3 copies in list 1 + 2 copies in list 2 = 5 total / 4 lists = 1.25
        assert by_id["BT2-010"].avg_copies == pytest.approx(1.25)

    def test_empty_decklists(self):
        assert compute_card_frequencies([]) == []

    def test_sorted_by_inclusion(self):
        decklists = _make_decklists()
        freqs = compute_card_frequencies(decklists)
        rates = [f.inclusion_rate for f in freqs]
        assert rates == sorted(rates, reverse=True)


# ---------------------------------------------------------------------------
# Winning differentials
# ---------------------------------------------------------------------------

class TestWinningDifferentials:
    def test_basic_differential(self):
        decklists = _make_decklists()
        diffs = compute_winning_differentials(decklists, top_n=4)

        by_id = {d.card_id: d for d in diffs}

        # TechA appears in winners (list 1, 2) but not in others (list 3, 4)
        assert by_id["BT2-010"].winner_inclusion == 1.0
        assert by_id["BT2-010"].other_inclusion == 0.0
        assert by_id["BT2-010"].differential == 1.0

        # TechB appears in others (list 3, 4) but not winners
        assert by_id["BT3-020"].winner_inclusion == 0.0
        assert by_id["BT3-020"].other_inclusion == 1.0
        assert by_id["BT3-020"].differential == -1.0

    def test_sorted_by_differential(self):
        decklists = _make_decklists()
        diffs = compute_winning_differentials(decklists, top_n=4)
        deltas = [d.differential for d in diffs]
        assert deltas == sorted(deltas, reverse=True)

    def test_no_winners(self):
        # All placements > 4
        decklists = [MockDecklistRecord(1, "X", {"digimon": []}, placement=5)]
        assert compute_winning_differentials(decklists) == []

    def test_no_others(self):
        # All placements <= 4
        decklists = [MockDecklistRecord(1, "X", {"digimon": []}, placement=1)]
        assert compute_winning_differentials(decklists) == []


# ---------------------------------------------------------------------------
# Card trends
# ---------------------------------------------------------------------------

class TestCardTrends:
    def test_basic_trend(self):
        decklists = _make_decklists()
        trends = compute_card_trends(decklists, periods=2)

        by_id = {t.card_id: t for t in trends}

        # TechA: in period 1 (lists 1,2) = 100%, period 2 (lists 3,4) = 0%
        # slope = (0 - 1) / 1 = -1.0
        assert by_id["BT2-010"].trend_slope == pytest.approx(-1.0)

        # TechB: period 1 = 0%, period 2 = 100%
        # slope = (1 - 0) / 1 = 1.0
        assert by_id["BT3-020"].trend_slope == pytest.approx(1.0)

    def test_sorted_by_abs_slope(self):
        decklists = _make_decklists()
        trends = compute_card_trends(decklists, periods=2)
        slopes = [abs(t.trend_slope) for t in trends]
        assert slopes == sorted(slopes, reverse=True)

    def test_no_dates(self):
        dl = MockDecklistRecord(1, "X", {"digimon": []}, event_date=None)
        assert compute_card_trends([dl]) == []

    def test_single_list(self):
        dl = MockDecklistRecord(1, "X", {"digimon": []}, event_date="2026-01-01")
        assert compute_card_trends([dl]) == []
