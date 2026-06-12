"""Build the implemented-cards allowlist snapshot for the deck builder.

Asks the Rust engine which cards have a registered ``CardEffect`` (DSL
YAML pack + raw-rust escape hatch) via the ``digimon-engine-cli pool``
subcommand, and writes the canonical card IDs to ``data/tested_cards.json``.

Historical note: this tool used to scan the legacy Python behavioral test
tree (``tests/behavioral/**``). That tree was retired with the Rust pivot,
which froze the snapshot at the pre-pivot card set — the engine's
``CardEffectRegistry`` is the source of truth now.

The snapshot is committed to the repo and consumed by:
- the hosted API via ``engine_py_legacy/engine/data/tested_cards.py``
- the desktop app, where ``digimon-engine`` bakes it into the binary via
  ``include_str!`` in ``digimon-engine/src/deck_tools.rs``

The deck builder uses it to restrict players to implemented cards.

Run from the repo root:

    python code/tools/build_tested_cards.py

By default the tool rewrites the snapshot; pass ``--check`` to instead
fail if the generated snapshot differs from the committed file. Pass
``--pool-json <path>`` to skip the cargo invocation and read a pre-dumped
``[card_id, ...]`` array instead.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
if str(REPO_ROOT / "code") not in sys.path:
    sys.path.insert(0, str(REPO_ROOT / "code"))

from data_paths import CARDS_JSON, TESTED_CARDS as OUTPUT_JSON  # noqa: E402


def collect_implemented_card_ids(pool_json: Path | None) -> list[str]:
    """Return the sorted implemented-card pool from the engine registry.

    With ``pool_json`` set, reads a pre-dumped JSON array. Otherwise runs
    ``cargo run -p digimon-engine-cli -- pool``, which prints the registry
    pool intersected with ``cards.json`` (test fixtures and token IDs are
    excluded by that intersection).
    """
    if pool_json is not None:
        ids = json.loads(pool_json.read_text())
    else:
        result = subprocess.run(
            [
                "cargo",
                "run",
                "-q",
                "-p",
                "digimon-engine-cli",
                "--",
                "--cards-json",
                str(CARDS_JSON),
                "pool",
            ],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            print(result.stderr, file=sys.stderr)
            raise RuntimeError(
                f"digimon-engine-cli pool failed with exit code {result.returncode}"
            )
        ids = json.loads(result.stdout)

    if not isinstance(ids, list) or not all(isinstance(i, str) for i in ids):
        raise RuntimeError("pool output is not a JSON array of card-id strings")
    # TEST-* are engine test fixtures (they have cards.json entries so the
    # DebugRunner can build them) — never player-facing deck-builder cards.
    return sorted(i for i in set(ids) if not i.upper().startswith("TEST-"))


def build_snapshot(card_ids: list[str]) -> dict:
    return {
        "version": 1,
        "generated_at": dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z"),
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
    parser.add_argument(
        "--pool-json",
        type=Path,
        default=None,
        help="Read the implemented pool from this JSON array instead of "
        "running digimon-engine-cli.",
    )
    args = parser.parse_args()

    if not CARDS_JSON.exists():
        print(f"error: {CARDS_JSON} does not exist", file=sys.stderr)
        return 2

    card_ids = collect_implemented_card_ids(args.pool_json)

    if not card_ids:
        print("error: no implemented cards found", file=sys.stderr)
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
                "`python code/tools/build_tested_cards.py` to refresh",
                file=sys.stderr,
            )
            return 1
        print(f"ok: {len(card_ids)} implemented cards (snapshot up to date)")
        return 0

    # Idempotent write: only touch the file if content changed.
    if OUTPUT_JSON.exists():
        existing = json.loads(OUTPUT_JSON.read_text())
        if _normalize_for_diff(existing) == _normalize_for_diff(new_payload):
            print(f"ok: {len(card_ids)} implemented cards (no changes)")
            return 0

    OUTPUT_JSON.write_text(json.dumps(new_payload, indent=2) + "\n")
    print(
        f"wrote {OUTPUT_JSON.relative_to(REPO_ROOT)} "
        f"({len(card_ids)} implemented cards)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
