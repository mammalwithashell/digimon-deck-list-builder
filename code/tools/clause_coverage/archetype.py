"""Resolve an archetype to the cards a campaign is about.

`data/deck_library.json` stores `archetypes` as a dict keyed by archetype name.
Each entry's `decklists` is a list of tournament entries, and each entry's
`decklist` field is a **JSON-encoded string** of card ids with duplicates for
copies -- so it must be `json.loads`-ed, not iterated.

The competitive core is a FRACTION of the archetype's recorded lists, never a
raw count. The published Toho Braves report describes its core as ">=33 of 45
lists"; hardcoding 33 would silently redefine the core for an archetype with a
different corpus size. 0.7 reproduces that figure's *card set* exactly (the
same 18 cards) -- though the true threshold is `ceil(45 * 0.7) = 32`, not the
report's rounded "33"; `test_real_library_reproduces_the_published_toho_figures`
pins the computed 32 alongside the 18-card core.

Standard library only.
"""

from __future__ import annotations

import json
from collections import Counter
from difflib import get_close_matches
from math import ceil
from pathlib import Path

#: A card is "core" if it appears in at least this fraction of the lists.
DEFAULT_CORE_FRACTION = 0.7


def load_archetypes(path: Path | str) -> dict:
    """Load `deck_library.json` -> ``{archetype_name: entry}``."""
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    archetypes = data.get("archetypes")
    if not isinstance(archetypes, dict):
        raise ValueError(f"{path}: expected an 'archetypes' object")
    return archetypes


def resolve(library: dict, name: str) -> str:
    """Canonical archetype name, case-insensitively.

    An unknown name raises ``LookupError`` **with near-misses**: a bare "not
    found" makes a caller guess, and a campaign dispatched at a misspelled
    archetype would otherwise resolve to nothing and report an empty plan as
    though the work were done.
    """
    if name in library:
        return name
    lowered = {k.lower(): k for k in library}
    if name.lower() in lowered:
        return lowered[name.lower()]
    close = get_close_matches(name, list(library), n=5, cutoff=0.6)
    hint = f" Did you mean: {', '.join(close)}?" if close else ""
    raise LookupError(f"no archetype named {name!r}.{hint}")


def _lists(entry: dict) -> list[list[str]]:
    """Every decklist as a list of card ids (duplicates preserved)."""
    out: list[list[str]] = []
    for dl in entry.get("decklists") or []:
        raw = dl.get("decklist")
        if not raw:
            continue
        if isinstance(raw, str):
            try:
                cards = json.loads(raw)
            except json.JSONDecodeError:
                continue
        else:
            cards = raw
        if isinstance(cards, list):
            out.append([c for c in cards if isinstance(c, str)])
    return out


def card_frequency(entry: dict) -> dict[str, int]:
    """``card_id -> how many LISTS contain it`` (copies within a list count once)."""
    counts: Counter[str] = Counter()
    for cards in _lists(entry):
        counts.update(set(cards))
    return dict(counts)


def pool(entry: dict) -> list[str]:
    """Every distinct card the archetype has played, sorted."""
    return sorted(card_frequency(entry))


def core(entry: dict, fraction: float = DEFAULT_CORE_FRACTION) -> dict:
    """The competitive core, plus the threshold and denominator it used.

    Returning the threshold and list count is not decoration: a report has to
    print ">=N of M lists", and a caller given only a card list would have to
    recompute them -- which is how a report ends up quoting a fraction it did
    not actually apply. The threshold is ``ceil(list_count * fraction)``.
    """
    lists = _lists(entry)
    list_count = len(lists)
    threshold = ceil(list_count * fraction) if list_count else 0
    freq = card_frequency(entry)
    return {
        "cards": sorted(c for c, n in freq.items() if n >= threshold) if list_count else [],
        "threshold": threshold,
        "list_count": list_count,
        "fraction": fraction,
    }
