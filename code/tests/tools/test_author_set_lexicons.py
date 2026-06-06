"""Tests for trait + card-name lexicons (tasks 2.2 / 2.4)."""

from tools.author_set.lexicons import (
    build_lexicons,
    build_name_lexicon,
    build_trait_lexicon,
)

SYNTH = [
    {"card_id": "A-1", "card_name_eng": "Agumon", "type_eng": ["Dinosaur", "ADVENTURE"]},
    {"card_id": "A-2", "card_name_eng": "Greymon", "type_eng": ["Dinosaur"]},
    {"card_id": "A-3", "card_name_eng": "Omnimon", "type_eng": ["Holy Warrior", "Royal Knight"]},
    {"card_id": "A-4", "card_name_eng": "Birdramon", "type_eng": []},  # no traits
]


def test_trait_lexicon_flattens_and_lowercases():
    traits = build_trait_lexicon(SYNTH)
    assert "dinosaur" in traits
    assert "adventure" in traits
    assert "royal knight" in traits  # multi-word trait preserved


def test_trait_lexicon_legacy_string_form():
    rows = [{"card_id": "X-1", "type_eng": "Aqua/Sea Animal"}]
    traits = build_trait_lexicon(rows)
    assert traits == {"aqua", "sea animal"}


def test_name_lexicon():
    names = build_name_lexicon(SYNTH)
    assert names == {"agumon", "greymon", "omnimon", "birdramon"}


def test_dict_and_list_inputs_equivalent():
    as_dict = {r["card_id"]: r for r in SYNTH}
    assert build_trait_lexicon(as_dict) == build_trait_lexicon(SYNTH)


def test_completeness_against_full_db_no_sampling():
    # The whole-DB trait lexicon must be large and contain the traits that
    # previously leaked as false-positive "keywords" with a partial lexicon.
    import json
    import io

    from data_paths import CARDS_JSON

    cards = json.load(io.open(CARDS_JSON, encoding="utf-8"))
    lex = build_lexicons(cards)
    # Whole-DB lexicon is ~282 traits; a per-set sample would be ~50-130, so this
    # guards against accidentally building from a sample rather than the full DB.
    assert len(lex["traits"]) >= 250
    # Traits that genuinely live in type_eng and previously leaked as candidate
    # "keywords" under a partial lexicon. (Note: some bracketed display names like
    # "[Aqua]"/"[Sovereign]" are NOT stored traits — those are caught by the
    # positional "[X] trait" rule in the keyword gate, not the lexicon.)
    for present in ("sea animal", "rock", "royal knight", "machine", "wizard", "mutant"):
        assert present in lex["traits"], f"{present} must be in the complete trait lexicon"
    assert len(lex["card_names"]) >= 1500
