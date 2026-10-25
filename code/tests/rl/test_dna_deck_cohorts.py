import pytest

from tools.cloud_search.audit_dna_deck_cohorts import acting_deck, capable_lists


def test_capability_uses_targets_and_preserves_list_variants():
    cards = {"target": {"dna_costs": [{}]}, "support": {"name": "DNA Charge"},
             "ordinary": {}}
    decks = [{"cards": ["target", "target", "ordinary"]},
             {"cards": ["ordinary", "support"]}]
    assert capable_lists(decks, cards) == {0: {"target": 2}}


def test_mover_selects_own_list_with_global_game_offset():
    manifest = {"game_start": 68, "games": 2, "game_decks": [[21, 17], [8, 4]]}
    assert acting_deck(manifest, 69, 0) == 8
    assert acting_deck(manifest, 69, 1) == 4
    for game, mover in [(67, 0), (70, 0), (68, 2)]:
        with pytest.raises(ValueError):
            acting_deck(manifest, game, mover)
