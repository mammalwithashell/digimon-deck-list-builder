import json
from collections import Counter
from pathlib import Path


ST6_EXPECTED_COUNTS = {
    "ST6-01": 4,
    "ST6-02": 4,
    "ST6-03": 4,
    "ST6-04": 4,
    "ST6-05": 4,
    "ST6-06": 4,
    "ST6-07": 4,
    "ST6-08": 2,
    "ST6-09": 4,
    "ST6-10": 4,
    "ST6-11": 2,
    "ST6-12": 2,
    "ST6-13": 2,
    "ST6-14": 4,
    "ST6-15": 4,
    "ST6-16": 2,
}


def _st6_archetype():
    doc = json.loads(Path("data/deck_library.json").read_text(encoding="utf-8"))
    return doc["archetypes"]["Starter Deck Venomous Violet"]


def test_st6_starter_deck_library_entry_has_exact_product_counts():
    archetype = _st6_archetype()
    decklist = json.loads(archetype["decklists"][0]["decklist"])
    counts = Counter(decklist)

    assert counts == ST6_EXPECTED_COUNTS
    assert counts["ST6-01"] == 4
    assert len(decklist) == 54
    assert sum(count for card_id, count in counts.items() if card_id != "ST6-01") == 50


def test_st6_starter_deck_is_manual_product_data_not_meta_stats():
    archetype = _st6_archetype()
    stats = archetype["stats"]
    decklist = archetype["decklists"][0]

    assert decklist["source"] == "manual"
    assert decklist["format"] == "starter"
    assert decklist["placement"] == "Starter Deck"
    assert stats["times_played"] == 0
    assert stats["meta_share"] == 0.0
    assert stats["conversion_rate"] == 0.0
    assert "digilab_stats" not in archetype


def test_st6_starter_deck_cards_are_all_implemented_in_rust_registry():
    from digimon_engine import load_implemented_card_ids

    implemented = load_implemented_card_ids()
    assert set(ST6_EXPECTED_COUNTS).issubset(implemented)
