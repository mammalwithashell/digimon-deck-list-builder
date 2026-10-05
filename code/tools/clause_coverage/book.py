"""Merge newly extracted cards into the clause-text book (`qa/exam-clause-text.json`).

The book is the exam denominator the harness and MCP read. A card absent from
it is invisible to `exam_plan` / `exam_status`, which used to report zero
outstanding work for it. `add` extracts only the missing cards and appends them
as a new sorted block (cards sorted, clauses sorted by id); existing cards and
clauses are never re-extracted, re-sorted or rewritten, so a merge shows up in
git as additions (plus the trailing commas JSON forces on the previous last
elements and the `source` / `denominator` fields).

The file's own format is preserved: indent 2, insertion key order, non-ASCII
text verbatim, and whichever line ending / trailing-newline convention it
already uses (the committed book is CRLF with no trailing newline).

Usage::

    PYTHONPATH=code python -m tools.clause_coverage.book add \
        --book qa/exam-clause-text.json --card-ids EX10-025 EX8-067
"""
from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

_CODE = Path(__file__).resolve().parents[2]
if str(_CODE) not in sys.path:
    sys.path.insert(0, str(_CODE))

from tools.clause_coverage.extract import run as extract_run  # noqa: E402


def _denominator(cards: list[str], clauses: list[dict]) -> dict:
    """Same shape `extract.build_extract_result` emits, recomputed over the merged book."""
    by_zone = Counter(c.get("zone", "") for c in clauses)
    by_source = Counter(c.get("source", "") for c in clauses)
    image_required = [c["id"] for c in clauses if c.get("source") == "image-required"]
    return {
        "total_clauses": len(clauses),
        "total_cards": len(cards),
        "by_zone": dict(sorted(by_zone.items())),
        "by_source": dict(sorted(by_source.items())),
        "image_required_count": len(image_required),
        "image_required_clause_ids": image_required,
    }


def merge_into_book(book_path: Path, card_ids: list[str], *, extract=extract_run) -> list[str]:
    if book_path.exists():
        raw = book_path.read_bytes().decode("utf-8")
        data = json.loads(raw)
        newline = "\r\n" if "\r\n" in raw else "\n"
        trailing = raw.endswith("\n")
    else:
        data, newline, trailing = {"cards": [], "clauses": []}, "\n", True
    known = set(data.get("cards", []))
    known |= {c["id"].split("#", 1)[0] for c in data.get("clauses", [])}
    missing = sorted({cid for cid in card_ids if cid not in known})
    if not missing:
        return []
    fresh = extract(missing, f"book add ({len(missing)} ids)")
    existing_ids = {c["id"] for c in data.get("clauses", [])}
    new_clauses = sorted(
        (c for c in fresh.get("clauses", []) if c["id"] not in existing_ids),
        key=lambda c: c["id"],
    )
    data["cards"] = list(data.get("cards", [])) + missing
    data["clauses"] = list(data.get("clauses", [])) + new_clauses
    data["source"] = f"{data.get('source', '')} + book add {', '.join(missing)}".strip(" +")
    data["denominator"] = _denominator(data["cards"], data["clauses"])
    text = json.dumps(data, indent=2, ensure_ascii=False)
    if newline != "\n":
        text = text.replace("\n", newline)
    if trailing:
        text += newline
    book_path.write_bytes(text.encode("utf-8"))
    return missing


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    add = sub.add_parser("add")
    add.add_argument("--book", type=Path, default=Path("qa/exam-clause-text.json"))
    add.add_argument("--card-ids", nargs="+", required=True)
    args = parser.parse_args(argv)
    added = merge_into_book(args.book, args.card_ids)
    print(json.dumps({"added": added}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
