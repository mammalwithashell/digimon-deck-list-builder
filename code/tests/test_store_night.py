"""Tests for the store night recommender and digilab_client enhancements."""

import json
import math
import statistics
from unittest.mock import patch

import pytest

from server.digilab_client import (
    ColorDistribution,
    DecklistRecord,
    PeriodMeta,
    PlayerResult,
    PlayerSummary,
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

    def test_event_type_filter(self):
        clause, params = _build_scope_clause(
            store_ids=[3], event_type="locals"
        )
        assert "event_type ILIKE" in clause
        assert params == ([3], "%locals%")

    def test_all_filters_combined(self):
        clause, params = _build_scope_clause(
            store_ids=[3], since_date="2025-12-01", event_type="locals"
        )
        assert "store_id" in clause
        assert "event_date" in clause
        assert "event_type" in clause
        assert params == ([3], "2025-12-01", "%locals%")

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


# ---------------------------------------------------------------------------
# Player scouting (Features 1, 2, 3)
# ---------------------------------------------------------------------------

def _make_player_summaries():
    """Build sample PlayerSummary objects for testing."""
    players = []

    # Regular skilled player — plays Rocks mostly
    p1 = PlayerSummary(player_name="Alice")
    p1.archetypes_played = {"Rocks": 6, "Medusamon": 2}
    p1.stores_attended = {"Card Haven": 8}
    p1.total_results = 8
    p1.last_seen = "2026-02-15"
    p1.results = [
        PlayerResult("Alice", "Rocks", "Card Haven", 3, "2026-02-15", 1, 4, 1),
        PlayerResult("Alice", "Rocks", "Card Haven", 3, "2026-02-08", 2, 3, 1),
        PlayerResult("Alice", "Rocks", "Card Haven", 3, "2026-02-01", 1, 4, 0),
        PlayerResult("Alice", "Rocks", "Card Haven", 3, "2026-01-25", 3, 3, 2),
        PlayerResult("Alice", "Rocks", "Card Haven", 3, "2026-01-18", 2, 3, 1),
        PlayerResult("Alice", "Rocks", "Card Haven", 3, "2026-01-11", 5, 2, 2),
        PlayerResult("Alice", "Medusamon", "Card Haven", 3, "2026-01-04", 4, 2, 2),
        PlayerResult("Alice", "Medusamon", "Card Haven", 3, "2025-12-28", 6, 1, 3),
    ]
    players.append(p1)

    # Occasional visitor — only 2 events
    p2 = PlayerSummary(player_name="Bob")
    p2.archetypes_played = {"Dark Masters": 2}
    p2.stores_attended = {"Card Haven": 2}
    p2.total_results = 2
    p2.last_seen = "2026-01-11"
    p2.results = [
        PlayerResult("Bob", "Dark Masters", "Card Haven", 3, "2026-01-11", 5, 2, 2),
        PlayerResult("Bob", "Dark Masters", "Card Haven", 3, "2025-12-28", 7, 1, 3),
    ]
    players.append(p2)

    # Another regular, less skilled
    p3 = PlayerSummary(player_name="Charlie")
    p3.archetypes_played = {"Medusamon": 3, "Rocks": 1, "Chaos Control": 1}
    p3.stores_attended = {"Card Haven": 5}
    p3.total_results = 5
    p3.last_seen = "2026-02-15"
    p3.results = [
        PlayerResult("Charlie", "Medusamon", "Card Haven", 3, "2026-02-15", 6, 1, 3),
        PlayerResult("Charlie", "Medusamon", "Card Haven", 3, "2026-02-01", 5, 2, 2),
        PlayerResult("Charlie", "Medusamon", "Card Haven", 3, "2026-01-18", 8, 0, 4),
        PlayerResult("Charlie", "Rocks", "Card Haven", 3, "2026-01-04", 3, 3, 1),
        PlayerResult("Charlie", "Chaos Control", "Card Haven", 3, "2025-12-28", 7, 1, 3),
    ]
    players.append(p3)

    return players


class TestPlayerLoyalty:
    def test_basic_loyalty(self):
        from tools.store_night import compute_player_loyalty

        players = _make_player_summaries()
        loyalty = compute_player_loyalty(players, min_events=3)

        # Alice and Charlie qualify (8 and 5 events), Bob doesn't (2)
        names = [l["player_name"] for l in loyalty]
        assert "Alice" in names
        assert "Charlie" in names
        assert "Bob" not in names

        alice = next(l for l in loyalty if l["player_name"] == "Alice")
        assert alice["primary_archetype"] == "Rocks"
        assert alice["loyalty_pct"] == 6 / 8

    def test_min_events_filter(self):
        from tools.store_night import compute_player_loyalty

        players = _make_player_summaries()
        loyalty = compute_player_loyalty(players, min_events=6)
        # Only Alice has >= 6 events
        assert len(loyalty) == 1
        assert loyalty[0]["player_name"] == "Alice"


class TestPlayerThreatProfiles:
    def test_threat_scoring(self):
        from tools.store_night import compute_player_threat_profiles

        players = _make_player_summaries()
        threats = compute_player_threat_profiles(players, min_events=3)

        # Alice should have highest threat (best WR + most events)
        assert threats[0]["player_name"] == "Alice"
        assert threats[0]["likely_archetype"] == "Rocks"

        # Threat score = wr * sqrt(events)
        alice_wins = sum(r.wins for r in players[0].results)
        alice_losses = sum(r.losses for r in players[0].results)
        alice_wr = alice_wins / (alice_wins + alice_losses)
        expected = alice_wr * math.sqrt(8)
        assert abs(threats[0]["threat_score"] - expected) < 0.01

    def test_empty_players(self):
        from tools.store_night import compute_player_threat_profiles
        assert compute_player_threat_profiles([], min_events=1) == []


class TestAttendanceDetection:
    def test_regularity_scoring(self):
        from tools.store_night import compute_attendance_profiles

        players = _make_player_summaries()
        profiles = compute_attendance_profiles(players, total_events=10, min_events=2)

        # All three have >= 2 events
        assert len(profiles) == 3

        # Alice: 8 distinct dates out of 10 events -> 80% regularity -> regular
        alice = next(p for p in profiles if p["player_name"] == "Alice")
        assert alice["is_regular"] is True
        assert alice["regularity_score"] == 0.8

        # Bob: 2 events out of 10 -> 20% -> not regular
        bob = next(p for p in profiles if p["player_name"] == "Bob")
        assert bob["is_regular"] is False

    def test_zero_total_events(self):
        from tools.store_night import compute_attendance_profiles
        players = _make_player_summaries()
        profiles = compute_attendance_profiles(players, total_events=0, min_events=2)
        for p in profiles:
            assert p["regularity_score"] == 0.0


# ---------------------------------------------------------------------------
# Meta dynamics (Features 7, 9)
# ---------------------------------------------------------------------------

class TestMetaVelocity:
    def _make_periods(self):
        p1 = PeriodMeta(
            period_label="Jan", period_start="2026-01-01", period_end="2026-01-31",
            archetypes={
                "Rocks": ScopedArchetypeStats("Rocks", meta_share=0.20, times_played=10),
                "Medusamon": ScopedArchetypeStats("Medusamon", meta_share=0.15, times_played=8),
            },
            total_results=50,
        )
        p2 = PeriodMeta(
            period_label="Feb", period_start="2026-02-01", period_end="2026-02-28",
            archetypes={
                "Rocks": ScopedArchetypeStats("Rocks", meta_share=0.25, times_played=12),
                "Medusamon": ScopedArchetypeStats("Medusamon", meta_share=0.10, times_played=5),
                "NewDeck": ScopedArchetypeStats("NewDeck", meta_share=0.05, times_played=3),
            },
            total_results=48,
        )
        return [p1, p2]

    def test_basic_velocity(self):
        from tools.store_night import compute_meta_velocity

        periods = self._make_periods()
        velocity = compute_meta_velocity(periods)

        assert velocity["Rocks"] == pytest.approx(0.05)    # 0.25 - 0.20
        assert velocity["Medusamon"] == pytest.approx(-0.05)  # 0.10 - 0.15
        assert velocity["NewDeck"] == pytest.approx(0.05)   # 0.05 - 0.0

    def test_single_period_empty(self):
        from tools.store_night import compute_meta_velocity
        p = PeriodMeta("Jan", "2026-01-01", "2026-01-31", {}, 0)
        assert compute_meta_velocity([p]) == {}


class TestMetaComparison:
    def test_side_by_side(self):
        from tools.store_night import compute_meta_comparison

        m1 = ScopedMetaResult(
            archetypes={
                "Rocks": ScopedArchetypeStats("Rocks", meta_share=0.20),
                "Medusamon": ScopedArchetypeStats("Medusamon", meta_share=0.10),
            },
            total_results=100,
        )
        m2 = ScopedMetaResult(
            archetypes={
                "Rocks": ScopedArchetypeStats("Rocks", meta_share=0.08),
                "Medusamon": ScopedArchetypeStats("Medusamon", meta_share=0.22),
                "NewDeck": ScopedArchetypeStats("NewDeck", meta_share=0.05),
            },
            total_results=80,
        )
        store_metas = {"Card Haven": m1, "Boardwalk": m2}
        comparison = compute_meta_comparison(store_metas)

        # Sorted by delta descending
        assert len(comparison) == 3

        # Check that all archetypes are present
        arch_names = {r["archetype"] for r in comparison}
        assert arch_names == {"Rocks", "Medusamon", "NewDeck"}

        # Medusamon delta = |0.22 - 0.10| = 0.12 (biggest)
        medusa = next(r for r in comparison if r["archetype"] == "Medusamon")
        assert medusa["delta"] == pytest.approx(0.12)

    def test_single_store_returns_empty(self):
        from tools.store_night import compute_meta_comparison
        m = ScopedMetaResult(archetypes={}, total_results=0)
        assert compute_meta_comparison({"only": m}) == []


# ---------------------------------------------------------------------------
# Color heatmap (Feature 11)
# ---------------------------------------------------------------------------

class TestColorHeatmap:
    def test_format_heatmap(self):
        from tools.store_night import format_color_heatmap

        dists = [
            ColorDistribution("Red", None, 20, 0.40),
            ColorDistribution("Blue", "White", 15, 0.30),
            ColorDistribution("Purple", None, 10, 0.20),
        ]
        output = format_color_heatmap(dists)
        assert "Red" in output
        assert "Blue / White" in output
        assert "40.0%" in output

    def test_empty_distributions(self):
        from tools.store_night import format_color_heatmap
        assert "No color data" in format_color_heatmap([])


# ---------------------------------------------------------------------------
# New dataclass construction tests
# ---------------------------------------------------------------------------

class TestNewDataclasses:
    def test_decklist_record(self):
        r = DecklistRecord(
            result_id=1,
            archetype_name="Rocks",
            decklist_json={"digimon": []},
            placement=1,
            wins=4,
            losses=1,
            player_name="Alice",
            event_date="2026-01-15",
            player_count=16,
        )
        assert r.archetype_name == "Rocks"
        assert r.player_count == 16

    def test_period_meta(self):
        pm = PeriodMeta(
            period_label="Jan",
            period_start="2026-01-01",
            period_end="2026-01-31",
            total_results=42,
        )
        assert pm.total_results == 42
        assert pm.archetypes == {}
