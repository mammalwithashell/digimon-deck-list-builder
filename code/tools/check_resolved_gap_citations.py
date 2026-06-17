#!/usr/bin/env python3
"""Flag card YAML headers that cite an ALREADY-RESOLVED gap as still blocking.

Card headers under `code/digimon-engine/cards/` document why a clause is omitted
or how it lowers, often citing gap IDs (`G-...`). When a gap closes, those
"BLOCKED / pending on G-X" notes go stale — an author trusting them believes a
working capability is still missing and may route around it (re-introducing the
escape hatches this whole effort removes). This check surfaces such stale
citations. See `dsl-substrate-integrity` (fix-dsl-substrate-rot-and-bugs §7).

Conservative by design — flags a comment line ONLY when it BOTH:
  * cites a gap ID that is RESOLVED (a `## … [G-X] …` header in qa/resolved-gaps.md), AND
  * carries a staleness marker (pending / blocked / until-closes / workaround /
    not-yet / TODO / stub / escape hatch / raw_rust),
  * and does NOT itself say resolved/closed/fixed/landed/shipped/done.

So legitimate "RESOLVED by G-X" / "resolved-gaps.md" notes are not flagged
(verified: 207 such citations pass, only the genuinely-stale ones flag).
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
RESOLVED_MD = REPO / "qa" / "resolved-gaps.md"
CARDS_DIR = REPO / "code" / "digimon-engine" / "cards"

GAP_RE = re.compile(r"G-[A-Z0-9-]+")
STALE_RE = re.compile(
    r"\b(pending|blocked|block(ed)? on|until .*(clos|resolv)|workaround|not yet"
    r"|awaiting|TODO|stub|escape hatch|raw_rust)\b",
    re.I,
)
RESOLVED_KW_RE = re.compile(
    r"\b(resolved|closed|fixed|landed|shipped|done)\b|\bno (more |)raw_rust\b", re.I
)
# Resolved-keyword lookaround: a "RESOLVED 2026-…" / "no more raw_rust" note often
# sits a line or two from the gap-id citation within the same comment block.
CONTEXT = 3


def resolved_gap_ids() -> set[str]:
    ids: set[str] = set()
    for line in RESOLVED_MD.read_text(encoding="utf-8").splitlines():
        if line.lstrip().startswith("#"):
            ids.update(GAP_RE.findall(line))
    return ids


def stale_citations() -> list[tuple[str, int, str]]:
    resolved = resolved_gap_ids()
    hits: list[tuple[str, int, str]] = []
    for f in sorted(CARDS_DIR.rglob("*.yaml")):
        lines = f.read_text(encoding="utf-8", errors="ignore").splitlines()
        for i, line in enumerate(lines, 1):
            cited = [g for g in GAP_RE.findall(line) if g in resolved]
            if not cited or not STALE_RE.search(line):
                continue
            # Not stale if a resolved-keyword sits in the nearby comment context
            # (RESOLVED dates / "no more raw_rust" notes often sit a line or two
            # from the gap-id citation within the same comment block).
            lo, hi = max(0, i - 1 - CONTEXT), min(len(lines), i + CONTEXT)
            if RESOLVED_KW_RE.search("\n".join(lines[lo:hi])):
                continue
            hits.append((f.relative_to(CARDS_DIR).as_posix(), i, line.strip()))
    return hits


def main() -> int:
    hits = stale_citations()
    if not hits:
        print("No card headers cite a resolved gap as still-blocking.")
        return 0
    print(f"Found {len(hits)} card header(s) citing a RESOLVED gap as still-blocking:", file=sys.stderr)
    for path, lineno, line in hits:
        print(f"  {path}:{lineno}  {line[:160]}", file=sys.stderr)
    print(
        "\nThe cited gap(s) are resolved (see qa/resolved-gaps.md) — the capability "
        "exists. Update the header to reflect the real status (implemented, or "
        "omitted/authorable — NOT 'blocked by <gap>').",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
