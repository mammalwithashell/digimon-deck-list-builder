"""Build the Comprehensive Rules obligation denominator.

Every numbered rule unit of the Comprehensive Rules (``parse_rules``) is one
*obligation*. ``qa/rules/cr_obligations.json`` is the generated denominator the
Rust ``cite!`` macro embeds with ``include_str!`` and the coverage report
reads. Statuses are hand-maintained in the sidecar
``qa/rules/cr_obligations_status.yaml``; an id absent from the sidecar is a
``gap`` (``blocks_training: false``), except heading-only units, which are
seeded ``not-testable``.

    python -m tools.rules.build_obligations [--source PATH] [--check]

``--check`` exits 1 if the committed JSON differs from a fresh build (same
contract as ``code/tools/build_tested_cards.py``).
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

import yaml

from tools.rules.parse_rules import parse_rules

REPO_ROOT = Path(__file__).resolve().parents[3]
DEFAULT_SOURCE = REPO_ROOT / "qa/rules/aegis/comprehensive-4.3.md"
DEFAULT_MECHANISMS = REPO_ROOT / "qa/rules/aegis/mechanisms-by-rule.json"
DEFAULT_SIDECAR = REPO_ROOT / "qa/rules/cr_obligations_status.yaml"
DEFAULT_OUTPUT = REPO_ROOT / "qa/rules/cr_obligations.json"

RULES_VERSION = "4.3"
VERSION = 1
STATUSES = ("gap", "proven", "not-testable")
MECHANICS_SET = [
    "effects-and-ordering",
    "keywords",
    "attack-and-timing",
    "security",
    "battle",
    "blocking",
    "rule-checks",
    "turn-flow",
    "digivolution-and-costs",
    "play-and-costs",
    "card-use-and-costs",
    "link",
]

_KEYWORD_ICON = re.compile(r"^<[^<>]+>$")
# Footnote markers glued after a sentence terminator ("...level.6").
_TRAILING_FOOTNOTE = re.compile(r"(?<=[.)])\d{1,2}$")
_TERMINATORS = (".", ")", ":")
_MAX_HEADING_WORDS = 6
# Rule-check enumerations: the enumerated leaves under 17-1-3 (ids 17-1-3-<n>-<m>,
# depth >= 5) are normative targets even when short and unterminated, e.g.
# 17-1-3-2-4 "Face-down cards placed on the field", so the heading heuristic never
# seeds them `not-testable`. The depth-4 sub-headings 17-1-3-1 "Deletion Targets"
# and 17-1-3-2 "Trash Targets" are real headings and are still seeded.
_NORMATIVE_LEAF = re.compile(r"^17-1-3-\d+-\d+")


def is_heading(text: str, rule_id: str | None = None) -> bool:
    """Heading-only unit: no sentence terminator and at most 6 words, or a bare keyword icon.

    Enumerated rule-check leaves (``_NORMATIVE_LEAF``) are never headings.
    """
    if rule_id is not None and _NORMATIVE_LEAF.match(rule_id):
        return False
    t = _TRAILING_FOOTNOTE.sub("", text.strip())
    if _KEYWORD_ICON.match(t):
        return True
    return not t.endswith(_TERMINATORS) and len(t.split()) <= _MAX_HEADING_WORDS


def _rel(path: Path) -> str:
    p = Path(path).resolve()
    try:
        return p.relative_to(REPO_ROOT).as_posix()
    except ValueError:
        return p.as_posix()


def _sha256(path: Path) -> str:
    # CRLF -> LF so the digest is stable across autocrlf checkouts.
    data = Path(path).read_bytes().replace(b"\r\n", b"\n")
    return hashlib.sha256(data).hexdigest()


def build(source: Path, sidecar: dict, mechanisms: dict) -> dict:
    units = parse_rules(Path(source))
    ids = {u.id for u in units}
    unknown = sorted(set(sidecar) - ids)
    if unknown:
        raise ValueError(f"sidecar has ids not in the rules source: {unknown[:10]}")
    bad = sorted(k for k, v in sidecar.items() if v is not None and not isinstance(v, dict))
    if bad:
        raise ValueError(f"sidecar entries must be mappings (status/tests/...): {bad[:10]}")
    stale = sorted(set(mechanisms) - ids)
    if stale:
        raise ValueError(f"mechanisms map has ids not in the rules source: {stale[:10]}")

    mech_set = set(MECHANICS_SET)
    obligations: dict[str, dict] = {}
    for u in units:
        entry = sidecar.get(u.id) or {}
        status = entry.get("status") or ("not-testable" if is_heading(u.text, u.id) else "gap")
        if status not in STATUSES:
            raise ValueError(f"{u.id}: invalid status {status!r}")
        obligations[u.id] = {
            "chapter": u.chapter,
            "text": u.text,
            "fingerprint": u.fingerprint,
            "mechanisms": list(mechanisms.get(u.id, [])),
            "status": status,
            "blocks_training": bool(entry.get("blocks_training", False)),
            "tests": list(entry.get("tests") or []),
            "gap_id": entry.get("gap_id"),
            "note": entry.get("note"),
        }

    summary: dict = {s: 0 for s in STATUSES}
    by_chapter: dict[str, dict] = {}
    mech = {"proven": 0, "gap_blocking": 0, "gap_nonblocking": 0, "not_testable": 0}
    for o in obligations.values():
        summary[o["status"]] += 1
        ch = by_chapter.setdefault(
            str(o["chapter"]), {"total": 0, "proven": 0, "gap": 0, "not_testable": 0}
        )
        ch["total"] += 1
        ch[o["status"].replace("-", "_")] += 1
        if mech_set & set(o["mechanisms"]):
            if o["status"] == "proven":
                mech["proven"] += 1
            elif o["status"] == "not-testable":
                mech["not_testable"] += 1
            elif o["blocks_training"]:
                mech["gap_blocking"] += 1
            else:
                mech["gap_nonblocking"] += 1
    summary["by_chapter"] = by_chapter
    summary["mechanics"] = mech

    return {
        "version": VERSION,
        "source": {
            "file": _rel(source),
            "rules_version": RULES_VERSION,
            "sha256": _sha256(source),
        },
        "mechanics_set": list(MECHANICS_SET),
        "obligations": obligations,
        "summary": summary,
    }


def serialise(payload: dict) -> str:
    return json.dumps(payload, sort_keys=True, indent=2, ensure_ascii=False) + "\n"


def write(path: Path, payload: dict) -> None:
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(serialise(payload))


def check(committed: Path, source: Path, sidecar: dict, mechanisms: dict) -> int:
    committed = Path(committed)
    if not committed.exists():
        print(f"error: {committed} is missing; run without --check to create it", file=sys.stderr)
        return 1
    fresh = serialise(build(source, sidecar, mechanisms))
    existing = committed.read_bytes().decode("utf-8").replace("\r\n", "\n")
    if existing != fresh:
        print(
            f"error: {committed} is stale; run "
            "`python -m tools.rules.build_obligations` to refresh",
            file=sys.stderr,
        )
        return 1
    return 0


def load_sidecar(path: Path) -> dict:
    p = Path(path)
    if not p.exists():
        return {}
    data = yaml.safe_load(p.read_text(encoding="utf-8")) or {}
    if not isinstance(data, dict):
        raise ValueError(f"{p}: sidecar must be a mapping of rule id -> entry")
    return {str(k): v for k, v in data.items()}


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    ap.add_argument("--sidecar", type=Path, default=DEFAULT_SIDECAR)
    ap.add_argument("--mechanisms", type=Path, default=DEFAULT_MECHANISMS)
    ap.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    ap.add_argument("--check", action="store_true", help="fail if the committed JSON is stale")
    args = ap.parse_args(argv)

    sidecar = load_sidecar(args.sidecar)
    mechanisms = json.loads(args.mechanisms.read_text(encoding="utf-8"))
    if args.check:
        return check(args.output, args.source, sidecar, mechanisms)
    payload = build(args.source, sidecar, mechanisms)
    write(args.output, payload)
    s = payload["summary"]
    print(
        f"{len(payload['obligations'])} obligations: gap={s['gap']} "
        f"proven={s['proven']} not-testable={s['not-testable']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
