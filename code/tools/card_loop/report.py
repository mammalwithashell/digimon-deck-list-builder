"""A run's `report.md` (design D13, spec "Report leads with the unadjudicated").

The FIRST line names the unmeasured, escalated and unavailable counts before
anything that reads like success, so a skim never mistakes a stopped run for a
passing one. Then: items per kind, where the open work sits, attempts and cost
by stage x family, the stop reason, the escalation queue, blocked items and
problems.

Pure functions of their inputs (no I/O except `write_report`).
"""
from __future__ import annotations

import os
from collections import Counter, defaultdict
from pathlib import Path
from typing import Iterable, Mapping

from .driver_contracts import TERMINAL_BY_KIND

_KINDS = ("card", "clause", "interaction")
_EXAMPLES = 5


def first_line(run_id: str, counts: Mapping, stop: Mapping | None = None) -> str:
    t = counts["total"]
    cards = counts.get("card") or {}
    exam_conf = t["confirmed"]
    head = (f"**{t['unmeasured']} unmeasured, {t['escalated']} escalated, "
            f"{t['unavailable']} unavailable** -- {exam_conf} confirmed, {t['terminal']} terminal")
    if cards.get("items"):
        head += (f"; cards {cards['implemented']}/{cards['items']} implemented"
                 f" ({cards['parked']} parked)")
    head += f" (of {t['items']} items; adjudicated {t['adjudicated']})"
    tail = f" | run `{run_id}`"
    if stop:
        tail += f" stopped: {stop.get('reason', '?')}"
    return head + tail


def _fmt_usd(v: float | None) -> str:
    return "unpriced" if v is None else f"${v:.2f}"


def _items_table(counts: Mapping) -> list[str]:
    cols = ("items", "confirmed", "terminal", "unavailable", "escalated", "unmeasured",
            "implemented", "parked", "adjudicated")
    out = ["| kind | " + " | ".join(cols) + " |", "|---" * (len(cols) + 1) + "|"]
    for kind in (*_KINDS, "total"):
        c = counts.get(kind)
        if not c or not c.get("items"):
            continue
        out.append(f"| {kind} | " + " | ".join(str(c[k]) for k in cols) + " |")
    return out + [""]


def _open_work(by_state: Mapping) -> list[str]:
    rows = []
    for kind, states in by_state.items():
        terminal = TERMINAL_BY_KIND.get(kind, ())
        for state, n in states.items():
            if state not in terminal:
                rows.append(f"| {kind} | {state} | {n} |")
    if not rows:
        return ["No open items.", ""]
    return ["| kind | state | items |", "|---|---|---|", *rows, ""]


def _attempts_table(attempts: Iterable) -> list[str]:
    cells: dict[tuple[str, str], dict] = defaultdict(
        lambda: {"n": 0, "outcomes": Counter(), "cost": 0.0, "unpriced": 0})
    for a in attempts:
        c = cells[(a.stage, a.family)]
        c["n"] += 1
        c["outcomes"][a.outcome] += 1
        if a.usage.cost_usd is None:
            c["unpriced"] += 1
        else:
            c["cost"] += a.usage.cost_usd
    if not cells:
        return ["No worker attempts in this run.", ""]
    out = ["| stage | family | attempts | accepted | other outcomes | cost USD | unpriced |",
           "|---|---|---|---|---|---|---|"]
    total_n = total_cost = total_unpriced = 0
    for (stage, family), c in sorted(cells.items()):
        other = ", ".join(f"{k} {v}" for k, v in sorted(c["outcomes"].items()) if k != "accepted") or "-"
        out.append(f"| {stage} | {family} | {c['n']} | {c['outcomes'].get('accepted', 0)} | {other} | "
                   f"{c['cost']:.4f} | {c['unpriced']} |")
        total_n += c["n"]
        total_cost += c["cost"]
        total_unpriced += c["unpriced"]
    out.append(f"| **total** | | {total_n} | | | {total_cost:.4f} | {total_unpriced} |")
    return out + [""]


def _grouped(reasons: Mapping[str, str], title: str) -> list[str]:
    if not reasons:
        return []
    groups: dict[str, list[str]] = defaultdict(list)
    for item, why in reasons.items():
        groups[why].append(item)
    out = [f"## {title}", ""]
    for why, items in sorted(groups.items(), key=lambda kv: (-len(kv[1]), kv[0])):
        examples = ", ".join(f"`{i}`" for i in items[:_EXAMPLES])
        more = f" (+{len(items) - _EXAMPLES} more)" if len(items) > _EXAMPLES else ""
        out.append(f"- {len(items)} x {why}: {examples}{more}")
    return out + [""]


def render_report(*, run_id: str, counts: Mapping, by_state: Mapping, stop: Mapping,
                  attempts: Iterable = (), escalations: Iterable[tuple[str, str]] = (),
                  blocked: Mapping[str, str] | None = None, failed: Mapping[str, str] | None = None,
                  problems: Iterable[str] = (), missing_components: Mapping[str, str] | None = None,
                  generated_at: str | None = None) -> str:
    attempts = list(attempts)
    lines = [first_line(run_id, counts, stop), "", f"# card-loop run `{run_id}`", ""]
    if generated_at:
        lines += [f"Written {generated_at}.", ""]

    lines += ["## Items", ""] + _items_table(counts)
    lines += ["## Open work by state", ""] + _open_work(by_state)
    lines += ["## Attempts and cost (this run, stage x family)", ""] + _attempts_table(attempts)

    lines += ["## Stop", "", f"- reason: **{stop.get('reason', '?')}**"]
    if stop.get("detail"):
        lines.append(f"- detail: {stop['detail']}")
    budget = stop.get("budget_usd")
    lines.append(f"- spent: {_fmt_usd(stop.get('spent_usd', 0.0))}"
                 + (f" of a {_fmt_usd(budget)} cap" if budget is not None else " (no USD cap)")
                 + (f"; {stop['unpriced_attempts']} attempt(s) unpriced"
                    if stop.get("unpriced_attempts") else ""))
    if stop.get("elapsed_s") is not None:
        cap = stop.get("wall_clock_hours")
        lines.append(f"- wall clock this session: {stop['elapsed_s'] / 3600:.2f} h"
                     + (f" of a {cap} h cap" if cap else " (no cap)"))
    if stop.get("plateau_limit"):
        lines.append(f"- attempts since the last adjudication: {stop.get('plateau', 0)} "
                     f"(plateau at {stop['plateau_limit']})")
    if stop.get("session_attempts") is not None:
        lines.append(f"- worker attempts this session: {stop['session_attempts']}")
    lines.append("")

    escalations = list(escalations)
    lines += ["## Escalations (human queue, not adjudicated)", ""]
    if escalations:
        lines += [f"- `{item}` -- {where}" for item, where in escalations]
    else:
        lines.append("None.")
    lines.append("")

    if missing_components:
        lines += ["## Components not built", ""]
        lines += [f"- {name}: {why}" for name, why in sorted(missing_components.items())]
        lines.append("")
    lines += _grouped(failed or {}, "Items that failed this session (retried on resume)")
    lines += _grouped(blocked or {}, "Waiting items")
    problems = list(problems)
    if problems:
        lines += ["## Problems", ""] + [f"- {p}" for p in problems] + [""]
    return "\n".join(lines)


def write_report(path: str | os.PathLike, text: str) -> Path:
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    with open(p, "w", encoding="utf-8", newline="\n") as f:
        f.write(text if text.endswith("\n") else text + "\n")
    return p
