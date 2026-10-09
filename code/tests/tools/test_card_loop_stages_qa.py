"""Q&A interaction stages: CLASSIFY (two independent families) and ENCODE
(author + blind verify), design D5-D7. FakeWorkers only."""
from __future__ import annotations

import json

import pytest

from tools.card_loop.driver_contracts import ItemRecord, check_transition
from tools.card_loop.stages.base import StageDeferred
from tools.card_loop.stages.classify import ClassifyExecutor
from tools.card_loop.stages.encode import EncodeExecutor
from tools.card_loop.stages.termination import TerminationCheckExecutor
from tools.card_loop.stages.testing import FakeCommands, failed, make_ctx, ok, sources
from tools.card_loop.workers.fake import FakeWorker

CLAUSE = "ST23-04#effect#0"
CLAUSE_ROWS = [
    {"id": CLAUSE, "card_id": "ST23-04", "zone": "effect", "label": "Effect", "timings": ["On Play"],
     "keyword": None, "text": "[On Play] 1 of your opponent's Digimon gets -5000 DP for the turn."},
    {"id": "ST23-04#inherited#0", "card_id": "ST23-04", "zone": "inherited", "label": "Inherited",
     "timings": [], "keyword": None, "text": "[Your Turn] This Digimon gets +1000 DP."},
]
CARD_QA = {"qa": {"Q77": {"q_id": "Q77", "date": "2025-01-01",
                          "question": "Can the -5000 target a Tamer?",
                          "answer": "No. It only affects Digimon.", "card_ids": ["ST23-04"]}}}
SC = "qa/dcgo-exams/ST23/ST23-04-effect0.yaml"
SCENARIO = """card: ST23-04
clause: ST23-04#effect#0
seed: 1
decks:
  p0: {stack: [ST23-04], rest: gd}
  p1: {stack: [], rest: quiet}
steps:
  - actor: 0
    do: {hatch: {}}
"""
ITEM = "interaction:qa:Q77"


@pytest.fixture
def repo(tmp_path):
    (tmp_path / "data" / "card_bundles").mkdir(parents=True)
    (tmp_path / "data" / "card_bundles" / "ST23-04.md").write_text("# Murasamemon  (ST23-04)\n", encoding="utf-8")
    p = tmp_path / SC
    p.parent.mkdir(parents=True)
    p.write_text(SCENARIO, encoding="utf-8")
    return tmp_path


def _workers(claude=(), codex=()):
    return {"claude": FakeWorker(list(claude), family="claude"),
            "codex": FakeWorker(list(codex), family="codex")}


def _ctx(repo, workers, cmds=None):
    return make_ctx(repo, workers=workers, run_command=cmds or FakeCommands(),
                    sources=sources(clauses=CLAUSE_ROWS, card_qa=CARD_QA))


def _cls(classification, citation="qa:Q77", clauses=(CLAUSE,)):
    return ok({"q_id": "Q77", "classification": classification,
               "reasoning": "the answer decides how the effect resolves on the board",
               "citation": citation, "examined_clauses": list(clauses)})


def _check(item, out):
    check_transition(item.kind, item.state, out.next_state)


# ================================================================ CLASSIFY


def test_probe_interactions_skip_classification(repo):
    workers = _workers()
    item = ItemRecord(item=f"interaction:probe:{CLAUSE}:scope:neg", state="CLASSIFY")
    out = ClassifyExecutor().run(_ctx(repo, workers), item)
    _check(item, out)
    assert out.next_state == "AUTHORING" and not out.attempts
    assert workers["claude"].calls == workers["codex"].calls == 0


def test_both_behavioral_goes_to_authoring_and_both_saw_the_same_packet(repo):
    # The line is authored first; the encoding anchors on it (second pilot).
    workers = _workers(claude=[_cls("behavioral")],
                       codex=[_cls("behavioral", clauses=(CLAUSE, "ST23-04#inherited#0"))])
    item = ItemRecord(item=ITEM, state="CLASSIFY")
    out = ClassifyExecutor().run(_ctx(repo, workers), item)
    _check(item, out)
    assert out.next_state == "AUTHORING"
    a, b = workers["claude"].received[0], workers["codex"].received[0]
    assert a.prompt == b.prompt and a.references == b.references and a.schema_path == b.schema_path
    assert "Can the -5000 target a Tamer?" in a.prompt
    assert {x.assignment for x in out.attempts} == {"routed", "forced"}
    assert all(x.outcome == "accepted" and x.stage == "classify_qa" for x in out.attempts)
    assert out.data["classification"]["examined_clauses"] == [CLAUSE, "ST23-04#inherited#0"]
    assert not out.corrections


def test_agreed_textual_ends_through_the_termination_check_without_another_call(repo):
    workers = _workers(claude=[_cls("textual", "qa:Q77 traits field")], codex=[_cls("textual", "qa:Q77 card data")])
    ctx = _ctx(repo, workers, cmds=FakeCommands().on("verdict-set", 0, "verdict-set: qa:Q77 -> unavailable"))
    item = ItemRecord(item=ITEM, state="CLASSIFY")
    out = ClassifyExecutor().run(ctx, item)
    _check(item, out)
    assert out.next_state == "TERMINATION_CHECK" and out.data["terminal"] == "textual"
    assert out.data["termination"]["agreed"] is True
    nxt = ItemRecord(item=ITEM, state="TERMINATION_CHECK", data=dict(out.data))
    done = TerminationCheckExecutor().run(ctx, nxt)
    assert done.next_state == "TERMINAL" and done.adjudicated and done.data["terminal"] == "textual"
    assert workers["claude"].calls == workers["codex"].calls == 1


def test_terminating_disagreement_escalates_with_both_arguments(repo):
    workers = _workers(claude=[_cls("textual")], codex=[_cls("not_examinable")])
    item = ItemRecord(item=ITEM, state="CLASSIFY")
    out = ClassifyExecutor().run(_ctx(repo, workers), item)
    _check(item, out)
    assert out.next_state == "ESCALATED"
    assert {(a["family"], a["call"]) for a in out.escalation.arguments} == {
        ("claude", "textual"), ("codex", "not_examinable")}
    assert len(out.corrections) == 2 and all(c.kind == "family_disagreement" for c in out.corrections)
    assert all(a.outcome == "escalated" for a in out.attempts)


def test_one_behavioral_call_is_enough_to_examine(repo):
    workers = _workers(claude=[_cls("behavioral")], codex=[_cls("textual")])
    out = ClassifyExecutor().run(_ctx(repo, workers), ItemRecord(item=ITEM, state="CLASSIFY"))
    assert out.next_state == "AUTHORING"
    assert len(out.corrections) == 2      # the disagreement is still recorded


def test_a_terminating_class_without_the_ruling_citation_escalates(repo):
    workers = _workers(claude=[_cls("textual")], codex=[_cls("textual", citation="card data")])
    out = ClassifyExecutor().run(_ctx(repo, workers), ItemRecord(item=ITEM, state="CLASSIFY"))
    assert out.next_state == "ESCALATED" and "cite" in out.escalation.reason


def test_one_family_unavailable_escalates_before_any_call(repo):
    workers = _workers()
    ctx = _ctx(repo, workers)
    ctx.health.disable("codex", "quota")
    out = ClassifyExecutor().run(ctx, ItemRecord(item=ITEM, state="CLASSIFY"))
    assert out.next_state == "ESCALATED" and "single family" in out.escalation.reason
    assert workers["claude"].calls == 0 and not out.attempts


def test_quota_during_the_pair_escalates_as_single_family(repo):
    workers = _workers(claude=[_cls("behavioral")], codex=[failed("quota_exhausted", "usage limit")])
    out = ClassifyExecutor().run(_ctx(repo, workers), ItemRecord(item=ITEM, state="CLASSIFY"))
    assert out.next_state == "ESCALATED" and "single family" in out.escalation.reason
    assert [a.outcome for a in out.attempts] == ["accepted", "quota_exhausted"]


def test_a_failed_classification_call_defers(repo):
    bad = ok({"q_id": "Q77"})
    workers = _workers(claude=[_cls("behavioral")], codex=[bad, bad])
    with pytest.raises(StageDeferred) as e:
        ClassifyExecutor().run(_ctx(repo, workers), ItemRecord(item=ITEM, state="CLASSIFY"))
    assert [c.kind for c in e.value.outcome.corrections] == ["schema_invalid"]
    # the valid sibling answer is ledgered, not blamed; the stage re-runs both
    assert [a.outcome for a in e.value.outcome.attempts] == ["accepted", "schema_invalid"]
    assert e.value.outcome.next_state == "CLASSIFY"


# ================================================================ ENCODE

BLOCK = {"assert": [{"at": 0, "that": [{"key": "p1.field", "value_json": "[]"}]}]}


def _author(block=BLOCK, line_exercises_ruling=None, reasoning="the Tamer stays on the field untouched"):
    return ok({"q_id": "Q77", "mode": "author", "expect_ruling": block, "agrees": None,
               "line_exercises_ruling": line_exercises_ruling,
               "answer_quote": "It only affects Digimon.", "reasoning": reasoning})


def _verify(agrees=True, line_exercises_ruling=None, reasoning="the block asserts exactly the answer"):
    return ok({"q_id": "Q77", "mode": "verify", "expect_ruling": None, "agrees": agrees,
               "line_exercises_ruling": line_exercises_ruling,
               "answer_quote": "It only affects Digimon.", "reasoning": reasoning})


AUTHORED = "qa/dcgo-exams/ST23/ST23-04-qa-Q77.yaml"


def _enc_item(**data):
    # Since the second pilot the line is AUTHORED before it is encoded: the
    # block anchors on the line that exercises the ruling, not on a library
    # line that never reaches the situation (60% of pilot encodings escalated
    # on exactly that).
    d = {"classification": {"q_id": "Q77", "examined_clauses": [CLAUSE]}, "scenario_paths": [AUTHORED]}
    d.update(data)
    return ItemRecord(item=ITEM, state="ENCODE", data=d)


@pytest.fixture
def authored(repo):
    p = repo / AUTHORED
    p.write_text(SCENARIO.replace("seed: 1\n", 'interaction: {id: "qa:Q77", source: qa, kind: positive}\nseed: 1\n'),
                 encoding="utf-8")
    return repo


def test_encoding_author_and_blind_verifier_agree(authored):
    workers = _workers(claude=[_author()], codex=[_verify(True)])
    item = _enc_item()
    out = EncodeExecutor().run(_ctx(authored, workers), item)
    _check(item, out)
    assert out.next_state == "SIM"
    assert out.data["expect_ruling"] == {"q_id": "Q77", "assert": [{"at": 0, "that": {"p1.field": []}}]}
    assert out.data["base_scenario"] == AUTHORED
    author_pkt, verify_pkt = workers["claude"].received[0], workers["codex"].received[0]
    assert "ST23-04-qa-Q77.yaml" in author_pkt.prompt and "qa:Q77" in author_pkt.prompt
    # the verifier sees the candidate block, never the author's argument
    assert '"candidate"' in verify_pkt.prompt and "the Tamer stays on the field untouched" not in verify_pkt.prompt
    (a, v) = out.attempts
    assert (a.family, a.assignment, a.outcome) == ("claude", "routed", "accepted")
    assert (v.family, v.assignment, v.parent_attempt) == ("codex", "forced", a.attempt_id)


def test_without_an_authored_line_an_agreed_encoding_still_precedes_authoring(repo):
    # Items encoded under the earlier order (in flight at the upgrade) keep it.
    workers = _workers(claude=[_author()], codex=[_verify(True)])
    out = EncodeExecutor().run(_ctx(repo, workers), _enc_item(scenario_paths=[]))
    assert out.next_state == "AUTHORING" and out.data["base_scenario"] == SC


def test_a_verifier_who_finds_the_line_not_exercising_the_ruling_sends_it_back_to_authoring(authored):
    workers = _workers(claude=[_author()], codex=[_verify(False, line_exercises_ruling=False,
                                                           reasoning="step 3 is a normal digivolution; Fly Bullet is never used")])
    item = _enc_item()
    out = EncodeExecutor().run(_ctx(authored, workers), item)
    _check(item, out)
    assert out.next_state == "AUTHORING"
    assert out.data["expect_ruling"] is None
    assert "Fly Bullet is never used" in out.data["encode_feedback"]
    assert [a.outcome for a in out.attempts] == ["gate_failed", "accepted"]
    assert len(out.corrections) == 2 and out.escalation is None


def test_a_verifier_who_rejects_the_block_itself_lets_it_be_re_encoded(authored):
    workers = _workers(claude=[_author()], codex=[_verify(False, reasoning="memory 3 is the setup, not the outcome")])
    item = _enc_item()
    out = EncodeExecutor().run(_ctx(authored, workers), item)
    _check(item, out)
    assert out.next_state == "ENCODE", "the stage cap bounds the re-encodes"
    assert "memory 3 is the setup" in out.data["encode_feedback"]
    assert [a.outcome for a in out.attempts] == ["gate_failed", "accepted"]


def test_an_author_who_says_the_line_misses_the_ruling_sends_it_back_without_a_verifier(authored):
    workers = _workers(claude=[_author(block=None, line_exercises_ruling=False,
                                       reasoning="the line never plays the Option the question is about")])
    item = _enc_item()
    out = EncodeExecutor().run(_ctx(authored, workers), item)
    _check(item, out)
    assert out.next_state == "AUTHORING" and workers["codex"].calls == 0
    assert "never plays the Option" in out.data["encode_feedback"]
    assert [a.outcome for a in out.attempts] == ["accepted"]


def test_the_encoder_sees_the_earlier_rejection(authored):
    workers = _workers(claude=[_author()], codex=[_verify(True)])
    EncodeExecutor().run(_ctx(authored, workers), _enc_item(encode_feedback="verifier: memory 3 is the setup"))
    assert "memory 3 is the setup" in workers["claude"].received[0].prompt


def test_a_disagreement_without_an_authored_line_escalates(repo):
    workers = _workers(claude=[_author()], codex=[_verify(False)])
    item = _enc_item(scenario_paths=[])
    out = EncodeExecutor().run(_ctx(repo, workers), item)
    _check(item, out)
    assert out.next_state == "ESCALATED" and "verifier disagrees" in out.escalation.reason
    assert len(out.escalation.arguments) == 2 and len(out.corrections) == 2


def test_encoding_needs_both_families(repo):
    workers = _workers()
    ctx = _ctx(repo, workers)
    ctx.health.disable("claude", "quota")
    out = EncodeExecutor().run(ctx, _enc_item())
    assert out.next_state == "ESCALATED" and "single family" in out.escalation.reason
    assert workers["codex"].calls == 0


def test_probe_interactions_have_nothing_to_encode(repo):
    item = ItemRecord(item=f"interaction:probe:{CLAUSE}:scope", state="ENCODE")
    out = EncodeExecutor().run(_ctx(repo, _workers()), item)
    assert out.next_state == "AUTHORING"


def test_a_malformed_author_block_is_a_gate_failure_not_a_verification(repo):
    # The author's correction, retried in ENCODE under the stage cap -- not a
    # deferral: deferring escalated Q2671 after a single malformed reply.
    bad = ok({"q_id": "Q77", "mode": "author", "expect_ruling": BLOCK, "agrees": True, "line_exercises_ruling": None,
              "answer_quote": "x", "reasoning": "a reasoning long enough to pass"})
    workers = _workers(claude=[bad])
    o = EncodeExecutor().run(_ctx(repo, workers), _enc_item())
    assert workers["codex"].calls == 0
    assert o.next_state == "ENCODE" and "malformed" in o.reason
    assert [a.outcome for a in o.attempts] == ["gate_failed"]
    assert [(c.kind, c.by_gate) for c in o.corrections] == [("gate_fail", "encode_validate")]
