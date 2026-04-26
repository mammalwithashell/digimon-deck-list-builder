"""CLI: regenerate per-card metadata `.md` files under data/card_meta/.

Usage:
  python -m tools.build_card_meta                    # rebuild all
  python -m tools.build_card_meta --card BT17-007
  python -m tools.build_card_meta --set bt17
  python -m tools.build_card_meta --check            # CI: rebuild to tempdir, diff
  python -m tools.build_card_meta --coverage-check   # CI: assert no coverage regression

Tree layout: data/card_meta/<set_lower>/<card_id>.md, plus _coverage.md and
_coverage_baseline.json at the root.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from tools.resolve_deck import _load_cards_json_raw, build_card_meta_md  # noqa: E402

CARD_META_ROOT = _PROJECT_ROOT / "data" / "card_meta"


def set_id_from_card_id(card_id: str) -> str:
    """Bucket a card_id into a set-level subdirectory.

    Standard cards (BT17-007, ST2-13, AD1-005) bucket by lowercase prefix.
    Card_ids without a hyphen bucket into `_misc`.
    """
    if "-" not in card_id:
        return "_misc"
    return card_id.split("-", 1)[0].lower()


def write_card_meta(card_id: str, root: Path) -> Path:
    body, _ = build_card_meta_md(card_id)
    set_dir = root / set_id_from_card_id(card_id)
    set_dir.mkdir(parents=True, exist_ok=True)
    out = set_dir / f"{card_id}.md"
    # Force LF on Windows; the file is checked in and must diff stably.
    out.write_text(body, encoding="utf-8", newline="\n")
    return out


def build_one(card_id: str) -> tuple[str, int, int]:
    """Build one card's .md and return (card_id, n_parsed, n_unparsed) without writing."""
    _, parse_result = build_card_meta_md(card_id)
    return card_id, len(parse_result.parsed), len(parse_result.unparsed_lines)


def _all_card_ids() -> list[str]:
    return sorted(_load_cards_json_raw().keys())


def _filter_by_set(card_ids: list[str], set_id: str) -> list[str]:
    target = set_id.lower()
    return [c for c in card_ids if set_id_from_card_id(c) == target]


def cmd_check(card_ids: list[str] | None = None) -> int:
    """Rebuild every card to memory, compare against the on-disk tree.

    Returns 0 if every file matches; 1 if any file is missing or differs.
    Prints diffs to stderr.
    """
    ids = card_ids if card_ids is not None else _all_card_ids()
    failures: list[str] = []
    for cid in ids:
        body, _ = build_card_meta_md(cid)
        on_disk = CARD_META_ROOT / set_id_from_card_id(cid) / f"{cid}.md"
        if not on_disk.exists():
            failures.append(f"{cid}: missing on disk at {on_disk}")
            continue
        actual = on_disk.read_text(encoding="utf-8")
        if actual != body:
            failures.append(f"{cid}: contents differ from generator output")
    if failures:
        for f in failures:
            print(f, file=sys.stderr)
        print(
            f"--check failed for {len(failures)}/{len(ids)} cards. "
            "Run `python -m tools.build_card_meta` and commit the diff.",
            file=sys.stderr,
        )
        return 1
    print(f"--check OK ({len(ids)} cards match on disk)")
    return 0


def cmd_build(args: argparse.Namespace) -> int:
    if args.card:
        ids = [args.card]
    elif args.set:
        ids = _filter_by_set(_all_card_ids(), args.set)
        if not ids:
            print(f"no cards matched set {args.set!r}", file=sys.stderr)
            return 2
    else:
        ids = _all_card_ids()
    CARD_META_ROOT.mkdir(parents=True, exist_ok=True)
    for cid in ids:
        write_card_meta(cid, CARD_META_ROOT)
    print(f"wrote {len(ids)} card meta files to {CARD_META_ROOT}")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="build_card_meta")
    parser.add_argument("--card", help="rebuild only this card_id")
    parser.add_argument("--set", help="rebuild only cards in this set (lowercase prefix)")
    parser.add_argument("--check", action="store_true", help="rebuild to tempdir and diff vs disk")
    parser.add_argument("--coverage-check", action="store_true", help="assert coverage didn't regress")
    args = parser.parse_args(argv)

    if args.check:
        return cmd_check()
    if args.coverage_check:
        print("--coverage-check not implemented yet (see Task 8)", file=sys.stderr)
        return 2
    return cmd_build(args)


if __name__ == "__main__":
    raise SystemExit(main())
