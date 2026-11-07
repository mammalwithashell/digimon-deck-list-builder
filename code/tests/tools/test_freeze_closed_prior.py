import copy
import json

import pytest

from tools.cloud_search.freeze_closed_prior import audit


def fixture():
    snapshots = {role: {"decks": [{"card_ids": [f"{role}-{i}"], "archetype": f"a{i % 12}"}
                                 for i in range(count)]}
                 for role, count in (("train", 21), ("validation", 36), ("test", 23), ("pet_regression", 5))}
    candidates = [{"cards": row["card_ids"], "weight": 1.0} for row in snapshots["train"]["decks"]]
    population = [r["cards"][0] for r in candidates]
    prior = dict(schema_version=1, format_id="standard", provenance=json.dumps({"split": "train"}),
                 candidates=candidates, population=population, unseen_mass=0.1, fallback_attempts=128)
    slots = copy.deepcopy(candidates + candidates[:5])
    admission = {"cards": {c: {"admissible": True} for c in population}}
    return prior, snapshots, slots, admission, {"ready": True}, dict.fromkeys(population)


def test_valid_counts_and_overlap():
    report = audit(*fixture())
    assert report["distinct_lists"]["train"] == 21
    assert report["runtime_slots"] == 26
    assert not any(report["partition_overlaps"].values())


@pytest.mark.parametrize("mutation,match", [
    (lambda x: x[0]["candidates"][0].update(weight=2.0), "unequal"),
    (lambda x: x[1]["test"]["decks"][0].update(card_ids=["train-0"]), "leakage"),
    (lambda x: x[3]["cards"]["train-0"].update(admissible=False), "no longer admitted"),
    (lambda x: x[4].update(ready=False), "mechanics"),
    (lambda x: x[0].update(unseen_mass=0.2), "fallback settings"),
    (lambda x: x[0]["candidates"][0].update(cards=["test-0"]), "21 training lists"),
])
def test_reject_changed_contract(mutation, match):
    inputs = fixture()
    mutation(inputs)
    with pytest.raises(ValueError, match=match):
        audit(*inputs)
