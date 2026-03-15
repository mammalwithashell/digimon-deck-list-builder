"""Tests for tools/meta_loader.py — DeckIngestor parsers and processing.

All tests use mock data; no network calls.
"""

import json
import os
import sys

import pytest

# Ensure project root is importable
_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if _ROOT not in sys.path:
    sys.path.insert(0, _ROOT)

from tools.meta_loader import (
    DeckIngestor,
    IngestedDeck,
    ArchetypeMeta,
    ComputedStats,
    DigiLabStats,
    RE_DIGIMONMETA_ENTRY,
    _deck_fingerprint,
    _is_top_cut,
    _card_counts_diff,
)


# ─── DigimonMeta Code Parsing ───────────────────────────────────────

class TestDigimonMetaCodeParsing:
    """Tests for the DigimonMeta deck code regex."""

    def test_basic_code(self):
        code = "4nBT24-017a3nP-103a"
        matches = RE_DIGIMONMETA_ENTRY.findall(code)
        assert matches == [("4", "BT24-017"), ("3", "P-103")]

    def test_code_without_trailing_a(self):
        code = "4nBT24-017a2nBT22-096"
        matches = RE_DIGIMONMETA_ENTRY.findall(code)
        assert matches == [("4", "BT24-017"), ("2", "BT22-096")]

    def test_full_deck_code(self):
        code = (
            "4nBT22-001a4nEX11-013a4nBT19-017a2nBT22-018a"
            "2nBT24-019a2nBT19-019a4nBT24-023a4nBT24-027a"
        )
        matches = RE_DIGIMONMETA_ENTRY.findall(code)
        assert len(matches) == 8
        # Total cards
        total = sum(int(m[0]) for m in matches)
        assert total == 26  # Partial deck

    def test_promo_card_ids(self):
        code = "4nP-104a2nLM-028a"
        matches = RE_DIGIMONMETA_ENTRY.findall(code)
        assert matches == [("4", "P-104"), ("2", "LM-028")]

    def test_empty_code(self):
        matches = RE_DIGIMONMETA_ENTRY.findall("")
        assert matches == []

    def test_garbage_code(self):
        matches = RE_DIGIMONMETA_ENTRY.findall("not_a_valid_code")
        assert matches == []


# ─── Top-Cut Detection ──────────────────────────────────────────────

class TestTopCutDetection:

    def test_first_place_string(self):
        assert _is_top_cut("1st Place") is True

    def test_fourth_place_string(self):
        assert _is_top_cut("4th Place") is True

    def test_fifth_place_not_top_cut(self):
        assert _is_top_cut("5th Place") is False

    def test_numeric_placement_top(self):
        assert _is_top_cut("2") is True

    def test_numeric_placement_not_top(self):
        assert _is_top_cut("8") is False

    def test_numeric_with_large_event(self):
        # 32 players: top 25% = top 8
        assert _is_top_cut("6", event_players=32) is True

    def test_numeric_below_threshold_large_event(self):
        assert _is_top_cut("12", event_players=32) is False

    def test_none_placement(self):
        assert _is_top_cut(None) is False

    def test_ordinal_formats(self):
        assert _is_top_cut("1st") is True
        assert _is_top_cut("3rd") is True


# ─── Deck Fingerprint ───────────────────────────────────────────────

class TestDeckFingerprint:

    def test_same_deck_same_fingerprint(self):
        a = {"BT24-017": 4, "P-103": 2}
        b = {"P-103": 2, "BT24-017": 4}
        assert _deck_fingerprint(a) == _deck_fingerprint(b)

    def test_different_deck_different_fingerprint(self):
        a = {"BT24-017": 4, "P-103": 2}
        b = {"BT24-017": 3, "P-103": 3}
        assert _deck_fingerprint(a) != _deck_fingerprint(b)

    def test_empty_deck(self):
        fp = _deck_fingerprint({})
        assert isinstance(fp, str)
        assert len(fp) > 0


# ─── Card Counts Diff ───────────────────────────────────────────────

class TestCardCountsDiff:

    def test_identical_decks(self):
        a = {"BT24-017": 4, "P-103": 2}
        assert _card_counts_diff(a, a) == 0

    def test_one_card_different(self):
        a = {"BT24-017": 4, "P-103": 2}
        b = {"BT24-017": 4, "P-103": 1, "BT24-018": 1}
        assert _card_counts_diff(a, b) == 2  # -1 P-103 + +1 BT24-018

    def test_completely_different(self):
        a = {"BT24-017": 4}
        b = {"P-103": 4}
        assert _card_counts_diff(a, b) == 8  # 4 removed + 4 added


# ─── DeckIngestor Unit Tests ────────────────────────────────────────

class TestDeckIngestor:

    def test_resolve_archetypes_by_display_card(self):
        """Deck containing display_card_id maps to correct archetype."""
        ingestor = DeckIngestor()
        ingestor.archetypes["Medusamon"] = ArchetypeMeta(
            archetype_name="Medusamon",
            display_card_id="BT24-017",
        )
        ingestor.unresolved_decks = [
            IngestedDeck(
                deck_id="test_001",
                source="test",
                card_ids=["BT24-017"] * 4 + ["BT24-018"] * 4,
                card_counts={"BT24-017": 4, "BT24-018": 4},
            ),
        ]

        ingestor.resolve_archetypes()

        assert len(ingestor.unresolved_decks) == 0
        assert len(ingestor.archetypes["Medusamon"].decklists) == 1

    def test_resolve_archetypes_no_match_goes_to_unclassified(self):
        """Deck with no display card match goes to Unclassified."""
        ingestor = DeckIngestor()
        ingestor.archetypes["Medusamon"] = ArchetypeMeta(
            archetype_name="Medusamon",
            display_card_id="BT24-017",
        )
        ingestor.unresolved_decks = [
            IngestedDeck(
                deck_id="test_002",
                source="test",
                card_ids=["ST1-03"] * 4,
                card_counts={"ST1-03": 4},
            ),
        ]

        ingestor.resolve_archetypes()

        assert "Unclassified" in ingestor.archetypes
        assert len(ingestor.archetypes["Unclassified"].decklists) == 1

    def test_compute_meta_stats(self):
        """Meta shares sum to approximately 1.0, conversion rates correct."""
        ingestor = DeckIngestor()

        # Archetype A: 3 decks, 2 top cuts
        ingestor.archetypes["A"] = ArchetypeMeta(
            archetype_name="A",
            decklists=[
                IngestedDeck(deck_id="a1", source="test", is_top_cut=True, placement="1st Place"),
                IngestedDeck(deck_id="a2", source="test", is_top_cut=True, placement="2nd Place"),
                IngestedDeck(deck_id="a3", source="test", is_top_cut=False, placement="8"),
            ],
        )
        # Archetype B: 2 decks, 0 top cuts
        ingestor.archetypes["B"] = ArchetypeMeta(
            archetype_name="B",
            decklists=[
                IngestedDeck(deck_id="b1", source="test", is_top_cut=False),
                IngestedDeck(deck_id="b2", source="test", is_top_cut=False),
            ],
        )

        ingestor.compute_meta_stats()

        assert ingestor.archetypes["A"].stats.times_played == 3
        assert ingestor.archetypes["B"].stats.times_played == 2
        assert abs(ingestor.archetypes["A"].stats.meta_share - 0.6) < 0.01
        assert abs(ingestor.archetypes["B"].stats.meta_share - 0.4) < 0.01
        assert abs(ingestor.archetypes["A"].stats.conversion_rate - 2 / 3) < 0.01
        assert ingestor.archetypes["B"].stats.conversion_rate == 0.0

        # Meta shares sum to 1
        total_share = sum(a.stats.meta_share for a in ingestor.archetypes.values())
        assert abs(total_share - 1.0) < 0.001

    def test_deduplicate_exact_match(self):
        """Exact duplicate decklists are merged, keeping preferred source."""
        ingestor = DeckIngestor()
        ingestor.archetypes["A"] = ArchetypeMeta(
            archetype_name="A",
            decklists=[
                IngestedDeck(
                    deck_id="egman_001", source="egman",
                    card_ids=["BT24-017"] * 4,
                    card_counts={"BT24-017": 4},
                ),
                IngestedDeck(
                    deck_id="digimonmeta_001", source="digimonmeta",
                    card_ids=["BT24-017"] * 4,
                    card_counts={"BT24-017": 4},
                ),
            ],
        )

        removed = ingestor.deduplicate()

        assert removed == 1
        assert len(ingestor.archetypes["A"].decklists) == 1
        # DigimonMeta should win (higher priority)
        assert ingestor.archetypes["A"].decklists[0].source == "digimonmeta"

    def test_deduplicate_no_duplicates(self):
        """Unique decklists are preserved."""
        ingestor = DeckIngestor()
        ingestor.archetypes["A"] = ArchetypeMeta(
            archetype_name="A",
            decklists=[
                IngestedDeck(
                    deck_id="d1", source="egman",
                    card_ids=["BT24-017"] * 4,
                    card_counts={"BT24-017": 4},
                ),
                IngestedDeck(
                    deck_id="d2", source="egman",
                    card_ids=["P-103"] * 4,
                    card_counts={"P-103": 4},
                ),
            ],
        )

        removed = ingestor.deduplicate()
        assert removed == 0
        assert len(ingestor.archetypes["A"].decklists) == 2

    def test_save_and_load_roundtrip(self, tmp_path):
        """Save then load preserves all data."""
        path = str(tmp_path / "deck_library.json")

        ingestor = DeckIngestor()
        ingestor.archetypes["TestArch"] = ArchetypeMeta(
            archetype_name="TestArch",
            primary_color="Red",
            display_card_id="BT24-017",
            stats=ComputedStats(
                times_played=5,
                meta_share=0.5,
                top_cut_count=2,
                conversion_rate=0.4,
            ),
            decklists=[
                IngestedDeck(
                    deck_id="test_001",
                    source="digimonmeta",
                    source_url="https://example.com",
                    card_ids=["BT24-017"] * 4 + ["P-103"] * 2,
                    card_counts={"BT24-017": 4, "P-103": 2},
                    archetype_name="TestArch",
                    placement="1st Place",
                    is_top_cut=True,
                ),
            ],
        )

        ingestor.save(path)

        # Load into fresh ingestor
        ingestor2 = DeckIngestor()
        ingestor2.load(path)

        assert "TestArch" in ingestor2.archetypes
        arch = ingestor2.archetypes["TestArch"]
        assert arch.primary_color == "Red"
        assert arch.display_card_id == "BT24-017"
        assert len(arch.decklists) == 1
        assert arch.decklists[0].deck_id == "test_001"
        assert arch.decklists[0].card_ids == ["BT24-017"] * 4 + ["P-103"] * 2
        assert arch.decklists[0].is_top_cut is True

    def test_load_merges_without_duplication(self, tmp_path):
        """Loading into pre-populated ingestor merges without duplicating."""
        path = str(tmp_path / "deck_library.json")

        # First save
        ing1 = DeckIngestor()
        ing1.archetypes["A"] = ArchetypeMeta(
            archetype_name="A",
            decklists=[
                IngestedDeck(deck_id="d1", source="test", card_ids=["BT24-017"]),
            ],
        )
        ing1.save(path)

        # Second load + merge
        ing2 = DeckIngestor()
        ing2.archetypes["A"] = ArchetypeMeta(
            archetype_name="A",
            decklists=[
                IngestedDeck(deck_id="d2", source="test", card_ids=["P-103"]),
            ],
        )
        ing2.load(path)

        assert len(ing2.archetypes["A"].decklists) == 2
        ids = {d.deck_id for d in ing2.archetypes["A"].decklists}
        assert ids == {"d1", "d2"}

    def test_report_basic(self):
        """Report generates without errors."""
        ingestor = DeckIngestor()
        ingestor.archetypes["A"] = ArchetypeMeta(
            archetype_name="A",
            stats=ComputedStats(times_played=5, meta_share=1.0, conversion_rate=0.4),
            decklists=[IngestedDeck(deck_id="d1", source="test")],
        )

        report = ingestor.report()
        assert "A" in report
        assert "Deck Library Report" in report

    def test_import_file_text_format(self, tmp_path):
        """Import a text-format deck file."""
        deck_text = """// Test Deck
4 Medusamon BT24-017
4 Koromon BT14-002
42 Agumon ST1-03
5 Koromon ST1-01
"""
        path = tmp_path / "test_deck.txt"
        path.write_text(deck_text)

        ingestor = DeckIngestor()
        count = ingestor.import_file(str(path), archetype="TestArch")

        assert count == 1
        assert "TestArch" in ingestor.archetypes
        deck = ingestor.archetypes["TestArch"].decklists[0]
        assert deck.card_counts["BT24-017"] == 4
        assert deck.card_counts["BT14-002"] == 4
        assert len(deck.card_ids) == 55  # 4+4+42+5


# ─── Egman Parsing ──────────────────────────────────────────────────

class TestEgmanRowParsing:

    def test_parse_egman_row_basic(self):
        """Extracts archetype, placement, player count, date from 8-cell row."""
        from unittest.mock import MagicMock

        # Create mock cells matching real Egman 8-column layout:
        # [icon, archetype, player_name, placement, format, event_type, event_name(players), date]
        cells = []
        for text in ["", "Royal Knights", "AJ", "1st", "BT23", "Large Official Event", "Versus Games Orlando Regional (174)", "12/7/25"]:
            cell = MagicMock()
            cell.get_text.return_value = text
            cell.find.return_value = None
            cell.find_all.return_value = []
            cells.append(cell)

        archetype, placement, event_date, event_players, player_name = (
            DeckIngestor._parse_egman_row(cells)
        )

        assert archetype == "Royal Knights"
        assert placement == "1st"
        assert event_players == 174
        assert event_date == "12/7/25"
        assert player_name == "AJ"

    def test_parse_egman_row_empty(self):
        archetype, placement, event_date, event_players, player_name = (
            DeckIngestor._parse_egman_row([])
        )
        assert archetype is None
        assert placement is None
        assert player_name is None


# ─── DigiLabStats Persistence ────────────────────────────────────────

class TestDigiLabStatsPersistence:

    def test_save_load_roundtrip_with_times_played(self, tmp_path):
        """DigiLabStats.times_played survives save/load roundtrip."""
        path = str(tmp_path / "deck_library.json")

        ingestor = DeckIngestor()
        ingestor.archetypes["TestArch"] = ArchetypeMeta(
            archetype_name="TestArch",
            digilab_stats=DigiLabStats(
                times_played=42,
                conversion_rate=0.25,
                win_rate=0.60,
                top4_rate=0.15,
            ),
            decklists=[
                IngestedDeck(deck_id="d1", source="test", card_ids=["BT24-017"]),
            ],
        )

        ingestor.save(path)

        # Load into fresh ingestor
        ingestor2 = DeckIngestor()
        ingestor2.load(path)

        arch = ingestor2.archetypes["TestArch"]
        assert arch.digilab_stats is not None
        assert arch.digilab_stats.times_played == 42
        assert abs(arch.digilab_stats.conversion_rate - 0.25) < 1e-4
        assert abs(arch.digilab_stats.win_rate - 0.60) < 1e-4
        assert abs(arch.digilab_stats.top4_rate - 0.15) < 1e-4

    def test_times_played_in_json_output(self, tmp_path):
        """Verify times_played appears in the serialized JSON."""
        path = str(tmp_path / "deck_library.json")

        ingestor = DeckIngestor()
        ingestor.archetypes["A"] = ArchetypeMeta(
            archetype_name="A",
            digilab_stats=DigiLabStats(times_played=99, conversion_rate=0.5),
            decklists=[IngestedDeck(deck_id="d1", source="test", card_ids=["BT24-001"])],
        )
        ingestor.save(path)

        import json
        with open(path) as f:
            data = json.load(f)

        digilab = data["archetypes"]["A"]["digilab_stats"]
        assert "times_played" in digilab
        assert digilab["times_played"] == 99


# ─── JP Format Filter ─────────────────────────────────────────────────

class TestJPFormatFilter:

    def test_jp_regions_class_attribute(self):
        """JP_REGIONS is a set with JP, Japan, Korea."""
        assert "JP" in DeckIngestor.JP_REGIONS
        assert "Japan" in DeckIngestor.JP_REGIONS
        assert "Korea" in DeckIngestor.JP_REGIONS

    def test_jp_regions_excludes_western(self):
        """JP_REGIONS should NOT include western regions."""
        assert "USA" not in DeckIngestor.JP_REGIONS
        assert "Europe" not in DeckIngestor.JP_REGIONS


# ─── DigiLab JSON Conversion ──────────────────────────────────────────

class TestDigiLabJsonConversion:
    """Tests for DigiLab structured JSON → flat card ID list conversion."""

    def test_basic_conversion(self):
        """Standard DigiLab decklist JSON converts to flat card IDs."""
        decklist_json = {
            "digimon": [
                {"count": 4, "name": "Impmon (X Antibody)", "set": "BT12", "number": "073"},
                {"count": 2, "name": "Beelzemon", "set": "EX10", "number": "037"},
            ],
            "tamer": [
                {"count": 4, "name": "Ai & Mako", "set": "ST14", "number": "11"},
            ],
            "option": [
                {"count": 3, "name": "Death Slinger", "set": "EX2", "number": "071"},
            ],
            "egg": [
                {"count": 4, "name": "Yaamon", "set": "ST14", "number": "01"},
            ],
        }

        card_ids, card_counts = DeckIngestor._digilab_json_to_card_ids(decklist_json)

        assert card_counts["BT12-073"] == 4
        assert card_counts["EX10-037"] == 2
        assert card_counts["ST14-11"] == 4
        assert card_counts["EX2-071"] == 3
        assert card_counts["ST14-01"] == 4
        assert len(card_ids) == 17  # 4+2+4+3+4

    def test_empty_categories(self):
        """Missing or empty categories are handled gracefully."""
        decklist_json = {
            "digimon": [
                {"count": 4, "name": "Agumon", "set": "BT1", "number": "010"},
            ],
        }

        card_ids, card_counts = DeckIngestor._digilab_json_to_card_ids(decklist_json)

        assert card_counts == {"BT1-010": 4}
        assert len(card_ids) == 4

    def test_empty_decklist(self):
        """Empty JSON returns empty lists."""
        card_ids, card_counts = DeckIngestor._digilab_json_to_card_ids({})
        assert card_ids == []
        assert card_counts == {}

    def test_missing_fields_skipped(self):
        """Entries with missing set/number/count are skipped."""
        decklist_json = {
            "digimon": [
                {"count": 4, "name": "Agumon", "set": "BT1", "number": "010"},
                {"count": 0, "name": "BadCard", "set": "BT1", "number": "999"},  # count=0
                {"name": "NoCount", "set": "BT1", "number": "888"},  # missing count
                {"count": 2, "name": "NoSet", "number": "777"},  # missing set
            ],
        }

        card_ids, card_counts = DeckIngestor._digilab_json_to_card_ids(decklist_json)
        assert card_counts == {"BT1-010": 4}

    def test_promo_and_st_card_ids(self):
        """Promo (P-xxx) and Starter (ST-xx) card IDs format correctly."""
        decklist_json = {
            "tamer": [
                {"count": 2, "name": "Promo Tamer", "set": "P", "number": "077"},
            ],
            "egg": [
                {"count": 4, "name": "Starter Egg", "set": "ST14", "number": "01"},
            ],
        }

        card_ids, card_counts = DeckIngestor._digilab_json_to_card_ids(decklist_json)
        assert "P-077" in card_counts
        assert "ST14-01" in card_counts

    def test_digilab_source_priority(self):
        """DigiLab source has priority 2 (same as egman)."""
        from tools.meta_loader import SOURCE_PRIORITY
        assert SOURCE_PRIORITY["digilab"] == 2
