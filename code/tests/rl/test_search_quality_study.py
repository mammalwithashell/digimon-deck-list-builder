"""Guard budget-study statistics and conservative selection behavior."""
import json

import pytest

from tools.search_quality_study import cfg, js_distance, normalized, score_interval, summarize_matches, pair_job


def test_policy_distance_ignores_budget_scale_and_detects_disjoint_actions():
    a = normalized([[10, 20], [11, 60]])
    b = normalized([[10, 80], [11, 240]])
    assert js_distance(a, b) == pytest.approx(0)
    assert js_distance({10: 1.0}, {11: 1.0}) == pytest.approx(1)
    with pytest.raises(ValueError):
        normalized([[10, 0]])


def test_all_tie_small_panel_retains_finite_sample_uncertainty():
    low, high = score_interval([0.5] * 16, list(range(8)) * 2)
    assert low < 0.45 < 0.5 < 0.55 < high


def panel(tmp_path, alternative_score):
    baseline, alternative = cfg(320, 4), cfg(160, 4)
    plan = {"baseline": baseline, "decks": [{}] * 8, "head_pairs": 16, "anchor_pairs": 8, "match_seed": 42}
    results = tmp_path / "results"
    results.mkdir()
    for candidate in (baseline, alternative):
        opponents = ([baseline] if candidate != baseline else []) + [
            {"id": "greedy", "kind": "greedy"}, {"id": "policy", "kind": "policy"}]
        for opponent in opponents:
            for i in range(16 if opponent == baseline else 8):
                name, _, _ = pair_job(plan, candidate, opponent, i)
                scores = [1.0, 0.0] if opponent != baseline or alternative_score == 0.5 else [alternative_score] * 2
                (results / f"{name}.json").write_text(json.dumps({"seconds": 1, "games": [
                    {"candidate_seat": seat, "first_player": 0, "score": scores[seat], "truncated": False}
                    for seat in (0, 1)]}))
    sweep = {"metrics": [{"config": c, "seconds_per_position": seconds, "tactical_all_pass": True}
                         for c, seconds in ((baseline, 2), (alternative, 1))]}
    return plan, alternative, sweep


def test_matching_point_scores_do_not_establish_noninferiority(tmp_path):
    plan, alternative, sweep = panel(tmp_path, 0.5)
    result = summarize_matches(plan, tmp_path, [alternative], sweep)
    assert result["recommended_pilot_config"] == plan["baseline"]
    assert not result["candidate_gates"][0]["qualifies_pilot_gate"]


def test_clear_cheaper_win_can_qualify_but_cap_blocks_recommendation(tmp_path):
    plan, alternative, sweep = panel(tmp_path, 1.0)
    result = summarize_matches(plan, tmp_path, [alternative], sweep)
    assert result["recommended_pilot_config"] == alternative
    name, _, _ = pair_job(plan, alternative, plan["baseline"], 0)
    path = tmp_path / "results" / f"{name}.json"
    record = json.loads(path.read_text())
    record["games"][0]["truncated"] = True
    path.write_text(json.dumps(record))
    assert summarize_matches(plan, tmp_path, [alternative], sweep)["recommended_pilot_config"] == plan["baseline"]


def test_invalid_seat_pair_is_rejected(tmp_path):
    plan, alternative, sweep = panel(tmp_path, 1.0)
    name, _, _ = pair_job(plan, alternative, plan["baseline"], 0)
    path = tmp_path / "results" / f"{name}.json"
    record = json.loads(path.read_text())
    record["games"][1]["candidate_seat"] = 0
    path.write_text(json.dumps(record))
    with pytest.raises(AssertionError):
        summarize_matches(plan, tmp_path, [alternative], sweep)
