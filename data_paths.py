"""Canonical paths to the shared game-data files, with env-var overrides.

This module is the single source of truth for locating cards.json,
deck_library.json, archetype_aliases.json, and card_overrides.json — the
four shared data files consumed by both the Rust engine (source of
truth) and the transitional Python engine. Files live under
``<repo_root>/data/`` so neither language "owns" the data directory.

Each path can be overridden via an env var for tests, alternate
databases, or ad-hoc re-homing without touching code:

- ``DIGIMON_CARDS_JSON``
- ``DIGIMON_DECK_LIBRARY``
- ``DIGIMON_ARCHETYPE_ALIASES``
- ``DIGIMON_CARD_OVERRIDES``
- ``DIGIMON_TESTED_CARDS``
- ``DIGIMON_DATA_DIR`` — overrides the whole directory; individual file
  env vars still win if set.

Usage::

    from data_paths import CARDS_JSON
    with open(CARDS_JSON) as f:
        cards = json.load(f)

Rust callers use the matching ``DIGIMON_CARDS_JSON`` env var + fallback
walk-up, see ``digimon-engine-py/src/lib.rs::resolve_cards_path`` and
``digimon-engine/tests/mask_and_tensor/card_registry_parity.rs``.
"""

from __future__ import annotations

import os
from pathlib import Path

REPO_ROOT: Path = Path(__file__).resolve().parent

# Default data directory. ``DIGIMON_DATA_DIR`` overrides the root; each
# file env var below overrides the individual path.
DATA_DIR: Path = Path(os.environ.get("DIGIMON_DATA_DIR", REPO_ROOT / "data"))


def _resolve(env_var: str, default_name: str) -> Path:
    """Return the env-var-overridden path for a data file, or the default."""
    override = os.environ.get(env_var)
    if override:
        return Path(override)
    return DATA_DIR / default_name


CARDS_JSON: Path = _resolve("DIGIMON_CARDS_JSON", "cards.json")
DECK_LIBRARY: Path = _resolve("DIGIMON_DECK_LIBRARY", "deck_library.json")
ARCHETYPE_ALIASES: Path = _resolve("DIGIMON_ARCHETYPE_ALIASES", "archetype_aliases.json")
CARD_OVERRIDES: Path = _resolve("DIGIMON_CARD_OVERRIDES", "card_overrides.json")
TESTED_CARDS: Path = _resolve("DIGIMON_TESTED_CARDS", "tested_cards.json")
