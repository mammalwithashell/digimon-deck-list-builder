"""Correction events: who corrected which attempt, how, and how late (design D11).

File: `qa/card-loop/corrections.jsonl` (beside the attempt ledger), committed,
append-only, `merge=union`.

Row schema (keys in write order; `?` = omitted when empty):

    ts                 str   ISO-8601 UTC
    corrected_attempt  str   the attempt whose output was wrong
    by_attempt?        str   the correcting attempt          } exactly one of
    by_human?          str   who (a name, or "human")        } the three
    by_gate?           str   the deterministic gate that failed (validate, sim,
                             cards_behavioral, schema, ...)  }
    kind               str   KINDS (below)
    latency            str   immediate | late — fixed per kind
    stage              str|null  the corrected attempt's stage
    item?              str
    detail             str   free text: lines, calls, the gate's message

`by_gate` is an addition to the design's `by: attempt | human`: a failed
`cards_behavioral` run is neither, and recording it as either would lie.

Kinds and latency (the D11 table):

    immediate  review_reject        a reviewer attempt rejected the output
               gate_fail            a deterministic gate failed on it
               schema_invalid       output failed its schema after the retry
               family_disagreement  the other family's independent answer differed
    late       later_edit           a later diff changed lines blamed to it
               verdict_overturned   a terminating verdict it agreed with was overturned
               escalation_resolved_against  a human resolved an escalation against its call
               clause_not_exercised an interaction exam showed its "confirmed" line
                                    never triggered the clause
               fix_reverted         its fix was reverted, its branch rejected, or regressed
               false_alarm          its claimed interaction finding was a scenario error

The `detect_*` / constructor functions are pure: the driver gathers the facts
(blame, calls, gate results) and appends what they return.
"""
from __future__ import annotations

import os
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable, Mapping

from ._jsonl import LedgerProblem, append_row, read_rows, resolve_repo_path, utc_now_iso
from .config import LoopConfig

LATENCIES = ("immediate", "late")
KINDS = {
    "review_reject": "immediate",
    "gate_fail": "immediate",
    "schema_invalid": "immediate",
    "family_disagreement": "immediate",
    "later_edit": "late",
    "verdict_overturned": "late",
    "escalation_resolved_against": "late",
    "clause_not_exercised": "late",
    "fix_reverted": "late",
    "false_alarm": "late",
}
FIX_REVERT_REASONS = ("revert", "rejected_branch", "regression")
# Calls that are not a position: an abstaining worker is not "overruled".
ABSTENTIONS = (None, "", "undetermined")

CORRECTIONS_FILENAME = "corrections.jsonl"
_KNOWN = ("ts", "corrected_attempt", "by_attempt", "by_human", "by_gate", "kind", "latency",
          "stage", "item", "detail")


def default_corrections_path(config: LoopConfig | None = None) -> Path:
    """Sibling of the attempt ledger (no `LoopConfig` key of its own yet)."""
    attempts = resolve_repo_path((config or LoopConfig()).attempts_path)
    return attempts.with_name(CORRECTIONS_FILENAME)


@dataclass(frozen=True)
class Correction:
    ts: str
    corrected_attempt: str
    kind: str
    latency: str
    stage: str | None = None
    detail: str = ""
    by_attempt: str | None = None
    by_human: str | None = None
    by_gate: str | None = None
    item: str | None = None
    extra: dict = field(default_factory=dict)

    @property
    def by(self) -> tuple[str, str]:
        """("attempt"|"human"|"gate", who)."""
        for k in ("attempt", "human", "gate"):
            v = getattr(self, f"by_{k}")
            if v:
                return k, v
        return "unknown", ""

    def validate(self) -> None:
        if not self.corrected_attempt:
            raise ValueError("corrected_attempt is required")
        givers = [v for v in (self.by_attempt, self.by_human, self.by_gate) if v]
        if len(givers) != 1:
            raise ValueError("exactly one of by_attempt / by_human / by_gate is required")
        if self.kind not in KINDS:
            raise ValueError(f"unknown correction kind {self.kind!r}; expected {sorted(KINDS)}")
        if self.latency != KINDS[self.kind]:
            raise ValueError(f"kind {self.kind!r} is {KINDS[self.kind]}, not {self.latency!r}")
        if self.by_attempt and self.by_attempt == self.corrected_attempt:
            raise ValueError("an attempt cannot correct itself")

    def to_row(self) -> dict:
        row: dict = {"ts": self.ts, "corrected_attempt": self.corrected_attempt}
        for k in ("by_attempt", "by_human", "by_gate"):
            if getattr(self, k):
                row[k] = getattr(self, k)
        row.update(kind=self.kind, latency=self.latency, stage=self.stage)
        if self.item:
            row["item"] = self.item
        row["detail"] = self.detail
        for k, v in self.extra.items():
            row.setdefault(k, v)
        return row

    @classmethod
    def from_row(cls, row: dict) -> "Correction":
        def opt(k):
            v = row.get(k)
            return None if v in (None, "") else str(v)

        return cls(ts=str(row.get("ts") or ""), corrected_attempt=str(row.get("corrected_attempt") or ""),
                   kind=str(row.get("kind") or ""), latency=str(row.get("latency") or ""),
                   stage=opt("stage"), detail=str(row.get("detail") or ""),
                   by_attempt=opt("by_attempt"), by_human=opt("by_human"), by_gate=opt("by_gate"),
                   item=opt("item"), extra={k: v for k, v in row.items() if k not in _KNOWN})


class CorrectionLog:
    def __init__(self, path: str | os.PathLike | None = None, *, config: LoopConfig | None = None):
        self.path = resolve_repo_path(path) if path else default_corrections_path(config)

    def append(self, event: Correction) -> Correction:
        event.validate()
        append_row(self.path, event.to_row())
        return event

    def extend(self, events: Iterable[Correction]) -> list[Correction]:
        events = list(events)
        for e in events:          # validate all before writing any
            e.validate()
        for e in events:
            append_row(self.path, e.to_row())
        return events

    def read(self) -> tuple[list[Correction], list[LedgerProblem]]:
        return load_corrections(self.path)


def load_corrections(path: str | os.PathLike) -> tuple[list[Correction], list[LedgerProblem]]:
    """Tolerant reader: rows without `corrected_attempt` are skipped and reported;
    unknown kinds / latencies are kept and reported."""
    raw, problems = read_rows(path)
    p = str(resolve_repo_path(path))
    out = []
    for line, row in raw:
        if not row.get("corrected_attempt"):
            problems.append(LedgerProblem(p, line, "missing_keys", "missing corrected_attempt"))
            continue
        c = Correction.from_row(row)
        if c.kind not in KINDS or c.latency not in LATENCIES:
            problems.append(LedgerProblem(p, line, "bad_value",
                                          f"kind={c.kind!r} latency={c.latency!r}"))
        out.append(c)
    return out, problems


# ----------------------------------------------------------------------------
# constructors and detectors (pure)


def _event(kind: str, corrected: str, *, stage, item=None, detail="", ts=None,
           by_attempt=None, by_human=None, by_gate=None) -> Correction:
    e = Correction(ts=ts or utc_now_iso(), corrected_attempt=corrected, kind=kind,
                   latency=KINDS[kind], stage=stage, detail=detail, by_attempt=by_attempt,
                   by_human=by_human, by_gate=by_gate, item=item)
    e.validate()
    return e


def _stage_of(attempt: str, stage: str | None, attempt_stages: Mapping[str, str] | None):
    if attempt_stages and attempt in attempt_stages:
        return attempt_stages[attempt]
    return stage


def review_reject(corrected_attempt: str, *, reviewer_attempt: str, stage: str,
                  item: str | None = None, detail: str = "", ts: str | None = None) -> Correction:
    return _event("review_reject", corrected_attempt, by_attempt=reviewer_attempt, stage=stage,
                  item=item, detail=detail, ts=ts)


def gate_fail(corrected_attempt: str, *, gate: str, stage: str, item: str | None = None,
              detail: str = "", ts: str | None = None) -> Correction:
    """`gate`: validate | sim | cards_behavioral | oracle_scenario | ... (free vocabulary)."""
    return _event("gate_fail", corrected_attempt, by_gate=gate, stage=stage, item=item,
                  detail=detail, ts=ts)


def schema_invalid(corrected_attempt: str, *, stage: str, item: str | None = None,
                   detail: str = "", ts: str | None = None) -> Correction:
    """Output still failed its stage schema after the one retry (orchestration spec)."""
    return _event("schema_invalid", corrected_attempt, by_gate="schema", stage=stage, item=item,
                  detail=detail, ts=ts)


def family_disagreement(first: tuple[str, str | None], second: tuple[str, str | None], *,
                        stage: str, item: str | None = None, ts: str | None = None
                        ) -> list[Correction]:
    """Two independent answers to one terminating packet, as (attempt_id, call).

    Disagreement is an immediate correction of *each* by the other (neither is
    known right yet; the escalation's resolution later assigns the late
    correction to the one that was wrong). Agreement, or an abstention on
    either side, yields nothing.
    """
    (a, call_a), (b, call_b) = first, second
    if call_a in ABSTENTIONS or call_b in ABSTENTIONS or call_a == call_b:
        return []
    return [
        _event("family_disagreement", a, by_attempt=b, stage=stage, item=item, ts=ts,
               detail=f"{call_a} vs {call_b}"),
        _event("family_disagreement", b, by_attempt=a, stage=stage, item=item, ts=ts,
               detail=f"{call_b} vs {call_a}"),
    ]


def _ranges(lines: list[int]) -> str:
    lines = sorted(set(lines))
    out, start, prev = [], None, None
    for n in lines:
        if start is None:
            start = prev = n
        elif n == prev + 1:
            prev = n
        else:
            out.append(f"{start}" if start == prev else f"{start}-{prev}")
            start = prev = n
    if start is not None:
        out.append(f"{start}" if start == prev else f"{start}-{prev}")
    return ",".join(out)


def detect_late_edits(blame: Mapping[int, str | None], *, path: str,
                      by_attempt: str | None = None, by_human: str | None = None,
                      attempt_stages: Mapping[str, str] | None = None,
                      item: str | None = None, ts: str | None = None) -> list[Correction]:
    """One `later_edit` event per earlier attempt whose lines a later change touched.

    `blame` is `provenance.blame_attempts(repo, path, touched_old_lines, rev=<pre-edit rev>)`.
    Lines with no attempt (human / pre-loop) and the editor's own lines are skipped.
    """
    by_owner: dict[str, list[int]] = {}
    for line, owner in blame.items():
        if not owner or owner == by_attempt:
            continue
        by_owner.setdefault(owner, []).append(line)
    return [
        _event("later_edit", owner, by_attempt=by_attempt, by_human=by_human,
               stage=_stage_of(owner, None, attempt_stages), item=item, ts=ts,
               detail=f"{path}: lines {_ranges(lines)}")
        for owner, lines in sorted(by_owner.items())
    ]


_HUNK = re.compile(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@")


def touched_old_lines(diff_text: str) -> dict[str, list[int]]:
    """Old-side line numbers a unified diff deletes or replaces, per old path.

    Pure insertions touch no existing line and are not attributed to anyone:
    adding a test or a new clause next to a worker's lines is not, by itself,
    a correction of them. New files (`--- /dev/null`) contribute nothing.
    """
    out: dict[str, list[int]] = {}
    old_path: str | None = None
    old_line = old_left = new_left = 0
    for raw in diff_text.splitlines():
        in_hunk = old_left > 0 or new_left > 0
        if not in_hunk:
            if raw.startswith("diff --git "):
                old_path = None
            elif raw.startswith("--- "):
                p = raw[4:].strip().split("\t")[0]
                old_path = None if p == "/dev/null" else (p[2:] if p.startswith("a/") else p)
            else:
                m = _HUNK.match(raw)
                if m:
                    old_line = int(m.group(1))
                    old_left = int(m.group(2)) if m.group(2) is not None else 1
                    new_left = int(m.group(4)) if m.group(4) is not None else 1
            continue
        # inside a hunk: the header's line counts say exactly where it ends, so a
        # deleted line whose content starts with "-- " is never mistaken for a header
        if raw.startswith("\\"):
            continue
        if raw.startswith("-"):
            if old_path is not None:
                out.setdefault(old_path, []).append(old_line)
            old_line += 1
            old_left -= 1
        elif raw.startswith("+"):
            new_left -= 1
        else:                       # context (" " or an empty line from a stripped editor)
            old_line += 1
            old_left -= 1
            new_left -= 1
    return out


def detect_overturned_verdict(calls: Mapping[str, str | None], *, overturned: str, new_verdict: str,
                              stage: str | None = None,
                              attempt_stages: Mapping[str, str] | None = None,
                              by_attempt: str | None = None, by_human: str | None = None,
                              item: str | None = None, ts: str | None = None) -> list[Correction]:
    """A terminating verdict was overturned: correct EVERY attempt that made that call.

    `calls` maps attempt id -> the call it returned (e.g. both triage attempts
    -> "dcgo_quirk"). Attempts that called something else were right to and
    are not corrected.
    """
    if overturned == new_verdict:
        return []
    return [
        _event("verdict_overturned", aid, by_attempt=by_attempt, by_human=by_human,
               stage=_stage_of(aid, stage, attempt_stages), item=item, ts=ts,
               detail=f"{overturned} -> {new_verdict}")
        for aid, call in sorted(calls.items()) if call == overturned
    ]


def detect_escalation_resolution(calls: Mapping[str, str | None], *, resolution: str,
                                 by_human: str = "human", stage: str | None = None,
                                 attempt_stages: Mapping[str, str] | None = None,
                                 item: str | None = None, ts: str | None = None
                                 ) -> list[Correction]:
    """A human resolved an escalation: correct every attempt whose call lost.

    Abstentions (`undetermined`, no call) took no position and are skipped.
    """
    return [
        _event("escalation_resolved_against", aid, by_human=by_human,
               stage=_stage_of(aid, stage, attempt_stages), item=item, ts=ts,
               detail=f"called {call}, resolved {resolution}")
        for aid, call in sorted(calls.items())
        if call not in ABSTENTIONS and call != resolution
    ]


def detect_unexercised_clauses(confirmed_lines: Mapping[str, str | None],
                               exercised: Mapping[str, bool], *,
                               by_attempt: str | None = None, by_human: str | None = None,
                               stage: str = "author_clause",
                               attempt_stages: Mapping[str, str] | None = None,
                               ts: str | None = None) -> list[Correction]:
    """An interaction exam showed a "confirmed" clause line never fired its clause.

    `confirmed_lines`: clause id -> the attempt that authored the confirmed
    line (its scenario's `meta.produced_by`). `exercised`: clause id -> whether
    the interaction exam observed the clause trigger on that line. A clause
    missing from `exercised` was not measured and is not a correction.
    """
    out = []
    for clause, author in sorted(confirmed_lines.items()):
        if not author or exercised.get(clause, True):
            continue
        out.append(_event("clause_not_exercised", author, by_attempt=by_attempt,
                          by_human=by_human, stage=_stage_of(author, stage, attempt_stages),
                          item=f"clause:{clause}", ts=ts,
                          detail=f"confirmed line for {clause} never triggered it"))
    return out


def fix_reverted(fix_attempt: str, *, reason: str, stage: str = "fix_card",
                 by_attempt: str | None = None, by_human: str | None = None,
                 item: str | None = None, detail: str = "", ts: str | None = None) -> Correction:
    if reason not in FIX_REVERT_REASONS:
        raise ValueError(f"reason must be one of {FIX_REVERT_REASONS}")
    return _event("fix_reverted", fix_attempt, by_attempt=by_attempt, by_human=by_human,
                  stage=stage, item=item, ts=ts, detail=f"{reason}: {detail}".rstrip(": "))


def false_alarm(interaction_attempt: str, *, by_attempt: str | None = None,
                by_human: str | None = None, stage: str = "author_interaction",
                item: str | None = None, detail: str = "", ts: str | None = None) -> Correction:
    """A claimed interaction finding turned out to be a scenario error."""
    return _event("false_alarm", interaction_attempt, by_attempt=by_attempt, by_human=by_human,
                  stage=stage, item=item, ts=ts, detail=detail)
