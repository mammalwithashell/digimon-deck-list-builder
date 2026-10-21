#!/usr/bin/env python3
"""Renumber-aware diff of two Comprehensive Rules Manual text extractions.

Feed it two ``pdftotext -raw`` outputs (e.g. ``docs/digimon-rules/cr-4.3.txt`` and the
next revision's extraction). It parses ``N-N-N. text`` units from each, pairs units by
normalised text (exact match first, then a fuzzy match within the same chapter), and
reports **added / removed / reworded / renumbered-only** rules plus a ``renumber_map.json``
(old number -> new number) that downstream scripts can apply to citations.

This produced ``docs/digimon-rules/changelog-3.6-to-4.3.md`` (2026-10-15). Thresholds:
a pair is "reworded" at similarity >= 0.72 (or >= 0.5 when the number did not change);
anything weaker is reported as removed + added, which is the safe default for a human
to adjudicate. The engine-relevance column of the changelog is written by hand.

Usage::

    python code/tools/rules/diff_rules_versions.py OLD.txt NEW.txt OUT_DIR
    # -> OUT_DIR/diff_report.json, renumber_map.json, old_units.json, new_units.json

Standard library only.
"""
from __future__ import annotations

import difflib
import json
import re
import sys
from pathlib import Path

UNIT_RE = re.compile(r"^(\d+(?:-\d+)+)\.\s*(.*)$")
TOP_RE = re.compile(r"^(\d{1,2})\.\s+[A-Z]")  # "15. Effect Rules" chapter headings


def parse_units(path: Path) -> dict[str, str]:
    """rule-number -> full text (joined continuation lines, page numbers stripped)."""
    text = path.read_text(encoding="utf-8", errors="replace")
    # Stop at Update History (its lines look like rule refs but are a changelog).
    body = text.split("\nUpdate History\n")
    # The TOC also contains "Update History ....44" – the split above only matches
    # the standalone heading line, so body[0] is the manual, body[1:] the history.
    if len(body) > 1:
        text = body[0]
    units: dict[str, list[str]] = {}
    cur: str | None = None
    for raw in text.splitlines():
        line = raw.rstrip("\f").strip()
        if not line:
            continue
        m = UNIT_RE.match(line)
        if m:
            num, rest = m.group(1), m.group(2)
            if num in units:
                # Duplicate heading (e.g. a continuation that happens to look like a
                # rule); treat as continuation of the current unit.
                if cur is not None:
                    units[cur].append(line)
                continue
            cur = num
            units[num] = [rest]
            continue
        if TOP_RE.match(line) and len(line) < 60:
            cur = None  # chapter heading, not a unit
            continue
        if cur is not None:
            units[cur].append(line)
    out: dict[str, str] = {}
    for k, parts in units.items():
        s = " ".join(parts)
        # pdftotext -raw glues the page number onto the last word of the page:
        # "...can't be27 executed" -> drop a trailing 1-2 digit run stuck to a letter.
        s = re.sub(r"(?<=[a-z\.,;:\)\"'])\d{1,2}(?=\s|$)", "", s)
        out[k] = re.sub(r"\s+", " ", s).strip()
    return out


def norm(s: str) -> str:
    s = s.lower()
    s = re.sub(r"\(\s*(see|refer to|for details)[^)]*\)", "", s)
    s = re.sub(r"\b\d+(-\d+)+\b", "#", s)  # cross-refs renumber too
    s = re.sub(r"[^a-z0-9#<>\[\] ]+", " ", s)
    s = re.sub(r"\s+", " ", s).strip()
    return s


def section_key(num: str) -> str:
    return num.split("-")[0]


def main(old_path: str, new_path: str, out_dir: str) -> None:
    old = parse_units(Path(old_path))
    new = parse_units(Path(new_path))
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)

    old_n = {k: norm(v) for k, v in old.items()}
    new_n = {k: norm(v) for k, v in new.items()}

    # Pass 1: same number, same text -> unchanged.
    pairs: dict[str, str] = {}  # old -> new
    unchanged = []
    for k in old:
        if k in new and old_n[k] == new_n[k]:
            pairs[k] = k
            unchanged.append(k)
    used_new = set(pairs.values())

    # Pass 2: exact normalised-text match elsewhere -> renumbered.
    by_text: dict[str, list[str]] = {}
    for k, t in new_n.items():
        if k not in used_new:
            by_text.setdefault(t, []).append(k)
    renumbered = []
    for k in old:
        if k in pairs:
            continue
        cands = [c for c in by_text.get(old_n[k], []) if c not in used_new]
        if cands:
            # prefer the same chapter
            cands.sort(key=lambda c: (section_key(c) != section_key(k), c))
            pairs[k] = cands[0]
            used_new.add(cands[0])
            renumbered.append((k, cands[0]))

    # Pass 3: fuzzy match within the same chapter (reworded, maybe renumbered).
    reworded = []
    remaining_old = [k for k in old if k not in pairs]
    remaining_new = [k for k in new if k not in used_new]
    for k in remaining_old:
        best, best_r = None, 0.0
        for c in remaining_new:
            if c in used_new:
                continue
            if section_key(c) != section_key(k):
                continue
            r = difflib.SequenceMatcher(None, old_n[k], new_n[c]).ratio()
            if r > best_r:
                best, best_r = c, r
        # Same number + decent similarity, or very high similarity anywhere in chapter.
        if best is not None and (best_r >= 0.72 or (best == k and best_r >= 0.5)):
            pairs[k] = best
            used_new.add(best)
            reworded.append((k, best, round(best_r, 3)))

    removed = [k for k in old if k not in pairs]
    added = [k for k in new if k not in used_new]

    def sort_key(n: str):
        return tuple(int(p) for p in n.split("-"))

    report = {
        "old_units": len(old),
        "new_units": len(new),
        "unchanged": len(unchanged),
        "renumbered_only": sorted(renumbered, key=lambda p: sort_key(p[0])),
        "reworded": sorted(reworded, key=lambda p: sort_key(p[0])),
        "removed": sorted(removed, key=sort_key),
        "added": sorted(added, key=sort_key),
    }
    renumber_map = {k: v for k, v in pairs.items() if k != v}
    (out / "diff_report.json").write_text(json.dumps(report, indent=1), encoding="utf-8")
    (out / "renumber_map.json").write_text(
        json.dumps(dict(sorted(renumber_map.items(), key=lambda kv: sort_key(kv[0]))), indent=1),
        encoding="utf-8",
    )
    (out / "old_units.json").write_text(json.dumps(old, indent=1), encoding="utf-8")
    (out / "new_units.json").write_text(json.dumps(new, indent=1), encoding="utf-8")

    print(f"old units: {len(old)}  new units: {len(new)}")
    print(f"unchanged: {len(unchanged)}  renumbered-only: {len(renumbered)}  "
          f"reworded: {len(reworded)}  removed: {len(removed)}  added: {len(added)}")


if __name__ == "__main__":
    main(*sys.argv[1:4])
