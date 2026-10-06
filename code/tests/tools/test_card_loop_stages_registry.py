"""The EXECUTORS registry against the driver contract, and two items walked
through the executors with a minimal driver loop (check_transition + data merge)."""
from __future__ import annotations

import json

import pytest

from tools.card_loop import driver_contracts as dc
from tools.card_loop.stages import EXECUTORS, StageDeferred, build_executors
from tools.card_loop.stages.testing import FakeCommands, make_ctx, ok, sources
from tools.card_loop.workers.fake import FakeWorker

WORK_STATES = {"IMPLEMENTING", "REVIEW", "CLASSIFY", "ENCODE", "AUTHORING", "SIM", "ORACLE",
               "DIVERGED", "TRIAGE", "TERMINATION_CHECK", "FIX"}


def test_every_working_state_has_exactly_one_executor():
    assert set(EXECUTORS) == WORK_STATES
    for state, ex in EXECUTORS.items():
        assert ex.state == state and callable(ex.run)
    # driver-owned states are left to the driver
    for state in ("PENDING", "GATE", "PARKED", "IMPLEMENTED", "CONFIRMED", "TERMINAL",
                  "UNAVAILABLE", "ESCALATED"):
        assert state not in EXECUTORS
    assert set(build_executors()) == set(EXECUTORS)


def test_every_worker_state_of_the_contract_is_covered():
    assert set(dc.STAGE_FOR_STATE) <= set(EXECUTORS)


def _drive(ctx, item, *, max_steps=12):
    """Apply executor outcomes the way the driver will: validate, merge data."""
    trail = [item.state]
    for _ in range(max_steps):
        if dc.is_terminal(item.kind, item.state) or item.state not in EXECUTORS:
            break
        out = EXECUTORS[item.state].run(ctx, item)
        dc.check_transition(item.kind, item.state, out.next_state)
        item = dc.ItemRecord(item=item.item, state=out.next_state, attempts=item.attempts,
                             data={**item.data, **out.data})
        trail.append(item.state)
    return item, trail


CLAUSE = "ST23-04#effect#0"
SC = "qa/dcgo-exams/ST23/ST23-04-effect0.yaml"


@pytest.fixture
def repo(tmp_path):
    (tmp_path / "data" / "card_bundles").mkdir(parents=True)
    (tmp_path / "data" / "card_bundles" / "ST23-04.md").write_text("# Murasamemon  (ST23-04)\n", encoding="utf-8")
    (tmp_path / "data" / "starter_decks.json").write_text(json.dumps({"starter_decks": []}), encoding="utf-8")
    d = tmp_path / "qa" / "dcgo-exams" / "ST23"
    d.mkdir(parents=True)
    (d / "pool.json").write_text(json.dumps({"decks": [{"name": "gd"}, {"name": "quiet"}]}), encoding="utf-8")
    return tmp_path


def test_a_clause_walks_author_sim_oracle_diverge_triage_to_terminal(repo):
    def author(packet):
        # the worker writes its scenario into the run tree (merge stands in)
        (repo / SC).write_text("card: ST23-04\nclause: ST23-04#effect#0\nseed: 1\ndecks:\n"
                               "  p0: {stack: [], rest: gd}\n  p1: {stack: [], rest: quiet}\n"
                               "steps:\n  - actor: 0\n    do: {hatch: {}}\n", encoding="utf-8")
        return ok({"scenario_paths": [SC], "covers": [CLAUSE], "notes": ""})

    quirk = {"classification": "dcgo_quirk", "citation": {"kind": "rule", "ref": "7-3-1"},
             "reasoning": "DCGO offers a prompt the rules do not have"}
    workers = {"claude": FakeWorker({"author_clause": author, "triage": [ok(quirk)]}, family="claude"),
               "codex": FakeWorker({"triage": [ok(dict(quirk, citation={"kind": "dcgo", "ref": "ST23_04.cs:9"}))]},
                                   family="codex")}
    diverged = {"scenario": SC, "clause": CLAUSE, "ids": [CLAUSE], "verdict": "diverged",
                "first_divergence": "step 0: phase", "divergence": {"step": 0, "field": "phase", "ours": "a",
                                                                    "dcgo": "b"},
                "reason": "phase differs", "job_id": "j", "job_outcome": "completed", "refused": [],
                "recorded": [CLAUSE]}
    cmds = (FakeCommands()
            .on("--sim-only", 0, "exam: scenarios seen 1 / lowered 1 / run 1 / diffed 0 / failed 0\n")
            .on("--oracle", 1, json.dumps(diverged) + "\n")
            .on("verdict-triage", 0, "ok\n"))
    ctx = make_ctx(repo, workers=workers, run_command=cmds,
                   sources=sources(clauses=[{"id": CLAUSE, "card_id": "ST23-04", "label": "Effect",
                                             "text": "[On Play] ..."}]))
    item, trail = _drive(ctx, dc.ItemRecord(item=f"clause:{CLAUSE}", state="AUTHORING"))
    assert trail == ["AUTHORING", "SIM", "ORACLE", "DIVERGED", "TRIAGE", "TERMINATION_CHECK", "TERMINAL"]
    assert item.data["terminal"] == "dcgo_quirk" and item.data["citation"] == "general_rule.pdf 7-3-1"
    assert item.data["history"] == ["att-0001", "att-0002", "att-0003"]


def test_a_card_walks_implement_reject_implement_accept(repo):
    impl = ok({"files": [], "tests": [], "test_result_lines": [], "gaps": [], "notes": ""})
    workers = {"claude": FakeWorker({"implement": [impl, impl]}, family="claude"),
               "codex": FakeWorker({"review": [
                   ok({"verdict": "reject", "directives": [{"path": None, "directive": "x"}], "summary": "s"}),
                   ok({"verdict": "accept", "directives": [], "summary": "s"})]}, family="codex")}
    ctx = make_ctx(repo, workers=workers, sources=sources())
    item, trail = _drive(ctx, dc.ItemRecord(item="card:ST23-04", state="IMPLEMENTING"))
    assert trail == ["IMPLEMENTING", "REVIEW", "IMPLEMENTING", "REVIEW", "IMPLEMENTED"]
    assert item.data["implementer_family"] == "claude"


def test_stage_deferred_is_the_package_export():
    from tools.card_loop.stages.base import StageDeferred as SD
    assert StageDeferred is SD
