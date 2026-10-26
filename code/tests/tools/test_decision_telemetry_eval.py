from copy import deepcopy
import json

import pytest

from tools.cloud_search.decision_telemetry_eval import behavior_rows, checked_source
from tools.cloud_search.final_policy_eval import schedule
from tools.selfplay_scheduler import sha256


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


def test_nyaromon_attack_requires_dna_by_its_own_player_not_opponent():
    reports, expected = fixture()
    for game in reports[0]["games"]:
        game["effect_sequences"][0]["attacks_declared"] = 1
        game["cards"] = [dict(player=1, dna_digivolves=0), dict(player=2, dna_digivolves=1)]
    reports[0]["games"][1]["cards"][0]["dna_digivolves"] = 1
    bad = behavior_rows(reports, expected)["nyaromon_attacks_without_player_dna"]
    assert len(bad) == 1
    assert bad[0]["index"] == 0 and bad[0]["checkpoint"] == 3 and bad[0]["player"] == 1


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


@pytest.mark.parametrize("failure", [None, "input", "incomplete", "failed_game"])
def test_replay_source_requires_frozen_complete_evaluation(tmp_path, failure):
    rows = schedule([f"deck{i}" for i in range(13)], "adjacent", 42)
    for name in ("pool.json", "checkpoint3.zip", "checkpoint4.zip"):
        (tmp_path / name).write_text("frozen fixture")
    (tmp_path / "adjacent-schedule.json").write_text(json.dumps(rows))
    files = {p.name: sha256(p) for p in tmp_path.iterdir()}
    (tmp_path / "plan.json").write_text(json.dumps(dict(files=files)))
    (tmp_path / "state.json").write_text(json.dumps(dict(stage="failed" if failure == "incomplete" else "complete")))
    for shard in range(4):
        folder = tmp_path / "adjacent" / f"shard{shard}"
        folder.mkdir(parents=True)
        outcomes = [dict(row, winner=1, terminal_reason="security_attack") for row in rows if row["block"] % 4 == shard]
        if failure == "failed_game" and shard == 0:
            outcomes[0]["terminal_reason"] = "truncated"
        (folder / "outcomes.json").write_text(json.dumps(outcomes))
    if failure == "input":
        (tmp_path / "checkpoint4.zip").write_text("changed")
    if failure:
        with pytest.raises(ValueError):
            checked_source(tmp_path)
    else:
        assert len(checked_source(tmp_path)[2]) == 6760
