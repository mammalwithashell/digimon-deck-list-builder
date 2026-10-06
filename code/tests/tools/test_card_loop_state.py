"""Item state for a card-loop run (openspec add-card-authoring-loop, design D4, task 6.1).

Items come from the plan's work set and the committed ledgers (exam verdicts,
the interaction denominator, the escalation queue, the YAML specs); a run's
`events.jsonl` replays on top of them on resume.
"""
from __future__ import annotations

import json

import pytest

from tools.card_loop import escalations as esc_mod
from tools.card_loop import state as st
from tools.card_loop.driver_contracts import Escalation, Event

# Real card ids (the clause extractor reads data/cards.json):
#   BT8-084  effect#0..#2        -- has a spec; confirmed / dcgo_quirk / unreachable
#   BT7-056  effect#0, inherited#0 -- has a spec; diverged-undetermined / escalated
#   EX12-004 inherited#0         -- has a spec; DCGO has no script (plan unavailable)
#   EX12-073 effect#0..#2, security#0 -- NO spec: a card item to implement
POOL = ["BT7-056", "EX12-004", "BT8-084", "EX12-073"]
CORE = ["BT8-084", "EX12-073"]
RANKING = ["EX12-073", "BT7-056", "BT8-084", "EX12-004"]


def _clause_row(clause_id, verdict, **extra):
    return {"clause_id": clause_id, "card_id": clause_id.split("#")[0], "verdict": verdict,
            "recorded_at": "2026-10-01T00:00:00Z", **extra}


def _interaction(iid, cards, *, gating=True, source=None, sha=None):
    source = source or iid.split(":", 1)[0]
    return {"card_ids": list(cards), "gating": gating, "kind": "positive", "source": source,
            "text_sha256": sha or f"sha-{iid}"}


@pytest.fixture
def ws(tmp_path):
    cards = tmp_path / "cards"
    for set_dir, card in (("bt8", "BT8-084"), ("bt7", "BT7-056"), ("ex12", "EX12-004")):
        (cards / set_dir).mkdir(parents=True, exist_ok=True)
        (cards / set_dir / f"{card}.yaml").write_text(f"card: {card}\n", encoding="utf-8")
    scenarios = tmp_path / "scenarios"
    scenarios.mkdir()
    verdicts = tmp_path / "verdicts"
    verdicts.mkdir()
    (verdicts / "BT8-084.json").write_text(json.dumps({
        "version": 2,
        "clauses": {
            "BT8-084#effect#0": _clause_row("BT8-084#effect#0", "confirmed"),
            "BT8-084#effect#1": _clause_row("BT8-084#effect#1", "diverged", triage="dcgo_quirk",
                                            citation="general_rule.pdf 15-1-2"),
            "BT8-084#effect#2": _clause_row("BT8-084#effect#2", "unreachable",
                                            reason="no legal line: needs 2 copies in the breeding area"),
        },
        "interactions": {
            "probe:BT8-084#effect#1:optional_decline": {
                "interaction_id": "probe:BT8-084#effect#1:optional_decline", "card_ids": ["BT8-084"],
                "source": "probe", "kind": "positive", "verdict": "confirmed",
                "text_sha256": "sha-probe:BT8-084#effect#1:optional_decline",
                "recorded_at": "2026-10-01T00:00:00Z"},
        },
    }), encoding="utf-8")
    (verdicts / "BT7-056.json").write_text(json.dumps({
        "version": 1,
        "clauses": {"BT7-056#effect#0": _clause_row("BT7-056#effect#0", "diverged",
                                                    triage="undetermined")},
    }), encoding="utf-8")

    denominator = tmp_path / "interaction_denominator.json"
    interactions = {
        "qa:Q100": _interaction("qa:Q100", ["BT7-056", "EX12-004"]),
        "qa:Q200": _interaction("qa:Q200", ["EX12-073"]),
        "qa:Q300": _interaction("qa:Q300", ["BT9-999", "BT8-084"]),
        "probe:EX12-004#inherited#0:scope": _interaction("probe:EX12-004#inherited#0:scope", ["EX12-004"]),
        "probe:BT8-084#effect#1:optional_decline": _interaction(
            "probe:BT8-084#effect#1:optional_decline", ["BT8-084"]),
        "probe:BT7-056#effect#0:timing_gate:neg": _interaction(
            "probe:BT7-056#effect#0:timing_gate:neg", ["BT7-056"], gating=False),
    }
    per_card = {c: [] for c in POOL + ["BT9-999"]}
    for iid, row in interactions.items():
        for c in row["card_ids"]:
            per_card[c].append(iid)
    denominator.write_text(json.dumps({"version": 1, "cards": per_card,
                                       "interactions": interactions}), encoding="utf-8")

    escalations = tmp_path / "escalations"
    esc_mod.write_escalation(escalations, Escalation(item="clause:BT7-056#inherited#0",
                                                     reason="families disagree"),
                             run_id="older-run", ts="2026-10-01T00:00:00Z")

    paths = st.LedgerPaths(cards_dir=cards, scenarios_dir=scenarios, verdicts_dir=verdicts,
                           denominator_path=denominator, escalations_dir=escalations)
    plan = {"version": 1, "run_id": "r1", "base_sha": "abc",
            "work_set": {"pool": POOL, "core": CORE, "ranking": RANKING, "decklists": None},
            "dcgo": {"unavailable": ["EX12-004"]}}
    run_dir = tmp_path / "runs" / "r1"
    return {"paths": paths, "plan": plan, "run_dir": run_dir, "tmp": tmp_path}


def _clock():
    n = [0]

    def now():
        n[0] += 1
        return f"2026-10-05T00:00:{n[0]:02d}Z"
    return now


def _open(ws, **kw):
    return st.RunState.open(ws["plan"], ws["paths"], run_dir=ws["run_dir"], now=_clock(), **kw)


def _events(ws):
    return [json.loads(line) for line in
            (ws["run_dir"] / "events.jsonl").read_text(encoding="utf-8").splitlines() if line.strip()]


# --------------------------------------------------------------------------- construction


def test_items_and_initial_states_come_from_the_plan_and_the_committed_ledgers(ws):
    s = _open(ws)
    states = {i: r.state for i, r in s.records.items()}
    assert states == {
        "card:EX12-073": "PENDING",
        "clause:EX12-073#effect#0": "PENDING",
        "clause:EX12-073#effect#1": "PENDING",
        "clause:EX12-073#effect#2": "PENDING",
        "clause:EX12-073#security#0": "PENDING",
        "clause:BT8-084#effect#0": "CONFIRMED",
        "clause:BT8-084#effect#1": "TERMINAL",
        "clause:BT8-084#effect#2": "TERMINAL",
        "clause:BT7-056#effect#0": "DIVERGED",
        "clause:BT7-056#inherited#0": "ESCALATED",
        "clause:EX12-004#inherited#0": "UNAVAILABLE",
        "interaction:qa:Q100": "PENDING",
        "interaction:qa:Q200": "PENDING",
        "interaction:qa:Q300": "PENDING",
        "interaction:probe:EX12-004#inherited#0:scope": "UNAVAILABLE",
        "interaction:probe:BT8-084#effect#1:optional_decline": "CONFIRMED",
    }, "one card item per unimplemented card; no item for a non-gating interaction"

    r = s.records
    assert r["clause:BT8-084#effect#1"].data["terminal"] == "dcgo_quirk"
    assert r["clause:BT8-084#effect#1"].data["citation"] == "general_rule.pdf 15-1-2"
    assert r["clause:BT8-084#effect#2"].data["terminal"] == "unreachable"
    assert "breeding area" in r["clause:BT8-084#effect#2"].data["reason"]
    assert r["clause:BT7-056#effect#0"].data["triage"] == "undetermined"

    m = s.meta
    assert m["clause:EX12-073#effect#0"].requires == ("card:EX12-073",)
    assert m["interaction:qa:Q200"].requires == ("card:EX12-073",)
    assert m["clause:BT8-084#effect#0"].requires == ()
    assert m["interaction:qa:Q100"].cards == ("BT7-056", "EX12-004")
    assert m["interaction:qa:Q100"].source == "qa"
    assert m["interaction:qa:Q300"].cards == ("BT8-084",), "only pool cards count"
    assert m["clause:BT8-084#effect#0"].is_core and not m["clause:BT7-056#effect#0"].is_core


def test_a_shared_ruling_is_unavailable_only_when_every_pool_card_is(ws):
    s = _open(ws)
    assert s.records["interaction:qa:Q100"].state == "PENDING"            # BT7-056 has a script
    assert s.records["interaction:probe:EX12-004#inherited#0:scope"].state == "UNAVAILABLE"
    assert s.records["clause:EX12-004#inherited#0"].data["unavailable_source"] == "plan"


def test_items_are_ordered_core_first_then_ranking_then_kind_then_id(ws):
    order = [r.item for r in _open(ws).ordered()]
    assert order[:6] == [
        "card:EX12-073",                      # core, rank 0, the card item first
        "clause:EX12-073#effect#0", "clause:EX12-073#effect#1", "clause:EX12-073#effect#2",
        "clause:EX12-073#security#0", "interaction:qa:Q200",
    ]
    assert order.index("clause:BT8-084#effect#2") < order.index("clause:BT7-056#effect#0"), \
        "core cards before the tail even when the tail ranks higher"
    assert order.index("interaction:qa:Q100") > order.index("clause:BT7-056#inherited#0"), \
        "a card's clauses before its interactions"


def test_counts_never_count_escalated_as_adjudicated(ws):
    c = _open(ws).counts()
    t = c["total"]
    assert t["confirmed"] == 2 and t["terminal"] == 2 and t["unavailable"] == 2
    assert t["escalated"] == 1
    assert t["unmeasured"] == 9           # card + 4 EX12-073 clauses + BT7-056#effect#0 + 3 qa
    assert c["card"]["unmeasured"] == 1 and c["card"]["implemented"] == 0
    assert t["adjudicated"] == 6, "confirmed + terminal + unavailable; escalated is not"
    assert t["items"] == 16
    assert sum(t[k] for k in ("confirmed", "terminal", "unavailable", "escalated",
                              "unmeasured", "implemented", "parked")) == t["items"]


# --------------------------------------------------------------------------- events


def test_every_item_gets_one_creation_event(ws):
    _open(ws)
    first = _events(ws)
    assert len(first) == 16 and all(e["src"] is None for e in first)
    created = {e["item"]: e for e in first}
    assert created["clause:BT8-084#effect#0"]["dst"] == "CONFIRMED"
    assert "ledger" in created["clause:BT8-084#effect#0"]["reason"]
    _open(ws)
    assert len(_events(ws)) == 16, "re-opening does not log creations twice"


def test_open_read_only_writes_nothing(ws):
    _open(ws, write=False)
    assert not (ws["run_dir"] / "events.jsonl").exists()


def test_transition_validates_then_logs(ws):
    s = _open(ws)
    ev = s.transition("clause:EX12-073#effect#0", "AUTHORING", reason="start")
    assert isinstance(ev, Event) and ev.src == "PENDING" and ev.dst == "AUTHORING"
    assert s.records["clause:EX12-073#effect#0"].state == "AUTHORING"
    n = len(_events(ws))
    with pytest.raises(ValueError, match="not an allowed"):
        s.transition("clause:BT8-084#effect#0", "AUTHORING", reason="no")    # CONFIRMED is terminal
    assert len(_events(ws)) == n, "a refused transition is not logged"
    assert s.records["clause:BT8-084#effect#0"].state == "CONFIRMED"


# --------------------------------------------------------------------------- resume


def test_resume_replays_transitions_and_tolerates_a_torn_last_line(ws):
    s = _open(ws)
    s.transition("card:EX12-073", "IMPLEMENTING", reason="start")
    s.transition("card:EX12-073", "REVIEW", reason="diff merged", stage="implement",
                 attempt_id="A1", item_data={"spec": "ex12/EX12-073.yaml"}, count=("implement",))
    s.transition("clause:BT7-056#effect#0", "TRIAGE", reason="diverged")
    s.note("clause:BT7-056#effect#0", reason="merge failed", stage="triage", attempt_id="A2",
           item_data={"merge_error": ["conflict"]}, count=("triage",))
    with open(ws["run_dir"] / "events.jsonl", "ab") as f:
        f.write(b'{"ts": "2026-10-05T00:09:00Z", "run_id": "r1", "item": "clause:EX12-07')   # crash

    r = _open(ws)
    assert r.records["card:EX12-073"].state == "REVIEW"
    assert r.records["card:EX12-073"].attempts == {"implement": 1}
    assert r.records["card:EX12-073"].data["spec"] == "ex12/EX12-073.yaml"
    assert r.records["clause:BT7-056#effect#0"].state == "TRIAGE"
    assert r.records["clause:BT7-056#effect#0"].attempts == {"triage": 1}
    assert r.records["clause:BT7-056#effect#0"].data["merge_error"] == ["conflict"]
    assert r.records["clause:EX12-073#effect#0"].state == "PENDING"
    assert any("damaged last line" in p for p in r.problems), r.problems

    r.transition("clause:EX12-073#effect#0", "AUTHORING", reason="after the crash")
    lines = (ws["run_dir"] / "events.jsonl").read_text(encoding="utf-8").splitlines()
    assert json.loads(lines[-1])["dst"] == "AUTHORING", "the torn line stays isolated"


def test_resume_prefers_a_committed_adjudication_over_a_stale_event_state(ws):
    s = _open(ws)
    s.transition("clause:EX12-073#effect#0", "AUTHORING", reason="x")
    # crash after the oracle stage wrote a confirmed verdict, before its event
    (ws["paths"].verdicts_dir / "EX12-073.json").write_text(json.dumps({
        "version": 1,
        "clauses": {"EX12-073#effect#0": _clause_row("EX12-073#effect#0", "confirmed")}}),
        encoding="utf-8")
    r = _open(ws)
    assert r.records["clause:EX12-073#effect#0"].state == "CONFIRMED"
    last = [e for e in _events(ws) if e["item"] == "clause:EX12-073#effect#0"][-1]
    assert last["src"] == "AUTHORING" and last["dst"] == "CONFIRMED"
    assert last["reason"].startswith("resume:")


def test_deleting_an_escalation_file_withdraws_it_on_resume(ws):
    s = _open(ws)
    item = "clause:BT7-056#inherited#0"
    assert s.records[item].state == "ESCALATED"
    # the escalated attempt's stage data must not steer the new attempt (a
    # re-entered Q&A item kept its old expect_ruling in the second pilot)
    s.note(item, reason="stage data from the escalated pass",
           item_data={"expect_ruling": {"q_id": "Q1", "assert": []}, "scenario_paths": ["x.yaml"],
                      "triage_feedback": "old", "history": ["att-1"]})
    esc_mod.escalation_path(ws["paths"].escalations_dir, item).unlink()   # human: retry it
    r = _open(ws)
    assert r.records[item].state == "PENDING"
    assert r.records[item].attempts == {}
    assert "escalation withdrawn" in [e for e in _events(ws) if e["item"] == item][-1]["reason"]
    d = r.records[item].data
    assert d.get("expect_ruling") is None and d.get("scenario_paths") is None and d.get("triage_feedback") is None
    assert d.get("history") == ["att-1"], "the attempt history is kept"


def test_a_pending_card_whose_spec_appeared_is_implemented_on_resume(ws):
    _open(ws)
    (ws["paths"].cards_dir / "ex12" / "EX12-073.yaml").write_text("card: EX12-073\n", encoding="utf-8")
    r = _open(ws)
    assert r.records["card:EX12-073"].state == "IMPLEMENTED"
    assert r.meta["clause:EX12-073#effect#0"].requires == ("card:EX12-073",), \
        "clauses still wait on the card item, whose state now satisfies them"


def test_unavailable_items_reenter_when_dcgo_gains_the_script(ws):
    s = _open(ws)
    moved = s.reenter_available({"EX12-004"})
    assert sorted(moved) == ["clause:EX12-004#inherited#0",
                             "interaction:probe:EX12-004#inherited#0:scope"]
    assert s.records["clause:EX12-004#inherited#0"].state == "PENDING"
    assert s.records["interaction:qa:Q100"].state == "PENDING"


def test_snapshot_is_written_with_counts_and_items(ws):
    s = _open(ws)
    path = s.write_snapshot(extra={"stop": {"reason": "plateau"}})
    raw = path.read_bytes()
    assert b"\r\n" not in raw
    doc = json.loads(raw)
    assert doc["run_id"] == "r1" and doc["stop"]["reason"] == "plateau"
    assert doc["counts"]["total"]["items"] == 16
    assert {i["item"] for i in doc["items"]} == set(s.records)
    assert doc["items"][0]["meta"]["is_core"] is True
