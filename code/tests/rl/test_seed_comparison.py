import copy

import pytest

from tools.cloud_search.run_seed_comparison import compare_reports


def games(winners):
    return [dict(seed=20 + 2*i + i % 2, winner_id=w, terminal_reason="security_attack",
                 context=dict(panel="pet_same_list", anchor="greedy", agent_archetype="a",
                              opponent_archetype="b", agent_deck_id="a1", opponent_deck_id="b1"))
            for i,w in enumerate(winners)]


def test_paired_uncertainty_clusters_opening_seats_and_draws_are_half_points():
    control = games([1, 2, 1, 2])
    candidate = games([1, 1, 1, 1])
    report = compare_reports(control, candidate, samples=200)
    assert report["aggregate"]["difference"] == .5
    assert report["aggregate"]["ci95"] == [.5, .5]
    assert report["aggregate"]["paired_games"] == 4
    assert report["aggregate"]["noninferior_at_five_points"]
    assert compare_reports(games([2, 2]), games([None, None]), samples=200)["aggregate"]["difference"] == .5


def test_pairs_refuse_mismatched_cohorts_and_incomplete_games():
    control = games([1, 2])
    candidate = copy.deepcopy(control)
    candidate[0]["seed"] += 100
    with pytest.raises(ValueError, match="cohorts differ"):
        compare_reports(control, candidate)
    candidate = copy.deepcopy(control)
    candidate[0]["terminal_reason"] = "truncated"
    with pytest.raises(ValueError, match="incomplete"):
        compare_reports(control, candidate)
    with pytest.raises(ValueError, match="opening-seat pair"):
        compare_reports(control[:1], control[:1])
