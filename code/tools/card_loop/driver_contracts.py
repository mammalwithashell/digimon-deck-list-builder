"""The seams of the driver (design D4, D5, D13): item states and the allowed
transitions, the event row, and the interfaces the driver loop, the stage
executors, the gates, the gap lane and the merger agree on.

Everything here is data or a Protocol. The loop (`driver.py`, `state.py`), the
executors (`stages/`), the gates (`gates.py`), the gap lane (`gaps.py`) and the
merger (`merge.py`) import from here and never from each other's internals, so
each can be built and tested alone against fakes of the others.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Callable, Mapping, Protocol, Sequence

from .contracts import ITEM_KINDS, split_item_id

# ---------------------------------------------------------------------------
# States (design D4)
# ---------------------------------------------------------------------------

# card: implement -> review -> parked on a gap, or implemented.
CARD_STATES = (
    "PENDING", "IMPLEMENTING", "REVIEW", "PARKED", "IMPLEMENTED", "ESCALATED",
)

# clause / interaction: author -> sim -> oracle -> confirmed, or diverged ->
# triage -> (fix -> gate -> oracle) | (termination check -> terminal | escalated).
# An interaction sourced from a ruling first goes through CLASSIFY (D6) and,
# when behavioral, ENCODE (D7) before AUTHORING.
EXAM_STATES = (
    "PENDING", "CLASSIFY", "ENCODE", "AUTHORING", "SIM", "ORACLE",
    "CONFIRMED", "DIVERGED", "TRIAGE", "FIX", "GATE", "TERMINATION_CHECK",
    "TERMINAL", "UNAVAILABLE", "ESCALATED",
)

STATES_BY_KIND = {
    "card": CARD_STATES,
    "clause": EXAM_STATES,
    "interaction": EXAM_STATES,
}

# Terminal states, per kind. `ESCALATED` is terminal for the run but is NOT
# adjudicated (spec: it blocks readiness and goes to the human queue).
TERMINAL_BY_KIND = {
    "card": ("IMPLEMENTED", "ESCALATED"),
    "clause": ("CONFIRMED", "TERMINAL", "UNAVAILABLE", "ESCALATED"),
    "interaction": ("CONFIRMED", "TERMINAL", "UNAVAILABLE", "ESCALATED"),
}
ADJUDICATED_STATES = ("CONFIRMED", "TERMINAL", "UNAVAILABLE")

# How a `TERMINAL` exam item ended (D4): the record's `data["terminal"]`.
TERMINAL_REASONS = ("dcgo_quirk", "unreachable", "textual", "not_examinable")

_CARD_TRANSITIONS = {
    "PENDING": ("IMPLEMENTING", "IMPLEMENTED", "ESCALATED"),   # IMPLEMENTED: spec already exists
    "IMPLEMENTING": ("REVIEW", "PARKED", "ESCALATED"),
    "REVIEW": ("IMPLEMENTED", "IMPLEMENTING", "PARKED", "ESCALATED"),
    "PARKED": ("IMPLEMENTING",),                                # the gap closed (unpark)
    "IMPLEMENTED": (),
    "ESCALATED": (),
}

_EXAM_TRANSITIONS = {
    "PENDING": ("CLASSIFY", "AUTHORING", "UNAVAILABLE", "ESCALATED"),
    "CLASSIFY": ("ENCODE", "AUTHORING", "TERMINATION_CHECK", "ESCALATED"),
    "ENCODE": ("AUTHORING", "TERMINATION_CHECK", "ESCALATED"),
    "AUTHORING": ("SIM", "ESCALATED"),
    "SIM": ("ORACLE", "AUTHORING", "ESCALATED"),
    # ORACLE -> ORACLE: an unmeasured round trip (timeout, quarantine) retried
    # within the attempt cap. ORACLE -> AUTHORING: both engines contradicted
    # the scenario's expected prompts (spec "Prompt-sequence failures").
    "ORACLE": ("CONFIRMED", "DIVERGED", "ORACLE", "AUTHORING", "ESCALATED"),
    "DIVERGED": ("TRIAGE", "ESCALATED"),
    "TRIAGE": ("FIX", "TERMINATION_CHECK", "ESCALATED"),
    "FIX": ("GATE", "ESCALATED"),
    "GATE": ("ORACLE", "FIX", "ESCALATED"),
    "TERMINATION_CHECK": ("TERMINAL", "ESCALATED"),
    "CONFIRMED": (),
    "TERMINAL": (),
    "UNAVAILABLE": ("PENDING",),                                 # DCGO gained the script (resume)
    "ESCALATED": (),
}

TRANSITIONS_BY_KIND = {
    "card": _CARD_TRANSITIONS,
    "clause": _EXAM_TRANSITIONS,
    "interaction": _EXAM_TRANSITIONS,
}

# Which stage runs in which state. States not listed are driver-only work
# (sim check, oracle round trip, gates, termination check) that calls no model.
STAGE_FOR_STATE = {
    "IMPLEMENTING": "implement",
    "REVIEW": "review",
    "CLASSIFY": "classify_qa",
    "ENCODE": "encode_ruling",
    "AUTHORING": None,          # author_clause or author_interaction, by item kind
    "TRIAGE": "triage",
    "FIX": None,                # fix_card or fix_engine, by what triage blamed
}


def stage_for(kind: str, state: str, *, engine_fix: bool = False) -> str | None:
    """The worker stage a state runs, or None for driver-only states."""
    if state == "AUTHORING":
        return "author_interaction" if kind == "interaction" else "author_clause"
    if state == "FIX":
        return "fix_engine" if engine_fix else "fix_card"
    return STAGE_FOR_STATE.get(state)


def is_terminal(kind: str, state: str) -> bool:
    return state in TERMINAL_BY_KIND[kind]


def is_adjudicated(kind: str, state: str) -> bool:
    return state in ADJUDICATED_STATES or (kind == "card" and state == "IMPLEMENTED")


def check_transition(kind: str, src: str, dst: str) -> None:
    """Raise `ValueError` for a transition design D4 does not allow."""
    if kind not in ITEM_KINDS:
        raise ValueError(f"unknown item kind {kind!r}")
    table = TRANSITIONS_BY_KIND[kind]
    if src not in table:
        raise ValueError(f"unknown {kind} state {src!r}")
    if dst not in STATES_BY_KIND[kind]:
        raise ValueError(f"unknown {kind} state {dst!r}")
    if dst not in table[src]:
        raise ValueError(f"{kind}: {src} -> {dst} is not an allowed transition")


# ---------------------------------------------------------------------------
# Records
# ---------------------------------------------------------------------------


@dataclass
class ItemRecord:
    """One item's live state in a run. `data` is the executors' scratch space
    (scenario paths, last oracle result, the gap id, triage calls, the
    terminal reason); keep it JSON-serialisable."""

    item: str
    state: str
    attempts: dict = field(default_factory=dict)   # stage -> attempts made
    data: dict = field(default_factory=dict)

    @property
    def kind(self) -> str:
        return split_item_id(self.item)[0]

    @property
    def ident(self) -> str:
        return split_item_id(self.item)[1]

    def to_dict(self) -> dict:
        return {"item": self.item, "state": self.state,
                "attempts": dict(self.attempts), "data": dict(self.data)}

    @classmethod
    def from_dict(cls, d: Mapping) -> "ItemRecord":
        return cls(item=d["item"], state=d["state"],
                   attempts=dict(d.get("attempts") or {}), data=dict(d.get("data") or {}))


@dataclass(frozen=True)
class Event:
    """One row of `runs/card-loop/<run>/events.jsonl` (design D4: every transition)."""

    ts: str
    run_id: str
    item: str
    src: str | None           # None for the item's creation
    dst: str
    reason: str = ""
    stage: str | None = None
    attempt_id: str | None = None
    data: dict = field(default_factory=dict)

    def to_row(self) -> dict:
        row = {"ts": self.ts, "run_id": self.run_id, "item": self.item,
               "src": self.src, "dst": self.dst, "reason": self.reason}
        if self.stage:
            row["stage"] = self.stage
        if self.attempt_id:
            row["attempt_id"] = self.attempt_id
        if self.data:
            row["data"] = dict(self.data)
        return row

    @classmethod
    def from_row(cls, row: Mapping) -> "Event":
        return cls(ts=row["ts"], run_id=row["run_id"], item=row["item"], src=row.get("src"),
                   dst=row["dst"], reason=row.get("reason", ""), stage=row.get("stage"),
                   attempt_id=row.get("attempt_id"), data=dict(row.get("data") or {}))


@dataclass
class StageOutcome:
    """What one executor step established. The driver applies `next_state`
    (validated with `check_transition`), appends `events_data` to the
    transition event, ledgers `attempts` (already-built `ledger.Attempt`
    rows) and `corrections` (`corrections.Correction` rows), and records an
    `escalation` when the next state is ESCALATED."""

    next_state: str
    reason: str = ""
    attempts: list = field(default_factory=list)
    corrections: list = field(default_factory=list)
    data: dict = field(default_factory=dict)         # merged into ItemRecord.data
    events_data: dict = field(default_factory=dict)  # attached to the Event
    escalation: "Escalation | None" = None
    # Set by executors that produced a worker diff the driver must merge
    # before the item can advance (implement, fix_card, fix_engine).
    merge_request: "MergeRequest | None" = None
    # A newly adjudicated item counts against the plateau rule (D13).
    adjudicated: bool = False


class StageDeferred(Exception):
    """Raised by an executor that cannot advance the item now: a worker call
    failed (error, or schema-invalid after its one retry), or no family can
    take a non-terminating call. `outcome.next_state` equals the item's
    current state; its attempts and corrections are still ledgered, the
    attempt counts toward the stage's cap, and the item stays where it is.
    Terminating calls never defer: they escalate (design D5)."""

    def __init__(self, reason: str, outcome: "StageOutcome | None" = None):
        super().__init__(reason)
        self.reason = reason
        self.outcome = outcome


@dataclass(frozen=True)
class Escalation:
    """A human-queue entry (`qa/card-loop/escalations/<item>.md`, D5)."""

    item: str
    reason: str
    # Each argument: {"family", "attempt_id", "call", "citation", "reasoning"}.
    arguments: tuple = ()
    history: tuple = ()        # prior attempt ids, oldest first


@dataclass(frozen=True)
class MergeRequest:
    attempt_id: str
    family: str
    model: str | None
    artifacts: dict            # WorkerResult.artifacts: {"diff": path, "manifest": path}
    engine: bool = False       # engine-touching: own branch, human merge (D13)
    gap_id: str | None = None
    subject: str = ""


@dataclass
class MergeResult:
    ok: bool
    sha: str | None = None
    branch: str | None = None
    touched: list = field(default_factory=list)
    scope: dict = field(default_factory=dict)       # impact_scope output
    errors: list = field(default_factory=list)


@dataclass
class GateResult:
    """The fix gate (spec "Fixes pass the fix gate"): a citation, a test that
    fails before and passes after, and the scoped suite green."""

    passed: bool
    reasons: list = field(default_factory=list)
    evidence: dict = field(default_factory=dict)


# ---------------------------------------------------------------------------
# Interfaces
# ---------------------------------------------------------------------------


class RunContext(Protocol):
    """What executors, gates and the merger get from the driver. Attributes,
    not methods, so a test can pass a `types.SimpleNamespace`."""

    run_id: str
    run_dir: str                     # runs/card-loop/<run-id>
    repo: str                        # the run's working tree (the run branch)
    base_sha: str
    plan: Mapping                    # the frozen plan.json
    config: Any                      # LoopConfig
    workers: Mapping[str, Any]       # family -> Worker (contracts.Worker)
    health: Any                      # workers.health.VendorHealth
    ledger: Any                      # ledger.AttemptLedger
    corrections: Any                 # corrections.CorrectionLog
    scorecard: Any                   # router.ScorecardLike | None
    pool: Any                        # workers.pool.WorktreePool | None
    run_command: Callable            # (argv: list[str], cwd: str, timeout: float) -> (rc, stdout, stderr)
    now: Callable[[], str]           # RFC 3339 UTC
    new_attempt_id: Callable[[], str]


class StageExecutor(Protocol):
    """One per state that does work. `run` is synchronous; the driver's
    scheduler owns concurrency."""

    state: str

    def run(self, ctx: RunContext, item: ItemRecord) -> StageOutcome: ...


class Gate(Protocol):
    def check(self, ctx: RunContext, item: ItemRecord, merge: MergeResult) -> GateResult: ...


class Merger(Protocol):
    def merge(self, ctx: RunContext, request: MergeRequest) -> MergeResult: ...


class GapLane(Protocol):
    def park(self, item: str, gap_id: str, *, is_core: bool) -> None: ...
    def unpark(self, gap_id: str) -> Sequence[str]: ...
    def closed(self, ctx: RunContext) -> Sequence[str]: ...
    def ranked(self) -> Sequence[tuple[str, int]]: ...   # (gap_id, core clauses blocked), best first
