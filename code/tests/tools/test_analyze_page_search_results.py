import copy

import pytest

from tools.cloud_search.analyze_page_search_results import paired


def game(seed, winner):
    return dict(seed=seed, winner_id=winner,
        context=dict(anchor="fixed", agent_archetype="A", deck="exact-list"))


def test_checkpoint_comparison_is_paired_by_cohort_not_file_order():
    old = [game(2, 2), game(3, 2), game(4, 1), game(5, 1)]
    new = [game(5, 1), game(2, 1), game(4, 1), game(3, 1)]
    result = paired(old, new, samples=100)
    assert result["paired_games"] == 4
    assert result["delta_pp"] == 50
    assert result["improved_games"] == 2
    assert result["worsened_games"] == 0


@pytest.mark.parametrize("change", ["duplicate", "seed", "deck"])
def test_checkpoint_comparison_refuses_changed_cohorts(change):
    old = [game(2, 1), game(3, 2)]
    new = copy.deepcopy(old)
    if change == "duplicate":
        new[1] = copy.deepcopy(new[0])
    elif change == "seed":
        new[1]["seed"] = 5
    else:
        new[1]["context"]["deck"] = "different-list"
    with pytest.raises(AssertionError):
        paired(old, new, samples=10)
