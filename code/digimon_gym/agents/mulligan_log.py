"""Per-game mulligan log writer + wrapper.

Captures starting hand + mulligan choice from the pilot seat during
training, appended live to `models/<run>/mulligan_log.jsonl`. See
`docs/superpowers/specs/2026-05-23-mulligan-log-design.md`.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any, Dict, List

from data_paths import CARDS_JSON


SCHEMA_VERSION = 1


def _load_card_metadata() -> Dict[str, Dict[str, Any]]:
    """Load cards.json once at module import; used by helpers below.

    On any I/O or parse failure, log once to stderr and return an empty
    dict so callers degrade gracefully (helpers return zero histograms
    and False for tamer) rather than crash training.
    """
    try:
        return json.loads(Path(CARDS_JSON).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(
            f"[mulligan_log] cards.json unavailable; hand features will be empty: {exc!r}",
            file=sys.stderr,
            flush=True,
        )
        return {}


_CARDS = _load_card_metadata()


def _derive_lvl_counts(card_ids: List[str]) -> Dict[str, int]:
    """Return a histogram of levels 3..7 for the given card IDs.

    Unknown card IDs and cards without a level field contribute 0 to every
    bucket. Only Digimon levels 3-7 are bucketed; eggs (level 2) and
    Options/Tamers are ignored here (use `_derive_has_tamer` for tamers).
    """
    buckets = {str(lvl): 0 for lvl in range(3, 8)}
    for cid in card_ids:
        lvl = _CARDS.get(cid, {}).get("level")
        if isinstance(lvl, int) and 3 <= lvl <= 7:
            buckets[str(lvl)] += 1
    return buckets


def _derive_has_tamer(card_ids: List[str]) -> bool:
    """True if any card in the list is a Tamer.

    cards.json encodes card type as ``card_kind`` (int): 0=Digimon, 1=Tamer,
    2=Option, 3=DigiEgg.  A value of 1 means Tamer.
    """
    for cid in card_ids:
        if _CARDS.get(cid, {}).get("card_kind") == 1:
            return True
    return False
