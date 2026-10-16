"""Parse the Comprehensive Rules Manual text extraction into numbered rule units.

Input is a ``pdftotext -raw`` extraction (``qa/rules/aegis/comprehensive-4.3.md``).
``normalise``/``fingerprint`` define the stable identity of a rule unit across
re-extractions; the Rust test macro reproduces them byte-for-byte, so the step
order here is binding.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

RULE_RE = re.compile(r"^((?:1[0-8]|[1-9])(?:-\d+)+)\.\s*(.*)$")
START_PREFIX = "1-1."
STOP_PREFIX = "Update History"

_WS = re.compile(r"\s+")
_PAGE_NUM = re.compile(r"([a-z.)])\d{1,2}(\s|$)")
# "(For details, refer to ...)" plus any bare rule number glued right after it.
_CROSS_REF = re.compile(r"\(For details, refer to [^)]*\)(?:\d{1,2}(?:-\d+)+)?")
_BARE_RULE_NUM = re.compile(r"\b\d{1,2}(-\d+)+\b")


@dataclass(frozen=True)
class RuleUnit:
    id: str
    chapter: int
    text: str
    text_norm: str
    fingerprint: str


def normalise(text: str) -> str:
    t = _WS.sub(" ", text)
    t = _PAGE_NUM.sub(r"\1\2", t)
    t = t.replace("\u2019", "'").replace("\u201c", '"').replace("\u201d", '"')
    t = t.replace("\u3000", " ")
    t = _CROSS_REF.sub("", t)
    t = _BARE_RULE_NUM.sub("#", t)
    return t.lower().strip()


def fingerprint(text_norm: str) -> str:
    return hashlib.sha256(text_norm.encode("utf-8")).hexdigest()


def parse_rules(path: Path) -> list[RuleUnit]:
    raw_units: list[tuple[str, list[str]]] = []
    started = False
    for raw in Path(path).read_text(encoding="utf-8", errors="replace").splitlines():
        line = raw.strip()
        if not started:
            # The table of contents precedes the body; the first real rule is 1-1.
            if line.startswith(START_PREFIX):
                started = True
            else:
                continue
        elif line.startswith(STOP_PREFIX):
            break
        m = RULE_RE.match(line)
        if m:
            raw_units.append((m.group(1), [m.group(2)]))
        elif raw_units:
            raw_units[-1][1].append(line)

    units = []
    for rule_id, parts in raw_units:
        text = " ".join(parts).strip()
        text_norm = normalise(text)
        units.append(
            RuleUnit(
                id=rule_id,
                chapter=int(rule_id.split("-", 1)[0]),
                text=text,
                text_norm=text_norm,
                fingerprint=fingerprint(text_norm),
            )
        )
    return units


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("path", type=Path)
    ap.add_argument("--json", action="store_true", help="print units as JSON")
    args = ap.parse_args(argv)
    units = parse_rules(args.path)
    if args.json:
        sys.stdout.reconfigure(encoding="utf-8")
        print(json.dumps([asdict(u) for u in units], ensure_ascii=False, indent=2))
    else:
        print(f"{len(units)} rule units")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
