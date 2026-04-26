"""Deck-save hook that tags a deck's meta_tier + meta_archetype.

The library is built once per process on first call (a few tens of ms; keeps
the API process hot path free of repeated JSON parsing). Callers should be
fault-tolerant: if anything goes wrong, the deck still saves with NULL tier
and matchmaking treats it as unclassified (jank for casual filtering).
"""
from __future__ import annotations

import logging
from threading import Lock
from typing import Iterable, Optional

from server.classifier.meta_tier import (
    ClassifierLibrary,
    MetaTier,
    classify_deck,
    load_library_from_path,
)

logger = logging.getLogger(__name__)

_library: Optional[ClassifierLibrary] = None
_library_lock = Lock()


def get_library() -> Optional[ClassifierLibrary]:
    """Return the process-wide classifier library, building it on first call.
    Returns None if the deck_library.json file is missing or malformed."""
    global _library
    if _library is not None:
        return _library
    with _library_lock:
        if _library is None:
            try:
                _library = load_library_from_path()
                logger.info(
                    "meta_tier classifier: loaded %d archetype fingerprints",
                    len(_library.archetypes),
                )
            except (FileNotFoundError, ValueError) as exc:
                logger.warning("meta_tier classifier unavailable: %s", exc)
                return None
    return _library


def tag_deck(card_ids: Iterable[str]) -> tuple[Optional[str], Optional[MetaTier]]:
    """Classify a deck. Returns (archetype, tier) — both None if the
    classifier is unavailable or raised unexpectedly."""
    lib = get_library()
    if lib is None:
        return (None, None)
    try:
        arch, tier, _ = classify_deck(list(card_ids), lib)
    except Exception:
        logger.exception("meta_tier classification failed; leaving deck untagged")
        return (None, None)
    return (arch, tier)


def reset_library_cache() -> None:
    """Clear the cached library — used by tests and by any future endpoint
    that reloads tournament data."""
    global _library
    with _library_lock:
        _library = None
