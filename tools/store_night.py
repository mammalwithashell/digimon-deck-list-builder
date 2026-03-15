"""Store Night Recommender — decide which deck to bring and how to tune it.

Given a store name and your candidate archetypes, evaluates each deck against
the store's local meta via game simulation and recommends the best choice.
Optionally optimizes the winning deck's card list with targeted swaps.

Usage:
    python tools/store_night.py \
        --store "The Card Haven" \
        --archetypes "Rocks,Millenniummon,Dark Masters" \
        --library my_decks.json \
        --since 2025-12-01 \
        --optimize

No DB imports. No auth. Pure engine + training.
"""

from __future__ import annotations

import argparse
import json
import logging
import statistics
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

logger = logging.getLogger(__name__)

_DECK_LIBRARY_PATH = (
    Path(__file__).resolve().parent.parent
    / "digimon_gym" / "engine" / "data" / "deck_library.json"
)

_SOURCE_PREFERENCE: Dict[str, int] = {
    "digilab": 3,
    "digimonmeta": 2,
    "egman": 1,
}


# ---------------------------------------------------------------------------
# Personal deck library
# ---------------------------------------------------------------------------

def load_personal_library(path: str) -> dict:
    """Load the user's personal deck library JSON.

    Expected format::

        {
          "general_pool": ["BT24-099", ...],
          "Rocks": {
            "decklists": [
              {"name": "main", "deck": ["BT24-001", ...], "notes": "..."},
              ...
            ]
          },
          ...
        }

    Returns the parsed dict, or empty dict if the file doesn't exist.
    """
    p = Path(path)
    if not p.exists():
        logger.warning("Personal library not found at %s", path)
        return {}
    with open(p, "r", encoding="utf-8") as f:
        return json.load(f)


def resolve_deck_from_personal(
    library: dict,
    archetype: str,
) -> Optional[List[str]]:
    """Get the first decklist for an archetype from the personal library."""
    arch_data = library.get(archetype)
    if not arch_data or not isinstance(arch_data, dict):
        return None
    decklists = arch_data.get("decklists", [])
    if not decklists:
        return None
    deck = decklists[0].get("deck")
    if deck and isinstance(deck, list):
        return deck
    return None


def get_personal_deck_name(library: dict, archetype: str) -> str:
    """Get the name of the first decklist for display."""
    arch_data = library.get(archetype, {})
    if isinstance(arch_data, dict):
        decklists = arch_data.get("decklists", [])
        if decklists:
            return decklists[0].get("name", "unnamed")
    return "unnamed"


def collect_personal_pool_cards(
    library: dict,
    archetype: str,
) -> set:
    """Collect all unique card IDs from the user's lists for this archetype."""
    cards: set = set()
    arch_data = library.get(archetype)
    if not arch_data or not isinstance(arch_data, dict):
        return cards
    for dl in arch_data.get("decklists", []):
        deck = dl.get("deck", [])
        cards.update(deck)
    return cards


def get_general_pool(library: dict) -> List[str]:
    """Get the general tech card pool from the personal library."""
    return library.get("general_pool", [])


# ---------------------------------------------------------------------------
# Scraped deck library helpers
# ---------------------------------------------------------------------------

def resolve_deck_from_library(archetype: str) -> Optional[List[str]]:
    """Resolve the best decklist from deck_library.json for an archetype."""
    if not _DECK_LIBRARY_PATH.exists():
        return None

    with open(_DECK_LIBRARY_PATH, "r", encoding="utf-8") as f:
        library = json.load(f)

    arch = library.get("archetypes", {}).get(archetype)
    if not arch or not arch.get("decklists"):
        return None

    decklists = sorted(
        arch["decklists"],
        key=lambda d: _SOURCE_PREFERENCE.get(d.get("source", ""), 0),
        reverse=True,
    )

    for entry in decklists:
        raw = entry.get("decklist")
        if not raw:
            continue
        deck = json.loads(raw) if isinstance(raw, str) else raw
        if deck:
            return deck
    return None


def build_opponent_list(
    scoped_meta: Dict[str, Any],
    exclude_archetypes: set,
    min_plays: int = 3,
) -> Tuple[List[Tuple[List[str], float, str]], List[str]]:
    """Build weighted opponent list from scoped meta + deck library.

    Returns:
        (opponents, skipped) where opponents is list of
        (deck_ids, meta_weight, archetype_name) and skipped is list of
        archetype names that couldn't be resolved.
    """
    from digimon_gym.engine.data.deck_finder import load_implemented_card_ids

    if not _DECK_LIBRARY_PATH.exists():
        return [], []

    with open(_DECK_LIBRARY_PATH, "r", encoding="utf-8") as f:
        library = json.load(f)

    implemented = load_implemented_card_ids()
    lib_archetypes = library.get("archetypes", {})

    opponents: List[Tuple[List[str], float, str]] = []
    skipped: List[str] = []

    for arch_name, stats in scoped_meta.items():
        if arch_name in exclude_archetypes:
            continue
        if stats.times_played < min_plays:
            continue
        if stats.meta_share <= 0:
            continue

        lib_arch = lib_archetypes.get(arch_name)
        if not lib_arch or not lib_arch.get("decklists"):
            skipped.append(arch_name)
            continue

        # Find best fully-implemented decklist
        best_deck = None
        candidates = sorted(
            lib_arch["decklists"],
            key=lambda d: _SOURCE_PREFERENCE.get(d.get("source", ""), 0),
            reverse=True,
        )
        for entry in candidates:
            raw = entry.get("decklist", "[]")
            try:
                deck_ids = json.loads(raw) if isinstance(raw, str) else raw
            except (json.JSONDecodeError, TypeError):
                continue
            if not deck_ids:
                continue
            if all(cid in implemented for cid in deck_ids):
                best_deck = deck_ids
                break

        if best_deck is None:
            skipped.append(arch_name)
            continue

        opponents.append((best_deck, stats.meta_share, arch_name))

    return opponents, skipped


# ---------------------------------------------------------------------------
# Sleeper detection
# ---------------------------------------------------------------------------

def classify_archetypes(
    scoped_meta: Dict[str, Any],
    median_plays: float,
    min_plays: int = 3,
    sleeper_conv_threshold: float = 0.50,
) -> Tuple[List[dict], List[dict], List[dict]]:
    """Classify archetypes into threats, sleepers, and insufficient data.

    Returns (threats, sleepers, insufficient) — each a list of dicts with
    archetype stats.
    """
    play_floor = max(min_plays, int(median_plays / 2))

    threats: List[dict] = []
    sleepers: List[dict] = []
    insufficient: List[dict] = []

    for name, stats in sorted(
        scoped_meta.items(),
        key=lambda x: x[1].meta_share,
        reverse=True,
    ):
        entry = {
            "name": name,
            "meta_share": stats.meta_share,
            "win_rate": stats.win_rate,
            "conversion_rate": stats.conversion_rate,
            "times_played": stats.times_played,
        }

        if stats.times_played < play_floor:
            if stats.times_played > 0:
                insufficient.append(entry)
        elif stats.conversion_rate > sleeper_conv_threshold:
            sleepers.append(entry)
        else:
            threats.append(entry)

    return threats, sleepers, insufficient


# ---------------------------------------------------------------------------
# Core evaluation
# ---------------------------------------------------------------------------

def evaluate_archetypes(
    my_decks: Dict[str, List[str]],
    opponents: List[Tuple[List[str], float, str]],
    pilot_path: str = "greedy",
    games_per_eval: int = 50,
    n_workers: int = 1,
) -> Dict[str, dict]:
    """Evaluate each of the user's decks against the opponent field.

    Returns dict of archetype_name -> {etwr, matchups: {opp_name: wr}}.
    """
    from digimon_gym.agents.architect_simulator import DeckSimulator

    sim = DeckSimulator(
        pilot_policy_path=pilot_path,
        opponent_policy="greedy",
        games_per_eval=games_per_eval,
        max_turns=200,
        n_workers=n_workers,
        cache_enabled=True,
    )

    # Build opponent list in the format DeckSimulator expects
    opp_for_sim: List[Tuple[List[str], float]] = [
        (deck, weight) for deck, weight, _name in opponents
    ]

    results: Dict[str, dict] = {}

    for arch_name, deck in my_decks.items():
        # Overall ETWR
        etwr = sim.evaluate_deck(deck, opp_for_sim)

        # Per-opponent matchup breakdown
        matchups: Dict[str, float] = {}
        for opp_deck, _weight, opp_name in opponents:
            wr = sim.evaluate_deck(deck, [(opp_deck, 1.0)])
            matchups[opp_name] = wr

        results[arch_name] = {
            "etwr": etwr,
            "matchups": matchups,
        }

    return results


def run_optimization(
    archetype: str,
    deck: List[str],
    opponents: List[Tuple[List[str], float, str]],
    personal_library: dict,
    pilot_path: str = "greedy",
    episodes: int = 100,
    n_workers: int = 1,
) -> Optional[dict]:
    """Run architect optimization on the chosen deck.

    Builds a 3-layer candidate pool:
    1. Personal library cards for this archetype
    2. Scraped library cards for this archetype (via CandidatePool default)
    3. General pool tech cards

    Returns optimization metadata or None on failure.
    """
    from digimon_gym.agents.architect_optimizer import MetaOptimizer

    # Build extra_cards from personal pool + general pool
    personal_cards = collect_personal_pool_cards(personal_library, archetype)
    general_cards = get_general_pool(personal_library)
    extra_cards = list(personal_cards | set(general_cards))

    # Build meta config from opponents
    meta_config: Dict[str, Any] = {"archetypes": {}}
    for _deck, weight, name in opponents:
        meta_config["archetypes"][name] = {"local_meta_share": weight}

    try:
        optimizer = MetaOptimizer(
            archetype_name=archetype,
            base_deck=deck,
            meta_config=meta_config,
            pilot_policy_path=pilot_path,
            output_dir="store_night_runs",
            extra_cards=extra_cards if extra_cards else None,
        )

        metadata = optimizer.train(
            episodes=episodes,
            games_per_eval=50,
            max_swaps=10,
            n_workers=n_workers,
        )

        return metadata
    except Exception as exc:
        logger.error("Optimization failed: %s", exc)
        return None


# ---------------------------------------------------------------------------
# Report formatting
# ---------------------------------------------------------------------------

def print_report(
    store_name: str,
    since_date: Optional[str],
    total_results: int,
    median_plays: float,
    mean_plays: float,
    eval_results: Dict[str, dict],
    deck_names: Dict[str, str],
    threats: List[dict],
    sleepers: List[dict],
    insufficient: List[dict],
    skipped_opponents: List[str],
    optimization: Optional[dict] = None,
    optimize_archetype: Optional[str] = None,
    personal_pool_size: int = 0,
    scraped_pool_size: int = 0,
    general_pool_size: int = 0,
) -> None:
    """Print the full store night report."""

    print(f"\n{'=' * 70}")
    print(f"  Store Night: {store_name}")
    since_str = f" since {since_date}" if since_date else ""
    print(f"  Meta based on {total_results} results{since_str}")
    print(f"  Median archetype plays: {median_plays:.0f}  |  Mean: {mean_plays:.1f}")
    print(f"{'=' * 70}")

    # --- Your archetypes ranked ---
    ranked = sorted(
        eval_results.items(),
        key=lambda x: x[1]["etwr"],
        reverse=True,
    )

    print(f"\n  YOUR ARCHETYPES (ranked by ETWR):")
    print(f"  {'#':>2}  {'Archetype':<22} {'ETWR':>6}  {'List':<20} Top Matchups")
    print(f"  {'':>2}  {'-' * 22} {'-' * 6}  {'-' * 20} {'-' * 30}")

    for i, (arch_name, data) in enumerate(ranked, 1):
        etwr = data["etwr"]
        list_name = deck_names.get(arch_name, "scraped")

        # Top 3 matchups by meta share
        matchups = data["matchups"]
        top_matchups = sorted(matchups.items(), key=lambda x: x[1], reverse=True)[:3]
        mu_str = "  ".join(f"{n}({wr:.2f})" for n, wr in top_matchups)

        print(f"  {i:>2}. {arch_name:<22} {etwr:>5.3f}  {list_name:<20} {mu_str}")

    best_arch = ranked[0][0] if ranked else None
    if best_arch:
        print(f"\n  RECOMMENDATION: Bring {best_arch}")

    # --- Local meta threats ---
    if threats:
        print(f"\n  LOCAL META THREATS:")
        print(f"  {'Archetype':<22} {'Share':>6} {'WR':>6} {'Conv':>6} {'Plays':>6}")
        print(f"  {'-' * 22} {'-' * 6} {'-' * 6} {'-' * 6} {'-' * 6}")
        for t in threats[:15]:
            print(
                f"  {t['name']:<22} {t['meta_share'] * 100:>5.1f}% "
                f"{t['win_rate'] * 100:>5.1f}% {t['conversion_rate'] * 100:>5.1f}% "
                f"{t['times_played']:>6}"
            )

    # --- Sleepers ---
    if sleepers:
        print(f"\n  SLEEPERS (conv > 50%, sufficient sample):")
        print(f"  {'Archetype':<22} {'Share':>6} {'WR':>6} {'Conv':>6} {'Plays':>6}")
        print(f"  {'-' * 22} {'-' * 6} {'-' * 6} {'-' * 6} {'-' * 6}")
        for s in sleepers:
            print(
                f"  {s['name']:<22} {s['meta_share'] * 100:>5.1f}% "
                f"{s['win_rate'] * 100:>5.1f}% {s['conversion_rate'] * 100:>5.1f}% "
                f"{s['times_played']:>6}"
            )

    # --- Insufficient data ---
    if insufficient:
        print(f"\n  INSUFFICIENT DATA (too few plays to trust):")
        for ins in insufficient[:10]:
            print(
                f"  {ins['name']:<22} {ins['meta_share'] * 100:>5.1f}% "
                f"{ins['win_rate'] * 100:>5.1f}% {ins['conversion_rate'] * 100:>5.1f}% "
                f"{ins['times_played']:>6}  ? low sample"
            )

    # --- Skipped opponents ---
    if skipped_opponents:
        print(f"\n  Skipped {len(skipped_opponents)} archetypes (no simulatable decklists):")
        for name in skipped_opponents[:5]:
            print(f"    - {name}")
        if len(skipped_opponents) > 5:
            print(f"    ... and {len(skipped_opponents) - 5} more")

    # --- Optimization ---
    if optimization and optimize_archetype:
        opt_deck = optimization.get("optimized_deck")
        base_wr = optimization.get("base_win_rate", 0)
        best_wr = optimization.get("best_win_rate", 0)

        print(f"\n  OPTIMIZATION ({optimize_archetype}):")
        print(
            f"  Pool: {personal_pool_size + scraped_pool_size + general_pool_size} cards "
            f"(personal: {personal_pool_size}, scraped: {scraped_pool_size}, "
            f"general: {general_pool_size})"
        )
        print(f"  Base WR: {base_wr:.3f}  ->  Best WR: {best_wr:.3f}")

        if opt_deck:
            output_dir = optimization.get("output_dir", "store_night_runs")
            print(f"  Optimized deck saved to: {output_dir}")

    print()


# ---------------------------------------------------------------------------
# CLI entrypoint
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Store night recommender: pick and tune a deck for a store.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Quick recommendation (greedy evaluation)
  python tools/store_night.py --store "The Card Haven" \\
      --archetypes "Rocks,Millenniummon,Dark Masters" --library my_decks.json

  # With optimization for the top pick
  python tools/store_night.py --store "The Card Haven" \\
      --archetypes "Rocks,Millenniummon" --library my_decks.json --optimize

  # Recent meta only (last 3 months)
  python tools/store_night.py --store "Boardwalk Games" \\
      --archetypes "Rocks,Medusamon" --since 2025-12-01
        """,
    )

    parser.add_argument(
        "--store", required=True,
        help="Store name (looked up in DigiLab)",
    )
    parser.add_argument(
        "--archetypes", required=True,
        help="Comma-separated archetype names you'd consider bringing",
    )
    parser.add_argument(
        "--library", default="my_decks.json",
        help="Path to your personal deck library JSON (default: my_decks.json)",
    )
    parser.add_argument(
        "--since", default=None,
        help="Only consider tournaments after this date (ISO format, "
             "default: 3 months ago)",
    )
    parser.add_argument(
        "--games", type=int, default=50,
        help="Games per matchup for evaluation (default: 50)",
    )
    parser.add_argument(
        "--pilot", default="greedy",
        help='Pilot policy path or "greedy" (default: greedy)',
    )
    parser.add_argument(
        "--optimize", action="store_true",
        help="Run deck optimization on the top-ranked archetype",
    )
    parser.add_argument(
        "--optimize-episodes", type=int, default=100,
        help="Architect training episodes if optimizing (default: 100)",
    )
    parser.add_argument(
        "--workers", type=int, default=1,
        help="Parallel simulation workers (default: 1)",
    )
    parser.add_argument(
        "--min-plays", type=int, default=3,
        help="Minimum local plays for an archetype to count as a meta "
             "threat (default: 3)",
    )
    parser.add_argument(
        "--verbose", action="store_true",
        help="Enable debug logging",
    )

    args = parser.parse_args()

    logging.basicConfig(
        level=logging.DEBUG if args.verbose else logging.INFO,
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    )

    # --- Parse archetypes ---
    archetypes = [a.strip() for a in args.archetypes.split(",") if a.strip()]
    if not archetypes:
        print("ERROR: No archetypes provided.", file=sys.stderr)
        sys.exit(1)

    # --- Default since_date to 3 months ago ---
    since_date = args.since
    if since_date is None:
        three_months_ago = datetime.now(timezone.utc) - timedelta(days=90)
        since_date = three_months_ago.strftime("%Y-%m-%d")

    # --- Load personal library ---
    personal_lib = load_personal_library(args.library)
    if not personal_lib:
        print(
            f"Note: No personal library at {args.library}. "
            f"Using scraped decklists only.",
        )

    # --- Resolve store ---
    from digimon_gym.digilab_client import list_stores, get_scoped_meta

    try:
        all_stores = list_stores(min_tournaments=0)
    except Exception as exc:
        print(f"ERROR: Could not connect to DigiLab: {exc}", file=sys.stderr)
        sys.exit(1)

    store_map = {s.name.lower(): s for s in all_stores}
    store = store_map.get(args.store.lower())
    if not store:
        print(f"ERROR: Store not found: {args.store}", file=sys.stderr)
        print("Available stores:")
        for s in all_stores[:20]:
            print(f"  {s.name} (ID: {s.store_id}, {s.tournament_count} events)")
        sys.exit(1)

    print(f"Store: {store.name} (ID: {store.store_id})")
    print(f"Since: {since_date}")

    # --- Get scoped meta ---
    scoped_result = get_scoped_meta(
        store_ids=[store.store_id],
        since_date=since_date,
    )
    scoped = scoped_result.archetypes

    if not scoped:
        print("ERROR: No tournament data found for this store/date range.",
              file=sys.stderr)
        sys.exit(1)

    print(f"Meta: {scoped_result.total_results} results, "
          f"{len(scoped)} archetypes")

    # --- Resolve decks for your archetypes ---
    my_decks: Dict[str, List[str]] = {}
    deck_names: Dict[str, str] = {}

    for arch in archetypes:
        # Try personal library first
        deck = resolve_deck_from_personal(personal_lib, arch)
        if deck:
            my_decks[arch] = deck
            deck_names[arch] = get_personal_deck_name(personal_lib, arch)
            print(f"  {arch}: loaded from personal library "
                  f"(\"{deck_names[arch]}\")")
        else:
            # Fall back to scraped library
            deck = resolve_deck_from_library(arch)
            if deck:
                my_decks[arch] = deck
                deck_names[arch] = "scraped"
                print(f"  {arch}: loaded from deck_library.json (scraped)")
            else:
                print(f"  {arch}: WARNING - no decklist found, skipping")

    if not my_decks:
        print("ERROR: No valid decklists found for any archetype.",
              file=sys.stderr)
        sys.exit(1)

    # --- Build opponent list ---
    exclude = set(my_decks.keys())
    opponents, skipped = build_opponent_list(
        scoped, exclude, min_plays=args.min_plays
    )

    if not opponents:
        print("ERROR: No valid opponents could be built from meta data.",
              file=sys.stderr)
        sys.exit(1)

    print(f"Opponents: {len(opponents)} archetypes "
          f"({len(skipped)} skipped)")

    # --- Evaluate ---
    print(f"\nEvaluating {len(my_decks)} archetypes vs {len(opponents)} "
          f"opponents ({args.games} games/matchup)...")

    eval_results = evaluate_archetypes(
        my_decks=my_decks,
        opponents=opponents,
        pilot_path=args.pilot,
        games_per_eval=args.games,
        n_workers=args.workers,
    )

    # --- Classify meta ---
    threats, sleepers, insufficient = classify_archetypes(
        scoped,
        scoped_result.median_times_played,
        min_plays=args.min_plays,
    )

    # --- Optimization ---
    optimization = None
    optimize_archetype = None
    personal_pool_size = scraped_pool_size = general_pool_size = 0

    if args.optimize and eval_results:
        best = max(eval_results.items(), key=lambda x: x[1]["etwr"])
        optimize_archetype = best[0]
        best_deck = my_decks[optimize_archetype]

        # Count pool sizes for the report
        personal_cards = collect_personal_pool_cards(
            personal_lib, optimize_archetype
        )
        general_cards = set(get_general_pool(personal_lib))
        personal_pool_size = len(personal_cards - set(best_deck))
        general_pool_size = len(general_cards - personal_cards - set(best_deck))

        # Scraped pool size estimated from archetype decklists
        if _DECK_LIBRARY_PATH.exists():
            with open(_DECK_LIBRARY_PATH, "r", encoding="utf-8") as f:
                lib = json.load(f)
            scraped_cards: set = set()
            arch_data = lib.get("archetypes", {}).get(optimize_archetype, {})
            for dl in arch_data.get("decklists", []):
                raw = dl.get("decklist", "[]")
                try:
                    ids = json.loads(raw) if isinstance(raw, str) else raw
                    scraped_cards.update(ids)
                except (json.JSONDecodeError, TypeError):
                    pass
            scraped_pool_size = len(
                scraped_cards - personal_cards - general_cards - set(best_deck)
            )

        print(f"\nOptimizing {optimize_archetype} for this meta "
              f"({args.optimize_episodes} episodes)...")

        optimization = run_optimization(
            archetype=optimize_archetype,
            deck=best_deck,
            opponents=opponents,
            personal_library=personal_lib,
            pilot_path=args.pilot,
            episodes=args.optimize_episodes,
            n_workers=args.workers,
        )

    # --- Print report ---
    print_report(
        store_name=store.name,
        since_date=since_date,
        total_results=scoped_result.total_results,
        median_plays=scoped_result.median_times_played,
        mean_plays=scoped_result.mean_times_played,
        eval_results=eval_results,
        deck_names=deck_names,
        threats=threats,
        sleepers=sleepers,
        insufficient=insufficient,
        skipped_opponents=skipped,
        optimization=optimization,
        optimize_archetype=optimize_archetype,
        personal_pool_size=personal_pool_size,
        scraped_pool_size=scraped_pool_size,
        general_pool_size=general_pool_size,
    )


if __name__ == "__main__":
    main()
