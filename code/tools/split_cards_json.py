"""CLI: split data/cards.json into per-card JSON files beside Rust DSL YAML.

Usage:
  python code/tools/split_cards_json.py
  python code/tools/split_cards_json.py --card BT24-001
  python code/tools/split_cards_json.py --set bt24
  python code/tools/split_cards_json.py --check

Tree layout: code/digimon-engine/cards/<set_lower>/<card_id>.json.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Mapping

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
CARDS_JSON_PATH = _PROJECT_ROOT / "data" / "cards.json"
CARD_OUTPUT_ROOT = _PROJECT_ROOT / "code" / "digimon-engine" / "cards"


def set_id_from_card_id(card_id: str) -> str:
    """Bucket a card_id into a set-level subdirectory."""
    if "-" not in card_id:
        return "_misc"
    return card_id.split("-", 1)[0].lower()


def render_card_json(card: Mapping) -> str:
    """Render one card record with stable formatting and a trailing newline."""
    return json.dumps(card, ensure_ascii=False, indent=2) + "\n"


def write_card_json(card_id: str, card: Mapping, root: Path) -> Path:
    """Write one card JSON file under <root>/<set>/<card_id>.json."""
    set_dir = root / set_id_from_card_id(card_id)
    set_dir.mkdir(parents=True, exist_ok=True)
    out = set_dir / f"{card_id}.json"
    out.write_text(render_card_json(card), encoding="utf-8", newline="\n")
    return out


def _load_cards_json(path: Path = CARDS_JSON_PATH) -> dict[str, dict]:
    return json.loads(path.read_text(encoding="utf-8"))


def _all_card_ids(cards: Mapping[str, Mapping]) -> list[str]:
    return sorted(cards)


def _filter_by_set(card_ids: list[str], set_id: str) -> list[str]:
    target = set_id.lower()
    return [card_id for card_id in card_ids if set_id_from_card_id(card_id) == target]


def _select_card_ids(
    cards: Mapping[str, Mapping],
    *,
    card_id: str | None = None,
    set_id: str | None = None,
) -> tuple[int, list[str]]:
    if card_id:
        if card_id not in cards:
            print(f"unknown card {card_id!r}", file=sys.stderr)
            return 2, []
        return 0, [card_id]
    if set_id:
        ids = _filter_by_set(_all_card_ids(cards), set_id)
        if not ids:
            print(f"no cards matched set {set_id!r}", file=sys.stderr)
            return 2, []
        return 0, ids
    return 0, _all_card_ids(cards)


def cmd_build(
    cards: Mapping[str, Mapping],
    root: Path,
    *,
    card_id: str | None = None,
    set_id: str | None = None,
) -> int:
    rc, ids = _select_card_ids(cards, card_id=card_id, set_id=set_id)
    if rc != 0:
        return rc
    for cid in ids:
        write_card_json(cid, cards[cid], root)
    print(f"wrote {len(ids)} card JSON files to {root}")
    return 0


def cmd_check(
    cards: Mapping[str, Mapping],
    root: Path,
    *,
    card_id: str | None = None,
    set_id: str | None = None,
) -> int:
    rc, ids = _select_card_ids(cards, card_id=card_id, set_id=set_id)
    if rc != 0:
        return rc

    failures: list[str] = []
    for cid in ids:
        on_disk = root / set_id_from_card_id(cid) / f"{cid}.json"
        expected = render_card_json(cards[cid])
        if not on_disk.exists():
            failures.append(f"{cid}: missing on disk at {on_disk}")
            continue
        actual = on_disk.read_text(encoding="utf-8")
        if actual != expected:
            failures.append(f"{cid}: file differs from generator output")

    if failures:
        for failure in failures:
            print(failure, file=sys.stderr)
        print(
            f"--check failed for {len(failures)}/{len(ids)} cards. "
            "Run `python code/tools/split_cards_json.py` and commit the diff.",
            file=sys.stderr,
        )
        return 1

    print(f"--check OK ({len(ids)} card JSON files match on disk)")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="split_cards_json")
    parser.add_argument("--card", help="write or check only this card_id")
    parser.add_argument("--set", help="write or check only cards in this set")
    parser.add_argument("--check", action="store_true", help="diff generated output vs disk")
    args = parser.parse_args(argv)

    cards = _load_cards_json()
    if args.check:
        return cmd_check(cards, CARD_OUTPUT_ROOT, card_id=args.card, set_id=args.set)
    return cmd_build(cards, CARD_OUTPUT_ROOT, card_id=args.card, set_id=args.set)


if __name__ == "__main__":
    raise SystemExit(main())
