#!/usr/bin/env python
"""Rank archetypes from deck_library.json by meta_share.

Usage:
    python tools/rank_archetypes.py [--top N] [--min-coverage FLOAT]

Options:
    --top N             Show top N archetypes (default: 20)
    --min-coverage F    Only show archetypes with coverage >= F (default: 0.0)
"""
import argparse
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description="Rank archetypes by meta share")
    parser.add_argument("--top", type=int, default=20, help="Number of archetypes to show")
    parser.add_argument("--min-coverage", type=float, default=0.0,
                        help="Minimum script coverage to include")
    args = parser.parse_args()

    lib_path = Path(__file__).resolve().parent.parent / "digimon_gym" / "engine" / "data" / "deck_library.json"
    lib = json.loads(lib_path.read_text(encoding="utf-8"))

    archetypes = lib.get("archetypes", {})
    rows = []
    for name, data in archetypes.items():
        stats = data.get("stats", {})
        meta_share = stats.get("meta_share", 0.0)
        decklists = data.get("decklists", [])
        num_lists = len(decklists)

        # Count unique cards and compute script coverage
        all_cards = set()
        for dl in decklists:
            all_cards.update(json.loads(dl["decklist"]))
        coverage = data.get("script_coverage", None)

        rows.append({
            "name": name,
            "meta_share": meta_share,
            "num_lists": num_lists,
            "unique_cards": len(all_cards),
            "coverage": coverage,
        })

    # Filter by coverage
    if args.min_coverage > 0:
        rows = [r for r in rows if r["coverage"] is not None and r["coverage"] >= args.min_coverage]

    # Sort by meta_share descending, then by num_lists descending
    rows.sort(key=lambda r: (r["meta_share"], r["num_lists"]), reverse=True)

    # Display
    top = rows[:args.top]
    print(f"{'Rank':<5} {'Meta Share':>11} {'Lists':>6} {'Cards':>6} {'Coverage':>9}  Archetype")
    print("-" * 70)
    for i, r in enumerate(top, 1):
        share_pct = f"{r['meta_share'] * 100:.2f}%"
        cov = f"{r['coverage'] * 100:.0f}%" if r["coverage"] is not None else "n/a"
        print(f"{i:<5} {share_pct:>11} {r['num_lists']:>6} {r['unique_cards']:>6} {cov:>9}  {r['name']}")


if __name__ == "__main__":
    main()
