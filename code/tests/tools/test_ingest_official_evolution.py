"""Offline official circles must not rewrite the legacy runtime ABI."""
import copy
import hashlib
import json

import pytest

from tools import ingest_cards


def observation(folder, cid="EX12-007", colors=None, cost=1, **changes):
    doc = {
        "schema_version": 1, "source": "official",
        "parser_version": "official-html-v2",
        "source_schema_version": "world-card-list-html-v1",
        "raw_payload_digest": "a" * 64,
        "source_url": "https://world.digimoncard.com/cards/",
        "extracted": {"card_id": cid, "evolution_routes": [
            {"slot": 1, "colors": colors or ["Red", "Yellow"], "from_level": 2, "cost": cost}
        ]},
    }
    doc.update(changes)
    identity = {k: doc[k] for k in (
        "raw_payload_digest", "parser_version", "source_schema_version", "extracted"
    )}
    digest = hashlib.sha256((json.dumps(identity, sort_keys=True, ensure_ascii=False,
                                       indent=2, separators=(",", ": ")) + "\n").encode()).hexdigest()
    doc["observation_id"] = digest
    folder.mkdir(exist_ok=True)
    path = folder / (digest + ".json")
    path.write_text(json.dumps(doc), encoding="utf-8")
    return path


def inputs(tmp_path):
    cards = {
        "EX12-007": {"card_id": "EX12-007", "evo_costs": [
            {"card_color": 0, "level": 2, "memory_cost": 1}
        ], "index": 123, "unknown_future_field": {"keep": True}},
        "BT25-043": {"card_id": "BT25-043", "card_kind": 4,
                       "dual_card": {"name": "Habakiri", "cost": 6}},
    }
    path = tmp_path / "cards.json"
    path.write_text(json.dumps(cards), encoding="utf-8")
    return cards, path, tmp_path / "observations", tmp_path / "candidate"


def build(path, observations, output):
    return ingest_cards.build_official_evolution_candidate(path, observations, ["EX12-007"], output)


def test_projects_every_printed_color_preserving_every_other_field(tmp_path):
    cards, path, observations, output = inputs(tmp_path)
    source_bytes = path.read_bytes()
    evidence = observation(observations)
    report = build(path, observations, output)
    actual = json.loads((output / "cards.json").read_text())
    expected = copy.deepcopy(cards)
    expected["EX12-007"]["evo_costs"].append({"card_color": 2, "level": 2, "memory_cost": 1})
    assert actual == expected
    assert path.read_bytes() == source_bytes
    assert report["changes"][0]["card_id"] == "EX12-007"
    assert report["changes"][0]["field"] == "evo_costs"
    assert report["observations"][0]["observation_id"] == evidence.stem
    assert report["candidate_sha256"] == hashlib.sha256((output / "cards.json").read_bytes()).hexdigest()


@pytest.mark.parametrize("case", ["conflict", "missing", "wrong_source", "unknown_color", "bool_cost", "missing_routes", "bad_identity"])
def test_bad_or_ambiguous_evidence_fails_before_writing(tmp_path, case):
    _, path, observations, output = inputs(tmp_path)
    evidence = observation(observations)
    if case == "conflict":
        observation(observations, cost=2)
    elif case == "missing":
        evidence.unlink()
    elif case == "wrong_source":
        observation(observations, source="digimoncard_io")
    elif case == "unknown_color":
        observation(observations, colors=["Rainbow"])
    elif case == "bool_cost":
        observation(observations, cost=True)
    elif case == "missing_routes":
        observation(observations, extracted={"card_id": "EX12-007"})
    else:
        doc = json.loads(evidence.read_text())
        doc["extracted"]["evolution_routes"][0]["cost"] = 9
        evidence.write_text(json.dumps(doc))
    before = path.read_bytes()
    with pytest.raises(ValueError):
        build(path, observations, output)
    assert not output.exists()
    assert path.read_bytes() == before


def test_agreeing_printings_are_all_recorded(tmp_path):
    _, path, observations, output = inputs(tmp_path)
    observation(observations)
    observation(observations, raw_payload_digest="b" * 64)
    assert len(build(path, observations, output)["observations"]) == 2


def test_cannot_overwrite_input_or_reuse_candidate(tmp_path):
    _, path, observations, output = inputs(tmp_path)
    observation(observations)
    with pytest.raises(ValueError):
        build(path, observations, tmp_path)
    build(path, observations, output)
    with pytest.raises(ValueError):
        build(path, observations, output)


def test_missing_route_list_is_not_confused_with_explicit_no_circles(tmp_path):
    _, path, observations, output = inputs(tmp_path)
    observation(observations, extracted={"card_id": "EX12-007", "evolution_routes": []})
    build(path, observations, output)
    assert json.loads((output / "cards.json").read_text())["EX12-007"]["evo_costs"] == []
