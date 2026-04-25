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


class TestRestrictedList:
    def test_restricted_list_shape(self):
        from digimon_engine import restricted_list
        rl = restricted_list()
        assert hasattr(rl, "card_limits")
        assert hasattr(rl, "choice_groups")
        assert isinstance(rl.card_limits, dict)
        assert isinstance(rl.choice_groups, list)

    def test_restricted_list_known_entries(self):
        from digimon_engine import restricted_list
        rl = restricted_list()
        # Known banned entries from the official ENG list
        assert rl.card_limits.get("BT2-090") == 0  # Matt Ishida (banned)
        # Known restricted-to-1 entries
        assert rl.card_limits.get("BT1-090") == 1
        # Choice groups exist
        assert len(rl.choice_groups) >= 1


class TestExpandDeckDict:
    def test_expand_basic(self):
        from digimon_engine import expand_deck_dict
        out = expand_deck_dict({"BT1-001": 3, "BT1-002": 1})
        assert sorted(out) == sorted(["BT1-001", "BT1-001", "BT1-001", "BT1-002"])

    def test_expand_empty(self):
        from digimon_engine import expand_deck_dict
        assert expand_deck_dict({}) == []

    def test_expand_zero_count(self):
        from digimon_engine import expand_deck_dict
        out = expand_deck_dict({"BT1-001": 0})
        assert out == []


class TestEnums:
    def test_card_kind_variants_distinct(self):
        from digimon_engine import CardKind
        assert CardKind.Digimon != CardKind.Tamer
        assert CardKind.Option != CardKind.DigiEgg
        assert CardKind.Token != CardKind.Digimon

    def test_card_kind_pycard_returns_typed(self):
        from digimon_engine import CardDatabase, CardKind
        db = CardDatabase()
        # BT1-001 is a DigiEgg (Yokomon's egg form)
        card = db.get_card("BT1-001")
        assert card is not None
        assert card.card_kind == CardKind.DigiEgg
        # Sanity: also check a known non-egg card if present in db
        agumon = db.get_card("BT1-010")
        if agumon is not None:
            # BT1-010 is Agumon (Rookie Digimon)
            assert agumon.card_kind == CardKind.Digimon

    def test_game_phase_python_names_present(self):
        from digimon_engine import GamePhase
        # Python-side names callers will use post-Phase 3 cutover
        assert GamePhase.Start is not None
        assert GamePhase.End is not None
        assert GamePhase.SelectEffectChoice is not None
        assert GamePhase.Mulligan is not None
        assert GamePhase.SelectTarget is not None

    def test_game_phase_pairwise_distinct(self):
        from digimon_engine import GamePhase
        # Spot-check that representative variants are not aliases of one another.
        assert GamePhase.Start != GamePhase.Draw
        assert GamePhase.Main != GamePhase.End
        assert GamePhase.SelectTarget != GamePhase.SelectMaterial
        assert GamePhase.BlockTiming != GamePhase.CounterTiming
        assert GamePhase.SelectEffectChoice != GamePhase.SelectTarget


class TestTestedCards:
    def test_load_tested_cards_returns_set(self):
        from digimon_engine import load_tested_cards
        tested = load_tested_cards()
        assert isinstance(tested, set)
        # The alpha allowlist is non-trivial
        assert len(tested) > 100

    def test_is_card_tested_unknown_returns_false(self):
        from digimon_engine import is_card_tested
        assert is_card_tested("ZZ99-999") is False

    def test_is_card_tested_consistent_with_set(self):
        from digimon_engine import is_card_tested, load_tested_cards
        tested = load_tested_cards()
        sample = next(iter(tested))
        assert is_card_tested(sample) is True


class TestCardRegistry:
    def test_capacity_constant(self):
        from digimon_engine import REGISTRY_CAPACITY
        assert REGISTRY_CAPACITY == 20_000

    def test_embedding_dim_constant(self):
        from digimon_engine import EMBEDDING_DIM
        assert EMBEDDING_DIM == 16

    def test_construct(self):
        from digimon_engine import CardRegistry
        reg = CardRegistry()
        # The registry should at minimum have some entries
        assert reg.count() > 0

    def test_index_of_known_card(self):
        from digimon_engine import CardRegistry
        reg = CardRegistry()
        idx = reg.index_of("BT1-001")
        assert idx is not None
        assert idx > 0

    def test_index_of_unknown_card_returns_none(self):
        from digimon_engine import CardRegistry
        reg = CardRegistry()
        assert reg.index_of("ZZ99-999") is None

    def test_id_of_round_trips(self):
        from digimon_engine import CardRegistry
        reg = CardRegistry()
        idx = reg.index_of("BT1-001")
        assert idx is not None
        assert reg.id_of(idx) == "BT1-001"

    def test_norm_id_within_unit_interval(self):
        from digimon_engine import CardRegistry, REGISTRY_CAPACITY
        reg = CardRegistry()
        norm = reg.norm_id_of("BT1-001")
        assert 0.0 < norm <= 1.0
        assert abs(norm * REGISTRY_CAPACITY - reg.index_of("BT1-001")) < 0.5
