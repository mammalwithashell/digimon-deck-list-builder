"""Tests for the store night recommender and digilab_client enhancements."""

import json
import statistics
from unittest.mock import patch

import pytest

from digimon_gym.digilab_client import (
    ScopedArchetypeStats,
    ScopedMetaResult,
    _build_scope_clause,
)


# ---------------------------------------------------------------------------
# digilab_client: _build_scope_clause
# ---------------------------------------------------------------------------

class TestBuildScopeClause:
    def test_store_ids_only(self):
        clause, params = _build_scope_clause(store_ids=[3, 6])
        assert "store_id = ANY" in clause
        assert params == ([3, 6],)

    def test_scene_id_only(self):
        clause, params = _build_scope_clause(scene_id=1)
        assert "scene_id" in clause
        assert params == (1,)

    def test_store_with_since_date(self):
        clause, params = _build_scope_clause(
            store_ids=[3], since_date="2025-12-01"
        )
        assert "event_date" in clause
        assert "store_id" in clause
        assert params == ([3], "2025-12-01")

    def test_scene_with_since_date(self):
        clause, params = _build_scope_clause(
            scene_id=1, since_date="2025-06-01"
        )
        assert "event_date" in clause
        assert params == (1, "2025-06-01")

    def test_no_scope_raises(self):
        with pytest.raises(ValueError, match="Must provide"):
            _build_scope_clause()


# ---------------------------------------------------------------------------
# digilab_client: ScopedMetaResult
# ---------------------------------------------------------------------------

class TestScopedMetaResult:
    def test_basic_construction(self):
        archetypes = {
            "Rocks": ScopedArchetypeStats(
                archetype_name="Rocks",
                meta_share=0.10,
                conversion_rate=0.50,
                win_rate=0.62,
                times_played=8,
            ),
            "Medusamon": ScopedArchetypeStats(
                archetype_name="Medusamon",
                meta_share=0.15,
                conversion_rate=0.46,
                win_rate=0.48,
                times_played=13,
            ),
        }
        result = ScopedMetaResult(
            archetypes=archetypes,
            total_results=128,
            median_times_played=4.0,
            mean_times_played=3.5,
        )

        assert result.total_results == 128
        assert result.median_times_played == 4.0
        assert result.mean_times_played == 3.5
        assert len(result.archetypes) == 2

    def test_median_mean_computation(self):
        """Verify median/mean logic matches what get_scoped_meta computes."""
        play_counts = [13, 9, 8, 8, 7, 5, 4, 4, 3, 2, 1, 1, 1]
        median = statistics.median(play_counts)
        mean = statistics.mean(play_counts)
        assert median == 4
        assert round(mean, 2) == 5.08


# ---------------------------------------------------------------------------
# store_night: personal library loading
# ---------------------------------------------------------------------------

class TestPersonalLibrary:
    def test_load_personal_library(self, tmp_path):
        from tools.store_night import load_personal_library

        lib_path = tmp_path / "my_decks.json"
        data = {
            "general_pool": ["BT24-099", "EX10-068"],
            "Rocks": {
                "decklists": [
                    {"name": "anti-red", "deck": ["BT24-001", "BT24-002"]},
                    {"name": "standard", "deck": ["BT24-001", "BT24-003"]},
                ]
            },
        }
        lib_path.write_text(json.dumps(data))

        loaded = load_personal_library(str(lib_path))
        assert loaded["general_pool"] == ["BT24-099", "EX10-068"]
        assert len(loaded["Rocks"]["decklists"]) == 2

    def test_load_missing_returns_empty(self, tmp_path):
        from tools.store_night import load_personal_library

        loaded = load_personal_library(str(tmp_path / "nonexistent.json"))
        assert loaded == {}

    def test_resolve_from_personal(self):
        from tools.store_night import resolve_deck_from_personal

        lib = {
            "Rocks": {
                "decklists": [
                    {"name": "main", "deck": ["A", "B", "C"]},
                    {"name": "alt", "deck": ["D", "E"]},
                ]
            }
        }

        # Returns first decklist
        deck = resolve_deck_from_personal(lib, "Rocks")
        assert deck == ["A", "B", "C"]

        # Missing archetype
        assert resolve_deck_from_personal(lib, "Missing") is None

        # Empty decklists
        assert resolve_deck_from_personal({"X": {"decklists": []}}, "X") is None

    def test_collect_personal_pool(self):
        from tools.store_night import collect_personal_pool_cards

        lib = {
            "Rocks": {
                "decklists": [
                    {"name": "a", "deck": ["A", "B", "C"]},
                    {"name": "b", "deck": ["B", "C", "D"]},
                ]
            }
        }

        cards = collect_personal_pool_cards(lib, "Rocks")
        assert cards == {"A", "B", "C", "D"}

    def test_general_pool(self):
        from tools.store_night import get_general_pool

        lib = {"general_pool": ["X", "Y"]}
        assert get_general_pool(lib) == ["X", "Y"]
        assert get_general_pool({}) == []

    def test_get_personal_deck_name(self):
        from tools.store_night import get_personal_deck_name

        lib = {
            "Rocks": {
                "decklists": [{"name": "anti-red", "deck": []}]
            }
        }
        assert get_personal_deck_name(lib, "Rocks") == "anti-red"
        assert get_personal_deck_name(lib, "Missing") == "unnamed"


# ---------------------------------------------------------------------------
# store_night: sleeper detection
# ---------------------------------------------------------------------------

class TestSleeperClassification:
    def _make_scoped(self, entries):
        """Helper to make scoped meta dict from (name, share, wr, conv, plays)."""
        return {
            name: ScopedArchetypeStats(
                archetype_name=name,
                meta_share=share,
                win_rate=wr,
                conversion_rate=conv,
                times_played=plays,
            )
            for name, share, wr, conv, plays in entries
        }

    def test_basic_classification(self):
        from tools.store_night import classify_archetypes

        scoped = self._make_scoped([
            ("Medusamon", 0.10, 0.48, 0.46, 13),
            ("Rocks", 0.06, 0.62, 0.50, 8),
            ("Dark Masters", 0.04, 0.94, 1.00, 5),
            ("Chaos Control", 0.03, 0.83, 1.00, 2),
            ("Invisimon", 0.01, 0.00, 0.00, 1),
        ])

        # median plays = 5, floor = max(3, 5/2) = 3
        threats, sleepers, insufficient = classify_archetypes(
            scoped, median_plays=5.0, min_plays=3
        )

        threat_names = {t["name"] for t in threats}
        sleeper_names = {s["name"] for s in sleepers}
        insufficient_names = {i["name"] for i in insufficient}

        assert "Medusamon" in threat_names  # conv 46% < 50%
        assert "Rocks" in threat_names      # conv 50% == threshold (not >)
        assert "Dark Masters" in sleeper_names  # conv 100%, plays 5 >= 3
        assert "Chaos Control" in insufficient_names  # plays 2 < 3
        assert "Invisimon" in insufficient_names  # plays 1 < 3

    def test_high_median_raises_floor(self):
        from tools.store_night import classify_archetypes

        scoped = self._make_scoped([
            ("BigDeck", 0.10, 0.60, 0.80, 4),
        ])

        # median = 10 -> floor = max(3, 10/2) = 5 -> plays=4 is insufficient
        threats, sleepers, insufficient = classify_archetypes(
            scoped, median_plays=10.0, min_plays=3
        )
        assert len(insufficient) == 1
        assert len(sleepers) == 0

    def test_empty_meta(self):
        from tools.store_night import classify_archetypes

        threats, sleepers, insufficient = classify_archetypes(
            {}, median_plays=0.0
        )
        assert threats == []
        assert sleepers == []
        assert insufficient == []


# ---------------------------------------------------------------------------
# meta_loader: player_name field
# ---------------------------------------------------------------------------

class TestPlayerNameField:
    def test_ingested_deck_has_player_name(self):
        """IngestedDeck dataclass should accept player_name."""
        import sys
        sys.path.insert(0, str(
            __import__("pathlib").Path(__file__).resolve().parent.parent / "tools"
        ))
        from meta_loader import IngestedDeck

        deck = IngestedDeck(
            deck_id="test_1",
            source="egman",
            player_name="TestPlayer",
        )
        assert deck.player_name == "TestPlayer"

    def test_ingested_deck_player_name_optional(self):
        from meta_loader import IngestedDeck

        deck = IngestedDeck(deck_id="test_2", source="digilab")
        assert deck.player_name is None

    def test_egman_parser_returns_player_name(self):
        """The Egman parser should return player name as 5th tuple element."""
        from meta_loader import DeckIngestor
        from unittest.mock import MagicMock

        # Simulate 8 <td> cells
        cells = []
        for text in [
            "",                  # 0: icon
            "Rocks",             # 1: archetype
            "JohnDoe",           # 2: player name
            "1st",               # 3: placement
            "BT24",              # 4: format
            "Locals",            # 5: event type
            "Card Haven (16)",   # 6: event name + count
            "3/15/26",           # 7: date
        ]:
            cell = MagicMock()
            cell.get_text.return_value = text
            cell.find.return_value = None
            cells.append(cell)

        result = DeckIngestor._parse_egman_row(cells)
        assert len(result) == 5
        archetype, placement, event_date, event_players, player_name = result

        assert archetype == "Rocks"
        assert player_name == "JohnDoe"
        assert placement == "1st"
        assert event_players == 16
