"""Build the tested-cards allowlist snapshot for the alpha release.

Scans ``tests/behavioral/**/test_*.py`` for per-card behavioral test files
and writes the canonical card IDs to
``digimon_gym/engine/data/tested_cards.json``.

The snapshot is committed to the repo and bundled into the desktop sidecar
by ``desktop.spec`` (which globs ``digimon_gym/engine/data/*.json``). The
alpha deck builder uses it to restrict players to cards that have been
tested end-to-end.

Run from the repo root:

    python tools/build_tested_cards.py

Exits non-zero if any test filename cannot be resolved to a real card ID,
or if the generated snapshot differs from the committed file (use
``--write`` to update it).
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
BEHAVIORAL_DIR = REPO_ROOT / "tests" / "behavioral"
CARDS_JSON = REPO_ROOT / "digimon_gym" / "engine" / "data" / "cards.json"
OUTPUT_JSON = REPO_ROOT / "digimon_gym" / "engine" / "data" / "tested_cards.json"

TEST_FILE_RE = re.compile(r"^test_([a-z]+\d*)_(\d+)\.py$")


def _load_cards_by_prefix() -> dict[str, dict[int, str]]:
    """Return ``{PREFIX: {numeric_suffix: canonical_card_id}}``.

    Card IDs use varying zero-padding across sets (``ST1-01`` vs
    ``BT22-002`` vs ``LM-001``), so we key by the integer value of the
    suffix and let the caller look up by ``int(num)``.
    """
    with CARDS_JSON.open() as f:
        cards: dict[str, dict] = json.load(f)

    by_prefix: dict[str, dict[int, str]] = defaultdict(dict)
    for card_id in cards:
        if "-" not in card_id:
            continue
        prefix, suffix = card_id.split("-", 1)
        try:
            num = int(suffix)
        except ValueError:
            continue
        by_prefix[prefix.upper()][num] = card_id
    return by_prefix


def collect_tested_card_ids() -> tuple[list[str], list[str]]:
    """Scan behavioral test files and resolve to canonical card IDs.

    Returns ``(sorted_card_ids, warnings)``. Warnings contains filenames
    that could not be resolved (e.g. generic helper tests or stale files).
    """
    cards_by_prefix = _load_cards_by_prefix()

    found: set[str] = set()
    warnings: list[str] = []

    for test_file in sorted(BEHAVIORAL_DIR.rglob("test_*.py")):
        match = TEST_FILE_RE.match(test_file.name)
        if not match:
            # Tests like ``test_helpers.py`` or ``test_smoke.py`` are fine.
            continue
        prefix = match.group(1).upper()
        try:
            num = int(match.group(2))
        except ValueError:
            warnings.append(f"Non-numeric suffix: {test_file.relative_to(REPO_ROOT)}")
            continue

        set_cards = cards_by_prefix.get(prefix)
        if not set_cards:
            warnings.append(
                f"Unknown set '{prefix}' from {test_file.relative_to(REPO_ROOT)}"
            )
            continue

        canonical = set_cards.get(num)
        if canonical is None:
            warnings.append(
                f"No card {prefix}-{num} in cards.json "
                f"(from {test_file.relative_to(REPO_ROOT)})"
            )
            continue

        found.add(canonical)

    return sorted(found), warnings


def build_snapshot(card_ids: list[str]) -> dict:
    return {
        "version": 1,
        "generated_at": dt.datetime.utcnow().isoformat(timespec="seconds") + "Z",
        "card_count": len(card_ids),
        "card_ids": card_ids,
    }


def _normalize_for_diff(payload: dict) -> dict:
    """Strip the timestamp so diffs only care about card content."""
    return {k: v for k, v in payload.items() if k != "generated_at"}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="Fail if the generated snapshot differs from the committed file.",
    )
    args = parser.parse_args()

    if not BEHAVIORAL_DIR.exists():
        print(f"error: {BEHAVIORAL_DIR} does not exist", file=sys.stderr)
        return 2
    if not CARDS_JSON.exists():
        print(f"error: {CARDS_JSON} does not exist", file=sys.stderr)
        return 2

    card_ids, warnings = collect_tested_card_ids()

    for w in warnings:
        print(f"warning: {w}", file=sys.stderr)

    if not card_ids:
        print("error: no tested cards found", file=sys.stderr)
        return 2

    new_payload = build_snapshot(card_ids)

    if args.check:
        if not OUTPUT_JSON.exists():
            print(
                f"error: {OUTPUT_JSON} is missing; run without --check to create it",
                file=sys.stderr,
            )
            return 1
        existing = json.loads(OUTPUT_JSON.read_text())
        if _normalize_for_diff(existing) != _normalize_for_diff(new_payload):
            print(
                f"error: {OUTPUT_JSON} is stale; run "
                "`python tools/build_tested_cards.py` to refresh",
                file=sys.stderr,
            )
            return 1
        print(f"ok: {len(card_ids)} tested cards (snapshot up to date)")
        return 0

    # Idempotent write: only touch the file if content changed.
    if OUTPUT_JSON.exists():
        existing = json.loads(OUTPUT_JSON.read_text())
        if _normalize_for_diff(existing) == _normalize_for_diff(new_payload):
            print(f"ok: {len(card_ids)} tested cards (no changes)")
            return 0

    OUTPUT_JSON.write_text(json.dumps(new_payload, indent=2) + "\n")
    print(
        f"wrote {OUTPUT_JSON.relative_to(REPO_ROOT)} "
        f"({len(card_ids)} tested cards)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
