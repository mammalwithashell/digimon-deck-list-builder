from copy import deepcopy
import pytest
from tools.cloud_search.search_opportunity_report import summarize, load_rows
import json


def row():
    return {"schema": "search-opportunities-v1", "selected_plan": [0], "executed_actions": [0], "commitment_executed": True,
            "total_visits": 10, "plans": [
                {"plan": [100], "category": "tamer_digivolve", "action": {"card_id": "A"}, "selected": False, "visits": 9,
                 "prior_before_noise": .1, "prior_after_noise": .3, "selection_probability": .9},
                {"plan": [0], "category": "hard_play", "action": {"card_id": "A"}, "selected": True, "visits": 1,
                 "prior_before_noise": .9, "prior_after_noise": .7, "selection_probability": .1}]}


def test_choice_is_execution_not_visit_argmax():
    result = summarize([row()])
    assert result["counts"]["tamer_opportunity_roots"] == 1
    assert result["counts"]["tamer_executed_at_opportunity"] == 0
    assert result["counts"]["hard_play_selected_with_same_card_tamer_route"] == 1
    assert result["counts"]["selected_below_max_visits"] == 1
    assert result["means"]["tamer_visit_share"]["value"] == .9


def test_multiple_tamer_targets_count_one_opportunity():
    r = row()
    other = deepcopy(r["plans"][0]); other["plan"] = [101]; other["visits"] = 0
    other["prior_before_noise"] = other["prior_after_noise"] = other["selection_probability"] = 0
    r["plans"].append(other)
    counts = summarize([r])["counts"]
    assert counts["tamer_opportunity_roots"] == 1
    assert counts["tamer_plans"] == 2
    assert counts["tamer_zero_visit_plans"] == 1


def test_missing_priors_and_zero_visits_are_unmeasured():
    r = row(); r["total_visits"] = 0
    for p in r["plans"]:
        p["visits"] = 0
        p["prior_before_noise"] = p["prior_after_noise"] = None
        p["selection_probability"] = .5
    result = summarize([r])
    assert "tamer_visit_share" not in result["means"]
    assert "tamer_prior_before_noise_mass" not in result["means"]
    assert result["means"]["tamer_selection_probability_mass"]["value"] == .5


def test_incomplete_navigation_is_not_executed_commitment():
    r = row(); r["selected_plan"] = [59, 0]; r["plans"][1]["plan"] = [59, 0]
    r["executed_actions"] = [59]; r["commitment_executed"] = False
    assert summarize([r])["counts"]["executed_hard_play"] == 0
    r["commitment_executed"] = True
    with pytest.raises(ValueError, match="prefix"):
        summarize([r])


def test_rejects_mismatched_visits_or_legacy_inputs():
    r = row(); r["total_visits"] = 11
    with pytest.raises(ValueError, match="visit total"):
        summarize([r])
    with pytest.raises(ValueError, match="unsupported"):
        summarize([{}])


def test_native_driver_shape_and_model_attribution(tmp_path):
    r = row(); r["actor"] = 1
    path = tmp_path / "match.json"
    output = {"model_sha256":"candidate", "opponent_model_sha256":"opponent", "request":{"deck_indices":[0,1]},
        "games":[{"candidate_seat":0,"actions":[0,0,0,0,0],"search_diagnostics":[{"decision":4,"opportunity":r,"diagnostics":{"information":{"regime":"closed-list"}}}]}]}
    path.write_text(json.dumps(output))
    rows = list(load_rows(path))
    assert rows[0]["context"]["model"] == "opponent"
    assert rows[0]["context"]["information_regime"] == "closed-list"
    output["games"][0]["actions"][4] = 7
    path.write_text(json.dumps(output))
    with pytest.raises(ValueError, match="game trace"):
        list(load_rows(path))
