from copy import deepcopy

import pytest

from tools.cloud_search.decision_telemetry_eval import behavior_rows


def fixture():
    game = dict(context=dict(index=0, seed=2, block=0, deck1="VB", deck2="Other", model1=3, model2=4),
                winner_id=2, terminal_reason="security_attack",
                route_decisions=[dict(player=1, route="digivolve_raising", opportunities=4, choices=2),
                                 dict(player=2, route="digivolve_raising", opportunities=5, choices=4)],
                effect_sequences=[dict(player=1, card_id="EX12-001", opportunities=3, dna_completed=1)])
    other = deepcopy(game)
    other["context"].update(index=1, seed=3, model1=4, model2=3)
    reports = [dict(coverage=dict(effect_sequences="measured"), games=[game, other])]
    expected = {g["context"]["index"]: dict(g["context"], winner=g["winner_id"], terminal_reason=g["terminal_reason"])
                for g in reports[0]["games"]}
    return reports, expected


def test_actual_model_and_deck_attribution_across_role_swaps():
    reports, expected = fixture()
    result = behavior_rows(reports, expected)
    assert len(result["route_decisions"]) == 4
    assert {r["checkpoint"] for r in result["effect_sequences"]} == {3, 4}
    assert all(r["archetype"] == "VB" and r["dna_completed"] == 1 for r in result["effect_sequences"])
    assert all(r["choice_rate"] == (.5 if r["archetype"] == "VB" else .8) for r in result["route_decisions"])


@pytest.mark.parametrize("failure", ["missing", "duplicate", "outcome", "context", "coverage", "counters"])
def test_rejects_incomplete_or_misattributed_telemetry(failure):
    reports, expected = fixture()
    if failure == "missing":
        reports[0]["games"].pop()
    elif failure == "duplicate":
        reports[0]["games"].append(reports[0]["games"][0])
    elif failure == "outcome":
        reports[0]["games"][0]["winner_id"] = 1
    elif failure == "context":
        reports[0]["games"][0]["context"]["model1"] = 4
    elif failure == "coverage":
        reports[0]["coverage"]["effect_sequences"] = "unmeasured"
    else:
        del reports[0]["games"][0]["route_decisions"]
    with pytest.raises(ValueError):
        behavior_rows(reports, expected)
