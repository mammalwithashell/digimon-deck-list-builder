"""The training gate for the Architect pipeline (final review I5, 2026-10-21).

The Architect TRAINING entrypoints (`architect_training`, `architect_cotraining`)
read `data/deck_library.json` and build decks outside `MetaGauntlet.load`. They
apply the same two layers (spec
2026-10-15 §5b, CLAUDE.md rule 34), from the same committed artifacts, with the
same errors:

1. Mechanics: `gauntlet.require_mechanics_ready()` -- `MechanicsNotReadyError`
   when an obligation is flagged `blocks_training`, `MechanicsReadinessMissingError`
   when the artifact is missing.
2. Per card: `data/card_admissibility.json` -- a library decklist holding a
   non-admissible card is skipped (`NotAdmissibleDeckError` when an archetype has
   decklists but none is admissible), and the mutation candidate pool keeps only
   admissible cards. `CardAdmissibilityMissingError` when the artifact is missing.

`MetaOptimizer`, `CandidatePool` and `_pick_best_decklist` are shared with
hosted product surfaces (`code/server/routers/deck_optimizer.py` `/evaluate` and
`/optimize`, `code/tools/store_night.py`), so there the gate is opt-in
(`training_gate=True`), passed only by the training entrypoints; the default is
the pre-gate, unfiltered behaviour (re-review N1, controller ruling 2026-10-21:
the two-layer gate is a TRAINING gate). Within training there is no override.
The artifacts are read once per process: the cache is keyed
on the gauntlet loader in effect, so the test suite's permissive conftest
fixtures (which monkeypatch those loaders) still apply.
"""
from __future__ import annotations

import json
import logging
from typing import Any, Iterable, List, Optional, Sequence

import digimon_gym.agents.gauntlet as gauntlet

logger = logging.getLogger(__name__)

_mechanics_checked_for: Optional[object] = None
_admissibility_cache: Optional[tuple[object, Any]] = None


def require_training_gate() -> None:
    """Layer 1, once per process (per mechanics loader)."""
    global _mechanics_checked_for
    loader = gauntlet._load_mechanics_readiness
    if _mechanics_checked_for is loader:
        return
    gauntlet.require_mechanics_ready()
    _mechanics_checked_for = loader


def card_admissibility() -> Any:
    """Layer 2's committed artifact, loaded once per process (per loader)."""
    global _admissibility_cache
    loader = gauntlet._load_card_admissibility
    if _admissibility_cache is None or _admissibility_cache[0] is not loader:
        _admissibility_cache = (loader, loader())
    return _admissibility_cache[1]


def admissible_card_ids(card_ids: Iterable[str]) -> set[str]:
    """The subset of `card_ids` that is admissible."""
    adm = card_admissibility()
    return {cid for cid in card_ids if adm.admissible(cid)}


def is_admissible_deck(deck: Sequence[str]) -> bool:
    adm = card_admissibility()
    return all(adm.admissible(cid) for cid in deck)


def _parse_decklist(entry: dict) -> List[str]:
    raw = entry.get("decklist")
    if not raw:
        return []
    try:
        deck = json.loads(raw) if isinstance(raw, str) else list(raw)
    except (json.JSONDecodeError, TypeError):
        return []
    return [str(c) for c in deck]


def first_admissible_decklist(decklists: Sequence[dict], *, label: str) -> List[str]:
    """The first non-empty decklist in `decklists` order whose cards are all
    admissible. Skipped lists are logged. Raises `NotAdmissibleDeckError`
    (naming the first list's blockers) when lists exist but none is admissible,
    and `ValueError` when there is no non-empty list at all."""
    require_training_gate()
    first: Optional[List[str]] = None
    skipped = 0
    for entry in decklists:
        deck = _parse_decklist(entry)
        if not deck:
            continue
        if first is None:
            first = deck
        if is_admissible_deck(deck):
            if skipped:
                logger.info("%s: skipped %d non-admissible decklist(s)", label, skipped)
            return deck
        skipped += 1
    if first is None:
        raise ValueError(f"No valid decklists for {label}")
    gauntlet.validate_admissible_deck(first, card_admissibility(), label=label)
    raise AssertionError("unreachable: the first decklist was not admissible")  # pragma: no cover


def _reset_cache_for_tests() -> None:
    global _mechanics_checked_for, _admissibility_cache
    _mechanics_checked_for = None
    _admissibility_cache = None
