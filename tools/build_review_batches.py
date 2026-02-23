#!/usr/bin/env python3
"""Build per-card review candidates and 5-card batches for AI task queueing."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any


PROJECT_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SETS = ["EX10", "BT13", "EX11", "BT23", "BT21", "BT24"]
CARD_ID_RE = re.compile(r"^[A-Z0-9]+-[0-9]{3}(?:-[A-Z0-9]+)?$")
TOKEN_FILE_RE = re.compile(r"^[a-z0-9]+_[0-9]{3}_token\.py$")


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def load_cards(cards_path: Path) -> list[dict[str, Any]]:
    data = json.loads(cards_path.read_text(encoding="utf-8"))
    if isinstance(data, dict):
        return list(data.values())
    return data


def load_manifest(manifest_path: Path) -> dict[str, dict[str, Any]]:
    if not manifest_path.exists():
        return {}
    raw = json.loads(manifest_path.read_text(encoding="utf-8"))
    cards = raw.get("cards", {})
    if not isinstance(cards, dict):
        return {}
    return cards


def card_to_module_name(card_id: str) -> str:
    return card_id.replace("-", "_").lower()


def module_to_card_id(module_name: str) -> str:
    base = module_name.removesuffix(".py")
    parts = base.split("_")
    if len(parts) < 2:
        return base.upper()
    return f"{parts[0].upper()}-{'-'.join(p.upper() for p in parts[1:])}"


def card_sort_key(card_id: str) -> tuple[str, int, str]:
    if "-" not in card_id:
        return (card_id, 0, card_id)
    prefix, suffix = card_id.split("-", 1)
    num_str = suffix.split("-", 1)[0]
    try:
        num = int(num_str)
    except ValueError:
        num = 0
    return (prefix, num, suffix)


def chunked(items: list[dict[str, Any]], size: int) -> list[list[dict[str, Any]]]:
    return [items[i : i + size] for i in range(0, len(items), size)]


def build_candidates(
    *,
    set_ids: list[str],
    missing_sets: set[str],
    cards: list[dict[str, Any]],
    scripts_root: Path,
    manifest_cards: dict[str, dict[str, Any]],
    include_tokens: bool,
) -> tuple[list[dict[str, Any]], dict[str, dict[str, int | str]]]:
    all_candidates: list[dict[str, Any]] = []
    per_set_summary: dict[str, dict[str, int | str]] = {}

    cards_by_set: dict[str, list[str]] = {}
    for card in cards:
        card_id = str(card.get("card_id", "")).strip().upper()
        if not CARD_ID_RE.match(card_id):
            continue
        set_id = card_id.split("-", 1)[0]
        cards_by_set.setdefault(set_id, []).append(card_id)

    for set_id in set_ids:
        set_upper = set_id.upper()
        set_lower = set_upper.lower()
        set_cards = sorted(cards_by_set.get(set_upper, []), key=card_sort_key)

        manifest_set_cards = {
            cid: entry for cid, entry in manifest_cards.items() if cid.upper().startswith(f"{set_upper}-")
        }
        mode = "full" if (set_upper in missing_sets or not manifest_set_cards) else "diff_only"

        added = 0
        missing_generated = 0
        for card_id in set_cards:
            module_name = card_to_module_name(card_id)
            generated_path = scripts_root / "generated" / set_lower / f"{module_name}.py"
            if not generated_path.exists():
                missing_generated += 1
                continue

            reason = "full_set"
            should_add = mode == "full"
            if mode != "full":
                manifest_entry = manifest_cards.get(card_id)
                if not manifest_entry:
                    should_add = True
                    reason = "missing_manifest"
                else:
                    frozen_hash = str(manifest_entry.get("frozen_hash", ""))
                    generated_hash = sha256(generated_path)
                    if generated_hash != frozen_hash:
                        should_add = True
                        reason = "hash_diff"

            if should_add:
                all_candidates.append(
                    {
                        "card_id": card_id,
                        "set_id": set_lower,
                        "module_name": module_name,
                        "reason": reason,
                    }
                )
                added += 1

        token_added = 0
        if include_tokens:
            token_dir = scripts_root / "generated" / set_lower
            if token_dir.exists():
                for token_file in sorted(token_dir.glob("*_token.py")):
                    if not TOKEN_FILE_RE.match(token_file.name):
                        continue
                    module_name = token_file.stem
                    card_id = module_to_card_id(module_name)
                    if any(c["card_id"] == card_id for c in all_candidates):
                        continue
                    if mode == "full":
                        reason = "token_full_set"
                        should_add = True
                    else:
                        manifest_entry = manifest_cards.get(card_id)
                        if not manifest_entry:
                            reason = "token_missing_manifest"
                            should_add = True
                        else:
                            frozen_hash = str(manifest_entry.get("frozen_hash", ""))
                            generated_hash = sha256(token_file)
                            should_add = generated_hash != frozen_hash
                            reason = "token_hash_diff"
                    if should_add:
                        all_candidates.append(
                            {
                                "card_id": card_id,
                                "set_id": set_lower,
                                "module_name": module_name,
                                "reason": reason,
                            }
                        )
                        token_added += 1

        per_set_summary[set_upper] = {
            "mode": mode,
            "cards_in_db": len(set_cards),
            "candidates": added + token_added,
            "token_candidates": token_added,
            "missing_generated": missing_generated,
        }

    return all_candidates, per_set_summary


def main() -> int:
    parser = argparse.ArgumentParser(description="Build review_batch candidate lists and batches")
    parser.add_argument("--set", action="append", dest="sets", default=[])
    parser.add_argument("--missing-set", action="append", default=[])
    parser.add_argument("--batch-size", type=int, default=5)
    parser.add_argument(
        "--output",
        type=Path,
        default=PROJECT_ROOT / "data" / "review" / "six_set_review_plan.json",
    )
    parser.add_argument(
        "--cards-json",
        type=Path,
        default=PROJECT_ROOT / "digimon_gym" / "engine" / "data" / "cards.json",
    )
    parser.add_argument(
        "--scripts-root",
        type=Path,
        default=PROJECT_ROOT / "digimon_gym" / "engine" / "data" / "scripts",
    )
    parser.add_argument(
        "--manifest",
        type=Path,
        default=PROJECT_ROOT / "digimon_gym" / "engine" / "data" / "scripts" / "_frozen_manifest.json",
    )
    parser.add_argument("--no-include-tokens", action="store_true")
    args = parser.parse_args()

    set_ids = [s.upper() for s in (args.sets or DEFAULT_SETS)]
    missing_sets = {s.upper() for s in args.missing_set}
    include_tokens = not args.no_include_tokens

    cards = load_cards(args.cards_json)
    manifest_cards = load_manifest(args.manifest)
    candidates, per_set_summary = build_candidates(
        set_ids=set_ids,
        missing_sets=missing_sets,
        cards=cards,
        scripts_root=args.scripts_root,
        manifest_cards=manifest_cards,
        include_tokens=include_tokens,
    )
    batches = chunked(candidates, args.batch_size)

    payload = {
        "sets": set_ids,
        "missing_sets": sorted(missing_sets),
        "batch_size": args.batch_size,
        "summary": {
            "total_candidates": len(candidates),
            "total_batches": len(batches),
            "per_set": per_set_summary,
        },
        "candidates": candidates,
        "batches": batches,
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(payload, indent=2), encoding="utf-8")

    print(f"Wrote review plan: {args.output}")
    print(f"Total candidates: {len(candidates)}")
    print(f"Total batches: {len(batches)}")
    for set_id in set_ids:
        row = per_set_summary.get(set_id, {})
        print(
            f"  {set_id}: mode={row.get('mode')} cards={row.get('cards_in_db', 0)} "
            f"candidates={row.get('candidates', 0)} token={row.get('token_candidates', 0)} "
            f"missing_generated={row.get('missing_generated', 0)}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
