"""Map a user deck to a meta tier using tournament-archetype fingerprints.

Consumes the `deck_library.json` shape produced by `tools/meta_loader.py`
(per-archetype `meta_share` + `decklists`) and uses
`tools.decklist_analysis.compute_card_frequencies` to build a staple
fingerprint per archetype. A user deck is matched to the best-scoring
archetype by normalized staple coverage; the archetype's meta_share is then
mapped to a tier via `share_to_tier`.

The classifier is pure — it depends only on the in-memory library — so it can
be called synchronously from DB save paths without network I/O. The library
can be rebuilt from `deck_library.json` on app startup and reused.
"""
from __future__ import annotations

import json
import math
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterable, Literal, Optional

from tools.decklist_analysis import (
    compute_card_frequencies,
    digilab_json_to_card_counts,
)

MetaTier = Literal["meta", "rogue", "jank"]


# Defaults chosen from deck_library.json distribution (173 archetypes, top
# share 7.3%, 28 archetypes ≥1%, median 0.1%). "meta" here means "you'll
# regularly face this in a competitive room".
DEFAULT_META_THRESHOLD = 0.02
DEFAULT_ROGUE_THRESHOLD = 0.005
DEFAULT_STAPLE_MIN_RATE = 0.6
DEFAULT_MIN_MATCH_COVERAGE = 0.5


@dataclass(frozen=True)
class ArchetypeFingerprint:
    """Per-archetype staple profile distilled from tournament decklists."""

    name: str
    meta_share: float
    times_played: int
    staples: dict[str, float]  # card_id → inclusion rate across this archetype's lists


@dataclass
class ClassifierLibrary:
    """Precomputed fingerprints + tier thresholds, ready to classify against."""

    archetypes: list[ArchetypeFingerprint]
    meta_threshold: float = DEFAULT_META_THRESHOLD
    rogue_threshold: float = DEFAULT_ROGUE_THRESHOLD
    min_match_coverage: float = DEFAULT_MIN_MATCH_COVERAGE


def share_to_tier(
    meta_share: float,
    *,
    meta_threshold: float = DEFAULT_META_THRESHOLD,
    rogue_threshold: float = DEFAULT_ROGUE_THRESHOLD,
) -> MetaTier:
    if meta_share >= meta_threshold:
        return "meta"
    if meta_share >= rogue_threshold:
        return "rogue"
    return "jank"


def _parse_decklist_field(raw: Any) -> list[str]:
    """deck_library.json stores the card list either as a JSON-encoded string
    or as a pre-parsed list. Accept both and always return list[str]."""
    if isinstance(raw, str):
        try:
            parsed = json.loads(raw)
        except json.JSONDecodeError:
            return []
        return [str(c) for c in parsed] if isinstance(parsed, list) else []
    if isinstance(raw, list):
        return [str(c) for c in raw]
    return []


class _DecklistRecord:
    """Duck-typed stand-in for DigiLab's DecklistRecord — exposes the
    decklist_json shape `compute_card_frequencies` expects."""

    __slots__ = ("decklist_json", "is_top_cut", "event_date")

    def __init__(self, card_ids: list[str]) -> None:
        # compute_card_frequencies → digilab_json_to_card_counts reads the
        # structured {"digimon":[{count, set, number}], ...} form. We already
        # have raw card IDs, so bypass that path and set plain counts.
        self.decklist_json = _card_ids_to_digilab_json(card_ids)
        self.is_top_cut = False
        self.event_date = None


def _card_ids_to_digilab_json(card_ids: list[str]) -> dict:
    """Pack flat card IDs into the digilab_json_to_card_counts input shape."""
    buckets: dict[str, list[dict]] = {"digimon": [], "tamer": [], "option": [], "egg": []}
    counts: dict[str, int] = {}
    for cid in card_ids:
        counts[cid] = counts.get(cid, 0) + 1
    for cid, n in counts.items():
        # The bucket key doesn't matter — digilab_json_to_card_counts sums
        # across all of them — but it must be one of the known keys.
        set_code, _, number = cid.partition("-")
        buckets["digimon"].append({"count": n, "set": set_code, "number": number})
    return buckets


def build_library_from_deck_library(
    doc: dict,
    *,
    meta_threshold: float = DEFAULT_META_THRESHOLD,
    rogue_threshold: float = DEFAULT_ROGUE_THRESHOLD,
    staple_min_rate: float = DEFAULT_STAPLE_MIN_RATE,
    min_match_coverage: float = DEFAULT_MIN_MATCH_COVERAGE,
) -> ClassifierLibrary:
    """Distill the deck_library.json document into classifier fingerprints.

    Archetypes with no decklists are skipped (nothing to fingerprint against).
    Staples are cards with inclusion rate ≥ `staple_min_rate` across the
    archetype's decklists.
    """
    archetypes: list[ArchetypeFingerprint] = []
    for name, entry in (doc.get("archetypes") or {}).items():
        stats = entry.get("stats") or {}
        meta_share = float(stats.get("meta_share", 0.0))
        times_played = int(stats.get("times_played", 0))
        raw_lists = entry.get("decklists") or []
        parsed = [_parse_decklist_field(r.get("decklist")) for r in raw_lists]
        parsed = [p for p in parsed if p]
        if not parsed:
            continue

        records = [_DecklistRecord(p) for p in parsed]
        freqs = compute_card_frequencies(records)
        staples = {
            f.card_id: f.inclusion_rate
            for f in freqs
            if f.inclusion_rate >= staple_min_rate
        }
        if not staples:
            # No card is a staple at this threshold — likely very diverse
            # or single-list archetype. Fall back to any card seen.
            staples = {f.card_id: f.inclusion_rate for f in freqs}
            if not staples:
                continue

        archetypes.append(
            ArchetypeFingerprint(
                name=entry.get("archetype_name", name),
                meta_share=meta_share,
                times_played=times_played,
                staples=staples,
            )
        )

    archetypes.sort(key=lambda a: (-a.meta_share, -a.times_played, a.name))
    return ClassifierLibrary(
        archetypes=archetypes,
        meta_threshold=meta_threshold,
        rogue_threshold=rogue_threshold,
        min_match_coverage=min_match_coverage,
    )


def load_library_from_path(
    path: str | Path | None = None,
    **kwargs: Any,
) -> ClassifierLibrary:
    """Load `deck_library.json` from disk and build a library."""
    if path is None:
        path = (
            Path(__file__).resolve().parent.parent
            / "engine" / "data" / "deck_library.json"
        )
    with open(path, "r", encoding="utf-8") as f:
        doc = json.load(f)
    return build_library_from_deck_library(doc, **kwargs)


def classify_deck(
    card_ids: Iterable[str],
    library: ClassifierLibrary,
) -> tuple[Optional[str], MetaTier, float]:
    """Return (matched_archetype, tier, confidence) for a user deck.

    - matched_archetype: canonical name, or None if no archetype met the
      `min_match_coverage` threshold.
    - tier: "meta" / "rogue" from matched archetype's meta_share, or "jank"
      if no match (custom/homebrew) or matched share below rogue_threshold.
    - confidence: normalized staple-coverage score ∈ [0, 1].
    """
    unique = {str(c) for c in card_ids}
    if not unique or not library.archetypes:
        return (None, "jank", 0.0)

    best: tuple[float, Optional[ArchetypeFingerprint]] = (0.0, None)
    for fp in library.archetypes:
        coverage = _coverage(unique, fp)
        # Tie-break bias: well-sampled archetypes are more trustworthy matches
        # at equal coverage. sqrt(times_played) caps the tie-break weight.
        score = coverage * (1.0 + 0.01 * math.sqrt(max(fp.times_played, 1)))
        if score > best[0]:
            best = (score, fp)

    score, fp = best
    if fp is None or _coverage(unique, fp) < library.min_match_coverage:
        return (None, "jank", 0.0)

    coverage = _coverage(unique, fp)
    tier = share_to_tier(
        fp.meta_share,
        meta_threshold=library.meta_threshold,
        rogue_threshold=library.rogue_threshold,
    )
    return (fp.name, tier, round(coverage, 3))


def _coverage(deck_cards: set[str], fp: ArchetypeFingerprint) -> float:
    if not fp.staples:
        return 0.0
    # Weight staples by their inclusion rate — rare "almost-staples" count less
    # than must-includes, so a deck matching the must-includes scores close to 1.
    hit = sum(rate for cid, rate in fp.staples.items() if cid in deck_cards)
    total = sum(fp.staples.values())
    return hit / total if total else 0.0
