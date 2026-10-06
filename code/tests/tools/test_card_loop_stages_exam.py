"""Exam stages: AUTHORING, SIM, ORACLE (+ the prompt-mismatch router), DIVERGED,
TRIAGE, TERMINATION_CHECK (D5) and FIX. No real subprocess or model runs."""
from __future__ import annotations

import json

import pytest

from tools.card_loop.driver_contracts import ItemRecord, check_transition
from tools.card_loop.ledger import Attempt, AttemptLedger
from tools.card_loop.contracts import Usage
from tools.card_loop.stages.authoring import AuthoringExecutor
from tools.card_loop.stages.base import StageDeferred
from tools.card_loop.stages.fix import FixExecutor
from tools.card_loop.stages.oracle import OracleExecutor
from tools.card_loop.stages.sim import SimExecutor
from tools.card_loop.stages.termination import TerminationCheckExecutor
from tools.card_loop.stages.triage import DivergedExecutor, TriageExecutor
from tools.card_loop.stages.testing import FakeCommands, failed, make_ctx, ok, sources
from tools.card_loop.workers.fake import FakeWorker

CLAUSE = "ST23-04#effect#0"
SC = "qa/dcgo-exams/ST23/ST23-04-effect0.yaml"
POOL = "qa/dcgo-exams/ST23/glowing_dawn_pool.json"
CLAUSE_ROW = {"id": CLAUSE, "card_id": "ST23-04", "zone": "effect", "label": "Effect",
              "kind": "timing", "timings": ["On Play"], "keyword": None,
              "text": "[On Play] 1 of your opponent's Digimon gets -5000 DP for the turn. <Rush>"}

SCENARIO = """card: ST23-04
clause: ST23-04#effect#0
seed: 1
decks:
  p0: {stack: [ST23-04], rest: gd}
  p1: {stack: [], rest: quiet}
steps:
  - actor: 0
    do: {hatch: {}}
    expect: {prompt: breeding_action}
  - actor: 0
    do: {play: {card: ST23-04, from: hand}}
    expect: {prompt: main_phase}
  - actor: 0
    do: {select: {targets: [opp.field.0]}}
    expect: {prompt: SelectPermanentEffect}
  - actor: 0
    do: {pass: {}}
    expect: {prompt: main_phase}
"""

SIM_PASS = ("exam: x\n  lowered 4 step(s): []\n  assert: 0 check(s) over 0 assertion block(s), 0 failed\n"
            "exam: scenarios seen 1 / lowered 1 / run 1 / diffed 0 / failed 0\n")
SIM_FAIL = ("exam: x\n  FAILED: step 2: select target opp.field.0 is not a legal candidate\n"
            "exam: scenarios seen 1 / lowered 0 / run 0 / diffed 0 / failed 1\n")


@pytest.fixture
def repo(tmp_path):
    (tmp_path / "data" / "card_bundles").mkdir(parents=True)
    (tmp_path / "data" / "card_bundles" / "ST23-04.md").write_text("# Murasamemon  (ST23-04)\n", encoding="utf-8")
    (tmp_path / "data" / "starter_decks.json").write_text(json.dumps({"starter_decks": [{"id": "st1"}]}),
                                                          encoding="utf-8")
    d = tmp_path / "qa" / "dcgo-exams" / "ST23"
    d.mkdir(parents=True)
    (d / "glowing_dawn_pool.json").write_text(json.dumps({"decks": [{"name": "gd"}, {"name": "quiet"}]}),
                                              encoding="utf-8")
    (d / "NOTES-GLOWING-DAWN.md").write_text("notes\n", encoding="utf-8")
    return tmp_path


def _write_scenario(repo, rel=SC, text=SCENARIO):
    p = repo / rel
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text, encoding="utf-8")
    return rel


def _workers(claude=(), codex=()):
    return {"claude": FakeWorker(list(claude), family="claude"),
            "codex": FakeWorker(list(codex), family="codex")}


def _ctx(repo, workers=None, cmds=None, **kw):
    return make_ctx(repo, workers=workers or _workers(), run_command=cmds or FakeCommands(),
                    sources=kw.pop("sources", sources(clauses=[CLAUSE_ROW])), **kw)


def _item(state, item=f"clause:{CLAUSE}", **data):
    return ItemRecord(item=item, state=state, data=data)


def _check(item, out):
    check_transition(item.kind, item.state, out.next_state)


AUTHOR_OK = {"scenario_paths": [SC.replace("/", "\\")], "covers": [CLAUSE], "notes": ""}

# ================================================================ AUTHORING (clause)


def test_clause_authoring_routes_writes_the_canonical_path_and_merges(repo):
    workers = _workers(claude=[ok(AUTHOR_OK, artifacts={"diff": "d", "manifest": "m"})])
    item = _item("AUTHORING")
    out = AuthoringExecutor().run(_ctx(repo, workers), item)
    _check(item, out)
    assert out.next_state == "SIM"
    assert out.data["scenario_paths"] == [SC]            # normalised to POSIX
    assert out.data["author_attempt"] == "att-0001" and out.data["author_family"] == "claude"
    assert out.merge_request.engine is False and out.merge_request.artifacts == {"diff": "d", "manifest": "m"}
    (a,) = out.attempts
    assert (a.stage, a.outcome, a.assignment) == ("author_clause", "accepted", "routed")
    pkt = workers["claude"].received[0]
    for needle in (SC, "-5000 DP", "meta: {produced_by: att-0001}", "exam_authoring_guide",
                   "exam_validate", "Rush", "NOTES-GLOWING-DAWN.md"):
        assert needle in pkt.prompt, needle
    assert "data/card_bundles/ST23-04.md" in pkt.references


def test_an_existing_library_scenario_is_reused_without_a_model_call(repo):
    _write_scenario(repo)
    workers = _workers()
    item = _item("AUTHORING")
    out = AuthoringExecutor().run(_ctx(repo, workers), item)
    _check(item, out)
    assert out.next_state == "SIM" and out.data["scenario_paths"] == [SC]
    assert not out.attempts and out.merge_request is None
    assert workers["claude"].calls == 0


def test_reauthoring_carries_the_sim_failure_and_overwrites_the_same_file(repo):
    _write_scenario(repo)
    workers = _workers(claude=[ok(AUTHOR_OK)])
    item = _item("AUTHORING", scenario_paths=[SC], author_attempt="att-old",
                 sim_failure={SC: ["FAILED: step 2: select target opp.field.0 is not a legal candidate"]})
    out = AuthoringExecutor().run(_ctx(repo, workers), item)
    pkt = workers["claude"].received[0]
    assert "not a legal candidate" in pkt.prompt and SC in pkt.prompt
    assert out.data["sim_failure"] is None


def test_no_legal_line_escalates_with_the_authors_reason(repo):
    workers = _workers(claude=[ok({"scenario_paths": [], "covers": [], "notes": "needs 2 copies in trash"})])
    item = _item("AUTHORING")
    out = AuthoringExecutor().run(_ctx(repo, workers), item)
    _check(item, out)
    assert out.next_state == "ESCALATED" and "needs 2 copies in trash" in out.escalation.reason


# ================================================================ AUTHORING (interaction)

PROBE = "probe:ST23-04#effect#0:scope:neg"
DENOM = {"version": 1, "interactions": {
    PROBE: {"source": "probe", "card_ids": ["ST23-04"], "kind": "negative", "gating": True,
            "clause_id": CLAUSE, "family": "scope", "why": "targets 1 of your opponent's Digimon"},
    "qa:Q77": {"source": "qa", "card_ids": ["ST23-04"], "kind": "positive", "gating": True, "q_id": "Q77"}}}
CARD_QA = {"qa": {"Q77": {"q_id": "Q77", "date": "2025-01-01", "question": "Does the -5000 apply to a Tamer?",
                          "answer": "No, it only affects Digimon.", "card_ids": ["ST23-04"]}}}
INT_OK = {"scenario_paths": ["qa/dcgo-exams/ST23/ST23-04-effect0-scope-neg.yaml"], "covers": [CLAUSE],
          "notes": ""}


def _isrc():
    return sources(clauses=[CLAUSE_ROW], card_qa=CARD_QA, denominator=DENOM)


def test_interaction_author_is_forced_away_from_the_implementer(repo):
    from tools.card_loop.driver_contracts import ItemRecord as IR
    workers = _workers(codex=[ok(INT_OK)])
    items = {"card:ST23-04": IR(item="card:ST23-04", state="IMPLEMENTED", data={"implementer_family": "claude"})}
    ctx = _ctx(repo, workers, sources=_isrc(), items=items)
    item = _item("AUTHORING", item=f"interaction:{PROBE}")
    out = AuthoringExecutor().run(ctx, item)
    _check(item, out)
    assert workers["codex"].calls == 1 and workers["claude"].calls == 0
    (a,) = out.attempts
    assert (a.stage, a.assignment, a.family) == ("author_interaction", "forced", "codex")
    pkt = workers["codex"].received[0]
    assert "FIND A DIVERGENCE" in pkt.prompt and "NEGATIVE" in pkt.prompt
    assert 'id: "probe:ST23-04#effect#0:scope:neg"' in pkt.prompt and "kind: negative" in pkt.prompt


def test_the_implementer_is_read_from_the_attempt_ledger(repo):
    ledger = AttemptLedger(repo / "attempts.jsonl")
    ledger.append(Attempt(attempt_id="att-x", run_id="old", ts="t", stage="implement", item="card:ST23-04",
                          family="codex", model=None, effort=None, prompt_version="1",
                          assignment="routed", outcome="accepted", usage=Usage()))
    workers = _workers(claude=[ok(INT_OK)])
    ctx = _ctx(repo, workers, sources=_isrc(), ledger=ledger)
    AuthoringExecutor().run(ctx, _item("AUTHORING", item=f"interaction:{PROBE}"))
    assert workers["claude"].calls == 1


def test_forced_author_unavailable_defers_never_falls_back_to_the_implementer(repo):
    from tools.card_loop.driver_contracts import ItemRecord as IR
    items = {"card:ST23-04": IR(item="card:ST23-04", state="IMPLEMENTED", data={"implementer_family": "claude"})}
    ctx = _ctx(repo, _workers(), sources=_isrc(), items=items)
    ctx.health.disable("codex", "quota")
    with pytest.raises(StageDeferred, match="implementer"):
        AuthoringExecutor().run(ctx, _item("AUTHORING", item=f"interaction:{PROBE}"))
    assert ctx.workers["claude"].calls == 0


def test_qa_interaction_author_gets_the_ruling_and_the_agreed_block(repo):
    workers = _workers(claude=[ok(dict(INT_OK, scenario_paths=["qa/dcgo-exams/ST23/ST23-04-qa-Q77.yaml"]))])
    block = {"q_id": "Q77", "assert": [{"at": 3, "that": {"p1.field": []}}]}
    item = _item("AUTHORING", item="interaction:qa:Q77", expect_ruling=block, base_scenario=SC)
    out = AuthoringExecutor().run(_ctx(repo, workers, sources=_isrc()), item)
    assert out.next_state == "SIM"
    pkt = workers["claude"].received[0]
    assert "only affects Digimon" in pkt.prompt and "verbatim" in pkt.prompt
    assert "q_id: Q77" in pkt.prompt and "p1.field" in pkt.prompt
    assert "qa/dcgo-exams/ST23/ST23-04-qa-Q77.yaml" in pkt.prompt


# ================================================================ SIM


def test_sim_pass_moves_to_the_oracle_with_the_deck_book(repo):
    _write_scenario(repo)
    cmds = FakeCommands().on("--sim-only", 0, SIM_PASS)
    item = _item("SIM", scenario_paths=[SC], author_attempt="att-a")
    ctx = _ctx(repo, cmds=cmds)
    out = SimExecutor().run(ctx, item)
    _check(item, out)
    assert out.next_state == "ORACLE"
    assert out.data["deck_books"] == {SC: POOL}
    (call,) = cmds.calls
    assert call["cwd"] == str(repo) and "--decks" in call["argv"] and SC in call["argv"]
    assert not out.corrections


def test_sim_failure_returns_to_authoring_as_an_author_correction(repo):
    _write_scenario(repo)
    cmds = FakeCommands().on("--sim-only", 1, SIM_FAIL)
    item = _item("SIM", scenario_paths=[SC], author_attempt="att-a")
    out = SimExecutor().run(_ctx(repo, cmds=cmds), item)
    _check(item, out)
    assert out.next_state == "AUTHORING"
    assert out.data["sim_failure"] == {SC: ["FAILED: step 2: select target opp.field.0 is not a legal candidate"]}
    (c,) = out.corrections
    assert (c.kind, c.corrected_attempt, c.by_gate, c.stage) == ("gate_fail", "att-a", "sim", "author_clause")


def test_sim_tries_the_next_book_like_exam_sim_all(repo):
    _write_scenario(repo)
    (repo / "data" / "starter_decks.json").write_text(
        json.dumps({"starter_decks": [{"id": "gd"}, {"id": "quiet"}]}), encoding="utf-8")
    cmds = FakeCommands().on(("--sim-only", "--decks"), 1, SIM_FAIL).on("--sim-only", 0, SIM_PASS)
    out = SimExecutor().run(_ctx(repo, cmds=cmds), _item("SIM", scenario_paths=[SC]))
    assert out.next_state == "ORACLE" and out.data["deck_books"] == {SC: "data/starter_decks.json"}


def test_a_harness_that_cannot_start_defers_and_blames_no_author(repo):
    _write_scenario(repo)
    cmds = FakeCommands().raise_on("--sim-only", "[WinError 2] not found")
    with pytest.raises(StageDeferred, match="WinError 2") as e:
        SimExecutor().run(_ctx(repo, cmds=cmds), _item("SIM", scenario_paths=[SC], author_attempt="att-a"))
    assert not e.value.outcome.corrections


AGREED = {"q_id": "Q77", "assert": [{"at": 1, "that": {"p1.field": []}}]}
QA_SCENARIO = SCENARIO.replace("seed: 1\n", 'interaction: {id: "qa:Q77", source: qa, kind: positive}\nseed: 1\n')


def test_sim_refuses_a_scenario_that_altered_the_agreed_ruling_block(repo):
    rel = "qa/dcgo-exams/ST23/ST23-04-qa-Q77.yaml"
    _write_scenario(repo, rel, QA_SCENARIO + "expect_ruling:\n  q_id: Q77\n  assert:\n"
                                              "    - at: 1\n      that: {p1.field: [ST1-02]}\n")
    cmds = FakeCommands().on("--sim-only", 0, SIM_PASS)
    item = _item("SIM", item="interaction:qa:Q77", scenario_paths=[rel], expect_ruling=AGREED,
                 author_attempt="att-a", author_stage="author_interaction")
    out = SimExecutor().run(_ctx(repo, cmds=cmds), item)
    _check(item, out)
    assert out.next_state == "AUTHORING"
    assert "expect_ruling" in out.data["sim_failure"][rel][0]
    assert out.corrections[0].stage == "author_interaction"


def test_sim_accepts_the_agreed_ruling_block_verbatim(repo):
    rel = "qa/dcgo-exams/ST23/ST23-04-qa-Q77.yaml"
    _write_scenario(repo, rel, QA_SCENARIO + "expect_ruling:\n  q_id: Q77\n  assert:\n"
                                              "    - at: 1\n      that: {p1.field: []}\n")
    cmds = FakeCommands().on("--sim-only", 0, SIM_PASS)
    item = _item("SIM", item="interaction:qa:Q77", scenario_paths=[rel], expect_ruling=AGREED)
    assert SimExecutor().run(_ctx(repo, cmds=cmds), item).next_state == "ORACLE"


def test_a_missing_scenario_file_is_an_authoring_failure(repo):
    out = SimExecutor().run(_ctx(repo), _item("SIM", scenario_paths=[SC]))
    assert out.next_state == "AUTHORING" and "not found" in out.data["sim_failure"][SC][0]


# ================================================================ ORACLE


def _row(**kw):
    base = {"scenario": SC, "clause": CLAUSE, "ids": [CLAUSE], "verdict": "confirmed",
            "first_divergence": None, "divergence": None, "reason": None, "denominator": "8 rows",
            "job_id": "exam-ST23-04-effect0", "job_outcome": "completed", "sidecar": "s.jsonl",
            "backfilled": True, "backfill_note": "ok", "recorded": [CLAUSE], "refused": []}
    base.update(kw)
    return json.dumps(base)


def _oracle_item(**data):
    d = {"scenario_paths": [SC], "deck_books": {SC: POOL}, "author_attempt": "att-a"}
    d.update(data)
    return _item("ORACLE", **d)


def test_confirmed_is_adjudicated_with_the_one_call_argv(repo):
    _write_scenario(repo)
    cmds = FakeCommands().on("--oracle", 0, "note: GO\n" + _row() + "\n", "exam --oracle: scenarios 1")
    item = _oracle_item()
    out = OracleExecutor().run(_ctx(repo, cmds=cmds), item)
    _check(item, out)
    assert out.next_state == "CONFIRMED" and out.adjudicated
    argv = cmds.calls[0]["argv"]
    for part in ("--oracle", "--build", "--verdicts", "--backfill", "--clause-text-json", POOL):
        assert part in argv, part
    assert not out.attempts


def test_diverged_carries_the_whole_result(repo):
    _write_scenario(repo)
    row = _row(verdict="diverged", divergence={"step": 2, "field": "p1.field", "ours": "[]", "dcgo": "[x]"},
               first_divergence="step 2: p1.field")
    cmds = FakeCommands().on("--oracle", 1, row + "\n")
    item = _oracle_item()
    out = OracleExecutor().run(_ctx(repo, cmds=cmds), item)
    _check(item, out)
    assert out.next_state == "DIVERGED"
    assert out.data["oracle"] == json.loads(row)


def test_unmeasured_retries_the_oracle_and_only_the_unmeasured_paths(repo):
    _write_scenario(repo)
    sc2 = _write_scenario(repo, "qa/dcgo-exams/ST23/ST23-04-effect0-b.yaml")
    cmds = (FakeCommands()
            .on(("--oracle", SC + " "), 0, _row() + "\n")
            .on(("--oracle", "effect0-b"), 1, _row(scenario=sc2, verdict="unmeasured",
                                                   reason="oracle job timed out", job_outcome=None) + "\n"))
    item = _oracle_item(scenario_paths=[SC, sc2], deck_books={SC: POOL, sc2: POOL})
    out = OracleExecutor().run(_ctx(repo, cmds=cmds), item)
    _check(item, out)
    assert out.next_state == "ORACLE" and "timed out" in out.reason
    assert out.data["oracle_retry_paths"] == [sc2]
    # the retry runs only the unmeasured scenario, and confirms the item
    cmds2 = FakeCommands().on("--oracle", 0, _row(scenario=sc2) + "\n")
    item2 = ItemRecord(item=item.item, state="ORACLE", data={**item.data, **out.data})
    out2 = OracleExecutor().run(_ctx(repo, cmds=cmds2), item2)
    assert out2.next_state == "CONFIRMED"
    assert len(cmds2.calls) == 1 and sc2 in cmds2.calls[0]["argv"]


def test_no_json_line_is_unmeasured_not_a_crash(repo):
    _write_scenario(repo)
    cmds = FakeCommands().on("--oracle", 2, "", "error: --oracle needs --root")
    out = OracleExecutor().run(_ctx(repo, cmds=cmds), _oracle_item())
    assert out.next_state == "ORACLE" and "--root" in out.reason


def test_refused_verdicts_escalate(repo):
    _write_scenario(repo)
    cmds = FakeCommands().on("--oracle", 1, _row(refused=["orphan clause id ST23-04#effect#9"]) + "\n")
    item = _oracle_item()
    out = OracleExecutor().run(_ctx(repo, cmds=cmds), item)
    _check(item, out)
    assert out.next_state == "ESCALATED" and "orphan" in out.escalation.reason


MISMATCH = "DCGO job failed: prompt mismatch: step 4 expected prompt 'main_phase' but DCGO asked 'OptionalSkill'"


def _inspect(kind=None, optional=None, n=0, step=3):
    snap = {"step": step, "pending_kind": kind, "pending_optional": optional, "pending_prompt": None,
            "candidates": [[i, None] for i in range(n)]}
    return ("exam: x\n  lowered 4 step(s): []\n"
            + json.dumps({"snapshot": snap, "projection": {}, "complete": True, "steps_run": 4}, indent=2)
            + "\n" + SIM_PASS)


def test_missing_decline_goes_to_triage_through_diverged(repo):
    _write_scenario(repo)
    cmds = (FakeCommands()
            .on("--oracle", 1, _row(verdict="unmeasured", job_outcome="failed", reason=MISMATCH) + "\n")
            .on("--inspect", 0, _inspect()))
    item = _oracle_item()
    out = OracleExecutor().run(_ctx(repo, cmds=cmds), item)
    _check(item, out)
    assert out.next_state == "DIVERGED"
    assert out.data["prompt_route"] == "engines_disagree"
    ev = out.data["prompt_evidence"]
    assert ev["expected"] == "main_phase" and ev["dcgo_asked"] == "OptionalSkill" and ev["ours"] == "<action>"
    # DCGO row 4 maps back to scenario step 3 (the last `main_phase` expect)
    (inspect_call,) = cmds.argvs("--inspect")
    assert inspect_call[inspect_call.index("--inspect") + 1] == "3"
    assert not out.corrections


def test_both_engines_contradicting_the_scenario_goes_back_to_authoring(repo):
    _write_scenario(repo)
    cmds = (FakeCommands()
            .on("--oracle", 1, _row(verdict="unmeasured", job_outcome="failed", reason=MISMATCH) + "\n")
            .on("--inspect", 0, _inspect("Replacement", True, 2)))
    item = _oracle_item()
    out = OracleExecutor().run(_ctx(repo, cmds=cmds), item)
    _check(item, out)
    assert out.next_state == "AUTHORING"
    assert out.data["prompt_route"] == "scenario_wrong"
    assert out.data["prompt_evidence"]["ours"] == "OptionalSkill"
    (c,) = out.corrections
    assert (c.kind, c.corrected_attempt, c.by_gate) == ("gate_fail", "att-a", "oracle_scenario")


def test_a_divergence_before_the_mismatch_is_a_plain_divergence(repo):
    _write_scenario(repo)
    row = _row(verdict="diverged", job_outcome="failed", reason="memory differs; " + MISMATCH,
               divergence={"step": 1, "field": "p0.memory", "ours": "1", "dcgo": "2"})
    cmds = FakeCommands().on("--oracle", 1, row + "\n")
    out = OracleExecutor().run(_ctx(repo, cmds=cmds), _oracle_item())
    assert out.next_state == "DIVERGED" and not cmds.argvs("--inspect")
    assert out.data.get("prompt_route") is None


# ================================================================ DIVERGED / TRIAGE

DIV_ROW = json.loads(_row(verdict="diverged", divergence={"step": 2, "field": "p1.field", "ours": "[]",
                                                          "dcgo": "[x]"}))


def _triage_item(**data):
    d = {"scenario_paths": [SC], "oracle": DIV_ROW, "oracle_results": {SC: DIV_ROW}, "history": ["att-a"]}
    d.update(data)
    return _item("TRIAGE", **d)


def test_diverged_moves_to_triage_without_a_model():
    item = _item("DIVERGED")
    out = DivergedExecutor().run(None, item)
    _check(item, out)
    assert out.next_state == "TRIAGE" and not out.attempts


TRI_OURS = {"classification": "ours_wrong", "citation": {"kind": "rule", "ref": "16-36"},
            "reasoning": "the -DP applies to Digimon only per 16-36"}
TRI_QUIRK = {"classification": "dcgo_quirk", "citation": {"kind": "rule", "ref": "7-3-1"},
             "reasoning": "DCGO asks an extra prompt the rules do not have"}


def test_triage_ours_wrong_goes_to_fix_and_keeps_the_packet(repo):
    _write_scenario(repo)
    workers = _workers(claude=[ok(TRI_OURS)])
    item = _triage_item()
    out = TriageExecutor().run(_ctx(repo, workers), item)
    _check(item, out)
    assert out.next_state == "FIX"
    assert out.data["triage_first"]["call"] == "ours_wrong"
    pkt = workers["claude"].received[0]
    assert out.data["triage_packet"]["prompt"] == pkt.prompt
    assert '"p1.field"' in pkt.prompt and SC in pkt.prompt
    assert out.attempts[0].outcome == "accepted"


def test_triage_quirk_goes_to_the_termination_check(repo):
    _write_scenario(repo)
    item = _triage_item()
    out = TriageExecutor().run(_ctx(repo, _workers(claude=[ok(TRI_QUIRK)])), item)
    _check(item, out)
    assert out.next_state == "TERMINATION_CHECK"
    assert out.data["triage_first"]["citation"] == {"kind": "rule", "ref": "7-3-1"}


def test_triage_undetermined_escalates(repo):
    _write_scenario(repo)
    res = {"classification": "undetermined", "citation": None, "reasoning": "the sources do not decide it"}
    item = _triage_item()
    out = TriageExecutor().run(_ctx(repo, _workers(claude=[ok(res)])), item)
    _check(item, out)
    assert out.next_state == "ESCALATED"
    (arg,) = out.escalation.arguments
    assert arg["call"] == "undetermined" and arg["family"] == "claude"


# ================================================================ TERMINATION_CHECK


def _term_item(first=TRI_QUIRK, item=f"clause:{CLAUSE}"):
    return ItemRecord(item=item, state="TERMINATION_CHECK", data={
        "scenario_paths": [SC], "oracle": DIV_ROW, "history": ["att-a", "att-t1"],
        "triage_packet": {"prompt": "THE SAME PACKET", "references": [SC], "prompt_version": "1"},
        "triage_first": {"family": "claude", "attempt_id": "att-t1", "call": first["classification"],
                         "citation": first["citation"], "reasoning": first["reasoning"]}})


def test_both_families_agreeing_with_citations_terminate_and_triage_the_store(repo):
    workers = _workers(codex=[ok(dict(TRI_QUIRK, citation={"kind": "dcgo", "ref": "ST23_04.cs:88"}))])
    cmds = FakeCommands().on("verdict-triage", 0, "verdict-triage: ok\n")
    item = _term_item()
    out = TerminationCheckExecutor().run(_ctx(repo, workers, cmds), item)
    _check(item, out)
    assert out.next_state == "TERMINAL" and out.adjudicated
    assert out.data["terminal"] == "dcgo_quirk"
    assert out.data["citation"] == "general_rule.pdf 7-3-1"
    pkt = workers["codex"].received[0]
    assert pkt.prompt == "THE SAME PACKET" and pkt.references == (SC,)
    (a,) = out.attempts
    assert (a.family, a.assignment, a.parent_attempt, a.outcome) == ("codex", "forced", "att-t1", "accepted")
    argv = cmds.calls[0]["argv"]
    assert argv[1:] == ["verdict-triage", "--clause", CLAUSE, "--triage", "dcgo_quirk",
                        "--citation", "general_rule.pdf 7-3-1"]


def test_disagreement_escalates_with_both_arguments(repo):
    workers = _workers(codex=[ok(TRI_OURS)])
    item = _term_item()
    out = TerminationCheckExecutor().run(_ctx(repo, workers), item)
    _check(item, out)
    assert out.next_state == "ESCALATED"
    calls = {(a["family"], a["call"]) for a in out.escalation.arguments}
    assert calls == {("claude", "dcgo_quirk"), ("codex", "ours_wrong")}
    assert {c.corrected_attempt for c in out.corrections} == {"att-t1", "att-0001"}
    assert all(c.kind == "family_disagreement" for c in out.corrections)
    assert out.attempts[0].outcome == "escalated"
    assert "att-t1" in out.escalation.history


def test_agreement_without_a_citation_escalates(repo):
    workers = _workers(codex=[ok(dict(TRI_QUIRK, citation=None))])
    out = TerminationCheckExecutor().run(_ctx(repo, workers), _term_item())
    assert out.next_state == "ESCALATED" and "citation" in out.escalation.reason


def test_one_family_available_escalates_without_a_call(repo):
    ctx = _ctx(repo)
    ctx.health.disable("codex", "quota")
    out = TerminationCheckExecutor().run(ctx, _term_item())
    assert out.next_state == "ESCALATED" and not out.attempts
    assert ctx.workers["claude"].calls == 0


def test_unreachable_terminates_but_the_store_has_no_writer_for_it(repo):
    unreachable = {"classification": "unreachable", "citation": {"kind": "rule", "ref": "4-1"},
                   "reasoning": "no legal line puts two copies in the breeding area"}
    workers = _workers(codex=[ok(unreachable)])
    cmds = FakeCommands()
    out = TerminationCheckExecutor().run(_ctx(repo, workers, cmds), _term_item(first=unreachable))
    assert out.next_state == "TERMINAL" and out.data["terminal"] == "unreachable"
    assert not cmds.calls and "not written" in out.data["verdict_store"]


def test_a_store_refusal_escalates_rather_than_diverging_silently(repo):
    workers = _workers(codex=[ok(TRI_QUIRK)])
    cmds = FakeCommands().on("verdict-triage", 1, "", "no stored verdict for `ST23-04#effect#0`")
    out = TerminationCheckExecutor().run(_ctx(repo, workers, cmds), _term_item())
    assert out.next_state == "ESCALATED" and "no stored verdict" in out.escalation.reason


def test_an_agreement_reached_by_classification_terminates_without_a_call(repo):
    item = ItemRecord(item="interaction:qa:Q77", state="TERMINATION_CHECK", data={
        "terminal": "textual", "citation": "qa:Q77 traits",
        "termination": {"agreed": True, "stage": "classify_qa", "terminal": "textual",
                        "citation": "qa:Q77 traits", "citations": ["qa:Q77 traits", "qa:Q77 card data"]}})
    ctx = _ctx(repo)
    out = TerminationCheckExecutor().run(ctx, item)
    _check(item, out)
    assert out.next_state == "TERMINAL" and out.adjudicated and out.data["terminal"] == "textual"
    assert ctx.workers["claude"].calls == 0 and ctx.workers["codex"].calls == 0


# ================================================================ FIX

FIX_OK = {"files": ["code/digimon-engine/cards/st23/ST23-04.yaml"], "tests": ["st23::st23_04::minus_dp"],
          "test_result_lines": ["test result: ok. 1 passed"], "gaps": [],
          "citation": {"kind": "rule", "ref": "16-36"}, "notes": ""}


def _fix_item(reasoning=TRI_OURS["reasoning"], **data):
    d = {"scenario_paths": [SC], "oracle": DIV_ROW,
         "triage_first": {"family": "claude", "attempt_id": "att-t1", "call": "ours_wrong",
                          "citation": TRI_OURS["citation"], "reasoning": reasoning}}
    d.update(data)
    return _item("FIX", **d)


def test_a_card_fix_goes_to_the_gate_with_a_card_merge(repo):
    workers = _workers(claude=[ok(FIX_OK, artifacts={"diff": "d"})])
    item = _fix_item()
    out = FixExecutor().run(_ctx(repo, workers), item)
    _check(item, out)
    assert out.next_state == "GATE" and out.data["engine_fix"] is False
    assert out.merge_request.engine is False and out.merge_request.artifacts == {"diff": "d"}
    (a,) = out.attempts
    assert a.stage == "fix_card" and a.parent_attempt == "att-t1"
    pkt = workers["claude"].received[0]
    assert "# produced_by: att-0001" in pkt.prompt and "16-36" in pkt.prompt
    assert "code/digimon-engine/cards/st23/ST23-04.yaml" in pkt.prompt


def test_triage_naming_an_engine_gap_routes_an_engine_fix(repo):
    workers = _workers(codex=[ok(FIX_OK)])
    item = _fix_item(reasoning="ours is wrong: the DSL vocabulary has no 'for each Tamer' scope (engine gap)")
    out = FixExecutor().run(_ctx(repo, workers), item)
    assert out.data["engine_fix"] is True and out.merge_request.engine is True
    assert out.attempts[0].stage == "fix_engine" and out.attempts[0].family == "codex"
    assert "engine_fix_reason" in out.data


def test_a_card_fix_that_finds_a_gap_becomes_an_engine_fix_in_the_same_step(repo):
    gap = {"kind": "dsl", "id": "G-DSL-EACH-TAMER", "summary": "no per-Tamer scope"}
    workers = _workers(claude=[ok(dict(FIX_OK, gaps=[gap]))], codex=[ok(FIX_OK)])
    out = FixExecutor().run(_ctx(repo, workers), _fix_item())
    assert out.next_state == "GATE"
    assert [a.stage for a in out.attempts] == ["fix_card", "fix_engine"]
    assert out.merge_request.engine is True and out.merge_request.gap_id == "G-DSL-EACH-TAMER"
    assert "G-DSL-EACH-TAMER" in workers["codex"].received[0].prompt


def test_a_fix_without_a_citation_escalates_as_a_finding(repo):
    workers = _workers(claude=[ok(dict(FIX_OK, citation={"kind": "rule", "ref": " "}))])
    item = _fix_item()
    out = FixExecutor().run(_ctx(repo, workers), item)
    _check(item, out)
    assert out.next_state == "ESCALATED" and "citation" in out.escalation.reason
    assert out.merge_request is None


def test_an_engine_fix_still_missing_substrate_escalates(repo):
    gap = {"kind": "engine", "id": "G-ENG-X", "summary": "needs a new trigger window"}
    workers = _workers(codex=[ok(dict(FIX_OK, gaps=[gap]))])
    out = FixExecutor().run(_ctx(repo, workers), _fix_item(engine_fix=True))
    assert out.next_state == "ESCALATED" and "G-ENG-X" in out.escalation.reason
