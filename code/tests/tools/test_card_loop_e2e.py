"""End-to-end driver run on fakes (openspec add-card-authoring-loop, task 6.9).

The REAL stage executors (`stages/`) run inside the REAL driver, with canned
workers (`FakeWorker`), canned harness commands (`FakeCommands`) and fakes for
the merger / gate / gap lane. It proves the whole loop composes: confirm,
diverge -> fix -> gate -> confirm, diverge -> quirk (both families agree),
diverge -> escalate (they disagree), park -> unpark, and resume after a crash
with only the outstanding work re-run. No subprocess, no model, no Unity.
"""
from __future__ import annotations

import json
import threading
from pathlib import Path

from tools.card_loop import driver as drv
from tools.card_loop import escalations as esc_mod
from tools.card_loop.config import LoopConfig
from tools.card_loop.contracts import split_item_id
from tools.card_loop.driver_contracts import GateResult, MergeResult
from tools.card_loop.ledger import load_attempts
from tools.card_loop.stages import build_executors
from tools.card_loop.stages.testing import FakeCommands, ok, sources
from tools.card_loop.state import ItemMeta, ItemSeed, Origin, RunState
from tools.card_loop.workers.fake import FakeWorker

CARD = "ST23-04"
EXAMS = "qa/dcgo-exams/ST23"
POOL = f"{EXAMS}/glowing_dawn_pool.json"


def clause(n: int) -> str:
    return f"{CARD}#effect#{n}"


def scenario_path(n: int) -> str:
    return f"{EXAMS}/{CARD}-effect{n}.yaml"


def scenario_text(n: int) -> str:
    return (f"card: {CARD}\nclause: {clause(n)}\nseed: 1\n"
            "decks:\n  p0: { stack: [ST23-04], rest: gd-exam }\n  p1: { stack: [], rest: gd-exam }\n"
            "steps:\n  - actor: 0\n    do: { pass: {} }\n  - actor: 0\n    do: { play: { card: ST23-04, from: hand } }\n")


def clause_row(n: int) -> dict:
    return {"id": clause(n), "card_id": CARD, "zone": "effect", "label": f"[Effect {n}]", "kind": "timing",
            "timings": ["On Play"], "keyword": None, "text": f"[On Play] clause {n}"}


SIM_PASS = ("exam: x\n  lowered 2 step(s): []\n  assert: 0 check(s) over 0 assertion block(s), 0 failed\n"
            "exam: scenarios seen 1 / lowered 1 / run 1 / diffed 0 / failed 0\n")


def oracle_row(n: int, verdict: str) -> str:
    row = {"scenario": scenario_path(n), "clause": clause(n), "ids": [clause(n)], "verdict": verdict,
           "first_divergence": None, "divergence": None, "mismatch": None, "reason": None,
           "denominator": "2 rows", "job_id": f"exam-{CARD}-effect{n}", "job_outcome": "completed",
           "sidecar": "s.jsonl", "backfilled": verdict == "confirmed", "backfill_note": None,
           "recorded": [clause(n)], "refused": []}
    if verdict == "diverged":
        row.update(first_divergence="DIVERGED at step 1", reason="DIVERGED at step 1",
                   divergence={"step": 1, "field": "p1.field", "ours": "[]", "dcgo": "[x]"})
    return json.dumps(row) + "\n"


TRI_OURS = {"classification": "ours_wrong", "citation": {"kind": "rule", "ref": "16-36"},
            "reasoning": "the -DP applies to Digimon only per 16-36"}
TRI_QUIRK = {"classification": "dcgo_quirk", "citation": {"kind": "rule", "ref": "7-3-1"},
             "reasoning": "DCGO asks an extra prompt the rules do not have"}
FIX_OK = {"files": [f"code/digimon-engine/cards/st23/{CARD}.yaml"], "tests": ["st23::st23_04::minus_dp"],
          "test_result_lines": ["test result: ok. 1 passed"], "gaps": [],
          "citation": {"kind": "rule", "ref": "16-36"}, "notes": ""}
IMPL_GAP = {"files": [], "tests": [], "test_result_lines": [], "notes": "needs a DSL verb",
            "gaps": [{"kind": "dsl", "id": "G-DSL-E2E", "summary": "no verb for the clause"}]}
IMPL_OK = {"files": [f"code/digimon-engine/cards/st23/{CARD}.yaml"],
           "tests": [f"code/digimon-engine/tests/cards_behavioral/st23/st23_04.rs"],
           "test_result_lines": ["test result: ok. 3 passed"], "gaps": [], "notes": ""}
REVIEW_OK = {"verdict": "accept", "directives": [], "summary": "faithful"}


# --------------------------------------------------------------------------- fakes


class Merger:
    def __init__(self):
        self.calls = []

    def merge(self, ctx, request):
        self.calls.append(request)
        return MergeResult(ok=True, sha=f"sha-{len(self.calls)}", branch="card-loop/r1/run",
                           touched=list((request.artifacts or {}).get("files", [])))

    def close(self, ctx):
        pass


class Gate:
    def __init__(self):
        self.calls = []

    def check(self, ctx, item, merge):
        self.calls.append(item.item)
        return GateResult(passed=True, evidence={"merge": merge.sha if merge else None})


class Gaps:
    """Closes every parked gap on the second poll."""

    def __init__(self):
        self.parked: dict[str, str] = {}
        self.park_calls: list[tuple[str, str]] = []
        self.polls = 0
        self.recorded = []

    def park(self, item, gap_id, *, is_core):
        self.park_calls.append((item, gap_id))
        self.parked[item] = gap_id

    def unpark(self, gap_id):
        items = [i for i, g in self.parked.items() if g == gap_id]
        for i in items:
            del self.parked[i]
        return items

    def closed(self, ctx):
        self.polls += 1
        return sorted(set(self.parked.values())) if self.polls >= 2 else []

    def ranked(self):
        return []

    def note_branch(self, gap_id, branch, sha=None):
        pass

    def record_gap(self, gap, kind, **kw):
        self.recorded.append((gap, kind))
        return True


class Ticker:
    def __init__(self):
        self.n, self.a, self.lock = 0, 0, threading.Lock()

    def now(self):
        with self.lock:
            self.n += 1
            return f"2026-10-06T00:{self.n // 60:02d}:{self.n % 60:02d}Z"

    def new_id(self):
        with self.lock:
            self.a += 1
            return f"A{self.a:03d}"


def seed(item, state="PENDING", *, requires=()):
    kind, ident = split_item_id(item)
    return ItemSeed(item, ItemMeta(cards=(ident.split("#")[0],), is_core=True, rank=0,
                                   requires=tuple(requires), source=None),
                    Origin(state, f"seed {state}", {}, False), {})


PLAN = {"version": 1, "run_id": "r1", "base_sha": "abc",
        "work_set": {"pool": [CARD], "core": [CARD], "ranking": [CARD]},
        "dcgo": {"root": None, "scripts": {}, "unavailable": []}}


def repo_with_scenarios(tmp_path: Path, ns) -> Path:
    repo = tmp_path / "repo"
    (repo / EXAMS).mkdir(parents=True)
    (repo / POOL).write_text(json.dumps({"decks": [{"name": "gd-exam", "cards": [CARD] * 50,
                                                    "eggs": ["ST23-01"] * 4}]}), encoding="utf-8")
    for n in ns:
        (repo / scenario_path(n)).write_text(scenario_text(n), encoding="utf-8")
    return repo


def workers(repo: Path, author_writes: int | None = None):
    def author(packet):
        n = author_writes
        (repo / scenario_path(n)).write_text(scenario_text(n), encoding="utf-8")
        return ok({"scenario_paths": [scenario_path(n)], "covers": [clause(n)], "notes": "authored"})

    claude = {
        ("author_clause", f"clause:{clause(1)}"): author,
        ("triage", f"clause:{clause(2)}"): ok(TRI_OURS),
        ("fix_card", f"clause:{clause(2)}"): ok(FIX_OK),
        ("triage", f"clause:{clause(3)}"): ok(TRI_QUIRK),
        ("triage", f"clause:{clause(4)}"): ok(TRI_QUIRK),
        ("implement", f"card:{CARD}"): [ok(IMPL_GAP), ok(IMPL_OK)],
    }
    codex = {
        ("triage", f"clause:{clause(3)}"): ok(TRI_QUIRK),           # agrees: dcgo_quirk, cited
        ("triage", f"clause:{clause(4)}"): ok(TRI_OURS),            # disagrees -> escalate
        ("review", f"card:{CARD}"): ok(REVIEW_OK),
    }
    return {"claude": FakeWorker(claude, family="claude"), "codex": FakeWorker(codex, family="codex")}


def commands() -> FakeCommands:
    c = FakeCommands().on("--sim-only", 0, SIM_PASS)
    c.on(("--oracle", f"effect1.yaml"), 0, oracle_row(1, "confirmed"))
    c.on(("--oracle", f"effect2.yaml"), 0, oracle_row(2, "diverged"))
    c.on(("--oracle", f"effect2.yaml"), 0, oracle_row(2, "confirmed"))      # after the fix
    c.on(("--oracle", f"effect3.yaml"), 0, oracle_row(3, "diverged"))
    c.on(("--oracle", f"effect4.yaml"), 0, oracle_row(4, "diverged"))
    c.on("verdict-triage", 0, "verdict-triage: ok")
    c.on("verdict-set", 0, "verdict-set: ok")
    return c


def cfg(tmp: Path, **kw) -> LoopConfig:
    base = dict(attempts_path=str(tmp / "ledger" / "attempts.jsonl"), escalations_dir=str(tmp / "escalations"),
                concurrency=3, plateau_attempts=50, exploration_share=0.0,
                harness_root="H:/root", player_dir="P:/player")
    base.update(kw)
    return LoopConfig(**base)


def build(tmp_path: Path, *, seeds, repo: Path, config: LoopConfig, ticker: Ticker, cmds: FakeCommands,
          gaps=None, merger=None, gate=None, **kw):
    run_dir = tmp_path / "run"
    state = RunState.from_seeds("r1", run_dir, seeds, now=ticker.now,
                                escalated=esc_mod.load_escalated_items(config.escalations_dir))
    comps = drv.Components(executors=build_executors(), gate=gate or Gate(), merger=merger or Merger(),
                           gaps=gaps or Gaps())
    d = drv.Driver(PLAN, config, comps, workers=workers(repo, author_writes=1), state=state, run_dir=run_dir,
                   repo=repo, run_command=cmds, now=ticker.now, clock=lambda: 0.0,
                   new_attempt_id=ticker.new_id, serial=True, harness_bin="dcgo-harness",
                   clause_text_json="qa/exam-clause-text.json", **kw)
    d.ctx.sources = sources(clauses=[clause_row(n) for n in range(1, 5)])
    return d


def all_seeds():
    return [seed(f"card:{CARD}")] + [seed(f"clause:{clause(n)}") for n in range(1, 5)]


# --------------------------------------------------------------------------- the run


def test_the_whole_loop_composes_on_fakes(tmp_path):
    repo = repo_with_scenarios(tmp_path, [2, 3, 4])       # clause 1's scenario is authored by the worker
    ticker, cmds, gaps, merger, gate = Ticker(), commands(), Gaps(), Merger(), Gate()
    d = build(tmp_path, seeds=all_seeds(), repo=repo, config=cfg(tmp_path), ticker=ticker, cmds=cmds,
              gaps=gaps, merger=merger, gate=gate)
    result = d.run()

    states = {i: r.state for i, r in d.state.records.items()}
    assert states[f"clause:{clause(1)}"] == "CONFIRMED", states
    assert states[f"clause:{clause(2)}"] == "CONFIRMED", states          # diverged -> fix -> gate -> oracle
    assert states[f"clause:{clause(3)}"] == "TERMINAL", states           # both families: dcgo_quirk
    assert states[f"clause:{clause(4)}"] == "ESCALATED", states          # families disagree
    assert states[f"card:{CARD}"] == "IMPLEMENTED", states               # parked on a gap, then unparked

    # The clause-1 scenario was written by the authoring worker and sim-checked.
    assert (repo / scenario_path(1)).exists()
    assert any("--sim-only" in " ".join(c["argv"]) and "effect1.yaml" in " ".join(c["argv"]) for c in cmds.calls)
    # Two oracle runs for clause 2: the divergence, then the confirmation after the fix.
    assert len([c for c in cmds.calls if "--oracle" in c["argv"] and any("effect2.yaml" in a for a in c["argv"])]) == 2
    # The fix was merged and gated; the implementation was merged once (the gap attempt merged nothing).
    assert [r.engine for r in merger.calls].count(False) >= 2
    assert f"clause:{clause(2)}" in gate.calls
    # The quirk was triaged into the store; the ESCALATED item has a queue entry with both arguments.
    assert any("verdict-triage" in c["argv"] for c in cmds.calls)
    esc_files = list(Path(d.config.escalations_dir).glob("*.md"))
    assert len(esc_files) == 1
    text = esc_files[0].read_text(encoding="utf-8")
    assert "dcgo_quirk" in text and "ours_wrong" in text and "7-3-1" in text and "16-36" in text
    # The gap lane saw the card parked on its gap, and the card re-entered after the gap closed.
    # (Tracker writes through `record_gap` happen only when the driver manages a real run tree.)
    assert gaps.park_calls == [(f"card:{CARD}", "G-DSL-E2E")]
    assert not gaps.parked
    # Every worker call is in the ledger with the family that made it.
    attempts = load_attempts(d.config.attempts_path)[0]
    stages = sorted({(a.stage, a.family) for a in attempts})
    assert ("implement", "claude") in stages and ("review", "codex") in stages
    assert ("triage", "claude") in stages and ("triage", "codex") in stages
    assert ("fix_card", "claude") in stages and ("author_clause", "claude") in stages
    # The report leads with the unadjudicated.
    report = (tmp_path / "run" / "report.md").read_text(encoding="utf-8").splitlines()[0]
    assert "1 escalated" in report and "0 unmeasured" in report and report.index("unmeasured") < report.index("confirmed")
    assert result.stop.reason == "complete", result.stop


def test_resume_after_a_crash_reruns_only_the_outstanding_work(tmp_path):
    repo = repo_with_scenarios(tmp_path, [1, 2, 3, 4])
    ticker = Ticker()
    config = cfg(tmp_path)
    # First session: stop after two worker attempts ("crash" mid-run).
    d1 = build(tmp_path, seeds=all_seeds(), repo=repo, config=config, ticker=ticker, cmds=commands(),
               max_attempts=2)
    r1 = d1.run()
    assert r1.stop.reason != "complete"
    before = {i: r.state for i, r in d1.state.records.items()}
    done_before = {i for i, s in before.items() if s in ("CONFIRMED", "TERMINAL")}
    # Tear the last event line the way a crash would.
    events = tmp_path / "run" / "events.jsonl"
    text = events.read_text(encoding="utf-8")
    events.write_text(text + '{"ts": "x", "run_id": "r1", "item": "cla', encoding="utf-8")

    # Second session over the same run dir and ledgers.
    cmds2 = commands()
    d2 = build(tmp_path, seeds=all_seeds(), repo=repo, config=config, ticker=ticker, cmds=cmds2)
    resumed = {i: r.state for i, r in d2.state.records.items()}
    for item in done_before:
        assert resumed[item] == before[item], (item, resumed[item])
    r2 = d2.run()
    after = {i: r.state for i, r in d2.state.records.items()}
    assert r2.stop.reason == "complete", (r2.stop, after)
    assert after[f"clause:{clause(1)}"] == "CONFIRMED" and after[f"clause:{clause(4)}"] == "ESCALATED"
    # Nothing already adjudicated before the crash went back to the oracle.
    for item in done_before:
        n = item.split("#")[-1]
        assert not any("--oracle" in c["argv"] and any(f"effect{n}.yaml" in a for a in c["argv"]) for c in cmds2.calls), item
