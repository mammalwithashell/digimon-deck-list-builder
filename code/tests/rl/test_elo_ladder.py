"""Tests for the Elo ladder — turns pairwise round-robin results among a run's
checkpoints + champions + the greedy anchor into one comparable rating scale.

The rating math (Bradley-Terry / Elo fit), standard errors, cohort gating, and
upset (forgetting/cycling) detection are all pure and unit-tested here.
Part of add-model-evaluation-harness.
"""
import math

import pytest

from digimon_gym.agents.elo_ladder import (
    PlayerSpec, MatchupCell, compute_elo, elo_standard_errors,
    find_upsets, cohort_compatible, run_round_robin,
)


def cell(a, b, aw, bw, d=0):
    return MatchupCell(a_name=a, b_name=b, a_wins=aw, b_wins=bw, draws=d)


# ---- compute_elo ----

def test_dominant_player_rated_higher():
    r = compute_elo([cell("A", "B", 30, 0)])
    assert r["A"] > r["B"]


def test_symmetric_results_equal_ratings():
    r = compute_elo([cell("A", "B", 15, 15)])
    assert r["A"] == pytest.approx(r["B"], abs=1.0)


def test_transitive_ordering_recovered():
    cells = [cell("A", "B", 22, 8), cell("B", "C", 22, 8), cell("A", "C", 27, 3)]
    r = compute_elo(cells)
    assert r["A"] > r["B"] > r["C"]


def test_anchor_rating_is_held_fixed():
    r = compute_elo([cell("model", "greedy", 30, 10)], anchor="greedy", anchor_rating=1000.0)
    assert r["greedy"] == pytest.approx(1000.0)
    assert r["model"] > 1000.0  # it beats the anchor


def test_greedy_anchor_is_lowest_when_models_beat_it():
    cells = [
        cell("m1", "greedy", 30, 10), cell("m2", "greedy", 33, 7),
        cell("m1", "m2", 14, 16),
    ]
    r = compute_elo(cells, anchor="greedy", anchor_rating=1000.0)
    assert r["greedy"] < r["m1"] and r["greedy"] < r["m2"]


# ---- standard errors ----

def test_standard_error_shrinks_with_more_games():
    ratings = {"A": 1000.0, "B": 1000.0}
    se_few = elo_standard_errors([cell("A", "B", 5, 5)], ratings)
    se_many = elo_standard_errors([cell("A", "B", 50, 50)], ratings)
    assert se_many["A"] < se_few["A"]


def test_standard_error_infinite_without_games():
    se = elo_standard_errors([], {"A": 1000.0})
    assert math.isinf(se["A"])


# ---- cohort gating ----

def test_cohort_same_hash_compatible():
    a = PlayerSpec("a", "model", layout_hash="h1")
    b = PlayerSpec("b", "model", layout_hash="h1")
    assert cohort_compatible(a, b)


def test_cohort_mismatched_hash_incompatible():
    a = PlayerSpec("a", "model", layout_hash="h1")
    b = PlayerSpec("b", "model", layout_hash="h2")
    assert not cohort_compatible(a, b)


def test_greedy_bridges_any_cohort():
    g = PlayerSpec("greedy", "greedy")
    m = PlayerSpec("m", "model", layout_hash="h2")
    assert cohort_compatible(g, m) and cohort_compatible(m, g)


# ---- upset / forgetting detection ----

def test_upset_flagged_when_later_loses_to_earlier():
    order = {"step100k": 1, "step500k": 5}
    # the later checkpoint (500k) loses the head-to-head
    cells = [cell("step500k", "step100k", 4, 8)]
    ups = find_upsets(cells, order)
    assert len(ups) == 1
    assert ups[0].later == "step500k" and ups[0].earlier == "step100k"


def test_no_upset_when_later_wins():
    order = {"step100k": 1, "step500k": 5}
    cells = [cell("step500k", "step100k", 9, 3)]
    assert find_upsets(cells, order) == []


# ---- integration: a real 3-player round-robin rates greedy lowest ----

from pathlib import Path  # noqa: E402

_ROOT = Path(__file__).resolve().parents[3]
_V22 = _ROOT / "cloud_downloads" / "v022-hf4zm2hl82qk48" / "models" / "pilot_ppo_starter_decks_generalist_v1.zip"
_V20 = _ROOT / "cloud_downloads" / "v020-3fm5tm9kci2isy" / "models" / "pilot_ppo_starter_decks_generalist_v1.zip"
_SNAP = _ROOT / "cloud_downloads" / "v022-hf4zm2hl82qk48" / "models" / "pilot_ppo_20260531_073321" / "deck_pool_snapshot.json"


@pytest.mark.slow
def test_round_robin_rates_greedy_lowest():
    if not (_V22.exists() and _V20.exists() and _SNAP.exists()):
        pytest.skip("champion models / snapshot not present locally")
    from digimon_gym.agents.gauntlet import GeneralistDeckPool
    from digimon_gym.digimon_gym import greedy_policy
    from digimon_gym.agents.pilot_training import MaskableRecurrentPPO, make_agent_opponent_fn
    from digimon_engine import load_implemented_card_ids

    H = "sha256:a20462fb"
    players = [
        PlayerSpec("v22", "model", str(_V22), "lstm", H),
        PlayerSpec("v20", "model", str(_V20), "lstm", H),
        PlayerSpec("greedy", "greedy"),
    ]
    pool = GeneralistDeckPool.from_snapshot(_SNAP, implemented_card_ids=load_implemented_card_ids())

    def load_model(spec):
        return MaskableRecurrentPPO.load(spec.weights_path, device="cpu")

    def opponent_fn_for(spec):
        return greedy_policy if spec.kind == "greedy" else make_agent_opponent_fn(spec.weights_path, algorithm="lstm")

    cells = run_round_robin(players, pool, n=8, base_seed=777,
                            tensor_profile="standard_lite_deck_v2",
                            load_model=load_model, opponent_fn_for=opponent_fn_for)
    assert len(cells) == 3  # v22-v20, v22-greedy, v20-greedy
    ratings = compute_elo(cells, anchor="greedy", anchor_rating=1000.0)
    # Both trained models beat greedy, so greedy is the lowest-rated.
    assert ratings["greedy"] < ratings["v22"]
    assert ratings["greedy"] < ratings["v20"]
