"""The card-loop driver (design D1, D4, D5, D13; tasks 6.1, 6.4, 6.8).

Code orchestrates; models are stateless workers behind stage executors. The
driver owns the per-item state machine (`state.RunState`), the scheduler, the
merge-before-transition rule, the fix gate step, the gap lane hand-off, the
attempt / correction ledgers, the escalation queue, the stop rules and the
run report. It builds ONLY against `driver_contracts`: the executors
(`stages/`), the fix gate (`gates.py`), the gap lane (`gaps.py`) and the
merger (`merge.py`) arrive through `Components`, and `default_components()`
imports them lazily, reporting any that are not built yet.

**Scheduling.** Each round: stop rules first; PARKED items whose gap closed go
back to IMPLEMENTING; the two driver-built-in steps (PENDING -> first stage by
kind, DIVERGED -> TRIAGE) run inline when no executor claims those states;
then ready items are picked in priority order (core first, plan ranking, card
before clause before interaction, id) -- skipping items waiting on a card
item, parked items and states with no executor. Worker-stage states and GATE
share `config.concurrency` slots on a thread pool; ORACLE runs strictly one at
a time on its own lane. Outcomes are applied on the main thread only, in
submission order when several finish together. `serial=True` runs one task
per round inline (deterministic; used by tests and the end-to-end fake run).

**Applying an outcome.** `check_transition` first (a refused transition is
logged as a note, its spend still ledgered, the item set aside for the
session); then a `merge_request` is merged BEFORE the transition -- a failed
merge keeps the item in its state with `data["merge_error"]`, rewrites the
attempt's outcome to `gate_failed`, logs a `gate_fail` correction by the
`merge` gate, and escalates once the stage's attempt cap is spent; then
attempts and corrections are ledgered, an escalation file is written for
ESCALATED, the transition event is appended, PARKED items are handed to the
gap lane. GATE is driver-owned: `Gate.check(ctx, item, last merge)`; pass ->
ORACLE, fail -> FIX with the reasons (a `gate_fail` correction against the
fix attempt), or ESCALATED when the fix stage's cap is spent. An ENGINE fix
that passes stays in GATE (`data["engine_wait"]`) until a human has merged
its `card-loop/<run>/engine-*` branch into the run branch (its sha is an
ancestor of the run tree's HEAD, checked every round and on resume), because
the oracle runs on the run tree. An executor that raises
`driver_contracts.StageDeferred` (a failed worker call, no family for a
non-terminating call) leaves the item where it is: its attempts and
corrections are ledgered, one attempt counts toward the stage's cap, and the
cap escalates.

**The run tree** (`manage_tree=True`; the CLI's real runs, and `--fake
--run-tree`): `ctx.repo` is a worktree on `card-loop/<run>/run` (never bare
`card-loop/<run>`: engine branches live under it). Before the first ORACLE
call of a session the clause-text book is extended to the whole pool
(`tools.clause_coverage.book add`) and committed; after every ORACLE and
TERMINATION_CHECK step the tree writes their executors make (verdicts,
`--backfill`, `verdict-triage`) are committed; at a stop the ledgers are.
Every commit carries `Loop-Attempt` / `Loop-Model: driver/default` /
`Loop-Run` trailers. Parked cards' reported gaps go to the trackers through
`GapLane.record_gap`. Nothing is committed off a run branch.

**Attempt caps** are per item per stage (`config.attempt_caps`), counted per
executor step that made worker calls (a two-family termination check is ONE
`triage` pass), restored on resume from the event log, and checked before an
executor runs: a spent cap escalates with the item's attempt history.
ORACLE -> ORACLE retries count under `oracle_retry` (cap
`attempt_caps["oracle_retry"]`, default 3). An upstream attempt restarts its
dependents' counters (`CAP_RESETS`: a new implement attempt may be reviewed
again, a new fix triaged again). An item that takes `MAX_IDLE_STEPS`
consecutive executor steps with no worker attempt and no adjudication is set
aside for the session, so an executor loop cannot spin below every cap.

**Stop rules** (D13), checked before every submission; in-flight work drains:
USD cap (`config.budget_usd`, summed over every attempt this run id has
ledgered, across sessions), wall-clock cap (`config.wall_clock_hours`, this
session, injectable clock), plateau (`config.plateau_attempts` consecutive
worker attempts with no new adjudication; this session), `max_attempts`
(worker attempts this session). With nothing runnable the run stops
`complete` (every item terminal) or `blocked`; Ctrl-C stops `interrupted`
(in-flight work drains first) and a driver exception stops `error` (then
re-raises). Every stop writes `state.json` and `report.md`. The run-level USD
cap stops the run; it does not escalate the open items (they stay unmeasured
and resume picks them up under a new cap).
"""
from __future__ import annotations

import argparse
import copy
import dataclasses
import importlib
import inspect
import json
import os
import subprocess
import sys
import time
import traceback
from concurrent.futures import FIRST_COMPLETED, ThreadPoolExecutor, wait
from dataclasses import dataclass, field, fields, replace
from pathlib import Path
from typing import Any, Callable, Mapping

from . import corrections as corr_mod
from ._jsonl import REPO_ROOT, utc_now_iso
from .config import LoopConfig, load_config
from .contracts import FAMILIES, WorkerResult, split_item_id
from .corrections import CorrectionLog
from .driver_contracts import (
    STATES_BY_KIND,
    Escalation,
    GateResult,
    ItemRecord,
    MergeRequest,
    MergeResult,
    StageDeferred,
    StageOutcome,
    check_transition,
    is_adjudicated,
    is_terminal,
    stage_for,
)
from .escalations import write_escalation
from .ledger import Attempt, AttemptLedger, dedupe_last, load_attempts, usage_from_dict
from .ledger import new_attempt_id as ledger_new_attempt_id
from .report import first_line, render_report, write_report
from .state import EVENTS_NAME, LedgerPaths, RunState
from .workers.fake import FakeWorker
from .workers.health import VendorHealth

ORACLE_STATES = frozenset({"ORACLE"})        # touch the oracle: one at a time
BUILTIN_STATES = ("PENDING", "DIVERGED")     # driver steps unless an executor claims them
ORACLE_RETRY_KEY = "oracle_retry"
DEFAULT_ORACLE_RETRY_CAP = 3
#: Counting an attempt of the key stage restarts its dependent stages' caps: a
#: re-implemented card may be reviewed again (default review cap 1 would
#: otherwise escalate every card after one "changes requested"), a new fix's
#: re-divergence may be triaged again, a re-authored scenario or a new fix gets
#: fresh oracle retries. The upstream caps (implement 3, fix 2, author 3) still
#: bound the loop.
CAP_RESETS = {
    "implement": ("review",),
    "fix_card": ("triage", ORACLE_RETRY_KEY),
    "fix_engine": ("triage", ORACLE_RETRY_KEY),
    "author_clause": (ORACLE_RETRY_KEY,),
    "author_interaction": (ORACLE_RETRY_KEY,),
}
#: An item that takes this many consecutive executor steps with no worker
#: attempt and no adjudication is set aside for the session (an executor loop
#: such as SIM -> AUTHORING -> SIM that never calls a worker would otherwise
#: spin forever below every cap).
MAX_IDLE_STEPS = 25
FAKE_DIR = "fake"
REPORT_NAME = "report.md"
ALL_STATES = frozenset(s for states in STATES_BY_KIND.values() for s in states)

# Run-tree management (`manage_tree=True`; the CLI turns it on for a run tree).
#: Executors that write into the run tree itself: the oracle's verdicts and
#: `--backfill`, the termination check's `verdict-triage`. After their step
#: the driver commits those writes so the tree never carries them across steps.
TREE_WRITING_STATES = frozenset({"ORACLE", "TERMINATION_CHECK"})
VERDICTS_DIR = "qa/qa-reports/exam-verdicts"
DEFAULT_CLAUSE_TEXT_BOOK = "qa/exam-clause-text.json"
#: `PYTHONPATH=code python -m tools.clause_coverage.book ...` without an env
#: override (`ctx.run_command` takes argv, cwd, timeout only).
BOOK_BOOTSTRAP = ("import runpy, sys; sys.path.insert(0, 'code'); "
                  "runpy.run_module('tools.clause_coverage.book', run_name='__main__', alter_sys=True)")
DRIVER_FAMILY = "driver"
PROTECTED_BRANCHES = ("main", "master")


def run_branch(run_id: str) -> str:
    """The run branch. Never bare `card-loop/<run_id>`: engine fixes land on
    `card-loop/<run_id>/engine-<gap>`, and a ref cannot be a branch and a directory."""
    return f"card-loop/{run_id}/run"


# ---------------------------------------------------------------------------
# components
# ---------------------------------------------------------------------------


@dataclass
class Components:
    """What the other group-6 modules provide. `executors` is keyed by item
    state; `missing` names each component that could not be loaded, and why."""

    executors: dict = field(default_factory=dict)
    gate: Any = None
    merger: Any = None
    gaps: Any = None
    missing: dict = field(default_factory=dict)


#: name -> module. What each must provide (the group-6 modules as built):
#:   stages  `build_executors()` -> {state: executor}   (or `EXECUTORS`)
#:   gate    `FixGate()`
#:   gaps    `RunGapLane(run_dir, plan, repo=<run tree>)`
#:   merger  `LoopMerger(gap_lane=<the RunGapLane>, worktree_root=...)`, `.close(ctx)` at run end
COMPONENT_MODULES = {
    "stages": "tools.card_loop.stages",
    "gate": "tools.card_loop.gates",
    "gaps": "tools.card_loop.gaps",
    "merger": "tools.card_loop.merge",
}


def _call_factory(fn: Callable, config: LoopConfig):
    try:
        sig = inspect.signature(fn)
    except (TypeError, ValueError):
        return fn(config)
    required = [p for p in sig.parameters.values()
                if p.kind in (p.POSITIONAL_ONLY, p.POSITIONAL_OR_KEYWORD) and p.default is p.empty]
    if required:
        return fn(config)
    if "config" in sig.parameters:
        return fn(config=config)
    return fn()


def _executor_registry(value) -> dict:
    pairs = list(value.items()) if isinstance(value, Mapping) else [(getattr(e, "state", None), e) for e in value]
    out = {}
    for state, ex in pairs:
        if state not in ALL_STATES:
            raise ValueError(f"executor {ex!r} names unknown state {state!r}")
        if not callable(getattr(ex, "run", None)):
            raise TypeError(f"executor for {state} has no run(ctx, item)")
        out[state] = ex
    return out


def default_components(config: LoopConfig | None = None, *, run_dir=None, plan: Mapping | None = None,
                       repo=None, worktree_root=None) -> Components:
    """Construct the executors, fix gate, gap lane and merger, tolerating any
    module that is not built (reported in `Components.missing`, never raised).
    `repo` is the run tree the gap lane reads and commits in; the merger gets
    the same gap lane so an engine merge notes its branch on it."""
    config = config or LoopConfig()
    comps = Components()
    mods = {}
    for name, module in COMPONENT_MODULES.items():
        try:
            mods[name] = importlib.import_module(module)
        except ModuleNotFoundError as e:
            if e.name and (e.name == module or module.startswith(e.name + ".")):
                comps.missing[name] = f"not built yet ({module})"
            else:
                comps.missing[name] = f"import failed: {e}"
        except Exception as e:  # a broken module must not take the driver down
            comps.missing[name] = f"import failed: {type(e).__name__}: {e}"

    def build(name: str, attr: str, make: Callable):
        mod = mods.get(name)
        if mod is None:
            return None
        obj = getattr(mod, attr, None)
        if obj is None:
            comps.missing[name] = f"{COMPONENT_MODULES[name]} has no {attr}"
            return None
        try:
            return make(obj)
        except Exception as e:
            comps.missing[name] = f"{COMPONENT_MODULES[name]}.{attr} failed: {type(e).__name__}: {e}"
            return None

    if "stages" in mods:
        stages = mods["stages"]
        factory = getattr(stages, "build_executors", None)
        try:
            value = _call_factory(factory, config) if callable(factory) else getattr(stages, "EXECUTORS")
            comps.executors = _executor_registry(value)
        except Exception as e:
            comps.missing["stages"] = f"executor registry unusable: {type(e).__name__}: {e}"
    comps.gate = build("gate", "FixGate", lambda cls: cls())
    if "gaps" in mods and run_dir is None:
        comps.missing["gaps"] = "the gap lane needs the run directory"
    elif "gaps" in mods:
        comps.gaps = build("gaps", "RunGapLane", lambda cls: cls(run_dir, plan, repo=repo))
    comps.merger = build("merger", "LoopMerger",
                         lambda cls: cls(gap_lane=comps.gaps, worktree_root=worktree_root))
    return comps


def default_workers(config: LoopConfig) -> tuple[dict, dict]:
    """The real Claude / Codex adapters, `({family: worker}, {family: why not})`."""
    specs = {"claude": ("tools.card_loop.workers.claude", "ClaudeWorker"),
             "codex": ("tools.card_loop.workers.codex", "CodexWorker")}
    workers, problems = {}, {}
    for family, (module, cls) in specs.items():
        try:
            workers[family] = getattr(importlib.import_module(module), cls)(config)
        except Exception as e:
            problems[family] = f"{type(e).__name__}: {e}"
    return workers, problems


# ---------------------------------------------------------------------------
# fakes for `--fake`
# ---------------------------------------------------------------------------


def _worker_result(d: Mapping) -> WorkerResult:
    return WorkerResult(status=d.get("status", "ok"), result=d.get("result"),
                        artifacts=dict(d.get("artifacts") or {}), usage=usage_from_dict(d.get("usage") or {}),
                        error=d.get("error"), transcript_path=d.get("transcript_path"))


def load_fake_workers(source) -> dict[str, FakeWorker]:
    """`--fake <canned.json>` -> one `FakeWorker` per family.

    Shape::

        {"default": {<WorkerResult>}?,                       # any unmatched call
         "responses": [{"family"?: "claude"|"codex",          # omitted = both
                        "stage"?: "...", "item"?: "...",      # >= one; key = (stage, item) | item | stage
                        "result": {<WorkerResult>} | "results": [{...}, ...]}]}

    A WorkerResult is `{"status": "ok", "result": {...}, "artifacts": {...},
    "usage": {"cost_usd": ...}, "error": null}`; `results` are consumed one per call.
    """
    doc = source if isinstance(source, Mapping) else json.loads(Path(source).read_text(encoding="utf-8"))
    keyed: dict[str, dict] = {f: {} for f in FAMILIES}
    default = _worker_result(doc["default"]) if doc.get("default") else None
    for n, entry in enumerate(doc.get("responses") or []):
        families = [entry["family"]] if entry.get("family") else list(FAMILIES)
        for f in families:
            if f not in FAMILIES:
                raise ValueError(f"responses[{n}]: unknown family {f!r}")
        stage, item = entry.get("stage"), entry.get("item")
        key = (stage, item) if stage and item else (item or stage)
        if not key:
            raise ValueError(f"responses[{n}] names neither a stage nor an item")
        raw = entry["results"] if "results" in entry else [entry["result"]]
        for f in families:
            keyed[f].setdefault(key, []).extend(_worker_result(r) for r in raw)
    return {f: FakeWorker(keyed[f], family=f, default=default) for f in FAMILIES}


# ---------------------------------------------------------------------------
# run context, stop, result
# ---------------------------------------------------------------------------


def default_run_command(argv, cwd, timeout):
    """`(argv, cwd, timeout) -> (rc, stdout, stderr)`; a timeout is rc 124."""
    try:
        p = subprocess.run(list(argv), cwd=cwd, capture_output=True, text=True, encoding="utf-8",
                           errors="replace", timeout=timeout)
    except subprocess.TimeoutExpired as e:
        def s(v):
            return v.decode("utf-8", "replace") if isinstance(v, bytes) else (v or "")
        return 124, s(e.stdout), s(e.stderr) + f"\ntimeout after {timeout}s"
    return p.returncode, p.stdout, p.stderr


@dataclass
class DriverContext:
    """`driver_contracts.RunContext` as a concrete object."""

    run_id: str
    run_dir: str
    repo: str
    base_sha: str | None
    plan: Mapping
    config: Any
    workers: Mapping
    health: Any
    ledger: Any
    corrections: Any
    scorecard: Any
    pool: Any
    run_command: Callable
    now: Callable[[], str]
    new_attempt_id: Callable[[], str]
    # Optional attributes the executors read with getattr (None = not configured):
    items: Mapping | None = None              # the live item map (item id -> ItemRecord)
    harness_bin: str | None = None
    clause_text_json: str | None = None
    sleep: Callable | None = None
    dcgo_root: str | None = None


@dataclass
class Stop:
    # complete | blocked | budget | wall_clock | plateau | max_attempts |
    # vendors_exhausted | interrupted | error
    reason: str
    detail: str = ""
    spent_usd: float = 0.0
    unpriced_attempts: int = 0
    budget_usd: float | None = None
    elapsed_s: float = 0.0
    wall_clock_hours: float | None = None
    plateau: int = 0
    plateau_limit: int | None = None
    session_attempts: int = 0

    def to_dict(self) -> dict:
        return dataclasses.asdict(self)


@dataclass
class RunResult:
    stop: Stop
    counts: dict
    blocked: dict
    failed: dict
    problems: list
    report_path: Path
    state_path: Path


@dataclass
class _Task:
    seq: int
    item: str
    state: str
    lane: str            # worker | oracle
    kind: str            # executor | gate
    stage: str | None
    record: ItemRecord   # a copy: executors never touch live state


def _anchor(repo: Path, path: str) -> Path:
    p = Path(path)
    return p if p.is_absolute() else repo / p


# ---------------------------------------------------------------------------
# the driver
# ---------------------------------------------------------------------------


class Driver:
    def __init__(self, plan: Mapping, config: LoopConfig, components: Components, workers: Mapping, *,
                 state: RunState | None = None, run_dir=None, repo=None, paths: LedgerPaths | None = None,
                 health=None, ledger=None, corrections=None, scorecard=None, pool=None,
                 run_command: Callable | None = None, now: Callable[[], str] | None = None,
                 clock: Callable[[], float] | None = None, new_attempt_id: Callable[[], str] | None = None,
                 serial: bool = False, max_attempts: int | None = None,
                 dcgo_presence: Callable | None = None, manage_tree: bool = False,
                 harness_bin: str | None = None, clause_text_json: str | None = None,
                 sleep: Callable | None = None, dcgo_root: str | None = None):
        self.plan = plan
        self.config = config
        self.components = components
        self.run_id = plan["run_id"]
        self.repo = Path(repo) if repo else REPO_ROOT
        self.run_dir = Path(run_dir) if run_dir else _anchor(self.repo, config.runs_dir) / self.run_id
        self.now = now or utc_now_iso
        self.clock = clock or time.monotonic
        self.paths = paths or LedgerPaths.for_repo(self.repo, config)
        self.state = state or RunState.open(plan, self.paths, run_dir=self.run_dir, now=self.now)
        self.ledger = ledger or AttemptLedger(_anchor(self.repo, config.attempts_path))
        self.corrections = corrections or CorrectionLog(Path(self.ledger.path).with_name("corrections.jsonl"))
        self.health = health or VendorHealth()
        self.serial = serial
        self.max_attempts = max_attempts
        self.dcgo_presence = dcgo_presence
        attempt_id = new_attempt_id or (lambda: ledger_new_attempt_id(self.run_id))
        self.ctx = DriverContext(
            run_id=self.run_id, run_dir=str(self.run_dir), repo=str(self.repo),
            base_sha=plan.get("base_sha"), plan=plan, config=config, workers=dict(workers or {}),
            health=self.health, ledger=self.ledger, corrections=self.corrections, scorecard=scorecard,
            pool=pool, run_command=run_command or default_run_command, now=self.now,
            new_attempt_id=attempt_id, items=self.state.records, harness_bin=harness_bin,
            clause_text_json=clause_text_json, sleep=sleep,
            dcgo_root=dcgo_root or (plan.get("dcgo") or {}).get("root"))
        # Run-tree management: commit tree writes (oracle verdicts/backfill,
        # termination triage, the clause-text book, the ledgers at a stop),
        # run the clause-text prelude, record parked gaps in the trackers.
        self.manage_tree = manage_tree
        self._prelude: str | None = None          # None = not run; "ok"; or the failure
        self._branch_ok: bool | None = None
        self._engine_wait_problems: set[str] = set()

        self._history: dict[str, list[str]] = {}
        self._attempt_index: dict[str, Attempt] = {}
        self._spent = 0.0
        self._unpriced = 0
        self._plateau = 0
        self._session_attempts = 0
        self._failed: dict[str, str] = {}
        self._idle_steps: dict[str, int] = {}
        self._last_merge: dict[str, MergeResult] = {}
        self._stop: Stop | None = None
        self._seq = 0
        self._started = 0.0
        self._ran = False
        self._load_ledger()

    # ------------------------------------------------------------------ ledger bookkeeping

    def _load_ledger(self) -> None:
        rows, problems = load_attempts(self.ledger.path)
        for p in problems:
            if p.kind == "malformed_json":
                self.state.problems.append(f"{Path(p.path).name} line {p.line} ignored: {p.message}")
        for a in dedupe_last(rows):
            if a.run_id == self.run_id:
                self._record_attempt(a)

    def _record_attempt(self, a: Attempt) -> None:
        self._history.setdefault(a.item, []).append(a.attempt_id)
        self._attempt_index[a.attempt_id] = a
        if a.usage.cost_usd is None:
            self._unpriced += 1
        else:
            self._spent += a.usage.cost_usd

    def _ledger(self, attempts, corrections) -> None:
        for a in attempts:
            self.ledger.append(a)
            self._record_attempt(a)
        if corrections:
            self.corrections.extend(list(corrections))

    def _tick(self, attempts, *, adjudicated: bool) -> None:
        self._session_attempts += len(attempts)
        self._plateau = 0 if adjudicated else self._plateau + len(attempts)

    # ------------------------------------------------------------------ caps

    def _cap(self, key: str | None) -> int | None:
        if key is None:
            return None
        if key == ORACLE_RETRY_KEY:
            return self.config.attempt_caps.get(key, DEFAULT_ORACLE_RETRY_CAP)
        return self.config.attempt_caps.get(key)

    def _stage_of(self, rec: ItemRecord) -> str | None:
        return stage_for(rec.kind, rec.state, engine_fix=bool(rec.data.get("engine_fix")))

    def _cap_key(self, rec: ItemRecord) -> str | None:
        return ORACLE_RETRY_KEY if rec.state in ORACLE_STATES else self._stage_of(rec)

    # ------------------------------------------------------------------ stops

    def _elapsed(self) -> float:
        return self.clock() - self._started

    def _check_stop(self) -> bool:
        if self._stop is not None:
            return True
        cfg = self.config
        if cfg.budget_usd is not None and self._spent >= cfg.budget_usd:
            self._stop = Stop("budget", f"spent ${self._spent:.2f} reached the ${cfg.budget_usd:.2f} cap")
        elif cfg.wall_clock_hours and self._elapsed() >= cfg.wall_clock_hours * 3600:
            self._stop = Stop("wall_clock", f"{self._elapsed() / 3600:.2f} h reached the "
                                            f"{cfg.wall_clock_hours} h cap")
        elif cfg.plateau_attempts and self._plateau >= cfg.plateau_attempts:
            self._stop = Stop("plateau", f"{self._plateau} consecutive completed attempts with no new "
                                         f"adjudication (K = {cfg.plateau_attempts})")
        elif self.max_attempts is not None and self._session_attempts >= self.max_attempts:
            self._stop = Stop("max_attempts", f"{self._session_attempts} worker attempts this session "
                                              f"(--max-attempts {self.max_attempts})")
        elif self.ctx.workers and not self.health.available():
            # Quota / credit exhaustion disables a family for the run; with none
            # left every model step would fail or escalate, so stop instead.
            why = "; ".join(f"{f}: {self.health.reason(f)}" for f in FAMILIES)
            self._stop = Stop("vendors_exhausted", f"no model family is available ({why})")
        return self._stop is not None

    def _idle_stop(self) -> Stop:
        open_items = [r for r in self.state.records.values() if not is_terminal(r.kind, r.state)]
        if not open_items:
            return Stop("complete", "every item is in a terminal state")
        return Stop("blocked", f"{len(open_items)} open item(s) and none runnable "
                               "(see 'Waiting items' / 'Items that failed')")

    # ------------------------------------------------------------------ readiness

    # A clause in one of these states has a fix in flight (or a divergence
    # awaiting triage): the engine under its card is about to change, and an
    # interaction exam authored now fails the very assertion the fix targets
    # and burns its attempts (Data Squad pilot: Q2304 under BT13-060#effect#2).
    CLAUSE_FIX_STATES = frozenset({"DIVERGED", "TRIAGE", "FIX", "GATE"})

    def _clauses_under_fix(self, rec: ItemRecord) -> list[tuple[str, str]]:
        meta = self.state.meta.get(rec.item)
        cards = set(meta.cards) if meta is not None else set()
        if not cards:
            return []
        out = []
        for other in self.state.records.values():
            if other.kind != "clause" or other.state not in self.CLAUSE_FIX_STATES:
                continue
            om = self.state.meta.get(other.item)
            if om is not None and cards & set(om.cards):
                out.append((other.item, other.state))
        return sorted(out)

    def _why_not_runnable(self, rec: ItemRecord) -> str | None:
        st = rec.state
        if st == "PARKED":
            return f"parked on gap {rec.data.get('gap_id') or '(no gap id)'}"
        unmet = self.state.unmet_requirements(rec.item)
        if unmet:
            return "waits for " + ", ".join(f"{r} ({self.state.records[r].state})" for r in unmet)
        if rec.kind == "interaction":
            fixing = self._clauses_under_fix(rec)
            if fixing:
                return ("waits for " + ", ".join(f"{c} ({s})" for c, s in fixing)
                        + " -- a clause of the same card is being fixed, so the engine under this exam is about to change")
        if st in BUILTIN_STATES and st not in self.components.executors:
            return None
        if st == "GATE" and rec.data.get("engine_wait"):
            ew = rec.data["engine_wait"]
            return (f"the fix gate passed; waits for a human to merge {ew.get('branch') or 'the engine branch'}"
                    f" ({(ew.get('sha') or '?')[:12]}) into the run branch")
        if st in ORACLE_STATES and self.manage_tree and self._prelude not in (None, "ok"):
            return f"the clause-text book prelude failed: {self._prelude}"
        if st == "GATE":
            if self.components.gate is None:
                why = self.components.missing.get("gate")
                return "the fix gate is not built" + (f" ({why})" if why else "")
            return None
        if st not in self.components.executors:
            why = self.components.missing.get("stages")
            return f"no executor for {st}" + (f" (stages: {why})" if why else "")
        return None

    def _scan_blocked(self) -> dict:
        out = {}
        for rec in self.state.ordered():
            if is_terminal(rec.kind, rec.state) or rec.item in self._failed:
                continue
            why = self._why_not_runnable(rec)
            if why is None and self._stop is not None and self._stop.reason not in ("complete", "blocked"):
                why = f"runnable in {rec.state}; not reached before the stop"
            if why:
                out[rec.item] = why
        return out

    def _ready_tasks(self, free: Mapping[str, int], busy=frozenset()):
        free = dict(free)
        for rec in self.state.ordered():
            if not any(v > 0 for v in free.values()):
                return
            st = rec.state
            if rec.item in busy or rec.item in self._failed or is_terminal(rec.kind, st):
                continue
            if st in BUILTIN_STATES and st not in self.components.executors:
                continue
            if self._why_not_runnable(rec) is not None:
                continue
            lane = "oracle" if st in ORACLE_STATES else "worker"
            if free.get(lane, 0) <= 0:
                continue
            key = self._cap_key(rec)
            cap = self._cap(key)
            if cap is not None and rec.attempts.get(key, 0) >= cap:
                self._escalate(rec, Escalation(
                    item=rec.item, reason=f"attempt cap: {key} {rec.attempts.get(key, 0)}/{cap} spent "
                                          f"(state {st})"))
                continue
            if self._check_stop():
                return
            if st in ORACLE_STATES and self.manage_tree and self._prelude is None and not self._run_prelude():
                continue
            free[lane] -= 1
            self._seq += 1
            snapshot = ItemRecord.from_dict(copy.deepcopy(rec.to_dict()))
            yield _Task(self._seq, rec.item, st, lane, "gate" if st == "GATE" else "executor",
                        self._stage_of(rec), snapshot)

    def _builtin_next(self, rec: ItemRecord) -> str:
        if rec.state == "DIVERGED":
            return "TRIAGE"
        if rec.kind == "card":
            return "IMPLEMENTING"
        source = self.state.meta[rec.item].source if rec.item in self.state.meta else None
        if rec.kind == "interaction" and (source or rec.data.get("source")) == "qa":
            return "CLASSIFY"
        return "AUTHORING"

    def _advance_builtins(self) -> None:
        for rec in self.state.ordered():
            st = rec.state
            if st not in BUILTIN_STATES or st in self.components.executors or rec.item in self._failed:
                continue
            if not self.state.requirements_met(rec.item):
                continue
            dst = self._builtin_next(rec)
            self.state.transition(rec.item, dst, reason=f"driver: {st} -> {dst}")

    # ------------------------------------------------------------------ gap lane

    def _park(self, rec: ItemRecord, *, record: bool = True) -> None:
        gaps = self.components.gaps
        gap_id = rec.data.get("gap_id")
        if not gap_id:
            self.state.problems.append(f"{rec.item} parked without a gap_id in its data")
            return
        if gaps is None:
            return
        try:
            gaps.park(rec.item, gap_id, is_core=self.state.meta[rec.item].is_core)
        except Exception as e:
            self.state.problems.append(f"gap lane park({rec.item}, {gap_id}) failed: {e!r}")
        if record:
            self._record_gaps(rec)

    def _unpark_closed(self) -> None:
        gaps = self.components.gaps
        if gaps is None or not any(r.state == "PARKED" for r in self.state.records.values()):
            return
        try:
            closed = list(gaps.closed(self.ctx))
        except Exception as e:
            self.state.problems.append(f"gap lane closed() failed: {e!r}")
            return
        for gid in closed:
            if gid in self.state.records:         # a lane that reports items directly
                items = [gid]
            else:
                try:
                    items = list(gaps.unpark(gid))
                except Exception as e:
                    self.state.problems.append(f"gap lane unpark({gid}) failed: {e!r}")
                    continue
            for item in items:
                rec = self.state.records.get(item)
                if rec is not None and rec.state == "PARKED":
                    self.state.transition(item, "IMPLEMENTING", reason=f"gap {gid} closed",
                                          item_data={"unparked_from": rec.data.get("gap_id")})

    def _record_gaps(self, rec: ItemRecord) -> None:
        """Tracker writes are orchestrator-only: the gaps a parked card's worker
        reported go to `qa/dsl-vocab-gaps.md` / `docs/RUST_ENGINE_GAPS.md`
        through the gap lane, which commits them on the run branch."""
        record = getattr(self.components.gaps, "record_gap", None)
        if not self.manage_tree or not callable(record):
            return
        for gap in rec.data.get("gaps") or ():
            if not isinstance(gap, Mapping) or gap.get("kind") not in ("dsl", "engine"):
                continue
            try:
                record(dict(gap), gap["kind"], attempt_id=rec.data.get("implement_attempt"),
                       family=rec.data.get("implementer_family"), model=None)
            except Exception as e:
                self.state.problems.append(f"recording gap {gap.get('id')} for {rec.item} failed: {e!r}")

    # ------------------------------------------------------------------ the run tree

    def _git(self, *args: str, timeout: float = 300) -> tuple:
        try:
            return self.ctx.run_command(["git", *args], str(self.repo), timeout)
        except Exception as e:
            return None, "", f"{type(e).__name__}: {e}"

    def _on_run_branch(self) -> bool:
        if self._branch_ok is None:
            rc, out, err = self._git("symbolic-ref", "-q", "--short", "HEAD", timeout=60)
            branch = out.strip() if rc == 0 else None
            self._branch_ok = bool(branch) and branch not in PROTECTED_BRANCHES
            if not self._branch_ok:
                self.state.problems.append(
                    f"the run tree {self.repo} is on {branch or 'a detached HEAD'}, not a run branch "
                    f"({run_branch(self.run_id)}): its writes are left uncommitted")
        return self._branch_ok

    @staticmethod
    def _porcelain_paths(out: str) -> list[str]:
        paths = []
        for line in out.splitlines():
            if len(line) < 4:
                continue
            path = line[3:].split(" -> ")[-1].strip()
            if path.startswith('"') and path.endswith('"'):
                path = path[1:-1]
            paths.append(path)
        return paths

    def _commit_paths(self, pathspecs, subject: str, *, attempt_id: str | None = None,
                      body: str | None = None) -> str | None:
        """Commit whatever is dirty under `pathspecs` in the run tree (and only
        that), with `Loop-Attempt` / `Loop-Model` trailers. Returns the commit
        subject, or None when nothing was dirty or the commit could not be made."""
        from .provenance import loop_commit_message

        pathspecs = [str(p) for p in pathspecs if p]
        if not pathspecs or not self._on_run_branch():
            return None
        rc, out, err = self._git("status", "--porcelain", "--untracked-files=all", "--", *pathspecs)
        if rc != 0:
            self.state.problems.append(f"git status in the run tree failed ({rc}): {(err or out).strip()[-300:]}")
            return None
        dirty = self._porcelain_paths(out)
        if not dirty:
            return None
        msg = loop_commit_message(subject, body, attempt_id=attempt_id or self.ctx.new_attempt_id(),
                                  family=DRIVER_FAMILY, model=None,
                                  extra_trailers=[f"Loop-Run: {self.run_id}"])
        for args in (("add", "--", *dirty), ("commit", "-q", "-m", msg, "--", *dirty)):
            rc, out, err = self._git(*args)
            if rc != 0:
                self.state.problems.append(f"git {args[0]} in the run tree failed ({rc}): "
                                           f"{(err or out).strip()[-300:]}")
                return None
        return subject

    def _clause_text_book(self) -> str:
        for v in (self.ctx.clause_text_json, self.plan.get("clause_text_json"),
                  getattr(self.config, "clause_text_json", None)):
            if v:
                return str(v)
        return DEFAULT_CLAUSE_TEXT_BOOK

    def _run_prelude(self) -> bool:
        """Before the first ORACLE call of a session: extend the clause-text book
        to the whole pool and commit it. A card missing from the book makes
        every oracle verdict for it an orphan refusal. Idempotent; once per session."""
        pool = list((self.plan.get("work_set") or {}).get("pool") or [])
        if not pool:
            self._prelude = "ok"
            return True
        book = self._clause_text_book()
        argv = [sys.executable, "-c", BOOK_BOOTSTRAP, "add", "--book", book, "--card-ids", *pool]
        try:
            rc, out, err = self.ctx.run_command(argv, str(self.repo), 1800)
        except Exception as e:
            rc, out, err = None, "", f"{type(e).__name__}: {e}"
        if rc != 0:
            self._prelude = f"`tools.clause_coverage.book add` exited {rc}: {(err or out).strip()[-300:]}"
            self.state.problems.append(f"clause-text book prelude: {self._prelude}")
            return False
        self._commit_paths([book], f"card-loop: clause-text book covers run {self.run_id}'s pool",
                           body=f"{len(pool)} pool card(s); added by the driver's run prelude.")
        self._prelude = "ok"
        return True

    def _commit_tree_writes(self, rec: ItemRecord, src: str, attempts) -> None:
        """After a step whose executor writes into the run tree (ORACLE's
        verdicts + `--backfill`, TERMINATION_CHECK's `verdict-triage`)."""
        paths = list(rec.data.get("scenario_paths") or [])
        for row in (rec.data.get("oracle_results") or {}).values():
            if isinstance(row, Mapping) and row.get("backfilled") and row.get("scenario"):
                paths.append(row["scenario"])
        paths.append(VERDICTS_DIR)
        what = "oracle verdict and backfill" if src in ORACLE_STATES else f"{src.lower()} verdict"
        self._commit_paths(list(dict.fromkeys(paths)), f"card-loop: {what} for {rec.item}",
                           attempt_id=attempts[0].attempt_id if attempts else None)

    def _commit_ledgers(self) -> None:
        paths = []
        for p in (Path(self.ledger.path), Path(self.corrections.path),
                  Path(self.ledger.path).with_name("audits.jsonl"), Path(self.paths.escalations_dir)):
            try:
                paths.append(Path(p).resolve().relative_to(self.repo.resolve()).as_posix())
            except ValueError:
                continue                      # outside the run tree (e.g. --fake ledgers)
        reason = self._stop.reason if self._stop else "stop"
        self._commit_paths(paths, f"card-loop: ledgers for run {self.run_id} ({reason})")

    def _release_engine_waits(self) -> None:
        """GATE items whose engine fix passed the gate wait for a human to merge
        the engine branch into the run branch (D13); once its sha is an
        ancestor of the run tree's HEAD, the item goes on to the oracle."""
        for rec in self.state.ordered():
            ew = rec.data.get("engine_wait") if rec.state == "GATE" else None
            if not ew or not ew.get("sha"):
                continue
            rc, out, err = self._git("merge-base", "--is-ancestor", ew["sha"], "HEAD", timeout=60)
            if rc == 0:
                self.state.transition(rec.item, "ORACLE", reason=f"engine fix {ew.get('branch')} "
                                                                  "landed on the run branch",
                                      item_data={"engine_wait": None, "engine_landed": dict(ew)})
            elif rc != 1 and rec.item not in self._engine_wait_problems:
                self._engine_wait_problems.add(rec.item)
                self.state.problems.append(f"{rec.item}: cannot tell whether {ew.get('branch')} landed "
                                           f"(git merge-base exited {rc}): {(err or out).strip()[-200:]}")

    def _round_start(self) -> None:
        self._unpark_closed()
        self._release_engine_waits()
        self._advance_builtins()

    # ------------------------------------------------------------------ start of a session

    def _default_dcgo_presence(self, cards):
        root = (self.plan.get("dcgo") or {}).get("root")
        if not root:
            return {}
        from .preflight import dcgo_script_presence
        return dcgo_script_presence(cards, root)

    def _on_start(self) -> None:
        scripts = (self.plan.get("dcgo") or {}).get("scripts") or {}
        missing = [c for c, p in scripts.items() if p is None]
        if missing:
            try:
                now = (self.dcgo_presence or self._default_dcgo_presence)(missing)
            except Exception as e:
                self.state.problems.append(f"DCGO re-check failed: {e!r}")
                now = {}
            newly = [c for c in missing if now.get(c)]
            if newly:
                self.state.reenter_available(newly)
        if self.components.gaps is not None:
            for rec in self.state.ordered():
                if rec.state == "PARKED":
                    self._park(rec, record=False)      # resume: the lane learns it again

    # ------------------------------------------------------------------ the loop

    def run(self) -> RunResult:
        if self._ran:
            raise RuntimeError("a Driver runs once; build a new one to resume")
        self._ran = True
        self._started = self.clock()
        try:
            self._on_start()
            if self.serial:
                self._loop_serial()
            else:
                self._loop_threaded()
        except KeyboardInterrupt:
            self._stop = self._stop or Stop("interrupted", "interrupted")
        except Exception as e:
            # The driver itself broke (a ledger write, a corrupt record): stop,
            # still write state.json + report.md, then surface the error.
            self._stop = Stop("error", f"{type(e).__name__}: {e}")
            self._finish_run()
            raise
        return self._finish_run()

    def _loop_serial(self) -> None:
        while not self._check_stop():
            self._round_start()
            task = next(self._ready_tasks({"worker": 1, "oracle": 1}), None)
            if task is None:
                if self._stop is None:
                    self._stop = self._idle_stop()
                return
            self._complete(task, self._execute(task))

    def _loop_threaded(self) -> None:
        lanes = {"worker": max(1, int(self.config.concurrency)), "oracle": 1}
        pool = ThreadPoolExecutor(max_workers=sum(lanes.values()), thread_name_prefix="card-loop")
        in_flight: dict = {}
        try:
            while True:
                try:
                    if not self._check_stop():
                        self._round_start()
                        busy ={t.item for t in in_flight.values()}
                        used = {lane: sum(1 for t in in_flight.values() if t.lane == lane) for lane in lanes}
                        free = {lane: lanes[lane] - used[lane] for lane in lanes}
                        for task in self._ready_tasks(free, busy):
                            in_flight[pool.submit(self._execute, task)] = task
                    if not in_flight:
                        if self._stop is None:
                            self._stop = self._idle_stop()
                        return
                    done, _ = wait(list(in_flight), return_when=FIRST_COMPLETED)
                    for fut in sorted(done, key=lambda f: in_flight[f].seq):
                        task = in_flight.pop(fut)
                        self._complete(task, fut.result())
                except KeyboardInterrupt:
                    if self._stop is not None and self._stop.reason == "interrupted":
                        raise
                    self._stop = Stop("interrupted", "interrupted; drained in-flight work")
        finally:
            pool.shutdown(wait=True, cancel_futures=True)

    def _execute(self, task: _Task):
        """Runs on a pool thread (or inline when serial). Never raises."""
        try:
            if task.kind == "gate":
                return ("gate", self.components.gate.check(self.ctx, task.record, self._merge_for(task.record)))
            return ("outcome", self.components.executors[task.state].run(self.ctx, task.record))
        except StageDeferred as e:
            return ("deferred", e)
        except Exception as e:
            return ("error", e, traceback.format_exc())

    def _complete(self, task: _Task, result) -> None:
        rec = self.state.records[task.item]
        if rec.state != task.state:
            self.state.problems.append(f"{task.item} moved to {rec.state} while its {task.state} step ran; "
                                       "outcome dropped")
            return
        tag = result[0]
        if tag == "error":
            self._executor_failed(rec, task, result[1], result[2])
        elif tag == "deferred":
            self._apply_deferred(rec, task, result[1])
        elif tag == "gate":
            if isinstance(result[1], GateResult):
                self._apply_gate(rec, result[1])
            else:
                self._executor_failed(rec, task, TypeError(f"gate returned {type(result[1]).__name__}"))
        elif isinstance(result[1], StageOutcome):
            self._apply_outcome(rec, task, result[1])
        else:
            self._executor_failed(rec, task, TypeError(
                f"executor for {task.state} returned {type(result[1]).__name__}, not StageOutcome"))

    # ------------------------------------------------------------------ applying outcomes

    def _executor_failed(self, rec: ItemRecord, task: _Task, exc: BaseException, tb: str | None = None) -> None:
        msg = f"{type(exc).__name__}: {exc}"
        self._failed[rec.item] = msg
        self.state.note(rec.item, reason=f"executor error in {task.state}: {msg}", stage=task.stage,
                        event_data={"traceback": tb[-2000:]} if tb else None)

    def _merge(self, request: MergeRequest) -> MergeResult:
        merger = self.components.merger
        if merger is None:
            why = self.components.missing.get("merger", "")
            return MergeResult(ok=False, errors=["the merger is not built" + (f" ({why})" if why else "")])
        try:
            res = merger.merge(self.ctx, request)
        except Exception as e:
            return MergeResult(ok=False, errors=[f"merger raised {type(e).__name__}: {e}"])
        if not isinstance(res, MergeResult):
            return MergeResult(ok=False, errors=[f"merger returned {type(res).__name__}, not MergeResult"])
        return res

    def _merge_for(self, rec: ItemRecord) -> MergeResult:
        if rec.item in self._last_merge:
            return self._last_merge[rec.item]
        m = rec.data.get("merge") or {}
        if not m:
            return MergeResult(ok=False, errors=["no merge is recorded for this item"])
        return MergeResult(ok=bool(m.get("ok", True)), sha=m.get("sha"), branch=m.get("branch"),
                           touched=list(m.get("touched") or []), scope=dict(m.get("scope") or {}),
                           errors=list(m.get("errors") or []))

    @staticmethod
    def _count_keys(src: str, dst: str, attempts) -> tuple[list[str], list[str]]:
        """`(count, reset)`: the attempt counters this step bumps (one per
        distinct stage among its attempts, plus `oracle_retry` for an ORACLE
        retry) and the dependent counters it restarts (`CAP_RESETS`)."""
        keys = sorted({a.stage for a in attempts})
        reset = sorted({r for k in keys for r in CAP_RESETS.get(k, ())} - set(keys))
        if src in ORACLE_STATES and dst == src:
            keys.append(ORACLE_RETRY_KEY)
        return keys, reset

    def _idle_check(self, rec: ItemRecord, *, progressed: bool) -> None:
        if progressed:
            self._idle_steps.pop(rec.item, None)
            return
        n = self._idle_steps[rec.item] = self._idle_steps.get(rec.item, 0) + 1
        if n >= MAX_IDLE_STEPS and not is_terminal(rec.kind, rec.state):
            msg = (f"{n} consecutive steps without a worker attempt or an adjudication "
                   f"(an executor loop?); last state {rec.state}")
            self._failed[rec.item] = msg
            self.state.note(rec.item, reason=f"set aside for this session: {msg}")

    def _apply_outcome(self, rec: ItemRecord, task: _Task, outcome: StageOutcome) -> None:
        item, kind, src, dst = rec.item, rec.kind, rec.state, outcome.next_state
        attempts = list(outcome.attempts or [])
        try:
            for a in attempts:
                a.validate()
            for c in outcome.corrections or ():
                c.validate()
        except Exception as e:
            self._executor_failed(rec, task, e)
            return
        ids = [a.attempt_id for a in attempts]
        first = ids[0] if ids else None
        stage = attempts[0].stage if attempts else task.stage
        count, reset = self._count_keys(src, dst, attempts)

        try:
            check_transition(kind, src, dst)
        except ValueError as e:
            self._ledger(attempts, outcome.corrections)
            self.state.note(item, reason=f"refused: the {src} step proposed {src} -> {dst}: {e}",
                            stage=stage, attempt_id=first, attempt_ids=ids, count=count, reset_keys=reset,
                            event_data={"refused_next_state": dst})
            self._failed[item] = str(e)
            self._tick(attempts, adjudicated=False)
            return

        item_data = dict(outcome.data or {})
        req = outcome.merge_request
        if req is not None:
            res = self._merge(req)
            if not res.ok:
                self._merge_failed(rec, outcome, attempts, req, res, count, reset, stage)
                return
            self._last_merge[item] = res
            item_data["merge"] = {"ok": True, "sha": res.sha, "branch": res.branch,
                                  "touched": list(res.touched), "scope": dict(res.scope),
                                  "attempt_id": req.attempt_id, "engine": req.engine}

        self._ledger(attempts, outcome.corrections)
        if dst == "ESCALATED":
            esc = outcome.escalation or Escalation(item=item, reason=outcome.reason or f"escalated from {src}")
            path = self._write_escalation(rec, esc)
            item_data.update(escalation=path.name, escalation_reason=esc.reason)
        self.state.transition(item, dst, reason=outcome.reason or f"{src} -> {dst}", stage=stage,
                              attempt_id=first, attempt_ids=ids, item_data=item_data,
                              event_data=outcome.events_data, count=count, reset_keys=reset)
        if dst == "PARKED":
            self._park(rec)
        if self.manage_tree and src in TREE_WRITING_STATES:
            self._commit_tree_writes(rec, src, attempts)
        newly = bool(outcome.adjudicated) or (is_adjudicated(kind, dst) and not is_adjudicated(kind, src))
        self._tick(attempts, adjudicated=newly)
        self._idle_check(rec, progressed=bool(attempts) or newly)

    def _escalate_if_capped(self, rec: ItemRecord, why: str) -> None:
        key = self._cap_key(rec)
        cap = self._cap(key)
        if cap is not None and rec.attempts.get(key, 0) >= cap:
            self._escalate(rec, Escalation(
                item=rec.item, reason=f"attempt cap: {key} {rec.attempts.get(key, 0)}/{cap} spent; {why}"))

    def _apply_deferred(self, rec: ItemRecord, task: _Task, exc: StageDeferred) -> None:
        """The stage could not run now (a failed worker call, no family for a
        non-terminating call): ledger what it carries, count ONE attempt toward
        the stage's cap, leave the item where it is, escalate at the cap."""
        outcome = getattr(exc, "outcome", None) or StageOutcome(next_state=rec.state)
        reason = getattr(exc, "reason", None) or str(exc) or outcome.reason or "deferred"
        attempts = list(outcome.attempts or [])
        try:
            for a in attempts:
                a.validate()
            for c in outcome.corrections or ():
                c.validate()
        except Exception as e:
            self._executor_failed(rec, task, e)
            return
        if outcome.next_state != rec.state:
            self.state.problems.append(f"{rec.item}: a deferral proposed {outcome.next_state}; "
                                       f"a deferred item stays in {rec.state}")
        count, reset = self._count_keys(rec.state, rec.state, attempts)
        key = self._cap_key(rec)
        if key and key not in count:
            count.append(key)
        ids = [a.attempt_id for a in attempts]
        self._ledger(attempts, outcome.corrections)
        self.state.note(rec.item, reason=f"deferred: {reason}", stage=attempts[0].stage if attempts else task.stage,
                        attempt_id=ids[0] if ids else None, attempt_ids=ids, count=count, reset_keys=reset,
                        item_data=outcome.data, event_data=outcome.events_data)
        self._tick(attempts, adjudicated=False)
        self._idle_check(rec, progressed=bool(attempts))
        self._escalate_if_capped(rec, f"the last step was deferred: {reason}")

    def _merge_failed(self, rec, outcome, attempts, req, res, count, reset, stage) -> None:
        item = rec.item
        errors = list(res.errors) or ["merge failed (no error given)"]
        attempts = [replace(a, outcome="gate_failed")
                    if a.attempt_id == req.attempt_id and a.outcome == "accepted" else a for a in attempts]
        req_stage = next((a.stage for a in attempts if a.attempt_id == req.attempt_id), stage) or "implement"
        corrections = list(outcome.corrections or []) + [corr_mod.gate_fail(
            req.attempt_id, gate="merge", stage=req_stage, item=item, detail="; ".join(errors), ts=self.now())]
        self._ledger(attempts, corrections)
        self.state.note(item, reason="merge failed: " + "; ".join(errors), stage=stage,
                        attempt_id=req.attempt_id, attempt_ids=[a.attempt_id for a in attempts],
                        item_data={"merge_error": errors}, count=count, reset_keys=reset,
                        event_data={"merge": {"ok": False, "errors": errors, "branch": res.branch}})
        self._tick(attempts, adjudicated=False)
        self._escalate_if_capped(rec, f"the last merge failed: {'; '.join(errors)}")

    def _apply_gate(self, rec: ItemRecord, result: GateResult) -> None:
        item = rec.item
        fix_stage = "fix_engine" if rec.data.get("engine_fix") else "fix_card"
        if result.passed:
            gate = {"gate": {"passed": True, "evidence": dict(result.evidence)}}
            merge = rec.data.get("merge") or {}
            if merge.get("engine"):
                # D13: an engine fix lands on its own branch for a human to merge;
                # the oracle runs on the run tree, so the item waits here until
                # that branch is part of the run branch (`_release_engine_waits`).
                wait = {"branch": merge.get("branch"), "sha": merge.get("sha"),
                        "attempt_id": merge.get("attempt_id"), "gap_id": rec.data.get("gap_id")}
                self.state.note(item, reason=f"fix gate passed; waiting for a human to merge "
                                             f"{wait['branch']} into {run_branch(self.run_id)}",
                                item_data={**gate, "engine_wait": wait})
                return
            self.state.transition(item, "ORACLE", reason="fix gate passed", item_data=gate)
            return
        reasons = [str(r) for r in result.reasons] or ["the fix gate failed (no reason given)"]
        fix_attempt = (rec.data.get("merge") or {}).get("attempt_id")
        if fix_attempt:
            self.corrections.append(corr_mod.gate_fail(fix_attempt, gate="fix_gate", stage=fix_stage, item=item,
                                                       detail="; ".join(reasons), ts=self.now()))
        data = {"gate": {"passed": False, "reasons": reasons, "evidence": dict(result.evidence)},
                "gate_reasons": reasons}
        n, cap = rec.attempts.get(fix_stage, 0), self._cap(fix_stage)
        if cap is not None and n >= cap:
            self._escalate(rec, Escalation(
                item=item, reason=f"fix gate failed and the {fix_stage} cap {n}/{cap} is spent: "
                                  + "; ".join(reasons)), item_data=data)
        else:
            self.state.transition(item, "FIX", reason="fix gate failed: " + "; ".join(reasons), item_data=data)

    def _write_escalation(self, rec: ItemRecord, esc: Escalation) -> Path:
        if not esc.history:
            esc = replace(esc, history=tuple(self._history.get(rec.item, ())))
        return write_escalation(self.paths.escalations_dir, esc, run_id=self.run_id, ts=self.now(),
                                attempts=self._attempt_index, state_before=rec.state, item_data=rec.data)

    def _escalate(self, rec: ItemRecord, esc: Escalation, *, item_data: Mapping | None = None) -> None:
        path = self._write_escalation(rec, esc)
        data = {**dict(item_data or {}), "escalation": path.name, "escalation_reason": esc.reason}
        self.state.transition(rec.item, "ESCALATED", reason=esc.reason, item_data=data)

    # ------------------------------------------------------------------ the end of a session

    def _finish_run(self) -> RunResult:
        stop = self._stop or Stop("interrupted", "the loop ended without a stop reason")
        cfg = self.config
        stop = replace(stop, spent_usd=self._spent, unpriced_attempts=self._unpriced,
                       budget_usd=cfg.budget_usd, elapsed_s=self._elapsed(),
                       wall_clock_hours=cfg.wall_clock_hours, plateau=self._plateau,
                       plateau_limit=cfg.plateau_attempts, session_attempts=self._session_attempts)
        self._stop = stop
        close = getattr(self.components.merger, "close", None)
        if callable(close):
            try:
                close(self.ctx)
            except Exception as e:
                self.state.problems.append(f"merger close() failed: {e!r}")
        if self.manage_tree:
            self._commit_ledgers()
        blocked = self._scan_blocked()
        counts = self.state.counts()
        extra = {"stop": stop.to_dict(), "blocked": blocked, "failed": dict(self._failed),
                 "components_missing": dict(self.components.missing), "health": self.health.snapshot()}
        state_path = self.state.write_snapshot(extra)
        escalations = []
        for rec in self.state.ordered():
            if rec.state == "ESCALATED":
                where = rec.data.get("escalation") or "(no file recorded)"
                why = rec.data.get("escalation_reason")
                escalations.append((rec.item, f"{where}" + (f" -- {why}" if why else "")))
        attempts = sorted(self._attempt_index.values(), key=lambda a: a.attempt_id)
        text = render_report(run_id=self.run_id, counts=counts, by_state=self.state.by_state(),
                             stop=stop.to_dict(), attempts=attempts, escalations=escalations,
                             blocked=blocked, failed=self._failed, problems=self.state.problems,
                             missing_components=self.components.missing, generated_at=self.now())
        report_path = write_report(self.run_dir / REPORT_NAME, text)
        return RunResult(stop=stop, counts=counts, blocked=blocked, failed=dict(self._failed),
                         problems=list(self.state.problems), report_path=report_path, state_path=state_path)


# ---------------------------------------------------------------------------
# CLI: run / resume / status
# ---------------------------------------------------------------------------


def ledger_paths(repo, config: LoopConfig) -> LedgerPaths:
    """Where the CLI reads the committed ledgers (a seam for tests)."""
    return LedgerPaths.for_repo(repo, config)


def _config_for(plan: Mapping, config_path: str | None, budget_usd: float | None) -> LoopConfig:
    """The plan's frozen config, or `--config` instead; `--budget-usd` on top."""
    if config_path:
        config = load_config(config_path)
    else:
        known = {f.name for f in fields(LoopConfig)}
        config = LoopConfig(**{k: v for k, v in (plan.get("config") or {}).items() if k in known})
    if budget_usd is not None:
        config = replace(config, budget_usd=budget_usd)
    return config


def _fake_config(config: LoopConfig, run_dir: Path) -> LoopConfig:
    """`--fake` never writes the committed ledgers: attempts, corrections and
    escalations go under `<run_dir>/fake/`."""
    fake = Path(run_dir) / FAKE_DIR
    return replace(config, attempts_path=str(fake / "attempts.jsonl"), escalations_dir=str(fake / "escalations"))


def _load_plan(path: Path) -> dict:
    return json.loads(Path(path).read_text(encoding="utf-8"))


def _runs_dir(arg: str | None) -> Path:
    return Path(arg) if arg else _anchor(REPO_ROOT, LoopConfig().runs_dir)


def _add_run_options(p: argparse.ArgumentParser) -> None:
    p.add_argument("--config", help="TOML config (replaces the plan's frozen config)")
    p.add_argument("--fake", metavar="CANNED_JSON",
                   help="every family's worker is a FakeWorker replaying this file; ledgers go under "
                        "<run_dir>/fake/")
    p.add_argument("--max-attempts", type=int, default=None, help="stop after N worker attempts this session")
    p.add_argument("--budget-usd", type=float, default=None, help="USD cap for the run (overrides config)")
    p.add_argument("--serial", action="store_true", help="one task at a time (deterministic)")
    p.add_argument("--worktree-root", default=None,
                   help="root for the worker worktree pool, the engine-fix and gate scratch trees and "
                        "(by default) the run tree; keep it SHORT on Windows (MAX_PATH). Omitted = no "
                        "worker pool, scratch trees under <run_dir>/wt")
    p.add_argument("--run-tree", default=None,
                   help="the run's working tree, checked out on card-loop/<run-id>/run (created from the "
                        "plan's base sha, or reused). Default for real runs: <worktree-root or "
                        "<run_dir>/wt>/cl-<run>-run; --fake without it runs in the current checkout "
                        "and commits nothing")


class RunTreeError(RuntimeError):
    pass


RUN_TREE_RECORD = "run_tree.json"


def default_run_tree(run_dir, run_id: str, worktree_root=None) -> Path:
    root = Path(worktree_root) if worktree_root else Path(run_dir) / "wt"
    slug = "".join(c if c.isalnum() or c in "_-" else "-" for c in run_id)[:32].strip("-") or "run"
    return root / f"cl-{slug}-run"


def _git_cp(cwd, *args) -> subprocess.CompletedProcess:
    return subprocess.run(["git", *args], cwd=str(cwd), capture_output=True, text=True,
                          encoding="utf-8", errors="replace")


def ensure_run_tree(repo, run_id: str, base_sha: str | None, tree) -> Path:
    """`tree` checked out on `run_branch(run_id)`: a new worktree (and branch,
    from `base_sha`) the first time, the same one after. Refuses a tree that is
    on another branch -- the loop never switches someone's checkout."""
    branch, tree = run_branch(run_id), Path(tree)
    listed = _git_cp(repo, "worktree", "list", "--porcelain")
    if listed.returncode != 0:
        raise RunTreeError(f"{repo} is not a git repository: {listed.stderr.strip()}")
    registered = {os.path.normcase(os.path.abspath(line[len("worktree "):]))
                  for line in listed.stdout.splitlines() if line.startswith("worktree ")}
    if os.path.normcase(os.path.abspath(tree)) in registered:
        current = _git_cp(tree, "symbolic-ref", "-q", "--short", "HEAD").stdout.strip()
        if current != branch:
            raise RunTreeError(f"{tree} is on {current or 'a detached HEAD'}, not the run branch {branch}")
        return tree
    if tree.exists() and any(tree.iterdir()):
        raise RunTreeError(f"{tree} exists and is not a worktree of {repo}")
    exists = _git_cp(repo, "rev-parse", "--verify", "-q", f"refs/heads/{branch}").returncode == 0
    tree.parent.mkdir(parents=True, exist_ok=True)
    args = (["worktree", "add", "-q", str(tree), branch] if exists
            else ["worktree", "add", "-q", "-b", branch, str(tree), base_sha or "HEAD"])
    cp = _git_cp(repo, *args)
    if cp.returncode != 0:
        raise RunTreeError(f"git {' '.join(args)} failed: {cp.stderr.strip()}")
    return tree


def _recorded_run_tree(run_dir) -> Path | None:
    try:
        doc = json.loads((Path(run_dir) / RUN_TREE_RECORD).read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None
    return Path(doc["path"]) if doc.get("path") else None


def _execute(plan: dict, run_dir: Path, args, *, command: str) -> int:
    try:
        config = _config_for(plan, args.config, args.budget_usd)
    except (OSError, ValueError, TypeError) as e:
        print(f"{command}: bad config: {e}", file=sys.stderr)
        return 2
    fake = bool(args.fake)
    if fake:
        config = _fake_config(config, run_dir)
        (Path(run_dir) / FAKE_DIR).mkdir(parents=True, exist_ok=True)   # marks the run as fake
    elif (plan.get("preflight") or {}).get("go") is False:
        print(f"{command}: the plan's preflight is NO-GO; no worker may be invoked (fix the failing "
              "checks and re-plan, or exercise the loop with --fake)", file=sys.stderr)
        return 1

    # The run tree: real runs always work in one (on card-loop/<run>/run); a
    # --fake run only when --run-tree is given (else the current checkout, no commits).
    use_tree = (not fake) or bool(args.run_tree)
    tree = None
    if use_tree:
        tree = Path(args.run_tree) if args.run_tree else (
            _recorded_run_tree(run_dir) or default_run_tree(run_dir, plan["run_id"], args.worktree_root))
    repo = tree or REPO_ROOT
    components = default_components(config, run_dir=run_dir, plan=plan, repo=repo,
                                    worktree_root=args.worktree_root)
    health = VendorHealth()
    if fake:
        try:
            workers = load_fake_workers(args.fake)
        except (OSError, ValueError, KeyError, TypeError) as e:
            print(f"{command}: cannot load --fake {args.fake}: {e}", file=sys.stderr)
            return 2
    else:
        if components.missing:
            print(f"{command}: components not built: "
                  + "; ".join(f"{k}: {v}" for k, v in sorted(components.missing.items()))
                  + " (`--fake` exercises what exists)", file=sys.stderr)
            return 3
        workers, problems = default_workers(config)
        for family, why in problems.items():
            health.disable(family, f"worker unavailable: {why}")
            print(f"{command}: {family} unavailable for this run: {why}", file=sys.stderr)
        if not workers:
            return 3

    if components.missing:
        print(f"{command}: running with what exists; not built:")
        for name, why in sorted(components.missing.items()):
            print(f"  {name}: {why}")

    if tree is not None:
        try:
            ensure_run_tree(REPO_ROOT, plan["run_id"], plan.get("base_sha"), tree)
        except RunTreeError as e:
            print(f"{command}: run tree: {e}", file=sys.stderr)
            return 2
        Path(run_dir).mkdir(parents=True, exist_ok=True)
        (Path(run_dir) / RUN_TREE_RECORD).write_text(
            json.dumps({"path": str(tree), "branch": run_branch(plan["run_id"])}) + "\n", encoding="utf-8")
        print(f"{command}: run tree {tree} on {run_branch(plan['run_id'])}")
    else:
        print(f"{command}: --fake without --run-tree: executors run in the current checkout and the "
              "driver commits nothing")

    pool = None
    if args.worktree_root and not fake:
        from .workers.pool import WorktreePool
        pool = WorktreePool(REPO_ROOT, plan.get("base_sha") or "HEAD", args.worktree_root,
                            config.worktree_pool_size, config.cargo_target_base, sccache_dir=config.sccache_dir)
    try:
        driver = Driver(plan, config, components, workers, run_dir=run_dir, repo=repo,
                        paths=ledger_paths(repo, config), health=health, pool=pool,
                        max_attempts=args.max_attempts, serial=args.serial, manage_tree=tree is not None)
        result = driver.run()
    finally:
        if pool is not None:
            pool.close(remove=False)
    print(first_line(plan["run_id"], result.counts, result.stop.to_dict()))
    print(f"  report: {result.report_path}")
    print(f"  state:  {result.state_path}")
    return 0 if result.stop.reason == "complete" else 1


def cli_run(argv: list[str] | None = None) -> int:
    """`python -m tools.card_loop run --plan runs/card-loop/<id>/plan.json ...` -- exit 0 when
    every item ended terminal, 1 when the run stopped with open work (or NO-GO),
    2 bad input / already started, 3 a component or worker is missing."""
    p = argparse.ArgumentParser(prog="python -m tools.card_loop run",
                                description="Execute a planned card-loop run.")
    p.add_argument("--plan", required=True, help="runs/card-loop/<run-id>/plan.json")
    _add_run_options(p)
    args = p.parse_args(argv)
    plan_path = Path(args.plan)
    try:
        plan = _load_plan(plan_path)
    except (OSError, ValueError) as e:
        print(f"run: cannot read plan {plan_path}: {e}", file=sys.stderr)
        return 2
    run_dir = plan_path.parent
    if (run_dir / EVENTS_NAME).exists():
        print(f"run: {run_dir} has already started ({EVENTS_NAME} exists); continue it with "
              f"`python -m tools.card_loop resume --run {plan.get('run_id')}`", file=sys.stderr)
        return 2
    return _execute(plan, run_dir, args, command="run")


def cli_resume(argv: list[str] | None = None) -> int:
    """`python -m tools.card_loop resume --run <id>`: ledgers first, events second;
    outstanding items only."""
    p = argparse.ArgumentParser(prog="python -m tools.card_loop resume",
                                description="Continue an interrupted card-loop run.")
    p.add_argument("--run", required=True, help="the run id")
    p.add_argument("--runs-dir", default=None, help="default: config runs_dir")
    _add_run_options(p)
    args = p.parse_args(argv)
    run_dir = _runs_dir(args.runs_dir) / args.run
    try:
        plan = _load_plan(run_dir / "plan.json")
    except (OSError, ValueError) as e:
        print(f"resume: cannot read {run_dir / 'plan.json'}: {e}", file=sys.stderr)
        return 2
    started_fake = (run_dir / FAKE_DIR).is_dir()
    if started_fake and not args.fake:
        print(f"resume: {args.run} was started with --fake; resume it with --fake too", file=sys.stderr)
        return 2
    if args.fake and not started_fake and (run_dir / EVENTS_NAME).exists():
        print(f"resume: {args.run} was started with real workers; --fake would mix canned results "
              "into its ledgers", file=sys.stderr)
        return 2
    return _execute(plan, run_dir, args, command="resume")


def cli_status(argv: list[str] | None = None) -> int:
    """`python -m tools.card_loop status --run <id> [--json]`: read-only."""
    p = argparse.ArgumentParser(prog="python -m tools.card_loop status",
                                description="Summarise a card-loop run's items (read-only).")
    p.add_argument("--run", required=True)
    p.add_argument("--runs-dir", default=None)
    p.add_argument("--json", action="store_true")
    args = p.parse_args(argv)
    run_dir = _runs_dir(args.runs_dir) / args.run
    try:
        plan = _load_plan(run_dir / "plan.json")
        config = _config_for(plan, None, None)
    except (OSError, ValueError, TypeError) as e:
        print(f"status: cannot read {run_dir / 'plan.json'}: {e}", file=sys.stderr)
        return 2
    if (run_dir / FAKE_DIR).is_dir():
        config = _fake_config(config, run_dir)
    tree = _recorded_run_tree(run_dir)
    repo = tree if tree is not None and tree.is_dir() else REPO_ROOT
    state = RunState.open(plan, ledger_paths(repo, config), run_dir=run_dir, write=False)
    last_stop = None
    snap = run_dir / "state.json"
    if snap.exists():
        try:
            last_stop = json.loads(snap.read_text(encoding="utf-8")).get("stop")
        except ValueError:
            pass
    counts = state.counts()
    if args.json:
        print(json.dumps({"run_id": state.run_id, "counts": counts, "by_state": state.by_state(),
                          "last_stop": last_stop, "problems": state.problems}, indent=2))
        return 0
    print(first_line(state.run_id, counts, last_stop))
    for kind, states in state.by_state().items():
        print(f"  {kind}: " + ", ".join(f"{s} {n}" for s, n in states.items()))
    for problem in state.problems:
        print(f"  problem: {problem}")
    return 0
