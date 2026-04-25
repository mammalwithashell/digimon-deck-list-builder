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
