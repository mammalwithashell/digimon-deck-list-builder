"""The attempt ledger: one row per worker call (design D11, spec model-scorecard).

File: `qa/card-loop/attempts.jsonl` (`LoopConfig.attempts_path`), committed,
append-only, `merge=union` in `.gitattributes` so runs on different branches
merge without conflicts.

Row schema (keys in write order; `?` = omitted when empty):

    attempt_id      str   new_attempt_id(run_id): unique, sorts chronologically
    run_id          str   the card-loop run that made the call
    ts              str   ISO-8601 UTC, "...Z", when the row was written
    stage           str   contracts.STAGES
    item            str   contracts.item_id(...), e.g. "card:BT7-056"
    family          str   contracts.FAMILIES
    model           str|null   model as invoked (null = the CLI's own default)
    effort          str|null
    prompt_version  str   the stage template's `version:` header
    assignment      str   ASSIGNMENTS: routed | explore | forced
    outcome         str   OUTCOMES (below)
    usage           obj   contracts.Usage fields
    parent_attempt? str   the attempt this one reviews / fixes / second-opinions
    artifacts?      obj   pointers, e.g. {"diff": ..., "manifest": ..., "transcript": ...}
    notes?          str

Outcome vocabulary — the attempt's final *immediate* outcome, written once the
driver has applied the checks that follow the call directly:

    accepted         output used: passed its schema, gates and review
    rejected         a reviewer rejected it (a correction event says by whom)
    gate_failed      a deterministic gate failed (validate, sim, cards_behavioral, ...)
    schema_invalid   output failed the stage schema, including the one retry
    error            the worker process failed (crash, timeout, transport)
    quota_exhausted  the vendor reported quota / credit exhaustion
    escalated        the call was handed to the human queue (e.g. families disagreed)

Late judgements (an overturned verdict, a blamed later edit) never rewrite a
row; they are correction events (`corrections.py`). If the same attempt id
appears twice — a union merge of a cherry-picked commit can duplicate a line —
`dedupe_last` keeps the last occurrence.

The reader is deliberately tolerant: the exam attempt log's reader once broke
on drifted keys. Unknown keys are preserved in `Attempt.extra`, missing
optional keys default, unknown vocabulary values are kept but reported, and
only a row missing its identity (`attempt_id`, `stage`, `family`) is skipped.
"""
from __future__ import annotations

import os
import re
import secrets
import threading
from dataclasses import asdict, dataclass, field, fields
from datetime import datetime, timedelta, timezone

from ._jsonl import LedgerProblem, append_row, read_rows, resolve_repo_path, utc_now_iso
from .config import LoopConfig
from .contracts import FAMILIES, STAGES, TaskPacket, Usage, WorkerResult

OUTCOMES = ("accepted", "rejected", "gate_failed", "schema_invalid", "error",
            "quota_exhausted", "escalated")
ASSIGNMENTS = ("routed", "explore", "forced")

# Worker statuses that already determine the outcome; `ok` does not — the
# driver decides accepted / rejected / gate_failed / escalated after its checks.
_STATUS_OUTCOME = {"schema_invalid": "schema_invalid", "error": "error",
                   "quota_exhausted": "quota_exhausted"}

_REQUIRED = ("attempt_id", "stage", "family")
_KNOWN = ("attempt_id", "run_id", "ts", "stage", "item", "family", "model", "effort",
          "prompt_version", "assignment", "outcome", "usage", "parent_attempt",
          "artifacts", "notes")
_USAGE_FIELDS = {f.name for f in fields(Usage)}

# ----------------------------------------------------------------------------
# attempt ids


_ID_LOCK = threading.Lock()
_last_stamp: list[datetime] = []


def _slug(run_id: str) -> str:
    s = re.sub(r"[^A-Za-z0-9_.-]+", "_", run_id.strip()) or "run"
    return s[:48]


def new_attempt_id(run_id: str, *, now: datetime | None = None) -> str:
    """`<UTC yyyymmddTHHMMSSffffff>Z-<run>-<6 hex>`.

    Sorts chronologically (lexicographic order of the fixed-width stamp), is
    strictly increasing within a process (a clash bumps the stamp by 1 µs) and
    unique across processes and branches (run id + 24 random bits). Contains no
    whitespace, so it is safe as a git trailer value.
    """
    stamp = (now or datetime.now(timezone.utc)).astimezone(timezone.utc)
    with _ID_LOCK:
        if _last_stamp and stamp <= _last_stamp[0]:
            stamp = _last_stamp[0] + timedelta(microseconds=1)
        _last_stamp[:] = [stamp]
    return f"{stamp.strftime('%Y%m%dT%H%M%S%f')}Z-{_slug(run_id)}-{secrets.token_hex(3)}"


# ----------------------------------------------------------------------------
# rows


def usage_to_dict(usage: Usage) -> dict:
    return asdict(usage)


def usage_from_dict(raw: object) -> Usage:
    """Tolerant: unknown keys dropped, wrong-typed values fall back to defaults."""
    if not isinstance(raw, dict):
        return Usage()
    kwargs = {}
    defaults = Usage()
    for name in _USAGE_FIELDS:
        if name not in raw:
            continue
        v = raw[name]
        d = getattr(defaults, name)
        if name == "cost_usd":
            kwargs[name] = float(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else None
        elif isinstance(d, bool):
            kwargs[name] = bool(v)
        elif isinstance(d, int):
            kwargs[name] = int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else d
        elif isinstance(d, float):
            kwargs[name] = float(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else d
    return Usage(**kwargs)


@dataclass(frozen=True)
class Attempt:
    attempt_id: str
    run_id: str
    ts: str
    stage: str
    item: str
    family: str
    model: str | None
    effort: str | None
    prompt_version: str
    assignment: str
    outcome: str
    usage: Usage = field(default_factory=Usage)
    parent_attempt: str | None = None
    artifacts: dict = field(default_factory=dict)
    notes: str | None = None
    # Keys this version of the reader does not know, kept verbatim so a
    # round-trip never loses data written by a newer writer.
    extra: dict = field(default_factory=dict)

    def validate(self) -> None:
        """Strict check used by the writer (the reader never calls this)."""
        if not self.attempt_id or any(c.isspace() for c in self.attempt_id):
            raise ValueError(f"bad attempt_id {self.attempt_id!r}")
        if self.stage not in STAGES:
            raise ValueError(f"unknown stage {self.stage!r}")
        if self.family not in FAMILIES:
            raise ValueError(f"unknown family {self.family!r}")
        if self.assignment not in ASSIGNMENTS:
            raise ValueError(f"unknown assignment {self.assignment!r}; expected {ASSIGNMENTS}")
        if self.outcome not in OUTCOMES:
            raise ValueError(f"unknown outcome {self.outcome!r}; expected {OUTCOMES}")
        if not self.run_id or not self.item or not self.prompt_version:
            raise ValueError("run_id, item and prompt_version are required")

    def to_row(self) -> dict:
        row = {
            "attempt_id": self.attempt_id, "run_id": self.run_id, "ts": self.ts,
            "stage": self.stage, "item": self.item, "family": self.family,
            "model": self.model, "effort": self.effort,
            "prompt_version": self.prompt_version, "assignment": self.assignment,
            "outcome": self.outcome, "usage": usage_to_dict(self.usage),
        }
        if self.parent_attempt:
            row["parent_attempt"] = self.parent_attempt
        if self.artifacts:
            row["artifacts"] = dict(self.artifacts)
        if self.notes:
            row["notes"] = self.notes
        for k, v in self.extra.items():
            row.setdefault(k, v)
        return row

    @classmethod
    def from_row(cls, row: dict) -> "Attempt":
        """Build from a ledger row without validating vocabulary."""
        def s(key, default=""):
            v = row.get(key, default)
            return v if isinstance(v, str) else (default if v is None else str(v))

        def opt(key):
            v = row.get(key)
            return None if v is None else (v if isinstance(v, str) else str(v))

        arts = row.get("artifacts")
        return cls(
            attempt_id=s("attempt_id"), run_id=s("run_id"), ts=s("ts"),
            stage=s("stage"), item=s("item"), family=s("family"),
            model=opt("model"), effort=opt("effort"),
            prompt_version=s("prompt_version"), assignment=s("assignment", "routed"),
            outcome=s("outcome"), usage=usage_from_dict(row.get("usage")),
            parent_attempt=opt("parent_attempt"),
            artifacts=dict(arts) if isinstance(arts, dict) else {},
            notes=opt("notes"),
            extra={k: v for k, v in row.items() if k not in _KNOWN},
        )


def outcome_for_status(status: str) -> str | None:
    """The outcome a worker status forces, or None when the driver must decide."""
    return _STATUS_OUTCOME.get(status)


def attempt_from_call(packet: TaskPacket, result: WorkerResult, *, run_id: str,
                      assignment: str, outcome: str | None = None,
                      parent_attempt: str | None = None, notes: str | None = None,
                      ts: str | None = None) -> Attempt:
    """Ledger row for one worker call, from its packet and result.

    `outcome` is required when the worker returned `ok` (the driver decides);
    for failure statuses it defaults to the status-implied outcome.
    """
    forced = outcome_for_status(result.status)
    if outcome is None:
        if forced is None:
            raise ValueError("worker status 'ok' needs an explicit outcome from the driver")
        outcome = forced
    artifacts = dict(result.artifacts or {})
    if result.transcript_path and "transcript" not in artifacts:
        artifacts["transcript"] = result.transcript_path
    if notes is None and result.error:
        notes = result.error
    return Attempt(
        attempt_id=packet.attempt_id, run_id=run_id, ts=ts or utc_now_iso(),
        stage=packet.stage, item=packet.item, family=packet.family,
        model=packet.model, effort=packet.effort, prompt_version=packet.prompt_version,
        assignment=assignment, outcome=outcome, usage=result.usage,
        parent_attempt=parent_attempt, artifacts=artifacts, notes=notes,
    )


# ----------------------------------------------------------------------------
# writer / reader


def default_attempts_path(config: LoopConfig | None = None):
    return resolve_repo_path((config or LoopConfig()).attempts_path)


class AttemptLedger:
    """Append-only writer and tolerant reader for one attempts file."""

    def __init__(self, path: str | os.PathLike | None = None, *, config: LoopConfig | None = None):
        self.path = resolve_repo_path(path) if path else default_attempts_path(config)

    def append(self, attempt: Attempt) -> Attempt:
        attempt.validate()
        append_row(self.path, attempt.to_row())
        return attempt

    def read(self) -> tuple[list[Attempt], list[LedgerProblem]]:
        return load_attempts(self.path)


def load_attempts(path: str | os.PathLike) -> tuple[list[Attempt], list[LedgerProblem]]:
    """Every readable attempt (file order) plus a list of problems.

    Rows missing an identity key are skipped (`missing_keys`); rows with an
    unknown stage / family / assignment / outcome are kept and reported
    (`bad_value`) so newer vocabulary never makes old tooling crash.
    """
    raw_rows, problems = read_rows(path)
    out: list[Attempt] = []
    p = str(resolve_repo_path(path))
    for line, row in raw_rows:
        missing = [k for k in _REQUIRED if not row.get(k)]
        if missing:
            problems.append(LedgerProblem(p, line, "missing_keys", f"missing {missing}"))
            continue
        a = Attempt.from_row(row)
        for key, vocab in (("stage", STAGES), ("family", FAMILIES),
                           ("assignment", ASSIGNMENTS), ("outcome", OUTCOMES)):
            if getattr(a, key) not in vocab:
                problems.append(LedgerProblem(p, line, "bad_value",
                                              f"{key}={getattr(a, key)!r} not in vocabulary"))
        out.append(a)
    return out, problems


def dedupe_last(attempts: list[Attempt]) -> list[Attempt]:
    """One attempt per id, keeping the last occurrence, in first-seen order."""
    last: dict[str, Attempt] = {}
    for a in attempts:
        last[a.attempt_id] = a  # re-assignment keeps the key's first position
    return list(last.values())


def index_attempts(attempts: list[Attempt]) -> dict[str, Attempt]:
    return {a.attempt_id: a for a in dedupe_last(attempts)}
