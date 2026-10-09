"""Human audit sample: ground truth for the scorecard (design D12 "Judge the judge").

Sampling is deterministic: an attempt is in the sample iff its outcome is
`accepted` and `stable_unit("audit", seed, attempt_id) < audit_rate`
(`LoopConfig.audit_rate`, decided 5%). Being a pure function of the id, the
sample is the same on every machine and branch, needs no queue file, and is
independent of the router's exploration draw (different hash namespace).

File: `qa/card-loop/audits.jsonl` (beside the attempt ledger), committed,
append-only, `merge=union`. Row schema (`?` = omitted when empty):

    ts              str   ISO-8601 UTC
    attempt_id      str
    verdict         str   agree | disagree
    sampled         bool  was the attempt in the deterministic sample when audited
    stage, item, family, model, prompt_version   copied from the attempt row
    auditor?        str
    note?           str

A re-audit of the same attempt appends a new row; the last one counts.
Only `sampled` audits estimate precision (an ad-hoc audit of a hand-picked
suspicious output would bias it), but any `disagree` marks the attempt as
corrected in the scorecard.

CLI (dispatched as `python -m tools.card_loop audit ...`):

    audit next [--stage S] [--json]          the next unaudited sampled attempt
    audit record <attempt_id> agree|disagree [--note TEXT] [--auditor NAME]
"""
from __future__ import annotations

import argparse
import json
import os
import sys
from dataclasses import dataclass, field
from pathlib import Path

from ._hashing import stable_unit
from ._jsonl import LedgerProblem, append_row, read_rows, resolve_repo_path, utc_now_iso
from .config import LoopConfig, load_config
from .contracts import STAGES
from .ledger import Attempt, dedupe_last, index_attempts, load_attempts

VERDICTS = ("agree", "disagree")
AUDITS_FILENAME = "audits.jsonl"
_KNOWN = ("ts", "attempt_id", "verdict", "sampled", "stage", "item", "family", "model",
          "prompt_version", "auditor", "note")


def default_audits_path(config: LoopConfig | None = None) -> Path:
    """Sibling of the attempt ledger (no `LoopConfig` key of its own yet)."""
    return resolve_repo_path((config or LoopConfig()).attempts_path).with_name(AUDITS_FILENAME)


def in_audit_sample(attempt_id: str, *, seed: int, rate: float) -> bool:
    return stable_unit("audit", seed, attempt_id) < rate


def sampled_attempts(attempts: list[Attempt], *, seed: int, rate: float,
                     stage: str | None = None) -> list[Attempt]:
    """Accepted attempts in the sample, oldest first (attempt ids sort by time)."""
    picked = [a for a in dedupe_last(attempts)
              if a.outcome == "accepted" and (stage is None or a.stage == stage)
              and in_audit_sample(a.attempt_id, seed=seed, rate=rate)]
    return sorted(picked, key=lambda a: a.attempt_id)


@dataclass(frozen=True)
class Audit:
    ts: str
    attempt_id: str
    verdict: str
    sampled: bool
    stage: str | None = None
    item: str | None = None
    family: str | None = None
    model: str | None = None
    prompt_version: str | None = None
    auditor: str | None = None
    note: str | None = None
    extra: dict = field(default_factory=dict)

    def validate(self) -> None:
        if not self.attempt_id:
            raise ValueError("attempt_id is required")
        if self.verdict not in VERDICTS:
            raise ValueError(f"verdict must be one of {VERDICTS}")

    def to_row(self) -> dict:
        row = {"ts": self.ts, "attempt_id": self.attempt_id, "verdict": self.verdict,
               "sampled": self.sampled, "stage": self.stage, "item": self.item,
               "family": self.family, "model": self.model,
               "prompt_version": self.prompt_version}
        if self.auditor:
            row["auditor"] = self.auditor
        if self.note:
            row["note"] = self.note
        for k, v in self.extra.items():
            row.setdefault(k, v)
        return row

    @classmethod
    def from_row(cls, row: dict) -> "Audit":
        def opt(k):
            v = row.get(k)
            return None if v in (None, "") else str(v)

        return cls(ts=str(row.get("ts") or ""), attempt_id=str(row.get("attempt_id") or ""),
                   verdict=str(row.get("verdict") or ""), sampled=bool(row.get("sampled", False)),
                   stage=opt("stage"), item=opt("item"), family=opt("family"),
                   model=opt("model"), prompt_version=opt("prompt_version"),
                   auditor=opt("auditor"), note=opt("note"),
                   extra={k: v for k, v in row.items() if k not in _KNOWN})


def make_audit(attempt: Attempt, verdict: str, *, seed: int, rate: float,
               auditor: str | None = None, note: str | None = None,
               ts: str | None = None) -> Audit:
    a = Audit(ts=ts or utc_now_iso(), attempt_id=attempt.attempt_id, verdict=verdict,
              sampled=in_audit_sample(attempt.attempt_id, seed=seed, rate=rate),
              stage=attempt.stage, item=attempt.item, family=attempt.family,
              model=attempt.model, prompt_version=attempt.prompt_version,
              auditor=auditor, note=note)
    a.validate()
    return a


def load_audits(path: str | os.PathLike) -> tuple[list[Audit], list[LedgerProblem]]:
    raw, problems = read_rows(path)
    p = str(resolve_repo_path(path))
    out = []
    for line, row in raw:
        if not row.get("attempt_id"):
            problems.append(LedgerProblem(p, line, "missing_keys", "missing attempt_id"))
            continue
        a = Audit.from_row(row)
        if a.verdict not in VERDICTS:
            problems.append(LedgerProblem(p, line, "bad_value", f"verdict={a.verdict!r}"))
            continue                      # an unreadable verdict is no ground truth
        out.append(a)
    return out, problems


def latest_audits(audits: list[Audit]) -> dict[str, Audit]:
    out: dict[str, Audit] = {}
    for a in audits:
        out[a.attempt_id] = a
    return out


def next_unaudited(attempts: list[Attempt], audits: list[Audit], *, seed: int, rate: float,
                   stage: str | None = None) -> tuple[Attempt | None, int, int]:
    """(next attempt to audit or None, sampled count, already-audited count)."""
    sample = sampled_attempts(attempts, seed=seed, rate=rate, stage=stage)
    done = latest_audits(audits)
    pending = [a for a in sample if a.attempt_id not in done]
    return (pending[0] if pending else None), len(sample), len(sample) - len(pending)


def describe(attempt: Attempt, *, runs_dir: str = "runs/card-loop") -> dict:
    """What a human needs to judge the output: identity plus artifact pointers."""
    pointers = dict(attempt.artifacts)
    pointers.setdefault("run_dir", f"{runs_dir.rstrip('/')}/{attempt.run_id}/")
    pointers.setdefault("commits", f'git log --all --grep="Loop-Attempt: {attempt.attempt_id}"')
    return {"attempt_id": attempt.attempt_id, "stage": attempt.stage, "item": attempt.item,
            "family": attempt.family, "model": attempt.model,
            "prompt_version": attempt.prompt_version, "assignment": attempt.assignment,
            "parent_attempt": attempt.parent_attempt, "ts": attempt.ts,
            "notes": attempt.notes, "artifacts": pointers}


# ----------------------------------------------------------------------------
# CLI


def _parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="python -m tools.card_loop audit",
                                description="Human audit sample of accepted worker outputs.")
    p.add_argument("--config", help="card-loop TOML config (seed, audit_rate, attempts_path)")
    p.add_argument("--attempts", help="attempt ledger path (default: config attempts_path)")
    p.add_argument("--audits", help="audit ledger path (default: beside the attempt ledger)")
    sub = p.add_subparsers(dest="cmd", required=True)
    n = sub.add_parser("next", help="show the next unaudited sampled attempt")
    n.add_argument("--stage", choices=STAGES)
    n.add_argument("--json", action="store_true")
    r = sub.add_parser("record", help="record a human verdict on an attempt")
    r.add_argument("attempt_id")
    r.add_argument("verdict", choices=VERDICTS)
    r.add_argument("--note")
    r.add_argument("--auditor", default=os.environ.get("USERNAME") or os.environ.get("USER"))
    return p


def _warn_problems(problems: list[LedgerProblem]) -> None:
    if problems:
        first = problems[0]
        print(f"warning: {len(problems)} unreadable ledger row(s); first: "
              f"{first.path}:{first.line} {first.kind} {first.message}", file=sys.stderr)


def cli_audit(argv: list[str]) -> int:
    try:
        args = _parser().parse_args(argv)
    except SystemExit as e:          # argparse: --help -> 0, usage error -> 2
        return int(e.code or 0)
    config = load_config(args.config)
    attempts_path = resolve_repo_path(args.attempts) if args.attempts else \
        resolve_repo_path(config.attempts_path)
    audits_path = resolve_repo_path(args.audits) if args.audits else default_audits_path(config)
    attempts, p1 = load_attempts(attempts_path)
    audits, p2 = load_audits(audits_path)
    _warn_problems(p1 + p2)

    if args.cmd == "next":
        nxt, n_sampled, n_done = next_unaudited(attempts, audits, seed=config.seed,
                                                rate=config.audit_rate, stage=args.stage)
        if nxt is None:
            msg = {"next": None, "sampled": n_sampled, "audited": n_done}
            print(json.dumps(msg) if args.json else
                  f"no unaudited sampled attempts ({n_done}/{n_sampled} sampled attempts audited)")
            return 0
        info = describe(nxt, runs_dir=config.runs_dir)
        if args.json:
            print(json.dumps({"next": info, "sampled": n_sampled, "audited": n_done}, indent=2))
            return 0
        print(f"audit {n_done + 1} of {n_sampled} sampled  (rate {config.audit_rate:.0%}, "
              f"seed {config.seed})")
        for key in ("attempt_id", "stage", "item", "family", "model", "prompt_version",
                    "assignment", "parent_attempt", "ts", "notes"):
            if info[key] is not None:
                print(f"  {key:<15}{info[key]}")
        print("  artifacts:")
        for k, v in info["artifacts"].items():
            print(f"    {k:<13}{v}")
        print(f"record with: python -m tools.card_loop audit record {nxt.attempt_id} "
              "agree|disagree [--note ...]")
        return 0

    by_id = index_attempts(attempts)
    target = by_id.get(args.attempt_id)
    if target is None:
        print(f"unknown attempt {args.attempt_id!r} in {attempts_path}", file=sys.stderr)
        return 1
    if target.outcome != "accepted":
        print(f"attempt {args.attempt_id} has outcome {target.outcome!r}; only accepted "
              "outputs are audited (rejections are already corrections)", file=sys.stderr)
        return 1
    audit = make_audit(target, args.verdict, seed=config.seed, rate=config.audit_rate,
                       auditor=args.auditor, note=args.note)
    append_row(audits_path, audit.to_row())
    tag = "" if audit.sampled else " (ad-hoc: outside the sample, excluded from precision)"
    print(f"recorded {args.verdict} for {args.attempt_id}{tag}")
    return 0
