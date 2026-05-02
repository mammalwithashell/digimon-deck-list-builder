"""Decklist analysis — card frequency, winning differentials, and trends.

Pure analysis functions that operate on DecklistRecord objects from digilab_client.
No DB imports, no network calls.
"""

from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple


@dataclass
class CardFrequency:
    """Per-card frequency stats across a set of decklists."""

    card_id: str
    inclusion_rate: float = 0.0
    avg_copies: float = 0.0
    total_lists: int = 0


@dataclass
class CardDifferential:
    """Difference in card usage between winners and non-winners."""

    card_id: str
    winner_inclusion: float = 0.0
    other_inclusion: float = 0.0
    differential: float = 0.0
    winner_avg_copies: float = 0.0
    other_avg_copies: float = 0.0


@dataclass
class CardTrend:
    """Card inclusion trend over time periods."""

    card_id: str
    period_rates: List[float] = field(default_factory=list)
    trend_slope: float = 0.0
    current_rate: float = 0.0


def digilab_json_to_card_counts(decklist_json: Dict[str, Any]) -> Dict[str, int]:
    """Convert DigiLab structured decklist JSON to card count dict.

    DigiLab format::

        {"digimon": [{"count": 4, "name": "...", "set": "BT12", "number": "073"}, ...],
         "tamer": [...], "option": [...], "egg": [...]}
    """
    card_counts: Dict[str, int] = {}
    for category in ("egg", "digimon", "tamer", "option"):
        for entry in decklist_json.get(category, []):
            set_code = entry.get("set", "")
            number = entry.get("number", "")
            count = entry.get("count", 0)
            if not set_code or not number or not count:
                continue
            card_id = f"{set_code}-{number}"
            card_counts[card_id] = card_counts.get(card_id, 0) + count
    return card_counts


def compute_card_frequencies(decklists: list) -> List[CardFrequency]:
    """Compute per-card inclusion rate and average copy count.

    Args:
        decklists: List of DecklistRecord objects (or anything with .decklist_json).

    Returns:
        List of CardFrequency sorted by inclusion rate descending.
    """
    if not decklists:
        return []

    total_lists = len(decklists)
    card_appearances: Dict[str, int] = defaultdict(int)
    card_total_copies: Dict[str, int] = defaultdict(int)

    for dl in decklists:
        counts = digilab_json_to_card_counts(dl.decklist_json)
        for card_id, count in counts.items():
            card_appearances[card_id] += 1
            card_total_copies[card_id] += count

    results = []
    for card_id in card_appearances:
        results.append(CardFrequency(
            card_id=card_id,
            inclusion_rate=card_appearances[card_id] / total_lists,
            avg_copies=card_total_copies[card_id] / total_lists,
            total_lists=card_appearances[card_id],
        ))

    return sorted(results, key=lambda x: x.inclusion_rate, reverse=True)


def compute_winning_differentials(
    decklists: list,
    top_n: int = 4,
) -> List[CardDifferential]:
    """Compare card usage between top-N finishers and the rest.

    Args:
        decklists: List of DecklistRecord objects.
        top_n: Placement cutoff for "winners" (default top 4).

    Returns:
        List of CardDifferential sorted by differential descending.
    """
    winners = [d for d in decklists if d.placement is not None and d.placement <= top_n]
    others = [d for d in decklists if d.placement is None or d.placement > top_n]

    if not winners or not others:
        return []

    def _card_stats(group: list) -> Tuple[Dict[str, float], Dict[str, float]]:
        n = len(group)
        appearances: Dict[str, int] = defaultdict(int)
        copies: Dict[str, int] = defaultdict(int)
        for dl in group:
            counts = digilab_json_to_card_counts(dl.decklist_json)
            for cid, cnt in counts.items():
                appearances[cid] += 1
                copies[cid] += cnt
        rates = {cid: appearances[cid] / n for cid in appearances}
        avg_copies = {cid: copies[cid] / n for cid in copies}
        return rates, avg_copies

    w_rates, w_copies = _card_stats(winners)
    o_rates, o_copies = _card_stats(others)

    all_cards = set(w_rates) | set(o_rates)
    results = []
    for cid in all_cards:
        wi = w_rates.get(cid, 0.0)
        oi = o_rates.get(cid, 0.0)
        results.append(CardDifferential(
            card_id=cid,
            winner_inclusion=wi,
            other_inclusion=oi,
            differential=wi - oi,
            winner_avg_copies=w_copies.get(cid, 0.0),
            other_avg_copies=o_copies.get(cid, 0.0),
        ))

    return sorted(results, key=lambda x: x.differential, reverse=True)


def compute_card_trends(
    decklists: list,
    periods: int = 3,
) -> List[CardTrend]:
    """Track card inclusion rate changes over time periods.

    Args:
        decklists: List of DecklistRecord objects with .event_date.
        periods: Number of time buckets.

    Returns:
        List of CardTrend sorted by absolute trend_slope descending.
    """
    dated = [d for d in decklists if d.event_date is not None]
    if len(dated) < 2:
        return []

    # Sort by date and bucket
    dated.sort(key=lambda d: d.event_date)
    bucket_size = max(1, len(dated) // periods)
    buckets: List[list] = []
    for i in range(periods):
        start = i * bucket_size
        end = len(dated) if i == periods - 1 else (i + 1) * bucket_size
        buckets.append(dated[start:end])

    # Remove empty trailing buckets
    buckets = [b for b in buckets if b]
    if len(buckets) < 2:
        return []

    # Compute per-card inclusion rate per bucket
    all_cards: set = set()
    bucket_rates: List[Dict[str, float]] = []
    for bucket in buckets:
        n = len(bucket)
        appearances: Dict[str, int] = defaultdict(int)
        for dl in bucket:
            counts = digilab_json_to_card_counts(dl.decklist_json)
            for cid in counts:
                appearances[cid] += 1
        rates = {cid: appearances[cid] / n for cid in appearances}
        bucket_rates.append(rates)
        all_cards.update(appearances.keys())

    results = []
    n_buckets = len(bucket_rates)
    for cid in all_cards:
        period_rates = [br.get(cid, 0.0) for br in bucket_rates]
        # Simple linear slope: (last - first) / (n_buckets - 1)
        slope = (period_rates[-1] - period_rates[0]) / (n_buckets - 1)
        results.append(CardTrend(
            card_id=cid,
            period_rates=period_rates,
            trend_slope=slope,
            current_rate=period_rates[-1],
        ))

    return sorted(results, key=lambda x: abs(x.trend_slope), reverse=True)
