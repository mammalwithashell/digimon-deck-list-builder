"""Source claim validation; native runtime continuity is checked in Rust harness tests."""
import copy
import json
from pathlib import Path
import pytest
from tools.card_loop.interactions import packets as pk


def watch():
    return {"BT9-109": {"version": 1, "deletion_dispatches": 0, "rule_trashes": []}}


def result(rows):
    return {"q_id": "Q1921", "mode": "author", "agrees": None,
            "expect_ruling": {"assert": [{"at": at, "that": [{"key": key, "value_json": json.dumps(value)}]}
                                        for at, key, value in rows]}}


def row(at, value=None, seat="p0"):
    return (at, seat + ".movement_observations", watch() if value is None else value)


def test_complete_baseline_then_rule_trash_is_authorable_and_preserves_source_order():
    post = watch()
    post["BT9-109"]["rule_trashes"] = [{"lower_sources": [{"owner": 0, "card_id": "ST2-02"}], "linked": []}]
    candidate = result([row(12), row(15, post)])
    assert pk.validate_encode_result(candidate, "author") == []
    block = pk.expect_ruling_block(candidate)
    assert block["assert"][1]["that"]["p0.movement_observations"] == post


@pytest.mark.parametrize("rows", [[row(15)], [row(15), row(15)], [row(12), row(15, seat="p1")]])
def test_each_named_seat_watch_requires_two_distinct_anchors(rows):
    assert any("earlier public baseline" in x for x in pk.validate_encode_result(result(rows), "author"))


def test_another_card_does_not_supply_this_watchs_baseline():
    different = {"ST2-07": watch()["BT9-109"]}
    assert any("earlier public baseline" in x for x in pk.validate_encode_result(result([row(12, different), row(15)]), "author"))


@pytest.mark.parametrize("at", ["12", True, -1, 12.5, None])
def test_invalid_anchor_cannot_be_coerced_into_a_distinct_baseline(at):
    candidate = result([row(at), row(12)])
    assert any("nonnegative integer" in x for x in pk.validate_encode_result(candidate, "author"))
    with pytest.raises(ValueError, match="nonnegative integer"):
        pk.expect_ruling_block(candidate)


@pytest.mark.parametrize("key,value", [("native_top", 42), ("binding_start", 12), ("movement_observation_at", 15),
                                      ("deletion_dispatches", True), ("version", 2)])
def test_native_identity_is_not_a_source_claim_and_malformed_counts_fail(key, value):
    malformed = copy.deepcopy(watch())
    malformed["BT9-109"][key] = value
    assert not pk.valid_movement_claim(malformed)
    assert pk.validate_encode_result(result([row(12, malformed), row(15, malformed)]), "author")


def test_incomplete_rule_trash_is_not_an_empty_observation():
    malformed = watch()
    malformed["BT9-109"]["rule_trashes"] = [{"lower_sources": []}]
    assert not pk.valid_movement_claim(malformed)


def test_encode_schema_exposes_semantic_watches_but_not_native_audit_keys():
    root = Path(pk.__file__).resolve().parents[1]
    schema = json.loads((root / "schemas" / "encode_ruling.json").read_text(encoding="utf-8"))
    def enums(value):
        if isinstance(value, dict):
            if "enum" in value: yield value["enum"]
            for child in value.values(): yield from enums(child)
        elif isinstance(value, list):
            for child in value: yield from enums(child)
    keys = next(values for values in enums(schema) if "p0.field" in values)
    assert {"p0.movement_observations", "p1.movement_observations"} <= set(keys)
    assert not any("native" in key or "movement_audit" in key or "movement_observation_at" in key for key in keys)
