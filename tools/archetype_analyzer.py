#!/usr/bin/env python3
"""Analyze a Digimon TCG archetype's card pool and implementation status.

Cross-references deck_library.json with _frozen_manifest.json and cards.json
to identify which cards need to be implemented for an archetype to be fully playable.

Usage:
    python tools/archetype_analyzer.py "CS Hudiemon"
    python tools/archetype_analyzer.py "Royal Knights" --show-shared
    python tools/archetype_analyzer.py --list
    python tools/archetype_analyzer.py --list --min-meta-share 0.03
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Dict, List, Optional, Set, Tuple

_PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if _PROJECT_ROOT not in sys.path:
    sys.path.insert(0, _PROJECT_ROOT)

_DATA_DIR = Path(_PROJECT_ROOT) / "digimon_gym" / "engine" / "data"
_DECK_LIBRARY_PATH = _DATA_DIR / "deck_library.json"
_MANIFEST_PATH = _DATA_DIR / "scripts" / "_frozen_manifest.json"
_DCGO_DIR = Path(_PROJECT_ROOT) / "DCGO" / "Assets" / "Scripts" / "CardEffect"


def load_library() -> dict:
    with open(_DECK_LIBRARY_PATH, "r", encoding="utf-8") as f:
        return json.load(f)


def load_frozen_ids() -> Set[str]:
    with open(_MANIFEST_PATH, "r", encoding="utf-8") as f:
        manifest = json.load(f)
    return set(manifest.get("cards", {}).keys())


def find_archetype(library: dict, name: str) -> Optional[Tuple[str, dict]]:
    """Find archetype by exact or fuzzy match."""
    archetypes = library.get("archetypes", {})

    # Exact match
    if name in archetypes:
        return name, archetypes[name]

    # Case-insensitive match
    lower = name.lower()
    for arch_name, arch_data in archetypes.items():
        if arch_name.lower() == lower:
            return arch_name, arch_data

    # Substring match
    matches = []
    for arch_name, arch_data in archetypes.items():
        if lower in arch_name.lower():
            matches.append((arch_name, arch_data))

    if len(matches) == 1:
        return matches[0]
    elif len(matches) > 1:
        print(f"Ambiguous archetype name '{name}'. Matches:")
        for m, _ in matches:
            print(f"  - {m}")
        return None

    print(f"Archetype '{name}' not found in deck library.")
    return None


def analyze_archetype(
    library: dict,
    arch_name: str,
    arch_data: dict,
    frozen_ids: Set[str],
    show_shared: bool = False,
) -> dict:
    """Analyze an archetype's card pool and implementation status.

    Returns a dict with analysis results for programmatic use.
    """
    decklists = arch_data.get("decklists", [])
    stats = arch_data.get("stats", {})

    # Collect all unique cards with frequency
    card_freq = Counter()  # card_id -> number of decklists containing it
    for dl in decklists:
        try:
            card_ids = json.loads(dl.get("decklist", "[]"))
        except (json.JSONDecodeError, TypeError):
            continue
        unique_in_deck = set(card_ids)
        for cid in unique_in_deck:
            card_freq[cid] += 1

    total_unique = len(card_freq)
    implemented = {cid for cid in card_freq if cid in frozen_ids}
    missing = {cid: freq for cid, freq in card_freq.items() if cid not in frozen_ids}
    coverage = len(implemented) / total_unique if total_unique > 0 else 0.0

    # Group missing by set
    by_set: Dict[str, List[Tuple[str, int]]] = defaultdict(list)
    for cid, freq in sorted(missing.items(), key=lambda x: -x[1]):
        set_id = cid.split("-")[0]
        by_set[set_id].append((cid, freq))

    # Check C# source availability
    sets_with_sources = set()
    sets_without_sources = set()
    for set_id in by_set:
        dcgo_path = _DCGO_DIR / set_id
        if dcgo_path.exists():
            sets_with_sources.add(set_id)
        else:
            sets_without_sources.add(set_id)

    # Categorize by usage frequency
    n_decks = len(decklists)
    core = []      # >75%
    common = []    # 50-75%
    tech = []      # 25-50%
    fringe = []    # <25%

    for cid, freq in sorted(missing.items(), key=lambda x: -x[1]):
        pct = freq / n_decks if n_decks > 0 else 0
        if pct > 0.75:
            core.append((cid, freq))
        elif pct > 0.50:
            common.append((cid, freq))
        elif pct > 0.25:
            tech.append((cid, freq))
        else:
            fringe.append((cid, freq))

    # Count fully playable decklists
    playable = 0
    for dl in decklists:
        try:
            card_ids = set(json.loads(dl.get("decklist", "[]")))
        except (json.JSONDecodeError, TypeError):
            continue
        if card_ids <= frozen_ids:
            playable += 1

    # Cross-archetype impact
    shared_archetypes: Dict[str, List[str]] = {}
    if show_shared and missing:
        for cid in missing:
            users = []
            for other_name, other_data in library.get("archetypes", {}).items():
                if other_name == arch_name:
                    continue
                for dl in other_data.get("decklists", []):
                    try:
                        if cid in json.loads(dl.get("decklist", "[]")):
                            users.append(other_name)
                            break
                    except (json.JSONDecodeError, TypeError):
                        continue
            if users:
                shared_archetypes[cid] = users

    result = {
        "archetype_name": arch_name,
        "meta_share": stats.get("meta_share", 0.0),
        "num_decklists": n_decks,
        "total_unique_cards": total_unique,
        "implemented_count": len(implemented),
        "missing_count": len(missing),
        "coverage_pct": round(coverage * 100, 1),
        "playable_decklists": playable,
        "missing_by_set": {
            set_id: [(cid, freq) for cid, freq in cards]
            for set_id, cards in sorted(by_set.items())
        },
        "sets_with_sources": sorted(sets_with_sources),
        "sets_without_sources": sorted(sets_without_sources),
        "priority": {
            "core": core,
            "common": common,
            "tech": tech,
            "fringe": fringe,
        },
        "shared_archetypes": shared_archetypes,
    }

    return result


def print_analysis(result: dict) -> None:
    """Pretty-print archetype analysis results."""
    print("=" * 65)
    print(f"Archetype: {result['archetype_name']}")
    print(f"Meta share: {result['meta_share']*100:.1f}%")
    print(f"Decklists: {result['num_decklists']}")
    print("=" * 65)
    print()
    print(f"Total unique cards: {result['total_unique_cards']}")
    print(f"Already implemented: {result['implemented_count']}")
    print(f"Missing: {result['missing_count']}")
    print(f"Coverage: {result['coverage_pct']}%")
    print(f"Fully playable decklists: {result['playable_decklists']}/{result['num_decklists']}")
    print()

    if not result["missing_by_set"]:
        print("All cards are implemented! The archetype is fully playable.")
        return

    print("Missing cards by set:")
    for set_id, cards in result["missing_by_set"].items():
        has_src = set_id in result["sets_with_sources"]
        src_tag = " [C# sources available]" if has_src else " [NO C# sources]"
        print(f"  {set_id}: {len(cards)} cards{src_tag}")
        for cid, freq in cards:
            pct = freq / result["num_decklists"] * 100 if result["num_decklists"] > 0 else 0
            print(f"    {cid} (in {freq}/{result['num_decklists']} decklists, {pct:.0f}%)")
    print()

    # Priority breakdown
    priority = result["priority"]
    if priority["core"]:
        print(f"Core cards (>75% usage) — {len(priority['core'])} cards:")
        for cid, freq in priority["core"]:
            print(f"  {cid} ({freq}/{result['num_decklists']})")
    if priority["common"]:
        print(f"Common staples (50-75% usage) — {len(priority['common'])} cards:")
        for cid, freq in priority["common"]:
            print(f"  {cid} ({freq}/{result['num_decklists']})")
    if priority["tech"]:
        print(f"Tech choices (25-50% usage) — {len(priority['tech'])} cards:")
        for cid, freq in priority["tech"]:
            print(f"  {cid} ({freq}/{result['num_decklists']})")
    if priority["fringe"]:
        print(f"Fringe includes (<25% usage) — {len(priority['fringe'])} cards:")
        for cid, freq in priority["fringe"]:
            print(f"  {cid} ({freq}/{result['num_decklists']})")
    print()

    # Cross-archetype impact
    if result["shared_archetypes"]:
        print("Cross-archetype impact (implementing these cards also helps):")
        for cid, archs in result["shared_archetypes"].items():
            print(f"  {cid}: {', '.join(archs)}")
        print()

    # Sets needing C# sources
    if result["sets_without_sources"]:
        print("WARNING: These sets have no C# sources for transpilation:")
        for s in result["sets_without_sources"]:
            print(f"  {s} — scripts must be written manually")


def list_archetypes(library: dict, frozen_ids: Set[str], min_meta_share: float = 0.0) -> None:
    """List all archetypes with their implementation coverage."""
    archetypes = library.get("archetypes", {})

    rows = []
    for name, data in archetypes.items():
        stats = data.get("stats", {})
        meta_share = stats.get("meta_share", 0.0)
        if meta_share < min_meta_share:
            continue

        decklists = data.get("decklists", [])
        card_freq = Counter()
        for dl in decklists:
            try:
                for cid in set(json.loads(dl.get("decklist", "[]"))):
                    card_freq[cid] += 1
            except (json.JSONDecodeError, TypeError):
                continue

        total = len(card_freq)
        impl = sum(1 for cid in card_freq if cid in frozen_ids)
        coverage = impl / total * 100 if total > 0 else 0.0

        # Count fully playable
        playable = 0
        for dl in decklists:
            try:
                if set(json.loads(dl.get("decklist", "[]"))) <= frozen_ids:
                    playable += 1
            except (json.JSONDecodeError, TypeError):
                continue

        rows.append((name, len(decklists), meta_share, total, impl, total - impl, coverage, playable))

    rows.sort(key=lambda r: -r[2])  # Sort by meta share

    print(f"{'Archetype':<28s} {'Decks':>5s} {'Meta%':>6s} {'Cards':>5s} "
          f"{'Impl':>5s} {'Miss':>5s} {'Cov%':>6s} {'Play':>5s}")
    print("-" * 80)

    for name, n_decks, meta_share, total, impl, miss, cov, playable in rows:
        marker = " *" if cov == 100.0 else ""
        print(f"{name:<28s} {n_decks:>5d} {meta_share*100:>5.1f}% {total:>5d} "
              f"{impl:>5d} {miss:>5d} {cov:>5.1f}% {playable:>4d}/{n_decks}{marker}")


def main():
    parser = argparse.ArgumentParser(
        description="Analyze archetype card pool and implementation status"
    )
    parser.add_argument(
        "archetype",
        nargs="?",
        help="Archetype name to analyze (fuzzy match supported)",
    )
    parser.add_argument(
        "--list",
        action="store_true",
        help="List all archetypes with coverage stats",
    )
    parser.add_argument(
        "--min-meta-share",
        type=float,
        default=0.0,
        help="Minimum meta share for --list (default: 0.0)",
    )
    parser.add_argument(
        "--show-shared",
        action="store_true",
        help="Show which other archetypes share missing cards",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output analysis as JSON",
    )

    args = parser.parse_args()

    library = load_library()
    frozen_ids = load_frozen_ids()

    if args.list:
        list_archetypes(library, frozen_ids, min_meta_share=args.min_meta_share)
        return

    if not args.archetype:
        parser.print_help()
        return

    match = find_archetype(library, args.archetype)
    if match is None:
        sys.exit(1)

    arch_name, arch_data = match
    result = analyze_archetype(
        library, arch_name, arch_data, frozen_ids, show_shared=args.show_shared
    )

    if args.json:
        # Convert tuples to lists for JSON serialization
        json_result = json.loads(json.dumps(result, default=str))
        json.dump(json_result, sys.stdout, indent=2)
        print()
    else:
        print_analysis(result)


if __name__ == "__main__":
    main()
