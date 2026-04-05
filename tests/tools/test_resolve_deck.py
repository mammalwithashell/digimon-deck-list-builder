"""Tests for tools/resolve_deck.py."""

import pytest
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))


class TestResolveCards:
    """Tests for resolve_cards() — enriching raw card IDs."""

    def test_single_frozen_card(self):
        """A card in the frozen manifest should have script_status='frozen'."""
        from tools.resolve_deck import resolve_cards

        entries = resolve_cards(["BT10-001"])
        assert len(entries) == 1
        entry = entries[0]
        assert entry.card_id == "BT10-001"
        assert entry.script_status == "frozen"
        assert entry.script_path is not None
        assert entry.card_name != ""
        assert entry.card_kind in ("Digimon", "Tamer", "Option", "DigiEgg")

    def test_missing_card(self):
        """A card ID not in manifest or on disk should be 'missing'."""
        from tools.resolve_deck import resolve_cards

        entries = resolve_cards(["ZZ99-999"])
        assert len(entries) == 1
        assert entries[0].script_status == "missing"
        assert entries[0].script_path is None
        assert entries[0].card_id == "ZZ99-999"

    def test_multiple_cards_sorted(self):
        """Multiple cards should be returned sorted by card_id."""
        from tools.resolve_deck import resolve_cards

        entries = resolve_cards(["BT10-003", "BT10-001", "BT10-002"])
        ids = [e.card_id for e in entries]
        assert ids == ["BT10-001", "BT10-002", "BT10-003"]

    def test_card_metadata_fields(self):
        """Card entries should have populated metadata fields from cards.json."""
        from tools.resolve_deck import resolve_cards

        entries = resolve_cards(["BT10-001"])
        entry = entries[0]
        assert isinstance(entry.colors, list)
        assert isinstance(entry.play_cost, int) or entry.play_cost is None
        assert isinstance(entry.effect_text, str)
        assert isinstance(entry.inherited_text, str)
        assert isinstance(entry.deck_frequency, int)
        assert entry.deck_frequency == 0  # no archetype context

    def test_deduplicates_input(self):
        """Duplicate card IDs in input should produce one entry."""
        from tools.resolve_deck import resolve_cards

        entries = resolve_cards(["BT10-001", "BT10-001", "BT10-001"])
        assert len(entries) == 1
