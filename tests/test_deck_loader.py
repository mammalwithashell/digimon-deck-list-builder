"""Tests for deck_loader: TTS/text parsing, auto-detect, validation, restricted list."""

import json
import pytest
from collections import Counter

from digimon_gym.engine.data.deck_loader import (
    RE_CARD_ID,
    CardRestriction,
    DeckValidationResult,
    RESTRICTED_LIST,
    expand_deck_dict,
    parse_deck,
    parse_text,
    parse_tts,
    summarize_deck,
    validate_deck,
)
from digimon_gym.engine.data.card_database import CardDatabase


# ─── Fixtures ────────────────────────────────────────────────────────

# The full Medusamon TTS export from the user's example
MEDUSAMON_TTS = json.dumps([
    "Exported from https://digimoncard.io",
    "BT23-005", "BT23-005",
    "BT21-007", "BT21-007",
    "BT21-008", "BT21-008", "BT21-008", "BT21-008",
    "BT24-008", "BT24-008", "BT24-008",
    "BT24-012", "BT24-012", "BT24-012", "BT24-012",
    "BT21-017", "BT21-017", "BT21-017",
    "BT24-011",
    "P-189", "P-189",
    "BT21-024",
    "BT24-016", "BT24-016", "BT24-016", "BT24-016",
    "BT21-025", "BT21-025",
    "BT24-017", "BT24-017", "BT24-017", "BT24-017",
    "BT24-018", "BT24-018", "BT24-018", "BT24-018",
    "P-103", "P-103", "P-103",
    "LM-027",
    "BT24-089", "BT24-089", "BT24-089", "BT24-089",
    "BT21-081", "BT21-081",
    "BT24-082", "BT24-082",
    "BT18-087", "BT18-087",
])

MEDUSAMON_TEXT = """\
// DigimonCard.io Deck List
2 Agumon BT21-007
1 Cyberdramon BT21-024
1 Cyclonemon BT24-011
4 Dimetromon BT24-012
3 Dimetromon BT21-017
2 Dimetromon P-189
2 Elizamon BT23-005
4 Elizamon BT21-008
3 Elizamon BT24-008
4 Lamiamon BT24-016
2 Lamiamon BT21-025
4 Medusamon BT24-017
3 Offense Training P-103
2 Owen Dreadnought BT21-081
2 Owen Dreadnought BT24-082
2 Owen Dreadnought BT18-087
1 Red Scramble LM-027
4 Styracomon BT24-018
4 Unique Emblem: Blazing Conductor BT24-089
"""


# ─── Card ID regex ───────────────────────────────────────────────────

class TestCardIdRegex:
    """Tests for the RE_CARD_ID pattern."""

    @pytest.mark.parametrize("card_id", [
        "BT24-017", "BT1-001", "ST1-01", "P-103", "P-008",
        "LM-027", "EX8-037", "EX10-001", "RB1-001",
    ])
    def test_valid_card_ids(self, card_id):
        assert RE_CARD_ID.match(card_id), f"{card_id} should match"

    @pytest.mark.parametrize("non_card", [
        "Exported from https://digimoncard.io",
        "// DigimonCard.io Deck List",
        "hello",
        "",
        "123",
        "bt24-017",  # lowercase
    ])
    def test_non_card_strings(self, non_card):
        assert not RE_CARD_ID.match(non_card), f"{non_card!r} should not match"


# ─── TTS Parser ──────────────────────────────────────────────────────

class TestParseTts:
    """Tests for parse_tts()."""

    def test_simple_array(self):
        raw = '["BT24-017", "BT24-017", "BT24-017"]'
        result = parse_tts(raw)
        assert result == ["BT24-017", "BT24-017", "BT24-017"]

    def test_filters_exported_header(self):
        raw = '["Exported from https://digimoncard.io", "BT24-017"]'
        result = parse_tts(raw)
        assert result == ["BT24-017"]

    def test_empty_array(self):
        result = parse_tts("[]")
        assert result == []

    def test_mixed_valid_invalid(self):
        raw = '["BT24-017", "not-a-card", "P-103", 42, "LM-027"]'
        result = parse_tts(raw)
        assert result == ["BT24-017", "P-103", "LM-027"]

    def test_invalid_json(self):
        with pytest.raises(ValueError, match="Invalid TTS JSON"):
            parse_tts("not json at all")

    def test_not_a_list(self):
        with pytest.raises(ValueError, match="expects a JSON array"):
            parse_tts('{"key": "value"}')

    def test_medusamon_deck(self):
        """Parse the full Medusamon TTS export."""
        result = parse_tts(MEDUSAMON_TTS)
        # 50 cards total (the "Exported from..." header is filtered out)
        assert len(result) == 50
        counts = Counter(result)
        assert counts["BT24-017"] == 4  # 4x Medusamon
        assert counts["BT24-018"] == 4  # 4x Styracomon
        assert counts["LM-027"] == 1    # 1x Red Scramble
        assert counts["P-103"] == 3     # 3x Offense Training


# ─── Text Parser ─────────────────────────────────────────────────────

class TestParseText:
    """Tests for parse_text()."""

    def test_simple_lines(self):
        raw = "4 Medusamon BT24-017\n2 Agumon BT21-007"
        result = parse_text(raw)
        assert len(result) == 6
        counts = Counter(result)
        assert counts["BT24-017"] == 4
        assert counts["BT21-007"] == 2

    def test_skips_comments(self):
        raw = "// DigimonCard.io Deck List\n4 Medusamon BT24-017"
        result = parse_text(raw)
        assert result == ["BT24-017"] * 4

    def test_skips_blank_lines(self):
        raw = "// Deck Name\n\n4 Medusamon BT24-017\n\n2 Agumon BT21-007\n"
        result = parse_text(raw)
        assert len(result) == 6

    def test_single_word_name(self):
        raw = "3 Agumon BT21-007"
        result = parse_text(raw)
        assert result == ["BT21-007"] * 3

    def test_multi_word_name(self):
        raw = "4 Unique Emblem: Blazing Conductor BT24-089"
        result = parse_text(raw)
        assert result == ["BT24-089"] * 4

    def test_no_valid_lines(self):
        with pytest.raises(ValueError, match="No valid card entries"):
            parse_text("// Just a comment\n\n")

    def test_medusamon_deck(self):
        """Parse the full Medusamon text export."""
        result = parse_text(MEDUSAMON_TEXT)
        assert len(result) == 50
        counts = Counter(result)
        assert counts["BT24-017"] == 4  # 4x Medusamon
        assert counts["BT24-018"] == 4  # 4x Styracomon
        assert counts["LM-027"] == 1    # 1x Red Scramble
        assert counts["P-103"] == 3     # 3x Offense Training

    def test_handles_extra_whitespace(self):
        raw = "  4   Medusamon   BT24-017  "
        result = parse_text(raw)
        assert result == ["BT24-017"] * 4


# ─── Auto-Detect Parser ─────────────────────────────────────────────

class TestParseDeck:
    """Tests for parse_deck() auto-detection."""

    def test_detects_tts(self):
        raw = '["BT24-017", "BT24-017"]'
        result = parse_deck(raw)
        assert result == ["BT24-017", "BT24-017"]

    def test_detects_text(self):
        raw = "2 Medusamon BT24-017"
        result = parse_deck(raw)
        assert result == ["BT24-017", "BT24-017"]

    def test_detects_tts_with_header(self):
        raw = '["Exported from https://digimoncard.io", "BT24-017"]'
        result = parse_deck(raw)
        assert result == ["BT24-017"]

    def test_empty_string(self):
        with pytest.raises(ValueError, match="Empty deck string"):
            parse_deck("")

    def test_garbage_input(self):
        with pytest.raises(ValueError, match="Could not parse"):
            parse_deck("this is just random garbage text with no card ids")

    def test_medusamon_tts(self):
        result = parse_deck(MEDUSAMON_TTS)
        assert len(result) == 50

    def test_medusamon_text(self):
        result = parse_deck(MEDUSAMON_TEXT)
        assert len(result) == 50

    def test_tts_and_text_produce_same_cards(self):
        """Both formats of the same deck should produce the same card counts."""
        tts_result = parse_deck(MEDUSAMON_TTS)
        text_result = parse_deck(MEDUSAMON_TEXT)
        assert Counter(tts_result) == Counter(text_result)


# ─── Utility Functions ───────────────────────────────────────────────

class TestExpandDeckDict:
    """Tests for expand_deck_dict()."""

    def test_basic_expansion(self):
        result = expand_deck_dict({"BT24-017": 4, "P-103": 2})
        assert Counter(result) == Counter({"BT24-017": 4, "P-103": 2})
        assert len(result) == 6

    def test_empty_dict(self):
        assert expand_deck_dict({}) == []

    def test_single_copy(self):
        result = expand_deck_dict({"LM-027": 1})
        assert result == ["LM-027"]


class TestSummarizeDeck:
    """Tests for summarize_deck()."""

    def test_basic_summary(self):
        card_ids = ["BT24-017"] * 4 + ["P-103"] * 2
        result = summarize_deck(card_ids)
        assert result == {"BT24-017": 4, "P-103": 2}

    def test_empty_list(self):
        assert summarize_deck([]) == {}


# ─── Restricted List Data ────────────────────────────────────────────

class TestRestrictedListData:
    """Tests that the restricted list data is correct."""

    def test_banned_cards_count(self):
        banned = {k: v for k, v in RESTRICTED_LIST.card_limits.items() if v == 0}
        assert len(banned) == 3

    def test_banned_card_ids(self):
        for card_id in ["BT2-090", "BT5-109", "EX5-065"]:
            assert RESTRICTED_LIST.card_limits.get(card_id) == 0, \
                f"{card_id} should be banned (limit 0)"

    def test_restricted_cards_count(self):
        restricted = {k: v for k, v in RESTRICTED_LIST.card_limits.items() if v == 1}
        assert len(restricted) == 47

    def test_some_restricted_cards(self):
        """Spot-check a few restricted cards."""
        for card_id in ["BT1-090", "P-008", "ST2-13", "EX1-068", "BT14-002"]:
            assert RESTRICTED_LIST.card_limits.get(card_id) == 1, \
                f"{card_id} should be restricted (limit 1)"

    def test_choice_groups_count(self):
        assert len(RESTRICTED_LIST.choice_groups) == 2

    def test_choice_group_1(self):
        group_a, group_b = RESTRICTED_LIST.choice_groups[0]
        assert "EX2-007" in group_a
        assert "EX7-064" in group_b

    def test_choice_group_2(self):
        group_a, group_b = RESTRICTED_LIST.choice_groups[1]
        assert "BT20-037" in group_a
        assert "BT17-035" in group_b
        assert "EX8-037" in group_b

    def test_total_card_limits(self):
        """3 banned + 47 restricted = 50 total."""
        assert len(RESTRICTED_LIST.card_limits) == 50


# ─── Deck Validation ─────────────────────────────────────────────────

def _make_valid_deck() -> list:
    """Create a valid 50+5 deck from ST1 cards for testing.

    Uses diverse cards respecting max 4 copies per card:
      4x ST1-01 (egg)  = 4 eggs
      + 13 different main deck cards x ~4 copies = 50 main
    """
    eggs = ["ST1-01"] * 4
    main = (
        ["ST1-02"] * 4 + ["ST1-03"] * 4 + ["ST1-04"] * 4 +
        ["ST1-05"] * 4 + ["ST1-06"] * 4 + ["ST1-07"] * 4 +
        ["ST1-08"] * 4 + ["ST1-09"] * 4 + ["ST1-10"] * 4 +
        ["ST1-11"] * 4 + ["ST1-12"] * 4 + ["ST1-13"] * 4 +
        ["ST1-14"] * 2  # 12*4 + 2 = 50
    )
    assert len(main) == 50
    return eggs + main


class TestValidateDeck:
    """Tests for validate_deck()."""

    def test_valid_deck(self):
        deck = _make_valid_deck()
        result = validate_deck(deck)
        assert result.is_valid
        assert result.errors == []

    def test_main_deck_too_small(self):
        deck = ["ST1-01"] * 5 + ["ST1-03"] * 40
        result = validate_deck(deck)
        assert not result.is_valid
        assert any("Main deck must be exactly 50" in e for e in result.errors)

    def test_main_deck_too_large(self):
        deck = ["ST1-01"] * 5 + ["ST1-03"] * 55
        result = validate_deck(deck)
        assert not result.is_valid
        assert any("Main deck must be exactly 50" in e for e in result.errors)

    def test_too_many_eggs(self):
        deck = ["ST1-01"] * 6 + ["ST1-03"] * 50
        result = validate_deck(deck)
        assert not result.is_valid
        assert any("Digi-Egg deck must be 0-5" in e for e in result.errors)

    def test_no_eggs_is_valid(self):
        """A deck with 0 eggs and 50 main cards is valid."""
        deck = (
            ["ST1-02"] * 4 + ["ST1-03"] * 4 + ["ST1-04"] * 4 +
            ["ST1-05"] * 4 + ["ST1-06"] * 4 + ["ST1-07"] * 4 +
            ["ST1-08"] * 4 + ["ST1-09"] * 4 + ["ST1-10"] * 4 +
            ["ST1-11"] * 4 + ["ST1-12"] * 4 + ["ST1-13"] * 4 +
            ["ST1-14"] * 2  # 12*4 + 2 = 50
        )
        result = validate_deck(deck)
        assert result.is_valid

    def test_per_card_copy_limit(self):
        """More than max_count_in_deck copies should error."""
        # ST1-03 has max_count_in_deck=4, so 5 copies should fail
        deck = (
            ["ST1-01"] * 4 +  # 4 eggs
            ["ST1-03"] * 5 +  # 5 copies — over the limit!
            ["ST1-04"] * 4 + ["ST1-05"] * 4 + ["ST1-06"] * 4 +
            ["ST1-07"] * 4 + ["ST1-08"] * 4 + ["ST1-09"] * 4 +
            ["ST1-10"] * 4 + ["ST1-11"] * 4 + ["ST1-12"] * 4 +
            ["ST1-13"] * 4 + ["ST1-14"] * 4 + ["ST1-15"] * 1  # = 50 main
        )
        result = validate_deck(deck)
        assert not result.is_valid
        assert any("5 copies exceeds max 4" in e for e in result.errors)

    def test_unknown_card_is_warning(self):
        """Unknown card IDs produce warnings, not errors."""
        deck = ["ST1-01"] * 5 + ["ST1-03"] * 49 + ["FAKE-999"]
        result = validate_deck(deck)
        assert any("Unknown card ID: FAKE-999" in w for w in result.warnings)

    def test_banned_card(self):
        """Including a banned card should produce an error."""
        # Use a custom restricted list to avoid needing the banned card in DB
        restriction = CardRestriction(
            card_limits={"ST1-03": 0},
            choice_groups=[],
        )
        deck = ["ST1-01"] * 5 + ["ST1-03"] * 50
        result = validate_deck(deck, restricted_list=restriction)
        assert not result.is_valid
        assert any("banned" in e.lower() for e in result.errors)

    def test_restricted_card_over_limit(self):
        """Restricted card at 2+ copies should error."""
        restriction = CardRestriction(
            card_limits={"ST1-03": 1},
            choice_groups=[],
        )
        deck = ["ST1-01"] * 5 + ["ST1-03"] * 2 + ["ST1-07"] * 48
        result = validate_deck(deck, restricted_list=restriction)
        assert not result.is_valid
        assert any("restricted limit of 1" in e for e in result.errors)

    def test_restricted_card_at_limit_is_valid(self):
        """Restricted card at exactly 1 copy should be fine."""
        restriction = CardRestriction(
            card_limits={"ST1-03": 1},
            choice_groups=[],
        )
        deck = ["ST1-01"] * 5 + ["ST1-03"] * 1 + ["ST1-07"] * 49
        result = validate_deck(deck, restricted_list=restriction)
        # Should not have restricted-list errors for ST1-03
        restricted_errors = [e for e in result.errors if "restricted" in e.lower()]
        assert restricted_errors == []

    def test_choice_group_violation(self):
        """Including cards from both sides of a choice group should error."""
        restriction = CardRestriction(
            card_limits={},
            choice_groups=[(["ST1-03"], ["ST1-07"])],
        )
        deck = ["ST1-01"] * 5 + ["ST1-03"] * 25 + ["ST1-07"] * 25
        result = validate_deck(deck, restricted_list=restriction)
        assert not result.is_valid
        assert any("Choice restriction violated" in e for e in result.errors)

    def test_choice_group_one_side_only_is_valid(self):
        """Including cards from only one side of a choice group is fine."""
        restriction = CardRestriction(
            card_limits={},
            choice_groups=[(["ST1-03"], ["ST1-07"])],
        )
        deck = ["ST1-01"] * 5 + ["ST1-03"] * 50
        result = validate_deck(deck, restricted_list=restriction)
        choice_errors = [e for e in result.errors if "Choice restriction" in e]
        assert choice_errors == []

    def test_no_restricted_list(self):
        """Passing an empty restricted list skips those checks."""
        no_restrictions = CardRestriction(card_limits={}, choice_groups=[])
        deck = _make_valid_deck()
        result = validate_deck(deck, restricted_list=no_restrictions)
        assert result.is_valid


# ─── Integration: Full Medusamon Deck ────────────────────────────────

class TestMedusaDeckIntegration:
    """Integration tests parsing and validating the full Medusamon deck."""

    def test_tts_parse_and_validate(self):
        card_ids = parse_tts(MEDUSAMON_TTS)
        assert len(card_ids) == 50
        result = validate_deck(card_ids)
        # Deck may have warnings for unknown cards (not all sets in DB)
        # but should not violate restricted list rules
        restricted_errors = [
            e for e in result.errors
            if "banned" in e.lower() or "restricted" in e.lower() or "choice" in e.lower()
        ]
        assert restricted_errors == [], f"Restricted list errors: {restricted_errors}"

    def test_text_parse_and_validate(self):
        card_ids = parse_text(MEDUSAMON_TEXT)
        assert len(card_ids) == 50
        result = validate_deck(card_ids)
        restricted_errors = [
            e for e in result.errors
            if "banned" in e.lower() or "restricted" in e.lower() or "choice" in e.lower()
        ]
        assert restricted_errors == [], f"Restricted list errors: {restricted_errors}"

    def test_formats_produce_identical_counts(self):
        tts_ids = parse_tts(MEDUSAMON_TTS)
        text_ids = parse_text(MEDUSAMON_TEXT)
        assert Counter(tts_ids) == Counter(text_ids)
