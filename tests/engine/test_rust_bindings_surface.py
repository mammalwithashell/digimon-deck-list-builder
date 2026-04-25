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


class TestCardDatabase:
    def test_construct(self):
        from digimon_engine import CardDatabase
        db = CardDatabase()
        assert db is not None

    def test_get_known_card(self):
        from digimon_engine import CardDatabase
        db = CardDatabase()
        card = db.get_card("BT1-001")
        assert card is not None
        # BT1-001 is Yokomon (a Rookie). The card_name field is whatever
        # cards.json carries; just verify it's a non-empty string.
        assert isinstance(card.card_name, str)
        assert len(card.card_name) > 0
        # Rookie level is 3 in this game.
        assert card.level is not None

    def test_get_unknown_card_returns_none(self):
        from digimon_engine import CardDatabase
        db = CardDatabase()
        assert db.get_card("ZZ99-999") is None

    def test_count_cards(self):
        from digimon_engine import CardDatabase
        db = CardDatabase()
        # Whole-database count should be in the thousands
        assert db.count() > 1000

    def test_card_id_field(self):
        from digimon_engine import CardDatabase
        db = CardDatabase()
        card = db.get_card("BT1-001")
        assert card is not None
        assert card.card_id == "BT1-001"


class TestDeckTools:
    def test_parse_tts_simple(self):
        from digimon_engine import parse_tts
        # TTS format is a JSON array of card-id strings.
        ids = parse_tts('["BT1-001", "BT1-002", "BT1-002"]')
        assert ids == ["BT1-001", "BT1-002", "BT1-002"]

    def test_parse_text_basic(self):
        from digimon_engine import parse_text
        # digimoncard.io text format: "<count> <name> <card_id>"
        ids = parse_text("1 Yokomon BT1-001\n2 Sukamon BT1-002")
        assert ids == ["BT1-001", "BT1-002", "BT1-002"]

    def test_parse_deck_dispatches_tts(self):
        from digimon_engine import parse_deck
        ids = parse_deck('["BT1-001"]')
        assert ids == ["BT1-001"]

    def test_parse_deck_dispatches_text(self):
        from digimon_engine import parse_deck
        ids = parse_deck("1 Yokomon BT1-001")
        assert ids == ["BT1-001"]

    def test_summarize_deck(self):
        from digimon_engine import summarize_deck
        summary = summarize_deck(["BT1-001", "BT1-001", "BT1-002"])
        assert summary["BT1-001"] == 2
        assert summary["BT1-002"] == 1

    def test_validate_deck_invalid_too_many_copies(self):
        from digimon_engine import validate_deck
        # 50 copies of one card violates the 4-copy limit
        result = validate_deck(["BT1-001"] * 50)
        assert hasattr(result, "is_valid")
        assert result.is_valid is False
        assert isinstance(result.errors, list)
        assert isinstance(result.warnings, list)

    def test_out_of_set_cards_returns_unknowns(self):
        from digimon_engine import out_of_set_cards
        # out_of_set_cards filters out cards in the tested-cards allowlist.
        # We don't know which IDs are tested at test-write time, but a
        # made-up ID definitely isn't, so it must appear in the output.
        bad = out_of_set_cards(["ZZ99-999"])
        assert "ZZ99-999" in bad

    def test_out_of_set_cards_dedupe(self):
        from digimon_engine import out_of_set_cards
        # Duplicates collapse — matches Python's first-seen semantics.
        bad = out_of_set_cards(["ZZ99-999", "ZZ99-999"])
        assert bad.count("ZZ99-999") == 1
