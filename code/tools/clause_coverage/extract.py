"""`extract` entry point: decklist / card-ID list -> per-card clause list.

This is the DENOMINATOR half of the clause-coverage pipeline (see the
package README). Usage::

    PYTHONPATH=code python -m tools.clause_coverage.extract \\
        --decklist qa/dcgo-harness/vb_pool.json --out clauses.json

    PYTHONPATH=code python -m tools.clause_coverage.extract \\
        --card-ids EX12-073 EX12-018

Output is a JSON document (written to `--out`, or stdout if omitted) plus a
human-readable summary (stdout when `--out` is given so it doesn't corrupt
piped JSON; stderr when `--out` is omitted and JSON itself goes to stdout).
The summary's headline number is always the image-required count -- see
README "Why image-required is the headline number".
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parents[3]
_CODE_DIR = _PROJECT_ROOT / "code"
if str(_CODE_DIR) not in sys.path:
    sys.path.insert(0, str(_CODE_DIR))

from data_paths import CARD_OVERRIDES, CARDS_JSON, DATA_DIR  # noqa: E402

from tools.clause_coverage.card_sources import (  # noqa: E402
    DEFAULT_IMG_DIR,
    extract_card_clauses,
    load_cards_index,
    load_image_cache,
    load_official_index,
    load_overrides_index,
)
from tools.clause_coverage.models import Clause  # noqa: E402

DEFAULT_OFFICIAL_JSON = DATA_DIR / "card_official.json"


def _load_decklist(path: Path, deck_name: str | None = None) -> tuple[list[str], str]:
    """Accept either a flat JSON list of card IDs, or the harness pool shape
    `{"decks": [{"name": ..., "cards": [...]}, ...]}` (e.g. `qa/dcgo-harness/vb_pool.json`).

    Returns (distinct card IDs, order preserved by first appearance;
    human-readable source description).
    """
    with open(path, encoding="utf-8") as f:
        data = json.load(f)

    if isinstance(data, list):
        cards = data
        desc = f"{path} (flat card-ID list, {len(data)} entries)"
    elif isinstance(data, dict) and "decks" in data:
        decks = data["decks"]
        if deck_name:
            decks = [d for d in decks if d.get("name") == deck_name]
            if not decks:
                raise SystemExit(f"No deck named {deck_name!r} in {path}")
        cards = []
        names = []
        for d in decks:
            cards.extend(d.get("cards", []))
            names.append(d.get("name", "?"))
        desc = f"{path} deck(s) {names} ({len(cards)} card entries, {len(set(cards))} distinct)"
    else:
        raise SystemExit(f"Unrecognized decklist JSON shape in {path}: expected a list or {{'decks': [...]}}")

    seen: set[str] = set()
    distinct: list[str] = []
    for cid in cards:
        if cid not in seen:
            seen.add(cid)
            distinct.append(cid)
    return distinct, desc


def build_extract_result(card_ids: list[str], source_desc: str, clauses: list[Clause]) -> dict:
    by_zone = Counter(c.zone for c in clauses)
    by_source = Counter(c.source for c in clauses)
    image_required_ids = [c.id for c in clauses if c.source == "image-required"]
    return {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "source": source_desc,
        "cards": card_ids,
        "clauses": [c.to_dict() for c in clauses],
        "denominator": {
            "total_clauses": len(clauses),
            "total_cards": len(card_ids),
            "by_zone": dict(by_zone),
            "by_source": dict(by_source),
            "image_required_count": len(image_required_ids),
            "image_required_clause_ids": image_required_ids,
        },
    }


def _print_summary(result: dict, *, file) -> None:
    d = result["denominator"]
    print(
        f"Clause extraction: {d['total_cards']} cards -> {d['total_clauses']} clauses",
        file=file,
    )
    print("  by zone:   " + "  ".join(f"{z}={n}" for z, n in d["by_zone"].items()), file=file)
    print("  by source: " + "  ".join(f"{s}={n}" for s, n in d["by_source"].items()), file=file)
    total = d["total_clauses"] or 1
    ir = d["image_required_count"]
    print(
        f"IMAGE-REQUIRED: {ir} / {d['total_clauses']} clauses ({ir / total:.1%}) "
        "need human/vision confirmation from the card image.",
        file=file,
    )
    for cid in d["image_required_clause_ids"]:
        print(f"    {cid}", file=file)


def run(
    card_ids: list[str],
    source_desc: str,
    *,
    cards_json_path: Path = CARDS_JSON,
    overrides_path: Path = CARD_OVERRIDES,
    official_json_path: Path = DEFAULT_OFFICIAL_JSON,
    image_cache_path: Path | None = None,
    img_dir: str | None = None,
) -> dict:
    """Library entry point (also used directly by tests)."""
    cards_index = load_cards_index(cards_json_path)
    overrides_index = load_overrides_index(overrides_path)
    official_index = load_official_index(official_json_path)
    image_cache = load_image_cache(image_cache_path)

    all_clauses: list[Clause] = []
    for cid in card_ids:
        all_clauses.extend(
            extract_card_clauses(
                cid,
                cards_index=cards_index,
                overrides_index=overrides_index,
                official_index=official_index,
                image_cache=image_cache,
                img_dir=img_dir,
            )
        )
    return build_extract_result(card_ids, source_desc, all_clauses)


def _use_utf8_streams() -> None:
    """Card text is full of full-width brackets etc.; Windows consoles
    default to cp1252 and choke on it. Best-effort, never fatal."""
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8")
        except (AttributeError, ValueError):
            pass


def main(argv: list[str] | None = None) -> None:
    _use_utf8_streams()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--decklist", type=Path, help="Path to a decklist JSON")
    parser.add_argument("--deck-name", help="Select one deck by name from a multi-deck decklist JSON")
    parser.add_argument("--card-ids", nargs="+", help="Explicit card IDs (space-separated)")
    parser.add_argument(
        "--image-cache",
        type=Path,
        help="JSON {card_id: {zone: text}} of human/vision-filled-in text for image-required slots",
    )
    parser.add_argument("--img-dir", default=DEFAULT_IMG_DIR, help="Card image directory")
    parser.add_argument("--out", type=Path, help="Write the clause-list JSON here (default: stdout)")
    parser.add_argument("--quiet", action="store_true", help="Suppress the human-readable summary")
    args = parser.parse_args(argv)

    if not args.decklist and not args.card_ids:
        parser.error("Provide --decklist PATH or --card-ids ID [ID ...]")

    if args.card_ids:
        seen: set[str] = set()
        card_ids = [c for c in args.card_ids if not (c in seen or seen.add(c))]
        source_desc = f"--card-ids ({len(card_ids)} explicit ids)"
    else:
        card_ids, source_desc = _load_decklist(args.decklist, args.deck_name)

    result = run(
        card_ids,
        source_desc,
        image_cache_path=args.image_cache,
        img_dir=args.img_dir,
    )

    out_text = json.dumps(result, indent=2, ensure_ascii=False)
    if args.out:
        args.out.write_text(out_text, encoding="utf-8")
        summary_file = sys.stdout
    else:
        print(out_text)
        summary_file = sys.stderr

    if not args.quiet:
        _print_summary(result, file=summary_file)


if __name__ == "__main__":
    main()
