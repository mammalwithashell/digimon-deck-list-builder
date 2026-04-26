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
import datetime
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


COVERAGE_BASELINE_PATH_NAME = "_coverage_baseline.json"
COVERAGE_REPORT_PATH_NAME = "_coverage.md"


def compute_coverage(stats: list[tuple[str, int, int]]) -> dict:
    """Bucket each card. A card with zero unparsed lines is fully parsed
    (including cards with no xros_req at all). One unparsed line + at least
    one parsed line is partially parsed. All-unparsed-no-parsed is wholly
    unparsed.
    """
    fully = partial = wholly = 0
    partial_cards: list[str] = []
    wholly_cards: list[str] = []
    for cid, n_parsed, n_unparsed in stats:
        if n_unparsed == 0:
            fully += 1
        elif n_parsed > 0:
            partial += 1
            partial_cards.append(cid)
        else:
            wholly += 1
            wholly_cards.append(cid)
    return {
        "fully_parsed": fully,
        "partially_parsed": partial,
        "wholly_unparsed": wholly,
        "partial_cards": partial_cards,
        "wholly_cards": wholly_cards,
    }


def write_coverage_report(stats: list[tuple[str, int, int]]) -> Path:
    cov = compute_coverage(stats)
    total = len(stats)
    lines = [
        "# xros_req parser coverage",
        "",
        f"Generated: {datetime.datetime.utcnow().isoformat(timespec='seconds')}Z",
        "",
        f"- Total cards: {total}",
        f"- Fully parsed (incl. no xros_req): {cov['fully_parsed']}",
        f"- Partially parsed: {cov['partially_parsed']}",
        f"- Wholly unparsed (with xros_req): {cov['wholly_unparsed']}",
        "",
        "## Cards with partial xros_req parses",
        "",
    ]
    if cov["partial_cards"]:
        lines += [f"- {c}" for c in sorted(cov["partial_cards"])]
    else:
        lines.append("_(none)_")
    lines += ["", "## Cards with wholly unparsed xros_req", ""]
    if cov["wholly_cards"]:
        lines += [f"- {c}" for c in sorted(cov["wholly_cards"])]
    else:
        lines.append("_(none)_")
    lines.append("")
    out = CARD_META_ROOT / COVERAGE_REPORT_PATH_NAME
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines), encoding="utf-8", newline="\n")
    return out


def _load_baseline() -> dict:
    path = CARD_META_ROOT / COVERAGE_BASELINE_PATH_NAME
    if not path.exists():
        return {"partially_parsed": 0, "wholly_unparsed": 0}
    return json.loads(path.read_text(encoding="utf-8"))


def cmd_coverage_check(stats_override: list[tuple[str, int, int]] | None = None) -> int:
    if stats_override is not None:
        stats = stats_override
    else:
        stats = [build_one(c) for c in _all_card_ids()]
    cov = compute_coverage(stats)
    baseline = _load_baseline()
    regressed = (
        cov["partially_parsed"] > baseline.get("partially_parsed", 0)
        or cov["wholly_unparsed"] > baseline.get("wholly_unparsed", 0)
    )
    if regressed:
        print(
            "coverage regressed: "
            f"partial {cov['partially_parsed']} (baseline {baseline.get('partially_parsed', 0)}), "
            f"wholly {cov['wholly_unparsed']} (baseline {baseline.get('wholly_unparsed', 0)})",
            file=sys.stderr,
        )
        return 1
    print(
        f"coverage OK (partial {cov['partially_parsed']}, wholly {cov['wholly_unparsed']})"
    )
    return 0


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
    stats: list[tuple[str, int, int]] = []
    for cid in ids:
        body, parse_result = build_card_meta_md(cid)
        set_dir = CARD_META_ROOT / set_id_from_card_id(cid)
        set_dir.mkdir(parents=True, exist_ok=True)
        (set_dir / f"{cid}.md").write_text(body, encoding="utf-8", newline="\n")
        stats.append((cid, len(parse_result.parsed), len(parse_result.unparsed_lines)))
    # Only refresh the report + baseline on a full build (--card/--set are partial).
    if not args.card and not args.set:
        write_coverage_report(stats)
        cov = compute_coverage(stats)
        baseline_payload = {
            "partially_parsed": cov["partially_parsed"],
            "wholly_unparsed": cov["wholly_unparsed"],
        }
        (CARD_META_ROOT / COVERAGE_BASELINE_PATH_NAME).write_text(
            json.dumps(baseline_payload, indent=2) + "\n", encoding="utf-8", newline="\n"
        )
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
        return cmd_coverage_check()
    return cmd_build(args)


if __name__ == "__main__":
    raise SystemExit(main())
