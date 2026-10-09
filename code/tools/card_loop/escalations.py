"""The human escalation queue (design D5, task 6.4): `qa/card-loop/escalations/`.

An item the loop could not adjudicate on its own -- the two families disagreed
on a terminating call, a call had no citation, only one family was available,
an attempt cap or the budget ran out -- is written here as one Markdown file
per item, `<item slug>.md`, and the item is `ESCALATED`: terminal for the run
but NOT adjudicated (it blocks readiness until a human resolves it).

File shape: a front-matter block whose `item:` line is the authoritative item
id (the slug is only a filename), then the reason, both families' arguments
(call, citation, reasoning), the attempt history and how to resolve it.

`index.jsonl` beside the files gets one append-only row per escalation (the
queue's history; files are deleted when resolved, rows are not). An open
escalation is a file that still exists: `load_escalated_items` reads the
directory, so `resume` sees a human's resolution as soon as the file is gone.
"""
from __future__ import annotations

import hashlib
import json
import os
import re
from pathlib import Path
from typing import Mapping

from ._jsonl import append_row, resolve_repo_path
from .driver_contracts import Escalation

INDEX_NAME = "index.jsonl"
_SLUG_MAX = 120
_FRONT = "---"


def item_slug(item: str) -> str:
    """A filesystem-safe filename stem for an item id (Windows forbids `:`).

    Distinct ids map to distinct slugs: the substitution can merge ids that
    differ only in punctuation (`#` vs `:`), so a short hash of the exact id
    is appended whenever anything was substituted or truncated.
    """
    slug = re.sub(r"[^A-Za-z0-9._-]+", "_", item).strip("_.") or "item"
    if slug != item or len(slug) > _SLUG_MAX:
        digest = hashlib.sha256(item.encode("utf-8")).hexdigest()[:8]
        slug = f"{slug[:_SLUG_MAX - 9]}-{digest}"
    return slug


def escalations_dir(path: str | os.PathLike) -> Path:
    return resolve_repo_path(path)


def escalation_path(directory: str | os.PathLike, item: str) -> Path:
    return escalations_dir(directory) / f"{item_slug(item)}.md"


def _one_line(value: object) -> str:
    return " ".join(str(value).split())


def _quote(text: str) -> list[str]:
    lines = str(text).replace("\r\n", "\n").replace("\r", "\n").strip().split("\n")
    return [f"> {line}" if line else ">" for line in lines]


def _argument_block(n: int, arg: Mapping) -> list[str]:
    family = arg.get("family") or "unknown family"
    call = arg.get("call") or "no call"
    attempt = arg.get("attempt_id")
    head = f"### {n}. {family}: `{call}`" + (f" (attempt `{attempt}`)" if attempt else "")
    out = [head, ""]
    citation = arg.get("citation")
    if citation:
        out.append(f"- Citation: {_one_line(citation)}")
    else:
        out.append("- Citation: **no citation given** (a terminating call needs one, design D5)")
    reasoning = arg.get("reasoning")
    out.append("- Reasoning:")
    out += _quote(reasoning) if reasoning else ["> (none recorded)"]
    out.append("")
    return out


def _history_table(history, attempts: Mapping | None) -> list[str]:
    if not history:
        return ["No attempts were recorded for this item in the run.", ""]
    out = ["| # | attempt | stage | family | model | outcome | cost USD |",
           "|---|---|---|---|---|---|---|"]
    for n, aid in enumerate(history, start=1):
        a = (attempts or {}).get(aid)
        if a is None:
            out.append(f"| {n} | `{aid}` | | | | | |")
            continue
        cost = a.usage.cost_usd
        out.append(f"| {n} | `{aid}` | {a.stage} | {a.family} | {a.model or '(default)'} | "
                   f"{a.outcome} | {'unpriced' if cost is None else f'{cost:.4f}'} |")
    out.append("")
    return out


def _how_to_resolve(item: str) -> list[str]:
    kind = item.split(":", 1)[0]
    lines = [
        "## How to resolve",
        "",
        "1. Check each argument's citation against the sources in priority order: "
        "`general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the "
        "official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "
        "\"Source priority\"). A call with no citation carries no weight.",
    ]
    if kind == "card":
        lines.append(
            "2. Implement or fix the card by hand under the fix gate (citation, a test that "
            "fails before and passes after, `cards_behavioral` green), or close the gap it "
            "parked on and let the loop retry it.")
    else:
        lines.append(
            "2. Record the decision on the verdict row: our engine right / DCGO differs -> "
            "`cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage "
            "dcgo_quirk --citation \"<source>\"`; our engine wrong -> `--triage ours_wrong` "
            "with the citation, then fix it under the fix gate (or let the loop's fix stage "
            "take it on the next run).")
    lines += [
        "3. Record which family's call was wrong as a late correction "
        "(`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard "
        "penalises it.",
        "4. Delete this file. The next `python -m tools.card_loop resume` re-reads the "
        "committed ledgers; the row in `index.jsonl` stays as history.",
        "",
    ]
    return lines


def render_escalation(esc: Escalation, *, run_id: str, ts: str, attempts: Mapping | None = None,
                      state_before: str | None = None, item_data: Mapping | None = None) -> str:
    front = [_FRONT, f"item: {esc.item}", f"run_id: {run_id}", f"escalated_at: {ts}"]
    if state_before:
        front.append(f"state_before: {state_before}")
    front += [f"reason: {_one_line(esc.reason)}", _FRONT, ""]

    body = [
        f"# Escalated: `{esc.item}`",
        "",
        f"**Why:** {esc.reason}",
        "",
        "This item is **not adjudicated**: it is terminal for the run but blocks readiness "
        "until a human resolves it (design D4/D5).",
        "",
        "## Arguments",
        "",
    ]
    if esc.arguments:
        for n, arg in enumerate(esc.arguments, start=1):
            body += _argument_block(n, arg)
    else:
        body += ["No model arguments were recorded: the driver escalated this item itself "
                 "(attempt cap, budget, or a missing second family).", ""]
    body += ["## Attempt history (oldest first)", ""]
    body += _history_table(esc.history, attempts)
    if item_data:
        body += ["## Item data", "", "```json",
                 json.dumps(dict(item_data), indent=2, sort_keys=True, default=str), "```", ""]
    body += _how_to_resolve(esc.item)
    return "\n".join(front + body)


def write_escalation(directory: str | os.PathLike, esc: Escalation, *, run_id: str, ts: str,
                     attempts: Mapping | None = None, state_before: str | None = None,
                     item_data: Mapping | None = None) -> Path:
    """Write `<slug>.md` (replacing an earlier open file for the same item) and
    append one `index.jsonl` row. Returns the file path."""
    d = escalations_dir(directory)
    d.mkdir(parents=True, exist_ok=True)
    path = escalation_path(d, esc.item)
    text = render_escalation(esc, run_id=run_id, ts=ts, attempts=attempts,
                             state_before=state_before, item_data=item_data)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    append_row(d / INDEX_NAME, {
        "ts": ts, "run_id": run_id, "item": esc.item, "reason": esc.reason,
        "families": [a.get("family") for a in esc.arguments if a.get("family")],
        "history": list(esc.history), "path": path.name,
    })
    return path


def read_front_matter(path: str | os.PathLike) -> dict:
    """The `key: value` front-matter of an escalation file ({} when absent)."""
    try:
        text = Path(path).read_text(encoding="utf-8-sig")
    except OSError:
        return {}
    lines = text.splitlines()
    if not lines or lines[0].strip() != _FRONT:
        return {}
    out = {}
    for line in lines[1:]:
        if line.strip() == _FRONT:
            return out
        key, sep, value = line.partition(":")
        if sep:
            out[key.strip()] = value.strip()
    return {}


def load_escalated_items(directory: str | os.PathLike) -> dict[str, Path]:
    """`{item id: file}` for every open escalation (a file that still exists).
    Markdown files without an `item:` front-matter line are not escalations."""
    d = escalations_dir(directory)
    if not d.is_dir():
        return {}
    out: dict[str, Path] = {}
    for p in sorted(d.glob("*.md")):
        item = read_front_matter(p).get("item")
        if item:
            out[item] = p
    return out
