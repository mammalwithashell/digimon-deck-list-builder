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
import os
import sys
from collections import Counter
from pathlib import Path

_CODE = Path(__file__).resolve().parents[2]
if str(_CODE) not in sys.path:
    sys.path.insert(0, str(_CODE))

from data_paths import CARD_OVERRIDES, CARDS_JSON  # noqa: E402
from tools.clause_coverage.card_sources import (  # noqa: E402
    load_cards_index,
    load_official_index,
    load_overrides_index,
)
from tools.clause_coverage.extract import DEFAULT_OFFICIAL_JSON  # noqa: E402
from tools.clause_coverage.extract import run as extract_run  # noqa: E402


class UnknownCardIds(ValueError):
    """Requested card ids that no card source knows (typo, wrong case, empty)."""


def known_card_ids() -> set[str]:
    """Every id the extractor can resolve: cards.json, the official mirror, or an override.

    Extraction itself never fails on an unknown id -- it emits a phantom
    `image-required` security clause -- so the book must check before writing.
    """
    ids: set[str] = set(load_cards_index(CARDS_JSON))
    ids |= set(load_official_index(DEFAULT_OFFICIAL_JSON))
    ids |= set(load_overrides_index(CARD_OVERRIDES))
    return ids


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


def _write_atomic(path: Path, payload: bytes) -> None:
    """Sibling temp file + `os.replace`, so a crash never leaves a torn book."""
    tmp = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    try:
        tmp.write_bytes(payload)
        os.replace(tmp, path)
    finally:
        if tmp.exists():
            tmp.unlink()


def merge_into_book(
    book_path: Path,
    card_ids: list[str],
    *,
    extract=extract_run,
    resolve=known_card_ids,
) -> list[str]:
    """Extract and append the cards the book lacks; return the ids added.

    Raises `UnknownCardIds` (writing nothing) if any missing id is not known to
    `resolve()`: an unresolvable id would otherwise be persisted with a phantom
    clause and inflate the denominator forever.
    """
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
    known_cards = resolve()
    unknown = [cid for cid in missing if cid not in known_cards]
    if unknown:
        raise UnknownCardIds(
            f"unknown card id(s) {unknown}: not in cards.json, card_official.json or "
            "card_overrides.json -- check spelling/case; nothing was written to the book"
        )
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
    _write_atomic(book_path, text.encode("utf-8"))
    return missing


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    add = sub.add_parser("add")
    add.add_argument("--book", type=Path, default=Path("qa/exam-clause-text.json"))
    add.add_argument("--card-ids", nargs="+", required=True)
    args = parser.parse_args(argv)
    try:
        added = merge_into_book(args.book, args.card_ids)
    except UnknownCardIds as e:
        print(f"error: {e}", file=sys.stderr)
        return 2
    print(json.dumps({"added": added}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
