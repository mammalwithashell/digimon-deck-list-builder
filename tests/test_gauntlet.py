"""Tests for training.gauntlet: MetaDistribution, DeckLibrary, VariantGenerator, MetaGauntlet."""

import json
import os
import tempfile
from collections import Counter

import numpy as np
import pytest

from training.gauntlet import (
    DeckEntry,
    DeckLibrary,
    MetaDistribution,
    MetaGauntlet,
    MetaGauntletEnv,
    VariantGenerator,
    calculate_reward_scale,
    parse_deck_any,
    parse_egman_text,
    parse_egman_url,
    parse_export_code,
)


# ─── Fixtures ────────────────────────────────────────────────────────

SAMPLE_META_CONFIG = {
    "archetypes": {
        "Red Hybrid": {"meta_share": 0.40, "win_rate": 0.55},
        "Blue Flare": {"meta_share": 0.10, "win_rate": 0.48},
        "Yellow Vaccine": {"meta_share": 0.12, "win_rate": 0.52},
        "Rogue": {"meta_share": 0.05, "win_rate": 0.42},
    }
}

# A minimal 50-card deck using ST1 cards (for testing)
MINIMAL_DECK = ["ST1-01"] * 5 + ["ST1-03"] * 45

# A more realistic deck with varying counts for variant generation testing
VARIED_DECK = (
    ["BT24-017"] * 4 +   # 4x core
    ["BT24-018"] * 4 +   # 4x core
    ["BT24-016"] * 4 +   # 4x core
    ["BT24-012"] * 4 +   # 4x core
    ["BT24-089"] * 4 +   # 4x core
    ["BT21-008"] * 4 +   # 4x core
    ["BT21-007"] * 2 +   # 2x flex
    ["BT21-017"] * 3 +   # 3x flex
    ["P-189"] * 2 +      # 2x flex
    ["BT24-008"] * 3 +   # 3x flex
    ["BT24-011"] * 1 +   # 1x flex
    ["P-103"] * 3 +      # 3x flex
    ["LM-027"] * 1 +     # 1x flex
    ["BT21-024"] * 1 +   # 1x flex
    ["BT21-025"] * 2 +   # 2x flex
    ["BT21-081"] * 2 +   # 2x flex
    ["BT24-082"] * 2 +   # 2x flex
    ["BT18-087"] * 2 +   # 2x flex
    ["BT23-005"] * 2     # 2x flex
)  # Total: 50 cards

# Egman Events format deck excerpt
EGMAN_DECK_TEXT = """\
4x BT24-017 Medusamon
4x BT24-018 Styracomon
4 BT24-016 Lamiamon
3x BT21-017 Dimetromon
2x BT21-007 Agumon
"""

# digimoncard.io text format
CARDIO_DECK_TEXT = """\
// DigimonCard.io Deck List
4 Medusamon BT24-017
4 Styracomon BT24-018
4 Lamiamon BT24-016
3 Dimetromon BT21-017
2 Agumon BT21-007
"""

# digimonmeta.com export code (dg= parameter uses 'a' delimiter)
EXPORT_CODE = "4nBT24-017 4nBT24-018 4nBT24-016 3nBT21-017 2nBT21-007"
EXPORT_CODE_NO_SPACES = "4nBT24-0174nBT24-0184nBT24-0163nBT21-0172nBT21-007"
EXPORT_CODE_COMMAS = "4nBT24-017,4nBT24-018,4nBT24-016,3nBT21-017,2nBT21-007"
EXPORT_CODE_A_DELIM = "4nBT24-017a4nBT24-018a4nBT24-016a3nBT21-017a2nBT21-007"

# Real digimonmeta.com deck URL parameter (Royal Knights)
DIGIMONMETA_DG = (
    "4nBT13-007a4nBT13-040a3nBT13-093a4nBT20-083a1nBT12-018a"
    "2nBT13-019a1nBT13-111a2nBT13-030a3nBT13-056a4nBT13-087a"
    "2nBT20-017a2nBT20-056a1nBT10-086a4nBT13-112a2nBT17-018a"
    "1nBT17-077a2nBT20-060a1nBT20-102a3nBT20-091a4nBT13-110a"
    "4nBT20-100"
)

# Egman Events deckbuilder URL format
EGMAN_URL_DECK = "BT21-064:4,EX8-009:4,EX4-006:1,BT24-017:4,ST1-03:2"
EGMAN_FULL_URL = (
    "https://deckbuilder.egmanevents.com/"
    "?deck=BT21-064:4,EX8-009:4,EX4-006:1&type=digimon"
)


# =====================================================================
#  MetaDistribution
# =====================================================================

class TestMetaDistribution:
    """Tests for MetaDistribution class."""

    def test_from_dict(self):
        meta = MetaDistribution.from_dict(SAMPLE_META_CONFIG)
        assert len(meta) == 4
        assert "Red Hybrid" in meta.archetypes

    def test_from_dict_flat(self):
        """Accept flat dict (no 'archetypes' wrapper)."""
        flat = SAMPLE_META_CONFIG["archetypes"]
        meta = MetaDistribution.from_dict(flat)
        assert len(meta) == 4

    def test_from_file(self):
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            json.dump(SAMPLE_META_CONFIG, f)
            f.flush()
            path = f.name
        try:
            meta = MetaDistribution.from_file(path)
            assert len(meta) == 4
        finally:
            os.unlink(path)

    def test_get_weighted_opponent_returns_valid_archetype(self):
        meta = MetaDistribution.from_dict(SAMPLE_META_CONFIG)
        for _ in range(100):
            archetype = meta.get_weighted_opponent()
            assert archetype in meta.archetypes

    def test_weighted_sampling_favors_high_share(self):
        """Red Hybrid at 40% should be sampled more often than Rogue at 5%."""
        meta = MetaDistribution.from_dict(SAMPLE_META_CONFIG)
        counts = Counter(meta.get_weighted_opponent() for _ in range(10000))
        assert counts["Red Hybrid"] > counts["Rogue"]
        # Red Hybrid should be roughly 8x more common than Rogue
        ratio = counts["Red Hybrid"] / max(counts["Rogue"], 1)
        assert ratio > 3  # conservative bound

    def test_get_archetype_stats(self):
        meta = MetaDistribution.from_dict(SAMPLE_META_CONFIG)
        stats = meta.get_archetype_stats("Red Hybrid")
        assert stats.meta_share == 0.40
        assert stats.win_rate == 0.55

    def test_get_archetype_stats_missing_raises(self):
        meta = MetaDistribution.from_dict(SAMPLE_META_CONFIG)
        with pytest.raises(KeyError):
            meta.get_archetype_stats("Nonexistent Archetype")

    def test_calculate_reward_scale(self):
        meta = MetaDistribution.from_dict(SAMPLE_META_CONFIG)
        assert meta.calculate_reward_scale("Red Hybrid") == pytest.approx(1.40)
        assert meta.calculate_reward_scale("Blue Flare") == pytest.approx(1.10)
        assert meta.calculate_reward_scale("Rogue") == pytest.approx(1.05)

    def test_calculate_reward_scale_unknown(self):
        meta = MetaDistribution.from_dict(SAMPLE_META_CONFIG)
        assert meta.calculate_reward_scale("Unknown") == 1.0

    def test_empty_archetypes_raises(self):
        with pytest.raises(ValueError, match="at least one archetype"):
            MetaDistribution({})

    def test_zero_total_share_raises(self):
        with pytest.raises(ValueError, match="Total meta share must be positive"):
            MetaDistribution.from_dict({
                "archetypes": {"A": {"meta_share": 0.0, "win_rate": 0.5}}
            })

    def test_normalization(self):
        """Shares that don't sum to 1.0 should still work."""
        meta = MetaDistribution.from_dict({
            "A": {"meta_share": 2.0, "win_rate": 0.5},
            "B": {"meta_share": 3.0, "win_rate": 0.5},
        })
        counts = Counter(meta.get_weighted_opponent() for _ in range(5000))
        # B should be ~60%, A ~40%
        assert counts["B"] > counts["A"]

    def test_default_win_rate(self):
        """Missing win_rate should default to 0.50."""
        meta = MetaDistribution.from_dict({
            "A": {"meta_share": 0.5},
        })
        stats = meta.get_archetype_stats("A")
        assert stats.win_rate == 0.50

    def test_repr(self):
        meta = MetaDistribution.from_dict(SAMPLE_META_CONFIG)
        assert "4 archetypes" in repr(meta)


# =====================================================================
#  Deck Parsers
# =====================================================================

class TestParseExportCode:
    """Tests for parse_export_code() (digimonmeta.com format)."""

    def test_space_separated(self):
        result = parse_export_code(EXPORT_CODE)
        assert len(result) == 17
        counts = Counter(result)
        assert counts["BT24-017"] == 4
        assert counts["BT21-007"] == 2

    def test_no_spaces(self):
        """Concatenated without delimiters is ambiguous; real format uses 'a'."""
        # The no-delimiter format is inherently ambiguous because digit boundaries
        # overlap (e.g., "...0174n..." — the regex can't split cleanly).
        # The real digimonmeta.com format always uses 'a' as delimiter.
        result = parse_export_code(EXPORT_CODE_NO_SPACES)
        # Just verify it parses without error and produces some cards
        assert len(result) > 0

    def test_comma_separated(self):
        result = parse_export_code(EXPORT_CODE_COMMAS)
        assert len(result) == 17

    def test_single_entry(self):
        result = parse_export_code("4nBT24-017")
        assert result == ["BT24-017"] * 4

    def test_various_set_prefixes(self):
        code = "2nST1-03 1nP-103 3nEX8-037 4nLM-027"
        result = parse_export_code(code)
        counts = Counter(result)
        assert counts["ST1-03"] == 2
        assert counts["P-103"] == 1
        assert counts["EX8-037"] == 3
        assert counts["LM-027"] == 4

    def test_a_delimiter(self):
        """digimonmeta.com uses 'a' as delimiter in dg= parameter."""
        result = parse_export_code(EXPORT_CODE_A_DELIM)
        assert len(result) == 17
        counts = Counter(result)
        assert counts["BT24-017"] == 4
        assert counts["BT21-007"] == 2

    def test_real_digimonmeta_deck(self):
        """Real Royal Knights deck from digimonmeta.com."""
        result = parse_export_code(DIGIMONMETA_DG)
        counts = Counter(result)
        assert counts["BT13-007"] == 4
        assert counts["BT13-040"] == 4
        assert counts["BT13-093"] == 3
        assert sum(counts.values()) == 54  # actual card count from this deck

    def test_invalid_code_raises(self):
        with pytest.raises(ValueError, match="No valid card entries"):
            parse_export_code("not a valid code at all")

    def test_empty_raises(self):
        with pytest.raises(ValueError, match="No valid card entries"):
            parse_export_code("")


class TestParseEgmanUrl:
    """Tests for parse_egman_url() (Egman Events deckbuilder URL format)."""

    def test_deck_parameter_only(self):
        result = parse_egman_url(EGMAN_URL_DECK)
        counts = Counter(result)
        assert counts["BT21-064"] == 4
        assert counts["EX8-009"] == 4
        assert counts["EX4-006"] == 1
        assert counts["BT24-017"] == 4
        assert counts["ST1-03"] == 2
        assert len(result) == 15

    def test_full_url(self):
        result = parse_egman_url(EGMAN_FULL_URL)
        counts = Counter(result)
        assert counts["BT21-064"] == 4
        assert counts["EX8-009"] == 4
        assert counts["EX4-006"] == 1

    def test_invalid_raises(self):
        with pytest.raises(ValueError, match="No valid card entries"):
            parse_egman_url("not a valid url")


class TestParseEgmanText:
    """Tests for parse_egman_text() (Egman Events + digimoncard.io)."""

    def test_egman_format_with_x(self):
        """Egman format: '4x BT24-017 Name'."""
        result = parse_egman_text(EGMAN_DECK_TEXT)
        counts = Counter(result)
        assert counts["BT24-017"] == 4
        assert counts["BT24-018"] == 4
        assert counts["BT24-016"] == 4
        assert counts["BT21-017"] == 3
        assert counts["BT21-007"] == 2

    def test_cardio_format(self):
        """digimoncard.io format: '4 Name BT24-017'."""
        result = parse_egman_text(CARDIO_DECK_TEXT)
        counts = Counter(result)
        assert counts["BT24-017"] == 4
        assert counts["BT24-018"] == 4

    def test_skips_comments(self):
        raw = "# Comment line\n4x BT24-017 Card Name\n// Another comment"
        result = parse_egman_text(raw)
        assert result == ["BT24-017"] * 4

    def test_mixed_formats(self):
        """Both Egman and digimoncard.io lines in same input."""
        raw = "4x BT24-017 Medusamon\n2 Agumon BT21-007"
        result = parse_egman_text(raw)
        counts = Counter(result)
        assert counts["BT24-017"] == 4
        assert counts["BT21-007"] == 2

    def test_empty_raises(self):
        with pytest.raises(ValueError, match="No valid card entries"):
            parse_egman_text("// Just comments\n\n")


class TestParseDeckAny:
    """Tests for parse_deck_any() auto-detection."""

    def test_detects_export_code(self):
        result = parse_deck_any(EXPORT_CODE)
        assert len(result) == 17

    def test_detects_tts_array(self):
        raw = '["BT24-017", "BT24-017", "BT21-007"]'
        result = parse_deck_any(raw)
        assert len(result) == 3

    def test_detects_json_dict(self):
        raw = '{"BT24-017": 4, "BT21-007": 2}'
        result = parse_deck_any(raw)
        counts = Counter(result)
        assert counts["BT24-017"] == 4
        assert counts["BT21-007"] == 2

    def test_detects_text(self):
        result = parse_deck_any(CARDIO_DECK_TEXT)
        assert len(result) == 17

    def test_detects_egman_text(self):
        result = parse_deck_any(EGMAN_DECK_TEXT)
        assert len(result) == 17

    def test_detects_egman_url(self):
        result = parse_deck_any(EGMAN_URL_DECK)
        counts = Counter(result)
        assert counts["BT21-064"] == 4

    def test_detects_egman_full_url(self):
        result = parse_deck_any(EGMAN_FULL_URL)
        counts = Counter(result)
        assert counts["BT21-064"] == 4

    def test_detects_a_delim_export(self):
        result = parse_deck_any(EXPORT_CODE_A_DELIM)
        assert len(result) == 17

    def test_empty_raises(self):
        with pytest.raises(ValueError, match="Empty deck string"):
            parse_deck_any("")

    def test_all_formats_produce_same_counts(self):
        """Export code, Egman text, and cardio text should produce same counts."""
        export_result = parse_deck_any(EXPORT_CODE)
        egman_result = parse_deck_any(EGMAN_DECK_TEXT)
        cardio_result = parse_deck_any(CARDIO_DECK_TEXT)
        assert Counter(export_result) == Counter(egman_result) == Counter(cardio_result)


# =====================================================================
#  DeckLibrary
# =====================================================================

class TestDeckLibrary:
    """Tests for DeckLibrary class."""

    def test_add_and_get_deck(self):
        lib = DeckLibrary()
        lib.add_deck("Red Hybrid", MINIMAL_DECK, name="Test Deck")
        deck = lib.get_deck("Red Hybrid")
        assert Counter(deck) == Counter(MINIMAL_DECK)

    def test_get_deck_missing_archetype_raises(self):
        lib = DeckLibrary()
        with pytest.raises(KeyError, match="No decks registered"):
            lib.get_deck("Nonexistent")

    def test_multiple_decks_per_archetype(self):
        lib = DeckLibrary()
        lib.add_deck("Red Hybrid", MINIMAL_DECK, name="Deck A")
        lib.add_deck("Red Hybrid", MINIMAL_DECK, name="Deck B")
        assert lib.deck_count("Red Hybrid") == 2

    def test_get_all_decks(self):
        lib = DeckLibrary()
        lib.add_deck("Red Hybrid", MINIMAL_DECK, name="Deck A")
        lib.add_deck("Red Hybrid", MINIMAL_DECK, name="Deck B")
        all_decks = lib.get_all_decks("Red Hybrid")
        assert len(all_decks) == 2

    def test_archetypes_property(self):
        lib = DeckLibrary()
        lib.add_deck("Red Hybrid", MINIMAL_DECK)
        lib.add_deck("Blue Flare", MINIMAL_DECK)
        assert set(lib.archetypes) == {"Red Hybrid", "Blue Flare"}

    def test_deck_count(self):
        lib = DeckLibrary()
        lib.add_deck("Red Hybrid", MINIMAL_DECK)
        lib.add_deck("Blue Flare", MINIMAL_DECK)
        lib.add_deck("Blue Flare", MINIMAL_DECK)
        assert lib.deck_count() == 3
        assert lib.deck_count("Blue Flare") == 2
        assert lib.deck_count("Nonexistent") == 0

    def test_add_deck_from_text(self):
        lib = DeckLibrary()
        lib.add_deck_from_text("Red Hybrid", CARDIO_DECK_TEXT)
        deck = lib.get_deck("Red Hybrid")
        assert Counter(deck)["BT24-017"] == 4

    def test_add_deck_from_export_code(self):
        lib = DeckLibrary()
        lib.add_deck_from_export_code("Red Hybrid", EXPORT_CODE)
        deck = lib.get_deck("Red Hybrid")
        assert Counter(deck)["BT24-017"] == 4

    def test_add_deck_from_egman_url(self):
        lib = DeckLibrary()
        lib.add_deck_from_egman_url("Gallantmon X", EGMAN_URL_DECK)
        deck = lib.get_deck("Gallantmon X")
        assert Counter(deck)["BT21-064"] == 4

    def test_add_deck_from_egman_full_url(self):
        lib = DeckLibrary()
        lib.add_deck_from_egman_url("Gallantmon X", EGMAN_FULL_URL)
        deck = lib.get_deck("Gallantmon X")
        assert Counter(deck)["BT21-064"] == 4

    def test_add_deck_from_tts(self):
        lib = DeckLibrary()
        tts = '["BT24-017", "BT24-017", "BT24-017", "BT24-017", "BT21-007"]'
        lib.add_deck_from_tts("Red Hybrid", tts)
        deck = lib.get_deck("Red Hybrid")
        assert Counter(deck)["BT24-017"] == 4

    def test_add_deck_from_json_dict(self):
        """JSON file with {card_id: count} format."""
        data = {"BT24-017": 4, "BT21-007": 2}
        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.json', delete=False
        ) as f:
            json.dump(data, f)
            path = f.name
        try:
            lib = DeckLibrary()
            lib.add_deck_from_json("Red Hybrid", path)
            deck = lib.get_deck("Red Hybrid")
            assert Counter(deck)["BT24-017"] == 4
        finally:
            os.unlink(path)

    def test_add_deck_from_json_wrapper(self):
        """JSON file with wrapper format: {"cards": {...}, "name": "..."}."""
        data = {
            "name": "My Red Hybrid",
            "cards": {"BT24-017": 4, "BT21-007": 2},
        }
        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.json', delete=False
        ) as f:
            json.dump(data, f)
            path = f.name
        try:
            lib = DeckLibrary()
            lib.add_deck_from_json("Red Hybrid", path)
            deck = lib.get_deck("Red Hybrid")
            assert Counter(deck)["BT24-017"] == 4
        finally:
            os.unlink(path)

    def test_add_deck_from_json_array(self):
        """JSON file with flat array format."""
        data = ["BT24-017", "BT24-017", "BT21-007"]
        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.json', delete=False
        ) as f:
            json.dump(data, f)
            path = f.name
        try:
            lib = DeckLibrary()
            lib.add_deck_from_json("Red Hybrid", path)
            deck = lib.get_deck("Red Hybrid")
            assert Counter(deck)["BT24-017"] == 2
        finally:
            os.unlink(path)

    def test_add_deck_from_file_json(self):
        data = {"BT24-017": 4}
        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.json', delete=False
        ) as f:
            json.dump(data, f)
            path = f.name
        try:
            lib = DeckLibrary()
            lib.add_deck_from_file("Red Hybrid", path)
            deck = lib.get_deck("Red Hybrid")
            assert Counter(deck)["BT24-017"] == 4
        finally:
            os.unlink(path)

    def test_add_deck_from_file_text(self):
        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.txt', delete=False
        ) as f:
            f.write(CARDIO_DECK_TEXT)
            path = f.name
        try:
            lib = DeckLibrary()
            lib.add_deck_from_file("Red Hybrid", path)
            deck = lib.get_deck("Red Hybrid")
            assert Counter(deck)["BT24-017"] == 4
        finally:
            os.unlink(path)

    def test_save_and_load(self):
        lib = DeckLibrary()
        lib.add_deck("Red Hybrid", MINIMAL_DECK, name="Test", source="manual")
        lib.add_deck("Blue Flare", MINIMAL_DECK, name="Test2", source="egman")

        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.json', delete=False
        ) as f:
            path = f.name

        try:
            lib.save(path)
            loaded = DeckLibrary.load(path)
            assert set(loaded.archetypes) == {"Red Hybrid", "Blue Flare"}
            assert loaded.deck_count() == 2
            deck = loaded.get_deck("Red Hybrid")
            assert Counter(deck) == Counter(MINIMAL_DECK)
        finally:
            os.unlink(path)

    def test_repr(self):
        lib = DeckLibrary()
        lib.add_deck("Red Hybrid", MINIMAL_DECK)
        lib.add_deck("Blue Flare", MINIMAL_DECK)
        assert "2 archetypes" in repr(lib)
        assert "2 decks" in repr(lib)

    def test_len(self):
        lib = DeckLibrary()
        assert len(lib) == 0
        lib.add_deck("Red Hybrid", MINIMAL_DECK)
        assert len(lib) == 1


class TestDeckEntry:
    """Tests for DeckEntry dataclass."""

    def test_to_dict_and_from_dict_roundtrip(self):
        entry = DeckEntry(
            card_ids=["BT24-017"] * 4 + ["BT21-007"] * 2,
            name="Test Deck",
            source="manual",
        )
        data = entry.to_dict()
        restored = DeckEntry.from_dict(data)
        assert Counter(restored.card_ids) == Counter(entry.card_ids)
        assert restored.name == "Test Deck"
        assert restored.source == "manual"


# =====================================================================
#  VariantGenerator
# =====================================================================

class TestVariantGenerator:
    """Tests for VariantGenerator class."""

    def test_analyze_core_and_flex(self):
        gen = VariantGenerator(core_threshold=4)
        core, flex = gen.analyze(VARIED_DECK)
        # Cards at 4 copies are core
        assert "BT24-017" in core
        assert core["BT24-017"] == 4
        # Cards below 4 copies are flex
        assert "BT21-007" in flex
        assert flex["BT21-007"] == 2

    def test_analyze_preserves_total(self):
        gen = VariantGenerator(core_threshold=4)
        core, flex = gen.analyze(VARIED_DECK)
        core_total = sum(core.values())
        flex_total = sum(flex.values())
        assert core_total + flex_total == len(VARIED_DECK)

    def test_generate_variant_preserves_size(self):
        gen = VariantGenerator(core_threshold=4)
        variant = gen.generate_variant(VARIED_DECK)
        assert len(variant) == len(VARIED_DECK)

    def test_generate_variant_preserves_core(self):
        gen = VariantGenerator(core_threshold=4)
        core, _ = gen.analyze(VARIED_DECK)
        variant = gen.generate_variant(VARIED_DECK, flex_variance=1.0)
        variant_counts = Counter(variant)
        for card_id, count in core.items():
            assert variant_counts[card_id] >= count, (
                f"Core card {card_id} should have at least {count} copies, "
                f"got {variant_counts[card_id]}"
            )

    def test_generate_variant_zero_variance_unchanged(self):
        gen = VariantGenerator(core_threshold=4)
        variant = gen.generate_variant(VARIED_DECK, flex_variance=0.0)
        assert Counter(variant) == Counter(VARIED_DECK)

    def test_generate_variant_with_flex_pool(self):
        gen = VariantGenerator(core_threshold=4)
        pool = ["ST1-03"] * 30  # plenty of replacements
        variant = gen.generate_variant(VARIED_DECK, flex_pool=pool, flex_variance=1.0)
        assert len(variant) == len(VARIED_DECK)
        # Should contain some ST1-03 replacements
        assert "ST1-03" in variant

    def test_all_core_deck(self):
        """A deck where all cards are at 4 copies has no flex slots."""
        all_core = ["BT24-017"] * 4 + ["BT24-018"] * 4 + ["BT24-016"] * 4
        gen = VariantGenerator(core_threshold=4)
        core, flex = gen.analyze(all_core)
        assert len(flex) == 0
        variant = gen.generate_variant(all_core)
        assert Counter(variant) == Counter(all_core)

    def test_custom_threshold(self):
        """Threshold of 3 means 3+ copies are core."""
        gen = VariantGenerator(core_threshold=3)
        deck = ["A-001"] * 3 + ["B-002"] * 2 + ["C-003"] * 1
        core, flex = gen.analyze(deck)
        assert "A-001" in core
        assert "B-002" in flex
        assert "C-003" in flex

    def test_analyze_multiple_consensus(self):
        gen = VariantGenerator(core_threshold=4)
        deck1 = ["A-001"] * 4 + ["B-002"] * 4 + ["C-003"] * 2
        deck2 = ["A-001"] * 4 + ["B-002"] * 3 + ["D-004"] * 3
        consensus_core, flex_pool = gen.analyze_multiple([deck1, deck2])
        # A-001 is 4x in both -> consensus core
        assert "A-001" in consensus_core
        # B-002 is 4x in deck1 but only 3x in deck2 -> NOT consensus core
        assert "B-002" not in consensus_core
        # C-003, D-004, B-002 should all appear in flex pool
        flex_ids = set(flex_pool)
        assert "B-002" in flex_ids
        assert "C-003" in flex_ids
        assert "D-004" in flex_ids

    def test_analyze_multiple_empty(self):
        gen = VariantGenerator()
        core, pool = gen.analyze_multiple([])
        assert core == {}
        assert pool == []


# =====================================================================
#  MetaGauntlet (Orchestrator)
# =====================================================================

class TestMetaGauntlet:
    """Tests for MetaGauntlet orchestrator."""

    def _make_gauntlet(self, use_variants=False):
        meta = MetaDistribution.from_dict(SAMPLE_META_CONFIG)
        lib = DeckLibrary()
        for archetype in meta.archetypes:
            lib.add_deck(archetype, MINIMAL_DECK, name=f"{archetype} deck")
        return MetaGauntlet(meta, lib, use_variants=use_variants)

    def test_sample_opponent(self):
        gauntlet = self._make_gauntlet()
        archetype, deck = gauntlet.sample_opponent()
        assert archetype in SAMPLE_META_CONFIG["archetypes"]
        assert len(deck) == len(MINIMAL_DECK)

    def test_reward_scale(self):
        gauntlet = self._make_gauntlet()
        scale = gauntlet.reward_scale("Red Hybrid")
        assert scale == pytest.approx(1.40)

    def test_reward_scale_unknown(self):
        gauntlet = self._make_gauntlet()
        assert gauntlet.reward_scale("Unknown") == 1.0

    def test_with_variants(self):
        meta = MetaDistribution.from_dict(SAMPLE_META_CONFIG)
        lib = DeckLibrary()
        for archetype in meta.archetypes:
            lib.add_deck(archetype, VARIED_DECK)
        gauntlet = MetaGauntlet(meta, lib, use_variants=True)
        archetype, deck = gauntlet.sample_opponent()
        assert len(deck) == len(VARIED_DECK)

    def test_warns_missing_archetype(self):
        meta = MetaDistribution.from_dict({
            "Missing": {"meta_share": 1.0, "win_rate": 0.5}
        })
        lib = DeckLibrary()  # empty library
        with pytest.warns(UserWarning, match="has no decks in the library"):
            MetaGauntlet(meta, lib, use_variants=False)


# =====================================================================
#  calculate_reward_scale (standalone)
# =====================================================================

class TestCalculateRewardScale:
    """Tests for the standalone calculate_reward_scale function."""

    def test_high_share(self):
        meta = MetaDistribution.from_dict(SAMPLE_META_CONFIG)
        assert calculate_reward_scale("Red Hybrid", meta) == pytest.approx(1.40)

    def test_low_share(self):
        meta = MetaDistribution.from_dict(SAMPLE_META_CONFIG)
        assert calculate_reward_scale("Rogue", meta) == pytest.approx(1.05)

    def test_unknown_archetype(self):
        meta = MetaDistribution.from_dict(SAMPLE_META_CONFIG)
        assert calculate_reward_scale("Unknown", meta) == 1.0


# =====================================================================
#  MetaGauntletEnv
# =====================================================================

class TestMetaGauntletEnv:
    """Tests for the MetaGauntletEnv Gymnasium wrapper."""

    def _make_env(self):
        from digimon_gym.digimon_gym import DigimonEnv
        meta = MetaDistribution.from_dict(SAMPLE_META_CONFIG)
        lib = DeckLibrary()
        for archetype in meta.archetypes:
            lib.add_deck(archetype, MINIMAL_DECK)
        gauntlet = MetaGauntlet(meta, lib, use_variants=False)
        base_env = DigimonEnv(deck1=MINIMAL_DECK)
        return MetaGauntletEnv(base_env, gauntlet)

    def test_reset_returns_obs_and_info(self):
        env = self._make_env()
        obs, info = env.reset()
        assert obs.shape == (981,)
        assert "action_mask" in info
        assert "opponent_archetype" in info
        assert "reward_scale" in info

    def test_reset_sets_archetype(self):
        env = self._make_env()
        obs, info = env.reset()
        assert info["opponent_archetype"] in SAMPLE_META_CONFIG["archetypes"]

    def test_reset_sets_reward_scale(self):
        env = self._make_env()
        obs, info = env.reset()
        archetype = info["opponent_archetype"]
        expected_share = SAMPLE_META_CONFIG["archetypes"][archetype]["meta_share"]
        assert info["reward_scale"] == pytest.approx(1.0 + expected_share)

    def test_step_returns_archetype_info(self):
        env = self._make_env()
        obs, info = env.reset()
        mask = info["action_mask"]
        valid_actions = np.where(mask > 0)[0]
        if len(valid_actions) > 0:
            obs, reward, terminated, truncated, info = env.step(int(valid_actions[0]))
            assert "opponent_archetype" in info

    def test_current_archetype_property(self):
        env = self._make_env()
        assert env.current_archetype is None
        env.reset()
        assert env.current_archetype is not None
