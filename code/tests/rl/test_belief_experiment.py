import pytest
from tools.prepare_belief_experiment import split_lists


def test_splits_are_disjoint_deduplicated_and_order_independent():
    rows = [{"cards": [str(i), "common"], "archetype": "fixture"} for i in range(20)]
    a = split_lists(rows + rows, 1729)
    b = split_lists(list(reversed(rows)), 1729)
    assert a == b
    sets = [{r["id"] for r in a[k]} for k in ("train", "validation", "test")]
    assert sum(map(len, sets)) == 20
    assert not sets[0] & sets[1] and not sets[0] & sets[2] and not sets[1] & sets[2]


def test_insufficient_distinct_decks_cannot_claim_heldout_evaluation():
    with pytest.raises(ValueError):
        split_lists([{"cards": ["same"]}] * 4, 1)
