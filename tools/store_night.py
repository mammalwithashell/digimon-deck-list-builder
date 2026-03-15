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
# Player scouting (Features 1, 2, 3)
# ---------------------------------------------------------------------------

def compute_player_loyalty(
    players: list,
    min_events: int = 3,
) -> List[dict]:
    """Compute archetype loyalty for each player.

    Args:
        players: List of PlayerSummary objects.
        min_events: Minimum events to include a player.

    Returns list of dicts sorted by event count descending.
    """
    results = []
    for p in players:
        if p.total_results < min_events:
            continue
        top_arch = max(p.archetypes_played.items(), key=lambda x: x[1])
        loyalty_pct = top_arch[1] / p.total_results
        history = sorted(
            p.archetypes_played.items(), key=lambda x: x[1], reverse=True
        )
        results.append({
            "player_name": p.player_name,
            "primary_archetype": top_arch[0],
            "loyalty_pct": loyalty_pct,
            "event_count": p.total_results,
            "last_seen": p.last_seen,
            "archetype_history": history,
        })
    return sorted(results, key=lambda x: x["event_count"], reverse=True)


def compute_player_threat_profiles(
    players: list,
    min_events: int = 3,
) -> List[dict]:
    """Compute threat profiles for players at a store.

    Combines win rate with attendance regularity to produce a threat score.
    """
    profiles = []
    for p in players:
        if p.total_results < min_events:
            continue

        total_wins = sum(r.wins for r in p.results)
        total_losses = sum(r.losses for r in p.results)
        total_games = total_wins + total_losses
        overall_wr = total_wins / total_games if total_games > 0 else 0.0

        # Most likely archetype + its win rate
        top_arch = max(p.archetypes_played.items(), key=lambda x: x[1])[0]
        arch_results = [r for r in p.results if r.archetype_name == top_arch]
        arch_wins = sum(r.wins for r in arch_results)
        arch_losses = sum(r.losses for r in arch_results)
        arch_games = arch_wins + arch_losses
        arch_wr = arch_wins / arch_games if arch_games > 0 else 0.0

        # Threat = win_rate * sqrt(events) to reward both skill and consistency
        import math
        threat_score = overall_wr * math.sqrt(p.total_results)

        profiles.append({
            "player_name": p.player_name,
            "likely_archetype": top_arch,
            "archetype_win_rate": arch_wr,
            "overall_win_rate": overall_wr,
            "total_games": total_games,
            "event_count": p.total_results,
            "threat_score": threat_score,
        })

    return sorted(profiles, key=lambda x: x["threat_score"], reverse=True)


def compute_attendance_profiles(
    players: list,
    total_events: int,
    min_events: int = 2,
) -> List[dict]:
    """Detect regular attendees vs one-time visitors.

    regularity_score = events_attended / total_events_in_range
    """
    profiles = []
    for p in players:
        if p.total_results < min_events:
            continue
        dates = [r.event_date for r in p.results if r.event_date]
        first_seen = min(dates) if dates else None
        last_seen = max(dates) if dates else None
        # Use distinct tournament dates as event count
        distinct_dates = len(set(dates))
        reg_score = distinct_dates / total_events if total_events > 0 else 0.0

        profiles.append({
            "player_name": p.player_name,
            "event_count": distinct_dates,
            "first_seen": first_seen,
            "last_seen": last_seen,
            "regularity_score": reg_score,
            "is_regular": reg_score >= 0.25,  # attended >= 25% of events
        })

    return sorted(profiles, key=lambda x: x["event_count"], reverse=True)


# ---------------------------------------------------------------------------
# Meta dynamics (Features 7, 9)
# ---------------------------------------------------------------------------

def compute_meta_velocity(
    period_metas: list,
) -> Dict[str, float]:
    """Compute share delta per archetype between first and last period.

    Returns dict of archetype -> delta (positive = rising).
    """
    if len(period_metas) < 2:
        return {}

    first = period_metas[0].archetypes
    last = period_metas[-1].archetypes
    all_archs = set(first) | set(last)

    velocity = {}
    for arch in all_archs:
        first_share = first[arch].meta_share if arch in first else 0.0
        last_share = last[arch].meta_share if arch in last else 0.0
        velocity[arch] = last_share - first_share

    return velocity


def compute_meta_comparison(
    store_metas: Dict[str, Any],
) -> List[dict]:
    """Compare meta shares across stores side-by-side.

    Args:
        store_metas: Dict of store_name -> ScopedMetaResult.

    Returns list of dicts with per-archetype per-store shares + delta.
    """
    store_names = list(store_metas.keys())
    if len(store_names) < 2:
        return []

    all_archs: set = set()
    for result in store_metas.values():
        all_archs.update(result.archetypes.keys())

    rows = []
    for arch in sorted(all_archs):
        shares = {}
        for sname, result in store_metas.items():
            stats = result.archetypes.get(arch)
            shares[sname] = stats.meta_share if stats else 0.0
        values = list(shares.values())
        delta = max(values) - min(values)
        rows.append({
            "archetype": arch,
            "shares": shares,
            "delta": delta,
        })

    return sorted(rows, key=lambda x: x["delta"], reverse=True)


# ---------------------------------------------------------------------------
# Color heatmap (Feature 11)
# ---------------------------------------------------------------------------

def format_color_heatmap(distributions: list) -> str:
    """Format color distribution as an ASCII table."""
    if not distributions:
        return "  No color data available."

    lines = []
    lines.append(f"  {'Color Pair':<30} {'Count':>6} {'Share':>7}")
    lines.append(f"  {'-' * 30} {'-' * 6} {'-' * 7}")
    for d in distributions[:15]:
        pair = d.primary_color
        if d.secondary_color:
            pair += f" / {d.secondary_color}"
        lines.append(f"  {pair:<30} {d.count:>6} {d.share * 100:>6.1f}%")
    return "\n".join(lines)


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
    player_loyalty: Optional[List[dict]] = None,
    player_threats: Optional[List[dict]] = None,
    attendance_profiles: Optional[List[dict]] = None,
    meta_velocity: Optional[Dict[str, float]] = None,
    period_metas: Optional[list] = None,
    color_distributions: Optional[list] = None,
    normalized_meta: Optional[Any] = None,
    decklist_reports: Optional[Dict[str, dict]] = None,
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
        header = f"  {'Archetype':<22} {'Share':>6} {'WR':>6} {'Conv':>6} {'Plays':>6}"
        if normalized_meta:
            header += f" {'NConv':>6}"
        print(header)
        sep = f"  {'-' * 22} {'-' * 6} {'-' * 6} {'-' * 6} {'-' * 6}"
        if normalized_meta:
            sep += f" {'-' * 6}"
        print(sep)
        for t in threats[:15]:
            line = (
                f"  {t['name']:<22} {t['meta_share'] * 100:>5.1f}% "
                f"{t['win_rate'] * 100:>5.1f}% {t['conversion_rate'] * 100:>5.1f}% "
                f"{t['times_played']:>6}"
            )
            if normalized_meta:
                ns = normalized_meta.archetypes.get(t["name"])
                nconv = ns.conversion_rate if ns else 0.0
                line += f" {nconv * 100:>5.1f}%"
            print(line)

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

    # --- Meta trends (Feature 7) ---
    if meta_velocity and period_metas:
        print(f"\n  META TRENDS ({len(period_metas)} periods):")
        print(f"  {'Archetype':<22} {'Delta':>7}  Periods")
        print(f"  {'-' * 22} {'-' * 7}  {'-' * 40}")
        # Show top risers and fallers
        sorted_vel = sorted(meta_velocity.items(), key=lambda x: abs(x[1]), reverse=True)
        for arch, delta in sorted_vel[:10]:
            arrow = "+" if delta > 0 else ""
            period_shares = []
            for pm in period_metas:
                s = pm.archetypes.get(arch)
                period_shares.append(f"{s.meta_share * 100:.0f}%" if s else "  -")
            print(f"  {arch:<22} {arrow}{delta * 100:>5.1f}%  {'->'.join(period_shares)}")

    # --- Player scouting (Features 1, 2, 3) ---
    if player_threats:
        print(f"\n  TOP THREATS BY PLAYER:")
        print(f"  {'Player':<18} {'Likely Deck':<18} {'WR':>6} {'Games':>6} {'Events':>6} {'Threat':>7}")
        print(f"  {'-' * 18} {'-' * 18} {'-' * 6} {'-' * 6} {'-' * 6} {'-' * 7}")
        for pt in player_threats[:10]:
            print(
                f"  {pt['player_name']:<18} {pt['likely_archetype']:<18} "
                f"{pt['overall_win_rate'] * 100:>5.1f}% {pt['total_games']:>6} "
                f"{pt['event_count']:>6} {pt['threat_score']:>6.1f}"
            )

    if player_loyalty:
        print(f"\n  PLAYER ARCHETYPE LOYALTY:")
        print(f"  {'Player':<18} {'Primary':<18} {'Loyalty':>7} {'Events':>6}  History")
        print(f"  {'-' * 18} {'-' * 18} {'-' * 7} {'-' * 6}  {'-' * 30}")
        for pl in player_loyalty[:10]:
            history_str = ", ".join(
                f"{a}({c})" for a, c in pl["archetype_history"][:3]
            )
            print(
                f"  {pl['player_name']:<18} {pl['primary_archetype']:<18} "
                f"{pl['loyalty_pct'] * 100:>5.0f}%  {pl['event_count']:>5}  {history_str}"
            )

    if attendance_profiles:
        regulars = [p for p in attendance_profiles if p["is_regular"]]
        if regulars:
            print(f"\n  REGULARS ({len(regulars)} players attending >= 25% of events):")
            for ap in regulars[:10]:
                print(
                    f"  {ap['player_name']:<18} {ap['event_count']} events  "
                    f"(first: {ap['first_seen'] or '?'}, last: {ap['last_seen'] or '?'}, "
                    f"regularity: {ap['regularity_score'] * 100:.0f}%)"
                )

    # --- Color heatmap (Feature 11) ---
    if color_distributions:
        print(f"\n  COLOR DISTRIBUTION:")
        print(format_color_heatmap(color_distributions))

    # --- Decklist analysis (Features 4, 5, 6) ---
    if decklist_reports:
        for arch_name, report in decklist_reports.items():
            print(f"\n  DECKLIST ANALYSIS: {arch_name} ({report.get('list_count', 0)} lists)")

            staples = report.get("staples", [])
            if staples:
                print(f"    Card Staples (>80% inclusion):")
                for cf in staples[:10]:
                    print(f"      {cf.card_id:<16} {cf.inclusion_rate * 100:>5.1f}%  "
                          f"avg {cf.avg_copies:.1f} copies")

            diffs = report.get("winning_tech", [])
            if diffs:
                print(f"    Winning Tech (top-4 vs rest):")
                for cd in diffs[:8]:
                    sign = "+" if cd.differential > 0 else ""
                    print(f"      {cd.card_id:<16} winners: {cd.winner_inclusion * 100:>5.1f}%  "
                          f"others: {cd.other_inclusion * 100:>5.1f}%  "
                          f"({sign}{cd.differential * 100:.1f}%)")

            trends = report.get("trends", [])
            if trends:
                rising = [t for t in trends if t.trend_slope > 0.05][:5]
                falling = [t for t in trends if t.trend_slope < -0.05][:5]
                if rising:
                    print(f"    Rising Cards:")
                    for ct in rising:
                        print(f"      {ct.card_id:<16} {ct.current_rate * 100:>5.1f}%  "
                              f"(+{ct.trend_slope * 100:.1f}%/period)")
                if falling:
                    print(f"    Falling Cards:")
                    for ct in falling:
                        print(f"      {ct.card_id:<16} {ct.current_rate * 100:>5.1f}%  "
                              f"({ct.trend_slope * 100:.1f}%/period)")

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

def print_meta_comparison(store_metas: Dict[str, Any]) -> None:
    """Print side-by-side meta comparison for multiple stores."""
    comparison = compute_meta_comparison(store_metas)
    if not comparison:
        print("  No comparison data.")
        return

    store_names = list(store_metas.keys())
    header = f"  {'Archetype':<22}"
    for sn in store_names:
        header += f" {sn[:12]:>12}"
    header += f" {'Delta':>7}"
    print(header)
    print(f"  {'-' * 22}" + f" {'-' * 12}" * len(store_names) + f" {'-' * 7}")

    for row in comparison[:20]:
        line = f"  {row['archetype']:<22}"
        for sn in store_names:
            share = row["shares"].get(sn, 0.0)
            line += f" {share * 100:>11.1f}%"
        line += f" {row['delta'] * 100:>6.1f}%"
        print(line)


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

  # Full analysis with player scouting, trends, and decklist analysis
  python tools/store_night.py --store "The Card Haven" \\
      --archetypes "Rocks" --players --trends --decklists --colors

  # Compare two stores
  python tools/store_night.py --store "The Card Haven" \\
      --archetypes "Rocks" --compare-stores "Boardwalk Games"

  # Filter to locals only
  python tools/store_night.py --store "The Card Haven" \\
      --archetypes "Rocks" --event-type locals
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
        "--event-type", default=None,
        help='Filter by event type (e.g. "locals", "regional")',
    )
    parser.add_argument(
        "--players", action="store_true",
        help="Enable player scouting (loyalty, skill, attendance)",
    )
    parser.add_argument(
        "--trends", action="store_true",
        help="Show meta velocity / trend analysis",
    )
    parser.add_argument(
        "--normalize", action="store_true",
        help="Show tournament-size-normalized conversion rates",
    )
    parser.add_argument(
        "--colors", action="store_true",
        help="Show color distribution heatmap",
    )
    parser.add_argument(
        "--decklists", action="store_true",
        help="Run decklist analysis (card frequency, winning tech, trends)",
    )
    parser.add_argument(
        "--compare-stores", default=None,
        help="Comma-separated additional store names for side-by-side comparison",
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
    from digimon_gym.digilab_client import (
        list_stores, get_scoped_meta, get_scoped_meta_normalized,
        get_player_history, get_color_distribution, get_meta_over_time,
        get_decklists_for_archetype,
    )

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
    if args.event_type:
        print(f"Event type filter: {args.event_type}")

    scope_kwargs = {
        "store_ids": [store.store_id],
        "since_date": since_date,
        "event_type": args.event_type,
    }

    # --- Get scoped meta ---
    scoped_result = get_scoped_meta(**scope_kwargs)
    scoped = scoped_result.archetypes

    if not scoped:
        print("ERROR: No tournament data found for this store/date range.",
              file=sys.stderr)
        sys.exit(1)

    print(f"Meta: {scoped_result.total_results} results, "
          f"{len(scoped)} archetypes")

    # --- Cross-store comparison (Feature 9) ---
    if args.compare_stores:
        compare_names = [n.strip() for n in args.compare_stores.split(",")]
        store_metas: Dict[str, Any] = {store.name: scoped_result}
        for cname in compare_names:
            cs = store_map.get(cname.lower())
            if cs:
                cm = get_scoped_meta(
                    store_ids=[cs.store_id],
                    since_date=since_date,
                    event_type=args.event_type,
                )
                store_metas[cs.name] = cm
            else:
                print(f"  Warning: comparison store not found: {cname}")

        if len(store_metas) >= 2:
            print(f"\n{'=' * 70}")
            print(f"  CROSS-STORE META COMPARISON")
            print(f"{'=' * 70}")
            print_meta_comparison(store_metas)

    # --- Resolve decks for your archetypes ---
    my_decks: Dict[str, List[str]] = {}
    deck_names: Dict[str, str] = {}

    for arch in archetypes:
        deck = resolve_deck_from_personal(personal_lib, arch)
        if deck:
            my_decks[arch] = deck
            deck_names[arch] = get_personal_deck_name(personal_lib, arch)
            print(f"  {arch}: loaded from personal library "
                  f"(\"{deck_names[arch]}\")")
        else:
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

    # --- Player scouting (Features 1, 2, 3) ---
    player_loyalty_data = None
    player_threat_data = None
    attendance_data = None

    if args.players:
        print("Fetching player history...")
        players = get_player_history(**scope_kwargs)
        if players:
            player_loyalty_data = compute_player_loyalty(players, min_events=2)
            player_threat_data = compute_player_threat_profiles(players, min_events=2)
            attendance_data = compute_attendance_profiles(
                players,
                total_events=store.tournament_count,
                min_events=2,
            )
            print(f"  {len(players)} players found")
        else:
            print("  No player data available (DB may lack player column)")

    # --- Meta trends (Feature 7) ---
    velocity = None
    period_metas = None
    if args.trends:
        print("Computing meta trends...")
        period_metas = get_meta_over_time(**scope_kwargs, periods=3)
        if period_metas:
            velocity = compute_meta_velocity(period_metas)

    # --- Tournament size normalization (Feature 8) ---
    normalized_meta = None
    if args.normalize:
        print("Computing size-normalized conversion rates...")
        normalized_meta = get_scoped_meta_normalized(**scope_kwargs)

    # --- Color distribution (Feature 11) ---
    color_dist = None
    if args.colors:
        color_dist = get_color_distribution(**scope_kwargs)

    # --- Decklist analysis (Features 4, 5, 6) ---
    decklist_reports = None
    if args.decklists:
        from tools.decklist_analysis import (
            compute_card_frequencies, compute_winning_differentials,
            compute_card_trends,
        )
        decklist_reports = {}
        # Analyze top threats
        top_threat_names = [t["name"] for t in threats[:5]]
        for arch_name in top_threat_names:
            print(f"Fetching decklists for {arch_name}...")
            records = get_decklists_for_archetype(arch_name, **scope_kwargs)
            if not records:
                continue

            freqs = compute_card_frequencies(records)
            staples = [f for f in freqs if f.inclusion_rate >= 0.80]
            diffs = compute_winning_differentials(records)
            card_trends = compute_card_trends(records)

            decklist_reports[arch_name] = {
                "list_count": len(records),
                "staples": staples,
                "winning_tech": diffs[:10] if diffs else [],
                "trends": card_trends,
            }

    # --- Optimization ---
    optimization = None
    optimize_archetype = None
    personal_pool_size = scraped_pool_size = general_pool_size = 0

    if args.optimize and eval_results:
        best = max(eval_results.items(), key=lambda x: x[1]["etwr"])
        optimize_archetype = best[0]
        best_deck = my_decks[optimize_archetype]

        personal_cards = collect_personal_pool_cards(
            personal_lib, optimize_archetype
        )
        general_cards = set(get_general_pool(personal_lib))
        personal_pool_size = len(personal_cards - set(best_deck))
        general_pool_size = len(general_cards - personal_cards - set(best_deck))

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
        player_loyalty=player_loyalty_data,
        player_threats=player_threat_data,
        attendance_profiles=attendance_data,
        meta_velocity=velocity,
        period_metas=period_metas,
        color_distributions=color_dist,
        normalized_meta=normalized_meta,
        decklist_reports=decklist_reports,
    )


if __name__ == "__main__":
    main()
