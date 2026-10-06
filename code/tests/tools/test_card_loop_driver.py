"""The card-loop driver (openspec add-card-authoring-loop, design D4/D5/D13, tasks 6.1/6.4/6.8).

Everything runs on fakes: scripted stage executors, a fake merger / gate / gap
lane, FakeWorker, an injected clock and deterministic ids. No subprocesses.
"""
from __future__ import annotations

import json
import threading
import time
import types
from pathlib import Path

import pytest

from tools.card_loop import driver as drv
from tools.card_loop import escalations as esc_mod
from tools.card_loop import report as report_mod
from tools.card_loop.config import LoopConfig
from tools.card_loop.contracts import Usage, split_item_id
from tools.card_loop.corrections import load_corrections
from tools.card_loop.driver_contracts import (
    Escalation,
    GateResult,
    MergeRequest,
    MergeResult,
    StageDeferred,
    StageOutcome,
)
from tools.card_loop.ledger import Attempt, load_attempts
from tools.card_loop.state import ItemMeta, ItemSeed, Origin, RunState

# --------------------------------------------------------------------------- fakes


class Ticker:
    """Deterministic RFC 3339 timestamps and attempt ids."""

    def __init__(self):
        self.n = 0
        self.a = 0
        self.lock = threading.Lock()

    def now(self):
        with self.lock:
            self.n += 1
            return f"2026-10-05T00:{self.n // 60:02d}:{self.n % 60:02d}Z"

    def new_id(self):
        with self.lock:
            self.a += 1
            return f"A{self.a:03d}"


class Clock:
    def __init__(self):
        self.t = 0.0

    def __call__(self):
        return self.t


class Exec:
    """A scripted stage executor: `fn(ctx, item) -> StageOutcome`."""

    def __init__(self, state, fn):
        self.state = state
        self.fn = fn
        self.calls: list[str] = []
        self._lock = threading.Lock()

    def run(self, ctx, item):
        with self._lock:
            self.calls.append(item.item)
        return self.fn(ctx, item)


def attempt(ctx, item, stage, *, family="claude", cost=0.1, outcome="accepted"):
    return Attempt(attempt_id=ctx.new_attempt_id(), run_id=ctx.run_id, ts=ctx.now(), stage=stage,
                   item=item.item, family=family, model=None, effort=None,
                   prompt_version="test@1", assignment="routed", outcome=outcome,
                   usage=Usage(cost_usd=cost))


def author_ok(cost=0.1):
    return Exec("AUTHORING", lambda ctx, it: StageOutcome(
        "SIM", reason="authored", attempts=[attempt(ctx, it, "author_clause", cost=cost)],
        data={"scenario": f"{it.ident}.yaml"}))


def sim_ok():
    return Exec("SIM", lambda ctx, it: StageOutcome("ORACLE", reason="sim clean"))


def oracle_confirms():
    return Exec("ORACLE", lambda ctx, it: StageOutcome("CONFIRMED", reason="agreed", adjudicated=True))


class FakeMerger:
    def __init__(self, driver_ref, results=None):
        self.driver_ref = driver_ref
        self.calls = []
        self.results = list(results or [])
        self.state_at_merge = []

    def merge(self, ctx, request):
        self.calls.append(request)
        d = self.driver_ref()
        if d is not None:
            self.state_at_merge.append({i: r.state for i, r in d.state.records.items()})
        if self.results:
            return self.results.pop(0)
        return MergeResult(ok=True, sha=f"sha-{len(self.calls)}", branch="card-loop/r1",
                           touched=["code/digimon-engine/cards/x.yaml"])


class FakeGate:
    def __init__(self, results):
        self.results = list(results)
        self.calls = []

    def check(self, ctx, item, merge):
        self.calls.append((item.item, merge))
        return self.results.pop(0)


class FakeGaps:
    def __init__(self, close_on_call=None):
        self.parked: dict[str, tuple[str, bool]] = {}
        self.park_calls = []
        self.closed_calls = 0
        self.close_on_call = close_on_call or {}
        self.unparked = []

    def park(self, item, gap_id, *, is_core):
        self.park_calls.append((item, gap_id, is_core))
        self.parked[item] = (gap_id, is_core)

    def unpark(self, gap_id):
        self.unparked.append(gap_id)
        items = [i for i, (g, _) in self.parked.items() if g == gap_id]
        for i in items:
            del self.parked[i]
        return items

    def closed(self, ctx):
        self.closed_calls += 1
        return list(self.close_on_call.get(self.closed_calls, ()))

    def ranked(self):
        return []


# --------------------------------------------------------------------------- harness


def seed(item, state="PENDING", *, core=True, rank=0, requires=(), source=None, data=None,
         strong=False):
    kind, ident = split_item_id(item)
    card = ident.split("#")[0] if kind != "interaction" else "BT1-001"
    return ItemSeed(item, ItemMeta(cards=(card,), is_core=core, rank=rank, requires=tuple(requires),
                                   source=source),
                    Origin(state, f"seed {state}", {}, strong), dict(data or {}))


PLAN = {"version": 1, "run_id": "r1", "base_sha": "abc",
        "work_set": {"pool": ["BT1-001"], "core": ["BT1-001"], "ranking": ["BT1-001"]},
        "dcgo": {"root": None, "scripts": {}, "unavailable": []}}


class Harness:
    def __init__(self, tmp_path, seeds, executors=(), *, config=None, gate=None, merger=None,
                 gaps=None, serial=True, clock=None, ticker=None, plan=None, **kw):
        self.tmp = Path(tmp_path)
        self.run_dir = self.tmp / "run"
        self.ticker = ticker or Ticker()
        self.clock = clock or Clock()
        self.config = config or cfg(self.tmp)
        self.state = RunState.from_seeds("r1", self.run_dir, seeds, now=self.ticker.now,
                                         escalated=esc_mod.load_escalated_items(self.config.escalations_dir))
        self.comps = drv.Components(executors={e.state: e for e in executors}, gate=gate,
                                    merger=merger, gaps=gaps)
        self.driver = drv.Driver(plan or PLAN, self.config, self.comps, workers={}, state=self.state,
                                 run_dir=self.run_dir, now=self.ticker.now, clock=self.clock,
                                 new_attempt_id=self.ticker.new_id, serial=serial, **kw)

    def run(self):
        return self.driver.run()

    def states(self):
        return {i: r.state for i, r in self.state.records.items()}

    def events(self, item=None):
        rows = [json.loads(line) for line in
                (self.run_dir / "events.jsonl").read_text(encoding="utf-8").splitlines() if line.strip()]
        return [r for r in rows if item is None or r["item"] == item]

    def attempts(self):
        return load_attempts(self.config.attempts_path)[0]

    def corrections(self):
        return load_corrections(Path(self.config.attempts_path).with_name("corrections.jsonl"))[0]


def cfg(tmp, **kw):
    base = dict(attempts_path=str(Path(tmp) / "ledger" / "attempts.jsonl"),
                escalations_dir=str(Path(tmp) / "escalations"), concurrency=3, plateau_attempts=10)
    base.update(kw)
    return LoopConfig(**base)


C1, C2, C3 = "clause:BT1-001#effect#0", "clause:BT1-001#effect#1", "clause:BT1-001#effect#2"


# --------------------------------------------------------------------------- the basic path


def test_a_clause_flows_from_pending_to_confirmed(tmp_path):
    h = Harness(tmp_path, [seed(C1)], [author_ok(), sim_ok(), oracle_confirms()])
    result = h.run()
    assert result.stop.reason == "complete"
    assert h.states() == {C1: "CONFIRMED"}
    chain = [(e["src"], e["dst"]) for e in h.events(C1)]
    assert chain == [(None, "PENDING"), ("PENDING", "AUTHORING"), ("AUTHORING", "SIM"),
                     ("SIM", "ORACLE"), ("ORACLE", "CONFIRMED")]
    authored = h.events(C1)[2]
    assert authored["stage"] == "author_clause" and authored["attempt_id"] == "A001"
    assert h.state.records[C1].data["scenario"] == "BT1-001#effect#0.yaml"
    assert [a.attempt_id for a in h.attempts()] == ["A001"]
    assert (h.run_dir / "state.json").exists() and (h.run_dir / "report.md").exists()


def test_pending_and_diverged_route_by_kind_without_executors(tmp_path):
    seeds = [seed("card:BT1-002"), seed(C1),
             seed("interaction:qa:Q100", source="qa"),
             seed("interaction:probe:BT1-001#effect#0:scope", source="probe"),
             seed(C2, "DIVERGED")]
    h = Harness(tmp_path, seeds)
    result = h.run()
    assert h.states() == {"card:BT1-002": "IMPLEMENTING", C1: "AUTHORING",
                          "interaction:qa:Q100": "CLASSIFY",
                          "interaction:probe:BT1-001#effect#0:scope": "AUTHORING",
                          C2: "TRIAGE"}
    assert result.stop.reason == "blocked"
    assert "no executor for AUTHORING" in result.blocked[C1]
    report = (h.run_dir / "report.md").read_text(encoding="utf-8")
    assert "no executor for AUTHORING" in report


P1 = "interaction:probe:BT1-001#effect#0:scope"


def test_an_interaction_waits_while_one_of_its_cards_clauses_is_being_fixed(tmp_path):
    # Data Squad pilot: BT13-060#effect#2 was DIVERGED -> FIX (our engine skips
    # the security check) while Q2304, an interaction on the same clause, burnt
    # its three authoring attempts failing the very assertion the fix targets.
    author = author_ok()
    h = Harness(tmp_path, [seed(C1, "FIX"), seed(P1, "AUTHORING", source="probe")], [author])
    result = h.run()
    assert author.calls == [], "the interaction must not be authored under an engine about to change"
    assert h.states()[P1] == "AUTHORING"
    assert result.stop.reason == "blocked"
    assert f"waits for {C1} (FIX)" in result.blocked[P1]


@pytest.mark.parametrize("clause_state", ["ESCALATED", "TERMINAL", "CONFIRMED"])
def test_an_interaction_runs_once_the_clause_is_no_longer_under_fix(tmp_path, clause_state):
    author = author_ok()
    h = Harness(tmp_path, [seed(C1, clause_state), seed(P1, "AUTHORING", source="probe")],
                [author, sim_ok(), oracle_confirms()])
    h.run()
    assert author.calls == [P1]
    assert h.states()[P1] == "CONFIRMED"


def test_an_illegal_next_state_is_refused_and_the_spend_still_ledgered(tmp_path):
    bad = Exec("AUTHORING", lambda ctx, it: StageOutcome(
        "CONFIRMED", attempts=[attempt(ctx, it, "author_clause")]))
    h = Harness(tmp_path, [seed(C1, "AUTHORING")], [bad])
    result = h.run()
    assert h.states() == {C1: "AUTHORING"}, "the item does not move"
    assert bad.calls == [C1], "not retried within the session"
    last = h.events(C1)[-1]
    assert last["src"] == last["dst"] == "AUTHORING" and "refused" in last["reason"]
    assert [a.attempt_id for a in h.attempts()] == ["A001"]
    assert C1 in result.failed and "not an allowed transition" in result.failed[C1]
    assert h.state.records[C1].attempts == {"author_clause": 1}


def test_an_executor_exception_is_contained(tmp_path):
    def boom(ctx, it):
        if it.item == C1:
            raise RuntimeError("harness crashed")
        return StageOutcome("SIM", attempts=[attempt(ctx, it, "author_clause")])
    h = Harness(tmp_path, [seed(C1, "AUTHORING"), seed(C2, "AUTHORING")],
                [Exec("AUTHORING", boom), sim_ok(), oracle_confirms()])
    result = h.run()
    assert h.states() == {C1: "AUTHORING", C2: "CONFIRMED"}
    assert "harness crashed" in result.failed[C1]
    assert "harness crashed" in (h.run_dir / "report.md").read_text(encoding="utf-8")


def test_clauses_wait_for_their_card_item(tmp_path):
    card = "card:BT1-001"
    order = []
    impl = Exec("IMPLEMENTING", lambda ctx, it: (order.append(it.item), StageOutcome(
        "REVIEW", attempts=[attempt(ctx, it, "implement")]))[1])
    review = Exec("REVIEW", lambda ctx, it: (order.append(it.item), StageOutcome(
        "IMPLEMENTED", attempts=[attempt(ctx, it, "review", family="codex")], adjudicated=True))[1])
    author = Exec("AUTHORING", lambda ctx, it: (order.append(it.item), StageOutcome(
        "SIM", attempts=[attempt(ctx, it, "author_clause")]))[1])
    h = Harness(tmp_path, [seed(card), seed(C1, requires=[card])],
                [impl, review, author, sim_ok(), oracle_confirms()])
    h.run()
    assert order == [card, card, C1]
    assert h.states() == {card: "IMPLEMENTED", C1: "CONFIRMED"}


# --------------------------------------------------------------------------- caps and stops


def test_attempt_cap_escalates_with_the_attempt_history(tmp_path):
    sim_fails = Exec("SIM", lambda ctx, it: StageOutcome("AUTHORING", reason="sim: prompt mismatch"))
    author = author_ok()
    h = Harness(tmp_path, [seed(C1, "AUTHORING")], [author, sim_fails],
                config=cfg(tmp_path, attempt_caps={"author_clause": 3}))
    result = h.run()
    assert author.calls == [C1] * 3
    assert h.states() == {C1: "ESCALATED"}
    path = esc_mod.escalation_path(h.config.escalations_dir, C1)
    text = path.read_text(encoding="utf-8")
    assert "attempt cap" in text and "author_clause" in text
    for aid in ("A001", "A002", "A003"):
        assert aid in text
    assert result.counts["total"]["escalated"] == 1 and result.counts["total"]["adjudicated"] == 0
    assert result.stop.reason == "complete", "escalated is terminal for the run"


def test_a_reimplemented_card_can_be_reviewed_again(tmp_path):
    """Default caps: implement 3, review 1 -- the review cap restarts with every
    implement attempt, or a card could never survive one "changes requested"."""
    card = "card:BT1-001"
    verdicts = iter(["IMPLEMENTING", "IMPLEMENTED"])
    impl = Exec("IMPLEMENTING", lambda ctx, it: StageOutcome(
        "REVIEW", attempts=[attempt(ctx, it, "implement")]))
    review = Exec("REVIEW", lambda ctx, it: (lambda v: StageOutcome(
        v, attempts=[attempt(ctx, it, "review", family="codex")], adjudicated=v == "IMPLEMENTED"))(next(verdicts)))
    h = Harness(tmp_path, [seed(card, "IMPLEMENTING")], [impl, review])
    h.run()
    assert h.states() == {card: "IMPLEMENTED"}
    assert review.calls == [card, card]
    assert h.state.records[card].attempts == {"implement": 2, "review": 1}


def test_a_new_fix_restarts_the_triage_cap(tmp_path):
    oracle_results = iter(["DIVERGED", "CONFIRMED"])
    oracle = Exec("ORACLE", lambda ctx, it: (lambda v: StageOutcome(v, adjudicated=v == "CONFIRMED"))(
        next(oracle_results)))
    triage = Exec("TRIAGE", lambda ctx, it: StageOutcome(
        "FIX", attempts=[attempt(ctx, it, "triage")], data={"triage": "ours_wrong"}))
    gate = FakeGate([GateResult(True), GateResult(True)])
    h = Harness(tmp_path, [seed(C1, "DIVERGED")], [triage, _fix_with_merge(), oracle], gate=gate,
                merger=FakeMerger(lambda: None))
    h.run()
    assert h.states() == {C1: "CONFIRMED"}
    assert triage.calls == [C1, C1], "the fix made a new divergence worth a fresh triage"


def test_an_executor_looping_without_attempts_is_set_aside(tmp_path):
    a = Exec("AUTHORING", lambda ctx, it: StageOutcome("SIM", reason="scenario exists"))
    s = Exec("SIM", lambda ctx, it: StageOutcome("AUTHORING", reason="sim failed"))
    h = Harness(tmp_path, [seed(C1, "AUTHORING"), seed(C2, "AUTHORING")], [a, s])
    result = h.run()
    assert result.stop.reason == "blocked"
    assert "without a worker attempt" in result.failed[C1]
    assert len(a.calls) < 2 * drv.MAX_IDLE_STEPS


def test_a_driver_error_still_writes_state_and_report(tmp_path):
    h = Harness(tmp_path, [seed(C1, "AUTHORING")], [author_ok(), sim_ok(), oracle_confirms()])

    def broken(_attempt):
        raise OSError("disk full")
    h.driver.ledger.append = broken
    with pytest.raises(OSError, match="disk full"):
        h.run()
    first = (h.run_dir / "report.md").read_text(encoding="utf-8").splitlines()[0]
    assert "stopped: error" in first
    assert json.loads((h.run_dir / "state.json").read_text(encoding="utf-8"))["stop"]["reason"] == "error"


def test_plateau_stops_the_run(tmp_path):
    sim_fails = Exec("SIM", lambda ctx, it: StageOutcome("AUTHORING"))
    h = Harness(tmp_path, [seed(C1, "AUTHORING"), seed(C2, "AUTHORING")], [author_ok(), sim_fails],
                config=cfg(tmp_path, plateau_attempts=3, attempt_caps={"author_clause": 100}))
    result = h.run()
    assert result.stop.reason == "plateau"
    assert len(h.attempts()) == 3
    report = (h.run_dir / "report.md").read_text(encoding="utf-8")
    assert "plateau" in report.splitlines()[0]


def test_an_adjudication_resets_the_plateau_counter(tmp_path):
    h = Harness(tmp_path, [seed(C1, "AUTHORING"), seed(C2, "AUTHORING"), seed(C3, "AUTHORING")],
                [author_ok(), sim_ok(), oracle_confirms()], config=cfg(tmp_path, plateau_attempts=2))
    result = h.run()
    assert result.stop.reason == "complete"
    assert set(h.states().values()) == {"CONFIRMED"}


def test_budget_stop(tmp_path):
    sim_fails = Exec("SIM", lambda ctx, it: StageOutcome("AUTHORING"))
    h = Harness(tmp_path, [seed(C1, "AUTHORING")], [author_ok(cost=0.6), sim_fails],
                config=cfg(tmp_path, budget_usd=1.0, attempt_caps={"author_clause": 100}))
    result = h.run()
    assert result.stop.reason == "budget"
    assert len(h.attempts()) == 2
    assert result.stop.spent_usd == pytest.approx(1.2)


def test_budget_counts_spend_already_ledgered_by_this_run(tmp_path):
    h1 = Harness(tmp_path, [seed(C1, "AUTHORING")], [author_ok(cost=0.6), sim_ok()],
                 config=cfg(tmp_path, budget_usd=1.0), max_attempts=1)
    assert h1.run().stop.reason == "max_attempts"
    h2 = Harness(tmp_path, [seed(C1, "AUTHORING"), seed(C2, "AUTHORING")],
                 [author_ok(cost=0.6), sim_ok(), oracle_confirms()], config=cfg(tmp_path, budget_usd=1.0),
                 ticker=h1.ticker)
    result = h2.run()
    assert result.stop.reason == "budget"
    assert result.stop.spent_usd == pytest.approx(1.2), "session 1's $0.60 counts against the cap"


def test_wall_clock_stop_uses_the_injected_clock(tmp_path):
    clock = Clock()

    def tick(state, nxt, stage=None):
        def fn(ctx, it):
            clock.t += 0.4 * 3600
            return StageOutcome(nxt, attempts=[attempt(ctx, it, stage)] if stage else [])
        return Exec(state, fn)

    h = Harness(tmp_path, [seed(C1, "AUTHORING"), seed(C2, "AUTHORING")],
                [tick("AUTHORING", "SIM", "author_clause"), tick("SIM", "ORACLE"),
                 tick("ORACLE", "DIVERGED")], config=cfg(tmp_path, wall_clock_hours=1.0), clock=clock)
    result = h.run()
    assert result.stop.reason == "wall_clock"
    assert clock.t == pytest.approx(1.2 * 3600)
    assert result.stop.elapsed_s == pytest.approx(1.2 * 3600)


def test_the_run_stops_when_every_vendor_is_exhausted(tmp_path):
    from tools.card_loop.workers.health import VendorHealth
    health = VendorHealth()
    author = author_ok()
    h = Harness(tmp_path, [seed(C1, "AUTHORING")], [author], health=health)
    h.driver.ctx.workers = {"claude": object(), "codex": object()}
    health.disable("claude", "credit balance too low")
    health.disable("codex", "quota")
    result = h.run()
    assert result.stop.reason == "vendors_exhausted"
    assert author.calls == []
    assert "credit balance too low" in result.stop.detail


def test_max_attempts_stops_after_n_worker_attempts(tmp_path):
    h = Harness(tmp_path, [seed(C1, "AUTHORING"), seed(C2, "AUTHORING")],
                [author_ok(), sim_ok(), oracle_confirms()], max_attempts=1)
    result = h.run()
    assert result.stop.reason == "max_attempts"
    assert len(h.attempts()) == 1


# --------------------------------------------------------------------------- report


def test_report_first_line_leads_with_the_unadjudicated(tmp_path):
    seeds = [seed(C1, "AUTHORING"), seed(C2, "UNAVAILABLE"), seed(C3, "ESCALATED"),
             seed("clause:BT1-001#effect#3", "TRIAGE")]       # no TRIAGE executor: stays open
    h = Harness(tmp_path, seeds, [author_ok(), sim_ok(), oracle_confirms()])
    h.run()
    first = (h.run_dir / "report.md").read_text(encoding="utf-8").splitlines()[0]
    assert first.index("unmeasured") < first.index("escalated") < first.index("unavailable") \
        < first.index("confirmed"), first
    assert "1 unmeasured" in first and "1 escalated" in first and "1 unavailable" in first
    assert "1 confirmed" in first
    snap = json.loads((h.run_dir / "state.json").read_text(encoding="utf-8"))
    assert snap["stop"]["reason"] == "blocked"


def test_report_first_line_function():
    counts = {"total": {"unmeasured": 4, "escalated": 2, "unavailable": 1, "confirmed": 9,
                        "terminal": 3, "implemented": 0, "parked": 0, "adjudicated": 13, "items": 19},
              "card": {"items": 0}}
    line = report_mod.first_line("r1", counts, {"reason": "budget"})
    assert line.startswith("**4 unmeasured, 2 escalated, 1 unavailable**")
    assert line.index("9 confirmed") > line.index("1 unavailable")
    assert "budget" in line


# --------------------------------------------------------------------------- escalation


def test_a_two_family_disagreement_escalates_with_both_arguments(tmp_path):
    def check(ctx, it):
        a = attempt(ctx, it, "triage", family="claude")
        b = attempt(ctx, it, "triage", family="codex")
        return StageOutcome("ESCALATED", reason="families disagree", attempts=[a, b],
                            escalation=Escalation(item=it.item, reason="families disagree: dcgo_quirk vs ours_wrong",
                                                  arguments=(
                                                      {"family": "claude", "attempt_id": a.attempt_id,
                                                       "call": "dcgo_quirk", "citation": "general_rule.pdf 15-1-2",
                                                       "reasoning": "processing order"},
                                                      {"family": "codex", "attempt_id": b.attempt_id,
                                                       "call": "ours_wrong", "citation": "BT1_001.cs:40",
                                                       "reasoning": "decline missing"})))
    h = Harness(tmp_path, [seed(C1, "TERMINATION_CHECK")], [Exec("TERMINATION_CHECK", check)])
    result = h.run()
    assert h.states() == {C1: "ESCALATED"}
    text = esc_mod.escalation_path(h.config.escalations_dir, C1).read_text(encoding="utf-8")
    assert "dcgo_quirk" in text and "ours_wrong" in text and "BT1_001.cs:40" in text
    assert "A001" in text and "A002" in text, "history filled from the item's attempts"
    ev = h.events(C1)[-1]
    assert ev["attempt_id"] == "A001" and ev["data"]["_driver"]["attempt_ids"] == ["A001", "A002"]
    assert h.state.records[C1].attempts == {"triage": 1}, "one termination check = one triage pass"
    assert result.counts["total"]["adjudicated"] == 0
    rows = [json.loads(x) for x in (Path(h.config.escalations_dir) / "index.jsonl").read_text(
        encoding="utf-8").splitlines()]
    assert rows[-1]["item"] == C1


# --------------------------------------------------------------------------- gaps


def test_a_parked_card_reenters_implementation_when_its_gap_closes(tmp_path):
    card = "card:BT1-001"
    calls = {"n": 0}

    def implement(ctx, it):
        calls["n"] += 1
        if calls["n"] == 1:
            return StageOutcome("PARKED", reason="DSL gap", attempts=[attempt(ctx, it, "implement")],
                                data={"gap_id": "G-DSL-1"})
        return StageOutcome("REVIEW", attempts=[attempt(ctx, it, "implement")])

    gaps = FakeGaps(close_on_call={2: ["G-DSL-1"]})
    review = Exec("REVIEW", lambda ctx, it: StageOutcome("IMPLEMENTED", adjudicated=True,
                                                         attempts=[attempt(ctx, it, "review", family="codex")]))
    # C2 keeps the run busy while the card is parked; C1 waits on the card.
    h = Harness(tmp_path, [seed(card), seed(C1, requires=[card]), seed(C2, "AUTHORING")],
                [Exec("IMPLEMENTING", implement), review, author_ok(), sim_ok(), oracle_confirms()],
                gaps=gaps)
    h.run()
    assert h.states() == {card: "IMPLEMENTED", C1: "CONFIRMED", C2: "CONFIRMED"}
    assert gaps.park_calls == [(card, "G-DSL-1", True)]
    assert gaps.unparked == ["G-DSL-1"]
    chain = [(e["src"], e["dst"]) for e in h.events(card)]
    assert ("IMPLEMENTING", "PARKED") in chain and ("PARKED", "IMPLEMENTING") in chain
    unpark = [e for e in h.events(card) if e["src"] == "PARKED"][0]
    assert "G-DSL-1" in unpark["reason"]


def test_parked_items_are_reparked_and_unparked_on_resume(tmp_path):
    card = "card:BT1-001"
    park = Exec("IMPLEMENTING", lambda ctx, it: StageOutcome(
        "PARKED", attempts=[attempt(ctx, it, "implement")], data={"gap_id": "G-1"}))
    g1 = FakeGaps()
    h1 = Harness(tmp_path, [seed(card)], [park], gaps=g1)
    assert h1.run().stop.reason == "blocked"
    assert g1.parked == {card: ("G-1", True)}

    g2 = FakeGaps()      # a fresh gap lane: the gap is still open
    h2 = Harness(tmp_path, [seed(card)], [park], gaps=g2, ticker=h1.ticker)
    result = h2.run()
    assert g2.parked == {card: ("G-1", True)}, "a fresh gap lane learns the parked item on resume"
    assert h2.states() == {card: "PARKED"}
    assert "parked on gap G-1" in result.blocked[card]

    g3 = FakeGaps(close_on_call={1: ["G-1"]})     # the gap's fix merged between sessions
    impl = Exec("IMPLEMENTING", lambda ctx, it: StageOutcome(
        "REVIEW", attempts=[attempt(ctx, it, "implement")]))
    review = Exec("REVIEW", lambda ctx, it: StageOutcome("IMPLEMENTED", adjudicated=True))
    h3 = Harness(tmp_path, [seed(card)], [impl, review], gaps=g3, ticker=h1.ticker)
    h3.run()
    assert impl.calls == [card] and h3.states() == {card: "IMPLEMENTED"}


# --------------------------------------------------------------------------- merges and the gate


def _impl_with_merge(engine=False):
    def fn(ctx, it):
        a = attempt(ctx, it, "implement")
        return StageOutcome("REVIEW", attempts=[a], merge_request=MergeRequest(
            attempt_id=a.attempt_id, family="claude", model=None,
            artifacts={"diff": "x.diff", "manifest": "x.json"}, engine=engine, subject="BT1-001"))
    return Exec("IMPLEMENTING", fn)


def test_merge_happens_before_the_transition(tmp_path):
    card = "card:BT1-001"
    holder = {}
    merger = FakeMerger(lambda: holder.get("d"))
    h = Harness(tmp_path, [seed(card, "IMPLEMENTING")], [_impl_with_merge()], merger=merger)
    holder["d"] = h.driver
    h.run()
    assert len(merger.calls) == 1 and merger.calls[0].attempt_id == "A001"
    assert merger.state_at_merge[0][card] == "IMPLEMENTING", "merged before the item advanced"
    assert h.states()[card] == "REVIEW"
    assert h.state.records[card].data["merge"]["sha"] == "sha-1"
    assert h.state.records[card].data["merge"]["attempt_id"] == "A001"


def test_a_failed_merge_keeps_the_item_counts_the_attempt_and_escalates_at_the_cap(tmp_path):
    card = "card:BT1-001"
    merger = FakeMerger(lambda: None, results=[MergeResult(ok=False, errors=["patch does not apply"]),
                                               MergeResult(ok=False, errors=["module not registered"])])
    impl = _impl_with_merge()
    h = Harness(tmp_path, [seed(card, "IMPLEMENTING")], [impl], merger=merger,
                config=cfg(tmp_path, attempt_caps={"implement": 2}))
    h.run()
    assert impl.calls == [card, card]
    assert h.states()[card] == "ESCALATED"
    assert [a.outcome for a in h.attempts()] == ["gate_failed", "gate_failed"]
    corr = h.corrections()
    assert [(c.corrected_attempt, c.by_gate, c.kind) for c in corr] == [
        ("A001", "merge", "gate_fail"), ("A002", "merge", "gate_fail")]
    stay = [e for e in h.events(card) if e["src"] == e["dst"] == "IMPLEMENTING"]
    assert len(stay) == 2 and "merge failed" in stay[0]["reason"]
    assert h.state.records[card].data["merge_error"] == ["module not registered"]
    assert h.state.records[card].attempts == {"implement": 2}


def _fix_with_merge():
    def fn(ctx, it):
        a = attempt(ctx, it, "fix_card")
        return StageOutcome("GATE", attempts=[a], merge_request=MergeRequest(
            attempt_id=a.attempt_id, family="claude", model=None, artifacts={"diff": "f.diff"}))
    return Exec("FIX", fn)


def test_gate_failure_returns_to_fix_and_a_pass_goes_to_the_oracle(tmp_path):
    gate = FakeGate([GateResult(False, reasons=["no failing-then-passing test"]),
                     GateResult(True, evidence={"cards_behavioral": "green"})])
    fix = _fix_with_merge()
    h = Harness(tmp_path, [seed(C1, "FIX")], [fix, oracle_confirms()], gate=gate,
                merger=FakeMerger(lambda: None))
    h.run()
    assert fix.calls == [C1, C1]
    assert h.states() == {C1: "CONFIRMED"}
    assert [c[1].sha for c in gate.calls] == ["sha-1", "sha-2"], "the gate sees the last merge"
    corr = h.corrections()
    assert [(c.corrected_attempt, c.by_gate) for c in corr] == [("A001", "fix_gate")]
    back = [e for e in h.events(C1) if (e["src"], e["dst"]) == ("GATE", "FIX")][0]
    assert "no failing-then-passing test" in back["reason"]
    assert h.state.records[C1].data["gate"]["passed"] is True


def test_gate_failure_at_the_fix_cap_escalates(tmp_path):
    gate = FakeGate([GateResult(False, reasons=["cards_behavioral red"])])
    h = Harness(tmp_path, [seed(C1, "FIX")], [_fix_with_merge()], gate=gate,
                merger=FakeMerger(lambda: None), config=cfg(tmp_path, attempt_caps={"fix_card": 1}))
    h.run()
    assert h.states() == {C1: "ESCALATED"}
    assert (h.events(C1)[-1]["src"], h.events(C1)[-1]["dst"]) == ("GATE", "ESCALATED")
    text = esc_mod.escalation_path(h.config.escalations_dir, C1).read_text(encoding="utf-8")
    assert "cards_behavioral red" in text


def test_the_gate_is_blocked_when_not_built(tmp_path):
    h = Harness(tmp_path, [seed(C1, "GATE")], [])
    result = h.run()
    assert result.stop.reason == "blocked" and "gate" in result.blocked[C1]


# --------------------------------------------------------------------------- concurrency


def test_worker_stages_run_concurrently_and_the_oracle_strictly_alone(tmp_path):
    live = {"w": 0, "o": 0, "w_max": 0, "o_max": 0}
    lock = threading.Lock()

    def tracked(state, nxt, lane, stage=None, pause=0.05):
        def fn(ctx, it):
            with lock:
                live[lane] += 1
                live[lane + "_max"] = max(live[lane + "_max"], live[lane])
            time.sleep(pause)
            with lock:
                live[lane] -= 1
            return StageOutcome(nxt, attempts=[attempt(ctx, it, stage)] if stage else [],
                                adjudicated=nxt == "CONFIRMED")
        return Exec(state, fn)

    items = [f"clause:BT1-001#effect#{i}" for i in range(6)]
    h = Harness(tmp_path, [seed(i, "AUTHORING") for i in items],
                # A 0.4 s window: three worker tasks must overlap even on a loaded
                # machine (the 50 ms default saw w_max == 2 beside two live pilots).
                [tracked("AUTHORING", "SIM", "w", "author_clause", pause=0.4), tracked("SIM", "ORACLE", "w", pause=0.01),
                 tracked("ORACLE", "CONFIRMED", "o", pause=0.02)],
                serial=False, config=cfg(tmp_path, concurrency=3))
    result = h.run()
    assert result.stop.reason == "complete"
    assert set(h.states().values()) == {"CONFIRMED"}
    assert live["w_max"] == 3, live
    assert live["o_max"] == 1, live


# --------------------------------------------------------------------------- resume


def test_resume_after_a_crash_runs_only_outstanding_items(tmp_path):
    a1, s1, o1 = author_ok(), sim_ok(), oracle_confirms()
    h1 = Harness(tmp_path, [seed(C1), seed(C2), seed(C3)], [a1, s1, o1], max_attempts=1)
    assert h1.run().stop.reason == "max_attempts"
    assert h1.states()[C1] == "SIM"
    with open(h1.run_dir / "events.jsonl", "ab") as f:
        f.write(b'{"ts": "2026-10-05T09:00:00Z", "run_id": "r1", "item": "clause:BT1-0')   # torn

    a2, s2, o2 = author_ok(), sim_ok(), oracle_confirms()
    h2 = Harness(tmp_path, [seed(C1), seed(C2), seed(C3)], [a2, s2, o2], ticker=h1.ticker)
    result = h2.run()
    assert result.stop.reason == "complete"
    assert a2.calls == [C2, C3], "C1 was already authored before the crash"
    assert s2.calls[0] == C1
    assert set(h2.states().values()) == {"CONFIRMED"}
    assert any("damaged last line" in p for p in result.problems)
    assert "damaged last line" in (h2.run_dir / "report.md").read_text(encoding="utf-8")


def test_resume_restores_attempt_counts_so_caps_hold_across_sessions(tmp_path):
    sim_fails = Exec("SIM", lambda ctx, it: StageOutcome("AUTHORING"))
    caps = {"author_clause": 3}
    h1 = Harness(tmp_path, [seed(C1, "AUTHORING")], [author_ok(), sim_fails],
                 config=cfg(tmp_path, attempt_caps=caps), max_attempts=2)
    h1.run()
    author = author_ok()
    h2 = Harness(tmp_path, [seed(C1, "AUTHORING")], [author, sim_fails],
                 config=cfg(tmp_path, attempt_caps=caps), ticker=h1.ticker)
    h2.run()
    assert author.calls == [C1], "only the third attempt is left"
    assert h2.states() == {C1: "ESCALATED"}
    text = esc_mod.escalation_path(h2.config.escalations_dir, C1).read_text(encoding="utf-8")
    for aid in ("A001", "A002", "A003"):
        assert aid in text, "history spans both sessions (from the attempt ledger)"


def test_unavailable_items_reenter_when_dcgo_gains_the_script(tmp_path):
    plan = {**PLAN, "dcgo": {"root": "x", "scripts": {"BT1-001": None}, "unavailable": ["BT1-001"]}}
    s = seed(C1, "UNAVAILABLE", data={"unavailable_source": "plan"})
    h = Harness(tmp_path, [s], [author_ok(), sim_ok(), oracle_confirms()], plan=plan,
                dcgo_presence=lambda pool: {"BT1-001": "BT1_001.cs"})
    h.run()
    assert h.states() == {C1: "CONFIRMED"}
    assert [e["dst"] for e in h.events(C1)][:2] == ["UNAVAILABLE", "PENDING"]


# --------------------------------------------------------------------------- StageDeferred


def test_a_deferred_step_is_ledgered_counted_and_escalates_at_the_cap(tmp_path):
    def defer(ctx, it):
        a = attempt(ctx, it, "author_clause", outcome="error")
        raise StageDeferred("author_clause worker claude returned error: boom",
                            StageOutcome(it.state, attempts=[a], data={"history": [a.attempt_id]}))
    ex = Exec("AUTHORING", defer)
    h = Harness(tmp_path, [seed(C1, "AUTHORING")], [ex], config=cfg(tmp_path, attempt_caps={"author_clause": 2}))
    h.run()
    assert ex.calls == [C1, C1]
    assert h.states() == {C1: "ESCALATED"}
    assert [a.outcome for a in h.attempts()] == ["error", "error"]
    stays = [e for e in h.events(C1) if e["src"] == e["dst"] == "AUTHORING"]
    assert len(stays) == 2 and all(e["reason"].startswith("deferred:") for e in stays)
    assert h.state.records[C1].data["history"] == ["A002"]
    text = esc_mod.escalation_path(h.config.escalations_dir, C1).read_text(encoding="utf-8")
    assert "author_clause 2/2" in text and "deferred" in text


def test_a_deferral_without_attempts_still_counts_toward_the_cap(tmp_path):
    from tools.card_loop.stages.base import StageDeferred as StagesDeferred   # subclasses the contract's
    ex = Exec("AUTHORING", lambda ctx, it: (_ for _ in ()).throw(
        StagesDeferred("no family can take author_interaction")))
    h = Harness(tmp_path, [seed(C1, "AUTHORING")], [ex], config=cfg(tmp_path, attempt_caps={"author_clause": 3}))
    result = h.run()
    assert ex.calls == [C1] * 3 and h.states() == {C1: "ESCALATED"}
    assert h.attempts() == [] and not result.failed


def test_a_failed_authoring_merge_keeps_the_item_in_authoring(tmp_path):
    def author(ctx, it):
        a = attempt(ctx, it, "author_clause")
        return StageOutcome("SIM", attempts=[a], data={"scenario_paths": ["qa/dcgo-exams/BT1/x.yaml"]},
                            merge_request=MergeRequest(attempt_id=a.attempt_id, family="claude", model=None,
                                                       artifacts={"diff": "d", "manifest": "m"}))
    merger = FakeMerger(lambda: None, results=[MergeResult(ok=False, errors=["scenario conflicts"])])
    h = Harness(tmp_path, [seed(C1, "AUTHORING")], [Exec("AUTHORING", author), sim_ok(), oracle_confirms()],
                merger=merger)
    h.run()
    chain = [(e["src"], e["dst"]) for e in h.events(C1)]
    assert chain[1:] == [("AUTHORING", "AUTHORING"), ("AUTHORING", "SIM"), ("SIM", "ORACLE"),
                         ("ORACLE", "CONFIRMED")]
    assert "merge failed" in h.events(C1)[1]["reason"]


# --------------------------------------------------------------------------- the run tree


def _tree_cmds(book_rc=0):
    from tools.card_loop.stages.testing import FakeCommands
    return (FakeCommands()
            .on(("tools.clause_coverage.book", "add"), book_rc, "", "" if book_rc == 0 else "book broke")
            .on(("symbolic-ref",), 0, "card-loop/r1/run\n")
            .on(("status", "exam-clause-text"), 0, " M qa/exam-clause-text.json\n")
            .on(("status", "exam-verdicts"), 0,
                " M qa/dcgo-exams/BT1/x.yaml\n?? qa/qa-reports/exam-verdicts/BT1-001.json\n")
            .on(("git", "add", "--"), 0)
            .on(("git", "commit"), 0))


def test_oracle_writes_are_committed_after_the_clause_text_prelude(tmp_path):
    cmds = _tree_cmds()
    seen = []

    def oracle(ctx, it):
        seen.append(len(cmds.argvs("clause_coverage.book")))
        return StageOutcome("CONFIRMED", adjudicated=True, data={"oracle_results": {
            "qa/dcgo-exams/BT1/x.yaml": {"scenario": "qa/dcgo-exams/BT1/x.yaml", "verdict": "confirmed",
                                         "backfilled": True}}})
    plan = {**PLAN, "work_set": {"pool": ["BT1-001", "BT1-002"], "core": [], "ranking": []}}
    tree = tmp_path / "tree"
    h = Harness(tmp_path, [seed(C1, "ORACLE", data={"scenario_paths": ["qa/dcgo-exams/BT1/x.yaml"]}),
                           seed(C2, "ORACLE")], [Exec("ORACLE", oracle)], plan=plan,
                run_command=cmds, repo=tree, manage_tree=True)
    h.run()
    assert h.states() == {C1: "CONFIRMED", C2: "CONFIRMED"}
    book = cmds.argvs("clause_coverage.book")
    assert len(book) == 1, "the prelude runs once per session"
    assert book[0][-3:] == ["--card-ids", "BT1-001", "BT1-002"] and "qa/exam-clause-text.json" in book[0]
    assert [c["cwd"] for c in cmds.calls if "clause_coverage.book" in " ".join(c["argv"])] == [str(tree)]
    assert seen == [1, 1], "the book was extended before the first oracle call"
    commits = cmds.argvs("git", "commit")
    assert "qa/exam-clause-text.json" in commits[0]
    oracle_commit = commits[1]
    msg = oracle_commit[oracle_commit.index("-m") + 1]
    assert msg.startswith("card-loop: oracle verdict and backfill for clause:BT1-001#effect#0")
    assert "Loop-Attempt: A" in msg and "Loop-Model: driver/default" in msg and "Loop-Run: r1" in msg
    assert oracle_commit[-2:] == ["qa/dcgo-exams/BT1/x.yaml", "qa/qa-reports/exam-verdicts/BT1-001.json"]
    status = cmds.argvs("status", "exam-verdicts")[0]
    assert "qa/dcgo-exams/BT1/x.yaml" in status and "qa/qa-reports/exam-verdicts" in status


def test_a_failed_prelude_holds_every_oracle_item(tmp_path):
    cmds = _tree_cmds(book_rc=1)
    oracle = oracle_confirms()
    h = Harness(tmp_path, [seed(C1, "ORACLE")], [oracle], run_command=cmds, repo=tmp_path / "t",
                manage_tree=True)
    result = h.run()
    assert oracle.calls == [] and h.states() == {C1: "ORACLE"}
    assert result.stop.reason == "blocked"
    assert "clause-text book prelude failed" in result.blocked[C1] and "book broke" in result.blocked[C1]


def test_nothing_is_committed_off_a_run_branch(tmp_path):
    from tools.card_loop.stages.testing import FakeCommands
    cmds = (FakeCommands().on(("tools.clause_coverage.book",), 0).on(("symbolic-ref",), 0, "main\n"))
    h = Harness(tmp_path, [seed(C1, "ORACLE")], [oracle_confirms()], run_command=cmds,
                repo=tmp_path / "t", manage_tree=True)
    result = h.run()
    assert cmds.argvs("commit") == [] and cmds.argvs("status") == []
    assert any("not a run branch" in p for p in result.problems)


def test_without_manage_tree_the_driver_runs_no_git(tmp_path):
    from tools.card_loop.stages.testing import FakeCommands
    cmds = FakeCommands()            # any call would raise
    h = Harness(tmp_path, [seed(C1, "ORACLE")], [oracle_confirms()], run_command=cmds)
    assert h.run().stop.reason == "complete" and cmds.calls == []


def test_an_engine_fix_waits_in_gate_until_a_human_merges_its_branch(tmp_path):
    from tools.card_loop.stages.testing import FakeCommands

    def fix(ctx, it):
        a = attempt(ctx, it, "fix_engine")
        return StageOutcome("GATE", attempts=[a], data={"engine_fix": True, "gap_id": "G-1"},
                            merge_request=MergeRequest(attempt_id=a.attempt_id, family="codex", model=None,
                                                       artifacts={"diff": "d"}, engine=True, gap_id="G-1"))
    merger = FakeMerger(lambda: None, results=[MergeResult(ok=True, sha="e" * 40, branch="card-loop/r1/engine-G-1")])
    cmds = FakeCommands().on(("merge-base", "--is-ancestor"), 1)
    h1 = Harness(tmp_path, [seed(C1, "FIX")], [Exec("FIX", fix), oracle_confirms()],
                 gate=FakeGate([GateResult(True)]), merger=merger, run_command=cmds)
    result = h1.run()
    assert h1.states() == {C1: "GATE"} and result.stop.reason == "blocked"
    assert "card-loop/r1/engine-G-1" in result.blocked[C1] and "human" in result.blocked[C1]
    assert h1.state.records[C1].data["engine_wait"]["sha"] == "e" * 40

    landed = FakeCommands().on(("merge-base", "--is-ancestor", "e" * 40, "HEAD"), 0)
    oracle = oracle_confirms()
    h2 = Harness(tmp_path, [seed(C1, "FIX")], [Exec("FIX", fix), oracle], gate=FakeGate([]),
                 run_command=landed, ticker=h1.ticker)
    h2.run()
    assert oracle.calls == [C1] and h2.states() == {C1: "CONFIRMED"}
    assert h2.state.records[C1].data["engine_wait"] is None


def test_parked_cards_record_their_gaps_through_the_gap_lane(tmp_path):
    card = "card:BT1-001"

    class RecordingGaps(FakeGaps):
        def __init__(self):
            super().__init__()
            self.recorded = []

        def record_gap(self, gap, kind, *, attempt_id=None, family=None, model=None, commit=True):
            self.recorded.append((gap["id"], kind, attempt_id, family))
            return True

    def park(ctx, it):
        a = attempt(ctx, it, "implement")
        return StageOutcome("PARKED", attempts=[a], data={
            "gap_id": "G-DSL-1", "implement_attempt": a.attempt_id, "implementer_family": "claude",
            "gaps": [{"kind": "dsl", "id": "G-DSL-1", "summary": "s"},
                     {"kind": "engine", "id": "G-ENG-2", "summary": "t"}]})
    gaps = RecordingGaps()
    h = Harness(tmp_path, [seed(card, "IMPLEMENTING")], [Exec("IMPLEMENTING", park)], gaps=gaps,
                manage_tree=True, repo=tmp_path / "t")
    h.run()
    assert gaps.recorded == [("G-DSL-1", "dsl", "A001", "claude"), ("G-ENG-2", "engine", "A001", "claude")]
    assert gaps.park_calls == [(card, "G-DSL-1", True)]


def test_executors_see_the_live_item_map_and_the_merger_is_closed(tmp_path):
    seen = {}

    def author(ctx, it):
        seen["live"] = ctx.items is h.state.records and ctx.items[it.item].state == "AUTHORING"
        seen["unset"] = (ctx.harness_bin, ctx.clause_text_json, ctx.sleep)
        return StageOutcome("SIM", attempts=[attempt(ctx, it, "author_clause")])

    class ClosingMerger(FakeMerger):
        closed = 0

        def close(self, ctx):
            ClosingMerger.closed += 1

    h = Harness(tmp_path, [seed(C1, "AUTHORING")], [Exec("AUTHORING", author), sim_ok(), oracle_confirms()],
                merger=ClosingMerger(lambda: None))
    h.run()
    assert seen == {"live": True, "unset": (None, None, None)}
    assert ClosingMerger.closed == 1


def test_ensure_run_tree_creates_and_reuses_the_run_branch(tmp_path):
    import subprocess

    def git(cwd, *args):
        return subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True, check=True).stdout

    repo = tmp_path / "repo"
    repo.mkdir()
    git(repo, "init", "-q")
    git(repo, "config", "user.email", "t@example.com")
    git(repo, "config", "user.name", "t")
    (repo / "a.txt").write_text("a\n", encoding="utf-8")
    git(repo, "add", "a.txt")
    git(repo, "commit", "-q", "-m", "init")
    base = git(repo, "rev-parse", "HEAD").strip()

    assert drv.run_branch("r1") == "card-loop/r1/run"
    tree = drv.default_run_tree(tmp_path / "runs" / "r1", "r1", tmp_path / "wt")
    assert tree == tmp_path / "wt" / "cl-r1-run"
    assert drv.ensure_run_tree(repo, "r1", base, tree) == tree
    assert git(tree, "symbolic-ref", "--short", "HEAD").strip() == "card-loop/r1/run"
    assert drv.ensure_run_tree(repo, "r1", base, tree) == tree, "reused on resume"

    other = tmp_path / "other"
    git(repo, "worktree", "add", "-q", "-b", "elsewhere", str(other))
    with pytest.raises(drv.RunTreeError, match="not the run branch"):
        drv.ensure_run_tree(repo, "r1", base, other)
    # an engine branch can still be created beside the run branch
    git(repo, "branch", "card-loop/r1/engine-G-1", base)


# --------------------------------------------------------------------------- components + CLI


def test_default_components_reports_what_is_not_built(monkeypatch):
    def missing(name, *a, **k):
        raise ModuleNotFoundError(f"No module named {name!r}", name=name)
    monkeypatch.setattr(drv.importlib, "import_module", missing)
    comps = drv.default_components(LoopConfig())
    assert comps.executors == {} and comps.gate is None and comps.merger is None and comps.gaps is None
    assert set(comps.missing) == {"stages", "gate", "merger", "gaps"}
    assert all("not built yet" in why for why in comps.missing.values())


def test_default_components_wires_the_group6_constructors(monkeypatch, tmp_path):
    ex = Exec("AUTHORING", lambda ctx, it: None)

    class FixGate:
        def check(self, ctx, item, merge):
            return GateResult(True)

    class RunGapLane:
        def __init__(self, run_dir, plan, *, repo=None):
            self.run_dir, self.plan, self.repo = run_dir, plan, repo

    class LoopMerger:
        def __init__(self, *, gap_lane=None, worktree_root=None):
            self.gap_lane, self.worktree_root = gap_lane, worktree_root

    mods = {
        "tools.card_loop.stages": types.SimpleNamespace(build_executors=lambda: {"AUTHORING": ex}),
        "tools.card_loop.gates": types.SimpleNamespace(FixGate=FixGate),
        "tools.card_loop.gaps": types.SimpleNamespace(RunGapLane=RunGapLane),
        "tools.card_loop.merge": types.SimpleNamespace(LoopMerger=LoopMerger),
    }
    monkeypatch.setattr(drv.importlib, "import_module", lambda name, *a, **k: mods[name])
    comps = drv.default_components(LoopConfig(), run_dir=tmp_path, plan=PLAN, repo=tmp_path / "tree",
                                   worktree_root="W:/wt")
    assert comps.missing == {}
    assert comps.executors == {"AUTHORING": ex}
    assert isinstance(comps.gate, FixGate)
    assert (comps.gaps.run_dir, comps.gaps.plan, comps.gaps.repo) == (tmp_path, PLAN, tmp_path / "tree")
    assert comps.merger.gap_lane is comps.gaps and comps.merger.worktree_root == "W:/wt"

    mods["tools.card_loop.merge"] = types.SimpleNamespace()
    comps = drv.default_components(LoopConfig())
    assert "the run directory" in comps.missing["gaps"]
    assert "LoopMerger" in comps.missing["merger"]


def test_default_components_loads_the_real_modules(tmp_path):
    comps = drv.default_components(LoopConfig(), run_dir=tmp_path, plan=PLAN, repo=tmp_path)
    assert comps.missing == {}, comps.missing
    assert {"IMPLEMENTING", "REVIEW", "AUTHORING", "SIM", "ORACLE", "TRIAGE", "FIX"} <= set(comps.executors)
    assert "GATE" not in comps.executors and "PENDING" not in comps.executors, "the driver's own"
    assert comps.merger.gap_lane is comps.gaps


def test_load_fake_workers(tmp_path):
    from tools.card_loop.contracts import TaskPacket
    canned = tmp_path / "canned.json"
    canned.write_text(json.dumps({
        "responses": [
            {"family": "claude", "stage": "author_clause", "item": C1,
             "result": {"status": "ok", "result": {"scenario": "a.yaml"}, "usage": {"cost_usd": 0.5}}},
            {"stage": "triage", "results": [{"status": "ok", "result": {"call": "dcgo_quirk"}},
                                            {"status": "ok", "result": {"call": "ours_wrong"}}]},
        ]}), encoding="utf-8")
    workers = drv.load_fake_workers(canned)
    assert set(workers) == {"claude", "codex"}

    def pk(stage, family, item=C1):
        return TaskPacket(stage=stage, family=family, item=item, attempt_id="x", prompt="p",
                          prompt_version="v", schema_path="s.json", worktree=".")
    r = workers["claude"].run(pk("author_clause", "claude"))
    assert r.result == {"scenario": "a.yaml"} and r.usage.cost_usd == 0.5
    assert workers["codex"].run(pk("triage", "codex")).result == {"call": "dcgo_quirk"}
    assert workers["codex"].run(pk("triage", "codex")).result == {"call": "ours_wrong"}
    with pytest.raises(LookupError):
        workers["codex"].run(pk("author_clause", "codex"))


@pytest.fixture
def cli_ws(tmp_path, monkeypatch):
    """A plan on a one-card pool over fixture ledgers; fake executors that use
    the --fake workers through the RunContext."""
    from tools.card_loop.ledger import attempt_from_call
    from tools.card_loop.contracts import TaskPacket
    from tools.card_loop.state import LedgerPaths

    cards = tmp_path / "cards" / "bt7"
    cards.mkdir(parents=True)
    (cards / "BT7-056.yaml").write_text("card: BT7-056\n", encoding="utf-8")
    (tmp_path / "scen").mkdir()
    (tmp_path / "verdicts").mkdir()
    den = tmp_path / "den.json"
    den.write_text(json.dumps({"version": 1, "cards": {"BT7-056": []}, "interactions": {}}), encoding="utf-8")
    paths = LedgerPaths(cards_dir=tmp_path / "cards", scenarios_dir=tmp_path / "scen",
                        verdicts_dir=tmp_path / "verdicts", denominator_path=den,
                        escalations_dir=tmp_path / "unused-esc")
    monkeypatch.setattr(drv, "ledger_paths", lambda repo, config: LedgerPaths(
        **{**paths.__dict__, "escalations_dir": Path(config.escalations_dir)}))

    def author(ctx, it):
        aid = ctx.new_attempt_id()
        packet = TaskPacket(stage="author_clause", family="claude", item=it.item, attempt_id=aid,
                            prompt="p", prompt_version="test@1", schema_path="s.json", worktree=".")
        res = ctx.workers["claude"].run(packet)
        return StageOutcome("SIM", attempts=[attempt_from_call(packet, res, run_id=ctx.run_id,
                                                               assignment="routed", outcome="accepted")],
                            data={"scenario": res.result["scenario"]})

    monkeypatch.setattr(drv, "default_components", lambda config=None, **kw: drv.Components(
        executors={"AUTHORING": Exec("AUTHORING", author), "SIM": sim_ok(), "ORACLE": oracle_confirms()},
        missing={"merger": "not built yet (test)"}))

    run_dir = tmp_path / "runs" / "cli1"
    run_dir.mkdir(parents=True)
    plan = {"version": 1, "run_id": "cli1", "base_sha": None,
            "config": {"plateau_attempts": 10},
            "work_set": {"pool": ["BT7-056"], "core": ["BT7-056"], "ranking": ["BT7-056"], "decklists": None},
            "dcgo": {"root": None, "scripts": {"BT7-056": "x.cs"}, "unavailable": []},
            "preflight": {"go": False, "checks": []}}
    (run_dir / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
    canned = tmp_path / "canned.json"
    one = {"status": "ok", "result": {"scenario": "s.yaml"}, "usage": {"cost_usd": 0.25}}
    canned.write_text(json.dumps({"responses": [
        {"family": "claude", "stage": "author_clause", "results": [one, one]}]}), encoding="utf-8")
    return {"run_dir": run_dir, "plan": run_dir / "plan.json", "canned": canned, "runs": tmp_path / "runs"}


def test_cli_run_fake_then_status_then_resume(cli_ws, capsys):
    rc = drv.cli_run(["--plan", str(cli_ws["plan"]), "--fake", str(cli_ws["canned"]), "--serial"])
    out = capsys.readouterr().out
    assert rc == 0, out
    assert "merger" in out and "not built" in out, "names what is missing"
    assert (cli_ws["run_dir"] / "report.md").exists() and (cli_ws["run_dir"] / "state.json").exists()
    fake = cli_ws["run_dir"] / "fake"
    assert (fake / "attempts.jsonl").exists(), "--fake never writes the committed ledgers"
    assert len(load_attempts(fake / "attempts.jsonl")[0]) == 2
    first = (cli_ws["run_dir"] / "report.md").read_text(encoding="utf-8").splitlines()[0]
    assert first.startswith("**0 unmeasured, 0 escalated, 0 unavailable** -- 2 confirmed")

    rc = drv.cli_run(["--plan", str(cli_ws["plan"]), "--fake", str(cli_ws["canned"])])
    assert rc == 2 and "resume" in capsys.readouterr().err

    rc = drv.cli_status(["--run", "cli1", "--runs-dir", str(cli_ws["runs"])])
    out = capsys.readouterr().out
    assert rc == 0 and "2 confirmed" in out and "CONFIRMED" in out

    rc = drv.cli_resume(["--run", "cli1", "--runs-dir", str(cli_ws["runs"])])
    assert rc == 2 and "--fake" in capsys.readouterr().err, "a fake run never resumes on real workers"

    rc = drv.cli_resume(["--run", "cli1", "--runs-dir", str(cli_ws["runs"]), "--fake", str(cli_ws["canned"])])
    assert rc == 0, capsys.readouterr()


def test_cli_resume_refuses_fake_workers_on_a_real_run(cli_ws, capsys):
    (cli_ws["run_dir"] / "events.jsonl").write_text("", encoding="utf-8")   # started for real
    rc = drv.cli_resume(["--run", "cli1", "--runs-dir", str(cli_ws["runs"]), "--fake", str(cli_ws["canned"])])
    assert rc == 2 and "real workers" in capsys.readouterr().err


def test_cli_run_refuses_a_no_go_plan_without_fake(cli_ws, capsys):
    rc = drv.cli_run(["--plan", str(cli_ws["plan"])])
    assert rc == 1 and "NO-GO" in capsys.readouterr().err
    assert not (cli_ws["run_dir"] / "events.jsonl").exists()


# --------------------------------------------------------------------------- the operator's STOP file


def test_a_stop_file_present_at_start_stops_before_any_work(tmp_path):
    author = author_ok()
    h = Harness(tmp_path, [seed(C1, "AUTHORING")], [author, sim_ok(), oracle_confirms()])
    h.run_dir.mkdir(parents=True, exist_ok=True)
    (h.run_dir / drv.STOP_FILE).write_text("", encoding="utf-8")
    result = h.run()
    assert result.stop.reason == "operator" and "STOP" in result.stop.detail
    assert author.calls == [] and h.states() == {C1: "AUTHORING"}
    assert (h.run_dir / "state.json").exists() and (h.run_dir / "report.md").exists()


def test_a_stop_file_written_mid_run_drains_in_flight_work_and_starts_nothing_new(tmp_path):
    # The second pilot was stopped twice with `taskkill /T /F`: in-flight
    # attempts were lost and a stale index.lock blocked every later merge.
    holder = {}

    def author_then_stop(ctx, it):
        (holder["run_dir"] / drv.STOP_FILE).write_text("", encoding="utf-8")
        return StageOutcome("SIM", reason="authored", attempts=[attempt(ctx, it, "author_clause")])

    author = Exec("AUTHORING", author_then_stop)
    sim = sim_ok()
    h = Harness(tmp_path, [seed(C1, "AUTHORING"), seed(C2, "AUTHORING")], [author, sim, oracle_confirms()])
    holder["run_dir"] = h.run_dir
    result = h.run()
    assert result.stop.reason == "operator"
    assert author.calls == [C1], "the first task completed; no new task started"
    assert h.states()[C1] == "SIM" and h.states()[C2] == "AUTHORING"
    assert sim.calls == [], "SIM was not started after the stop"


def test_report_tells_how_to_land_the_verdicts_into_readiness():
    # Rule 34: the readiness artifact is regenerated after any verdict change,
    # and a run's verdicts only count once its tree is merged. The report says so,
    # with the exact commands, whenever the run adjudicated anything.
    total = {"items": 3, "confirmed": 2, "terminal": 0, "unavailable": 0, "implemented": 0,
             "adjudicated": 2, "escalated": 0, "unmeasured": 1, "parked": 0}
    counts = {"total": total, "clause": dict(total)}
    stop = {"reason": "complete", "spent_usd": 1.0}
    text = report_mod.render_report(run_id="r", counts=counts, by_state={}, stop=stop,
                                    landing={"tree": "D:/cl-x/run", "branch": "card-loop/r/run"})
    assert "## Landing the verdicts" in text
    assert "card-loop/r/run" in text and "D:/cl-x/run" in text
    assert "python -m tools.clause_coverage.readiness" in text
    assert "--plan" in text
    # nothing adjudicated: nothing to land, no section
    none = {"total": dict(total, adjudicated=0, confirmed=0)}
    assert "## Landing the verdicts" not in report_mod.render_report(
        run_id="r", counts=none, by_state={}, stop=stop,
        landing={"tree": "D:/cl-x/run", "branch": "card-loop/r/run"})
    # a driver that does not manage a run tree (tests, fakes) has nothing to merge
    assert "## Landing the verdicts" not in report_mod.render_report(
        run_id="r", counts=counts, by_state={}, stop=stop)
