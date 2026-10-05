"""Model scorecard: which model is best at which stage, by measured corrections (D12).

Pure computation over the three ledgers (attempts, corrections, audits) so it
is fully testable on synthetic rows. Workers never see any of this: only the
driver's router reads it.

Cells are (stage, family, model, prompt_version) — prompt versions are never
pooled, so a prompt change starts a fresh comparison. Per cell:

    attempts              ledger rows (deduped by attempt_id)
    first_pass            outcome `accepted` and no immediate correction
    immediate_corrected   >= 1 immediate correction event
    late_corrected        >= 1 late correction event
    weighted_correction   (immediate + late_correction_weight * late) / attempts —
                          a penalty score, not a probability (late weighs 3x)
    good_units            accepted, no correction of any latency, no `disagree` audit
    corrected_cost        total USD / good_units, with an interval (see _stats)
    audit_precision       sampled audits that agree / sampled audits
    reviewer_precision    (review cells) sampled audits agreeing with outputs this
                          reviewer approved / such audits — "judge the judge"

Rates carry Wilson 95% intervals. A cell with any attempt that consumed tokens
but has no USD cost (Codex before a price table is configured) reports no
corrected cost rather than a guessed one; the router then cannot switch on it.
An attempt with neither tokens nor cost (a crash before any usage) counts $0.

Reviewer approval is inferred, not stored: review attempt R (stage `review`,
`parent_attempt` = A, outcome `accepted`) approved A unless a `review_reject`
correction names `corrected_attempt=A, by_attempt=R`.

CLI: `python -m tools.card_loop report models [--since 30d] [--stage S] [--json]`.
"""
from __future__ import annotations

import argparse
import json
import math
import re
import sys
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone

from ._jsonl import LedgerProblem, parse_ts, resolve_repo_path
from ._stats import cost_per_good_unit, wilson
from .audit import Audit, default_audits_path, latest_audits, load_audits
from .config import LoopConfig, load_config
from .contracts import STAGES
from .corrections import Correction, default_corrections_path, load_corrections
from .ledger import Attempt, dedupe_last, load_attempts


@dataclass(frozen=True)
class CellKey:
    stage: str
    family: str
    model: str | None
    prompt_version: str

    @property
    def label(self) -> str:
        return f"{self.family}/{self.model or 'default'}@{self.prompt_version}"


@dataclass(frozen=True)
class Facts:
    """Everything the scorecard derives about one attempt."""
    attempt: Attempt
    accepted: bool
    first_pass: bool
    immediate: bool
    late: bool
    good: bool
    cost: float | None          # None = consumed tokens but unpriced
    audit: str | None           # verdict of the latest *sampled* audit
    disagreed: bool             # any audit (sampled or not) disagreed


@dataclass(frozen=True)
class CellStats:
    key: CellKey
    attempts: int
    accepted: int
    first_pass: int
    immediate_corrected: int
    late_corrected: int
    good_units: int
    weighted_correction: float
    cost_total: float
    unpriced_attempts: int
    corrected_cost: float | None
    corrected_cost_ci: tuple[float, float] | None
    audited: int
    audit_agree: int
    reviewer_audited: int = 0
    reviewer_agree: int = 0

    def rate(self, num: int, den: int | None = None) -> float | None:
        den = self.attempts if den is None else den
        return num / den if den else None

    @property
    def first_pass_rate(self):
        return self.rate(self.first_pass)

    @property
    def first_pass_ci(self):
        return wilson(self.first_pass, self.attempts)

    @property
    def immediate_rate(self):
        return self.rate(self.immediate_corrected)

    @property
    def immediate_ci(self):
        return wilson(self.immediate_corrected, self.attempts)

    @property
    def late_rate(self):
        return self.rate(self.late_corrected)

    @property
    def late_ci(self):
        return wilson(self.late_corrected, self.attempts)

    @property
    def audit_precision(self):
        return self.rate(self.audit_agree, self.audited)

    @property
    def audit_precision_ci(self):
        return wilson(self.audit_agree, self.audited)

    @property
    def reviewer_precision(self):
        return self.rate(self.reviewer_agree, self.reviewer_audited)

    @property
    def reviewer_precision_ci(self):
        return wilson(self.reviewer_agree, self.reviewer_audited)

    def to_json(self) -> dict:
        def num(x):
            return None if x is None or (isinstance(x, float) and math.isinf(x)) else x

        def ci(t):
            return None if t is None else [num(t[0]), num(t[1])]

        return {
            "stage": self.key.stage, "family": self.key.family, "model": self.key.model,
            "prompt_version": self.key.prompt_version, "attempts": self.attempts,
            "accepted": self.accepted, "first_pass": self.first_pass,
            "first_pass_rate": self.first_pass_rate, "first_pass_ci": ci(self.first_pass_ci),
            "immediate_corrected": self.immediate_corrected,
            "immediate_rate": self.immediate_rate, "immediate_ci": ci(self.immediate_ci),
            "late_corrected": self.late_corrected, "late_rate": self.late_rate,
            "late_ci": ci(self.late_ci), "weighted_correction": self.weighted_correction,
            "good_units": self.good_units, "cost_total_usd": self.cost_total,
            "unpriced_attempts": self.unpriced_attempts,
            "corrected_cost_usd": num(self.corrected_cost),
            "corrected_cost_ci": ci(self.corrected_cost_ci),
            "audited": self.audited, "audit_agree": self.audit_agree,
            "audit_precision": self.audit_precision,
            "audit_precision_ci": ci(self.audit_precision_ci) if self.audited else None,
            "reviewer_audited": self.reviewer_audited, "reviewer_agree": self.reviewer_agree,
            "reviewer_precision": self.reviewer_precision,
            "reviewer_precision_ci": (ci(self.reviewer_precision_ci)
                                      if self.reviewer_audited else None),
        }


@dataclass(frozen=True)
class ReviewerInfluence:
    """An author cell's first-pass acceptance with and without one reviewer."""
    author: CellKey
    reviewer: CellKey
    with_rate: float | None          # over every attempt of the author cell
    with_n: int
    without_rate: float | None       # over the attempts this reviewer did NOT judge
    without_n: int
    judged: int                      # attempts this reviewer judged
    approved: int                    # ... and approved
    reviewer_precision: float | None
    adjusted_rate: float | None      # this reviewer's approvals discounted by its precision

    def to_json(self) -> dict:
        return {"author": self.author.label, "author_stage": self.author.stage,
                "reviewer": self.reviewer.label, "with": self.with_rate, "with_n": self.with_n,
                "without": self.without_rate, "without_n": self.without_n,
                "judged": self.judged, "approved": self.approved,
                "reviewer_precision": self.reviewer_precision,
                "precision_adjusted": self.adjusted_rate}


def _cost(a: Attempt) -> float | None:
    u = a.usage
    if u.cost_usd is not None:
        return float(u.cost_usd)
    used = u.input_tokens + u.output_tokens + u.cache_read_tokens + u.cache_write_tokens
    return 0.0 if used == 0 else None


class Scorecard:
    """Per-cell statistics over loaded ledger rows.

    `since` restricts which *attempts* are scored (by their `ts`); corrections
    and audits always apply to the attempts they name, whenever written.
    """

    def __init__(self, attempts: list[Attempt], corrections: list[Correction] = (),
                 audits: list[Audit] = (), *, late_weight: float = 3.0,
                 since: datetime | None = None):
        self.late_weight = late_weight
        self.all_attempts = dedupe_last(list(attempts))
        self.by_id = {a.attempt_id: a for a in self.all_attempts}
        self.corrections = list(corrections)
        self.audits = latest_audits(list(audits))
        self.since = since

        self._corr: dict[str, list[Correction]] = {}
        for c in self.corrections:
            self._corr.setdefault(c.corrected_attempt, []).append(c)
        rejected_by = {(c.corrected_attempt, c.by_attempt) for c in self.corrections
                       if c.kind == "review_reject" and c.by_attempt}
        # reviewer attempt id -> [(author attempt id, approved?)]
        self.judgements: dict[str, list[tuple[str, bool]]] = {}
        for r in self.all_attempts:
            if r.stage == "review" and r.parent_attempt and r.outcome == "accepted":
                self.judgements.setdefault(r.attempt_id, []).append(
                    (r.parent_attempt, (r.parent_attempt, r.attempt_id) not in rejected_by))

        def in_window(a: Attempt) -> bool:
            if since is None:
                return True
            t = parse_ts(a.ts)
            return t is not None and t >= since

        self.facts = {a.attempt_id: self._facts(a) for a in self.all_attempts if in_window(a)}

    # -- per attempt --------------------------------------------------------------------

    def _facts(self, a: Attempt) -> Facts:
        corr = self._corr.get(a.attempt_id, [])
        immediate = any(c.latency == "immediate" for c in corr)
        late = any(c.latency == "late" for c in corr)
        audit = self.audits.get(a.attempt_id)
        disagreed = audit is not None and audit.verdict == "disagree"
        accepted = a.outcome == "accepted"
        return Facts(attempt=a, accepted=accepted, first_pass=accepted and not immediate,
                     immediate=immediate, late=late,
                     good=accepted and not corr and not disagreed, cost=_cost(a),
                     audit=audit.verdict if audit is not None and audit.sampled else None,
                     disagreed=disagreed)

    # -- cells --------------------------------------------------------------------------

    @staticmethod
    def key_of(a: Attempt) -> CellKey:
        return CellKey(a.stage, a.family, a.model, a.prompt_version)

    def _summarize(self, key: CellKey, facts: list[Facts]) -> CellStats:
        n = len(facts)
        costs = [f.cost for f in facts if f.cost is not None]
        unpriced = sum(1 for f in facts if f.cost is None)
        good = sum(f.good for f in facts)
        imm = sum(f.immediate for f in facts)
        late = sum(f.late for f in facts)
        if unpriced or n == 0:
            cc, cc_ci = None, None
        else:
            lo, point, hi = cost_per_good_unit(costs, good)
            cc, cc_ci = point, (lo, hi)
        audited = [f for f in facts if f.audit is not None]
        r_aud = r_agree = 0
        if key.stage == "review":
            for f in facts:
                for author_id, approved in self.judgements.get(f.attempt.attempt_id, []):
                    au = self.audits.get(author_id)
                    if approved and au is not None and au.sampled:
                        r_aud += 1
                        r_agree += au.verdict == "agree"
        return CellStats(
            key=key, attempts=n, accepted=sum(f.accepted for f in facts),
            first_pass=sum(f.first_pass for f in facts), immediate_corrected=imm,
            late_corrected=late, good_units=good,
            weighted_correction=((imm + self.late_weight * late) / n) if n else 0.0,
            cost_total=sum(costs), unpriced_attempts=unpriced, corrected_cost=cc,
            corrected_cost_ci=cc_ci, audited=len(audited),
            audit_agree=sum(f.audit == "agree" for f in audited),
            reviewer_audited=r_aud, reviewer_agree=r_agree)

    def cells(self, stage: str | None = None) -> list[CellStats]:
        groups: dict[CellKey, list[Facts]] = {}
        for f in self.facts.values():
            if stage is None or f.attempt.stage == stage:
                groups.setdefault(self.key_of(f.attempt), []).append(f)
        order = {s: i for i, s in enumerate(STAGES)}
        keys = sorted(groups, key=lambda k: (order.get(k.stage, 99), k.stage, k.family,
                                             k.model or "", k.prompt_version))
        return [self._summarize(k, groups[k]) for k in keys]

    def family_stats(self, stage: str, family: str, prompt_version: str,
                     model: str | None = None) -> CellStats | None:
        """One family's stats in a stage at a prompt version (router input).

        `model=None` pools every model of the family; otherwise only that model.
        None when there are no attempts.
        """
        facts = [f for f in self.facts.values()
                 if f.attempt.stage == stage and f.attempt.family == family
                 and f.attempt.prompt_version == prompt_version
                 and (model is None or f.attempt.model == model)]
        if not facts:
            return None
        return self._summarize(CellKey(stage, family, model, prompt_version), facts)

    # -- lenient-reviewer view ----------------------------------------------------------

    def reviewer_influence(self, stage: str | None = None) -> list[ReviewerInfluence]:
        cell_stats = {c.key: c for c in self.cells()}
        author_groups: dict[CellKey, list[Facts]] = {}
        for f in self.facts.values():
            if f.attempt.stage != "review" and (stage is None or f.attempt.stage == stage):
                author_groups.setdefault(self.key_of(f.attempt), []).append(f)
        # author attempt id -> {reviewer cell: approved?}
        judged_by: dict[str, dict[CellKey, bool]] = {}
        for rid, pairs in self.judgements.items():
            rkey = self.key_of(self.by_id[rid])
            for author_id, approved in pairs:
                judged_by.setdefault(author_id, {})[rkey] = approved
        out = []
        for akey in sorted(author_groups, key=lambda k: (k.stage, k.label)):
            facts = author_groups[akey]
            reviewers = sorted({r for f in facts for r in judged_by.get(f.attempt.attempt_id, {})},
                               key=lambda k: k.label)
            n = len(facts)
            fp_all = sum(f.first_pass for f in facts)
            for rkey in reviewers:
                mine = [f for f in facts if rkey in judged_by.get(f.attempt.attempt_id, {})]
                rest = [f for f in facts if rkey not in judged_by.get(f.attempt.attempt_id, {})]
                approved = [f for f in mine if judged_by[f.attempt.attempt_id][rkey]]
                rstats = cell_stats.get(rkey)
                prec = rstats.reviewer_precision if rstats else None
                adjusted = None
                if prec is not None and n:
                    by_others = sum(f.first_pass for f in rest)
                    by_me = sum(f.first_pass for f in approved)
                    adjusted = (by_others + by_me * prec) / n
                out.append(ReviewerInfluence(
                    author=akey, reviewer=rkey, with_rate=fp_all / n if n else None, with_n=n,
                    without_rate=(sum(f.first_pass for f in rest) / len(rest)) if rest else None,
                    without_n=len(rest), judged=len(mine), approved=len(approved),
                    reviewer_precision=prec, adjusted_rate=adjusted))
        return out

    # -- output -------------------------------------------------------------------------

    def to_json(self, stage: str | None = None) -> dict:
        return {"late_correction_weight": self.late_weight,
                "since": self.since.isoformat() if self.since else None,
                "stage": stage,
                "cells": [c.to_json() for c in self.cells(stage)],
                "reviewer_influence": [r.to_json() for r in self.reviewer_influence(stage)]}


# ----------------------------------------------------------------------------
# rendering


def _pct(x: float | None) -> str:
    return "-" if x is None else f"{100 * x:.0f}%"


def _ci(t: tuple[float, float] | None, money: bool = False) -> str:
    if t is None:
        return ""
    if money:
        hi = "inf" if math.isinf(t[1]) else f"{t[1]:.2f}"
        return f"[{t[0]:.2f},{hi}]"
    return f"[{100 * t[0]:.0f},{100 * t[1]:.0f}]"


def render_table(card: Scorecard, stage: str | None = None) -> str:
    cells = card.cells(stage)
    if not cells:
        return "no attempts in the ledger for this selection"
    header = ["stage", "family/model", "prompt", "n", "1st-pass", "imm", "late",
              f"wcorr(x{card.late_weight:g})", "$/good", "audit", "rev-prec"]
    rows = []
    for c in cells:
        if c.corrected_cost is None:
            cost = "unpriced" if c.unpriced_attempts else "-"
        elif math.isinf(c.corrected_cost):
            cost = "inf (0 good)"
        else:
            cost = f"{c.corrected_cost:.2f} {_ci(c.corrected_cost_ci, money=True)}"
        audit = f"{c.audit_agree}/{c.audited}" if c.audited else "-"
        rev = (f"{_pct(c.reviewer_precision)} ({c.reviewer_agree}/{c.reviewer_audited})"
               if c.reviewer_audited else "-")
        rows.append([c.key.stage, f"{c.key.family}/{c.key.model or 'default'}",
                     c.key.prompt_version, str(c.attempts),
                     f"{_pct(c.first_pass_rate)} {_ci(c.first_pass_ci)}",
                     _pct(c.immediate_rate), _pct(c.late_rate), f"{c.weighted_correction:.2f}",
                     cost, audit, rev])
    widths = [max(len(r[i]) for r in [header, *rows]) for i in range(len(header))]
    lines = ["  ".join(v.ljust(w) for v, w in zip(r, widths)).rstrip() for r in [header, *rows]]
    infl = card.reviewer_influence(stage)
    if infl:
        lines += ["", "author first-pass acceptance with / without each reviewer:"]
        for r in infl:
            adj = f", precision-adjusted {_pct(r.adjusted_rate)}" if r.adjusted_rate is not None else ""
            lines.append(f"  {r.author.stage} {r.author.label} | reviewer {r.reviewer.label}: "
                         f"with {_pct(r.with_rate)} (n={r.with_n}), without "
                         f"{_pct(r.without_rate)} (n={r.without_n}){adj}")
    return "\n".join(lines)


_SINCE = re.compile(r"^(\d+)([hdw])$")


def parse_since(value: str | None, now: datetime | None = None) -> datetime | None:
    """`30d`, `12h`, `2w`, or an ISO date/datetime (UTC if no zone)."""
    if not value:
        return None
    now = now or datetime.now(timezone.utc)
    m = _SINCE.match(value.strip())
    if m:
        n, unit = int(m.group(1)), m.group(2)
        return now - timedelta(**{{"h": "hours", "d": "days", "w": "weeks"}[unit]: n})
    t = parse_ts(value.strip())
    if t is None:
        raise ValueError(f"--since: expected Nd / Nh / Nw or an ISO date, got {value!r}")
    return t


def load_scorecard(config: LoopConfig, *, attempts_path=None, corrections_path=None,
                   audits_path=None, since: datetime | None = None
                   ) -> tuple[Scorecard, list[LedgerProblem]]:
    a, p1 = load_attempts(resolve_repo_path(attempts_path) if attempts_path
                          else resolve_repo_path(config.attempts_path))
    c, p2 = load_corrections(resolve_repo_path(corrections_path) if corrections_path
                             else default_corrections_path(config))
    u, p3 = load_audits(resolve_repo_path(audits_path) if audits_path
                        else default_audits_path(config))
    return Scorecard(a, c, u, late_weight=config.late_correction_weight, since=since), p1 + p2 + p3


def _parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="python -m tools.card_loop report")
    sub = p.add_subparsers(dest="report", required=True, metavar="REPORT")
    m = sub.add_parser("models", help="per stage x model x prompt version scorecard")
    m.add_argument("--since", help="only attempts newer than Nd / Nh / Nw or an ISO date")
    m.add_argument("--stage", choices=STAGES)
    m.add_argument("--json", action="store_true")
    m.add_argument("--config")
    m.add_argument("--attempts")
    m.add_argument("--corrections")
    m.add_argument("--audits")
    return p


def cli_report(argv: list[str], *, now: datetime | None = None) -> int:
    try:
        args = _parser().parse_args(argv)
    except SystemExit as e:
        return int(e.code or 0)
    try:
        since = parse_since(args.since, now)
    except ValueError as e:
        print(str(e), file=sys.stderr)
        return 2
    config = load_config(args.config)
    card, problems = load_scorecard(config, attempts_path=args.attempts,
                                    corrections_path=args.corrections,
                                    audits_path=args.audits, since=since)
    if problems:
        first = problems[0]
        print(f"warning: {len(problems)} unreadable ledger row(s); first: "
              f"{first.path}:{first.line} {first.kind} {first.message}", file=sys.stderr)
    if args.json:
        out = card.to_json(args.stage)
        out["ledger_problems"] = len(problems)
        print(json.dumps(out, indent=2))
    else:
        print(render_table(card, args.stage))
    return 0
