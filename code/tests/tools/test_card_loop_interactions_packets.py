"""classify_qa / encode_ruling packets, schemas and two-family agreement (D5-D7)."""
from __future__ import annotations

import json
import re
from pathlib import Path

import pytest
import yaml

from tools.card_loop.contracts import FAMILIES, TERMINATING_STAGES
from tools.card_loop.interactions import packets as pk
from tools.card_loop.interactions.denominator import extract_clauses, load_card_qa

FIX = Path(__file__).resolve().parent / "fixtures" / "card_loop" / "interactions"
ROOT = Path(__file__).resolve().parents[3]


@pytest.fixture(scope="module")
def card_qa():
    return load_card_qa(FIX / "card_qa.json")


@pytest.fixture(scope="module")
def clauses():
    return extract_clauses(["BT7-056", "EX4-030"])


def _schema(name):
    return json.loads((ROOT / "code" / "tools" / "card_loop" / "schemas" / name).read_text(encoding="utf-8"))


SCENARIO = """card: BT7-056
clause: BT7-056#effect#0
interaction: { id: "qa:Q2001", source: qa, kind: positive }
seed: 1
decks:
  p0: { stack: [BT7-056], rest: vb-standard }
  p1: { stack: [], rest: vb-standard }
steps:
  - actor: 0
    do: { pass: {} }
"""


def test_both_stages_are_terminating_contract_stages():
    assert {"classify_qa", "encode_ruling"} <= set(TERMINATING_STAGES)


def test_classify_packet_carries_the_ruling_the_cards_and_their_clauses(card_qa, clauses):
    p = pk.classify_qa_inputs("Q2002", card_qa, clauses)
    assert p.stage == "classify_qa" and p.terminating and p.requires_two_families
    assert p.item == "interaction:qa:Q2002"
    assert p.inputs["ruling"]["answer"] == "Yes, it is."
    assert [c["card_id"] for c in p.inputs["cards"]] == ["BT7-056", "EX4-030"]
    assert any(c["id"] == "BT7-056#effect#0" for c in p.inputs["cards"][0]["clauses"])
    assert "data/card_bundles/EX4-030.md" in p.references
    assert p.schema_path.endswith("classify_qa.json") and Path(p.schema_path).exists()
    # The qa: prefix is accepted too.
    assert pk.classify_qa_inputs("qa:Q2002", card_qa, clauses).item == p.item
    with pytest.raises(KeyError, match="Q404"):
        pk.classify_qa_inputs("Q404", card_qa, clauses)


def test_an_independent_pair_is_the_same_packet_for_each_family(card_qa, clauses):
    p = pk.classify_qa_inputs("Q2001", card_qa, clauses)
    a, b = pk.independent_pair(p, attempt_ids={"claude": "a1", "codex": "a2"},
                               worktrees={"claude": "wt1", "codex": "wt2"}, prompt_version="1")
    assert {a.family, b.family} == set(FAMILIES)
    assert a.prompt == b.prompt and a.schema_path == b.schema_path and a.references == b.references
    assert a.attempt_id != b.attempt_id


def test_a_terminating_call_never_degrades_to_one_family(card_qa, clauses):
    p = pk.classify_qa_inputs("Q2001", card_qa, clauses)
    with pytest.raises(ValueError, match="escalate"):
        pk.independent_pair(p, attempt_ids={"claude": "a1"}, worktrees={"claude": "wt"},
                            prompt_version="1")


def test_rendered_prompts_are_deterministic(card_qa, clauses):
    p = pk.classify_qa_inputs("Q2001", card_qa, clauses)
    assert pk.render_prompt(p) == pk.render_prompt(pk.classify_qa_inputs("Q2001", card_qa, clauses))
    assert "qa:Q2001" in pk.render_prompt(p)


def test_verify_packets_carry_the_block_but_not_the_authors_argument(card_qa, clauses):
    author_result = {"q_id": "Q2001", "mode": "author", "agrees": None,
                     "answer_quote": "you must add it",
                     "reasoning": "the answer says the add is mandatory when possible",
                     "expect_ruling": {"assert": [{"at": 1, "that": [
                         {"key": "p0.hand", "value_json": "[\"BT7-056\"]"}]}]}}
    p = pk.encode_ruling_inputs("Q2001", card_qa, clauses, scenario_path="s.yaml",
                                scenario_yaml=SCENARIO, mode="verify",
                                candidate=author_result["expect_ruling"])
    blob = json.dumps(p.inputs)
    assert "candidate" in p.inputs and "mandatory when possible" not in blob
    assert p.inputs["assertion_keys"] == list(pk.ASSERTION_KEYS)
    with pytest.raises(ValueError):
        pk.encode_ruling_inputs("Q2001", card_qa, clauses, scenario_path="s", scenario_yaml="",
                                mode="verify")


# --- agreement --------------------------------------------------------------


def _cls(c, cite="qa:Q2002 traits"):
    return {"q_id": "Q2002", "classification": c, "reasoning": "x" * 25, "citation": cite,
            "examined_clauses": []}


def test_two_families_agreeing_on_a_terminating_class_end_the_ruling():
    a = pk.agree_classification("Q2002", _cls("textual"), _cls("textual"))
    assert a.agreed and a.value == "textual" and not a.escalate


@pytest.mark.parametrize("pair,why", [
    (("textual", "not_examinable"), "disagree"),
    ((None, "textual"), "one family"),
])
def test_disagreement_or_a_missing_family_escalates(pair, why):
    a = pk.agree_classification("Q2002", *(None if c is None else _cls(c) for c in pair))
    assert a.escalate and not a.agreed and a.value is None
    assert any(why in r for r in a.reasons)


def test_a_terminating_class_without_the_ruling_citation_escalates():
    a = pk.agree_classification("Q2002", _cls("textual"), _cls("textual", cite="card text"))
    assert a.escalate and any("cite" in r for r in a.reasons)


def test_either_family_saying_behavioral_means_examine():
    a = pk.agree_classification("Q2002", _cls("behavioral"), _cls("textual"))
    assert a.value == "behavioral" and not a.escalate


AUTHOR = {"q_id": "Q2001", "mode": "author", "agrees": None, "line_exercises_ruling": None, "answer_quote": "you must add it",
          "reasoning": "the answer says the add is mandatory when possible",
          "expect_ruling": {"assert": [{"at": 1, "that": [
              {"key": "p0.hand", "value_json": "[\"BT7-056\"]"},
              {"key": "p0.memory", "value_json": "3"}]}]}}
VERIFY_YES = {"q_id": "Q2001", "mode": "verify", "agrees": True, "line_exercises_ruling": None, "expect_ruling": None,
              "answer_quote": "you must add it", "reasoning": "the block asserts the added card"}


def test_an_encoding_is_accepted_only_when_the_other_family_verifies_it():
    assert pk.agree_encoding("Q2001", AUTHOR, VERIFY_YES).agreed
    no = {**VERIFY_YES, "agrees": False, "answer_quote": "the answer also says X"}
    a = pk.agree_encoding("Q2001", AUTHOR, no)
    assert a.escalate and "verifier disagrees" in a.reasons[0]
    assert pk.agree_encoding("Q2001", AUTHOR, None).escalate


def test_cross_field_rules_the_schema_cannot_express():
    assert pk.validate_encode_result(AUTHOR, "author") == []
    bad = {**AUTHOR, "agrees": True}
    assert any("agrees null" in p for p in pk.validate_encode_result(bad, "author"))
    bad_json = json.loads(json.dumps(AUTHOR))
    bad_json["expect_ruling"]["assert"][0]["that"][1]["value_json"] = "three"
    assert any("not JSON" in p for p in pk.validate_encode_result(bad_json, "author"))
    assert any("must set agrees" in p for p in pk.validate_encode_result({**VERIFY_YES, "agrees": None}, "verify"))


def test_an_accepted_encoding_becomes_a_scenario_expect_ruling_block_that_parses():
    block = pk.expect_ruling_block(AUTHOR)
    assert block == {"q_id": "Q2001", "assert": [{"at": 1, "that": {"p0.hand": ["BT7-056"], "p0.memory": 3}}]}
    text = SCENARIO + yaml.safe_dump({"expect_ruling": block}, sort_keys=False)
    doc = yaml.safe_load(text)
    assert doc["expect_ruling"]["q_id"] == "Q2001"
    assert doc["interaction"]["id"] == "qa:" + doc["expect_ruling"]["q_id"]


# --- schemas ----------------------------------------------------------------


def test_schemas_are_valid_and_accept_the_documented_results():
    jsonschema = pytest.importorskip("jsonschema")
    for name, good in (("classify_qa.json", _cls("textual")), ("encode_ruling.json", AUTHOR),
                       ("encode_ruling.json", VERIFY_YES)):
        schema = _schema(name)
        jsonschema.Draft202012Validator.check_schema(schema)
        jsonschema.validate(good, schema)


def test_schemas_reject_unknown_classes_keys_and_extra_fields():
    jsonschema = pytest.importorskip("jsonschema")
    with pytest.raises(jsonschema.ValidationError):
        jsonschema.validate(_cls("vibes"), _schema("classify_qa.json"))
    with pytest.raises(jsonschema.ValidationError):
        jsonschema.validate({**_cls("textual"), "confidence": 0.9}, _schema("classify_qa.json"))
    bad = json.loads(json.dumps(AUTHOR))
    bad["expect_ruling"]["assert"][0]["that"][0]["key"] = "p0.security.0"
    with pytest.raises(jsonschema.ValidationError):
        jsonschema.validate(bad, _schema("encode_ruling.json"))


def test_schemas_are_strict_mode_shaped():
    # Codex --output-schema (strict structured outputs) needs every property
    # required and every object closed.
    def walk(node):
        if isinstance(node, dict):
            if node.get("type") == "object":
                assert node.get("additionalProperties") is False, node
                assert set(node.get("required", [])) == set(node.get("properties", {})), node
            for v in node.values():
                walk(v)
        elif isinstance(node, list):
            for v in node:
                walk(v)

    walk(_schema("classify_qa.json"))
    walk(_schema("encode_ruling.json"))


def test_assertion_keys_match_the_rust_checker():
    src = (ROOT / "code" / "tools" / "dcgo-harness" / "src" / "exam" / "assertions.rs").read_text(encoding="utf-8")
    block = re.search(r"ASSERTION_KEYS: &\[&str\] = &\[(.*?)\];", src, re.S).group(1)
    keys = []
    for k in re.findall(r'"([^"]+)"', block):
        keys += [k.replace("p{0,1}", f"p{i}") for i in (0, 1)] if "p{0,1}" in k else [k]
    assert sorted(keys) == sorted(pk.ASSERTION_KEYS)
    enum = _schema("encode_ruling.json")["properties"]["expect_ruling"]["anyOf"][1][
        "properties"]["assert"]["items"]["properties"]["that"]["items"]["properties"]["key"]["enum"]
    assert sorted(enum) == sorted(pk.ASSERTION_KEYS)
