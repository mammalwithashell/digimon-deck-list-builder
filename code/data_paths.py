"""Canonical paths to the shared game-data files, with env-var overrides.

This module is the single source of truth for locating committed card-data
inputs and generated compatibility artifacts. Files live under
``<repo_root>/data/`` so neither language "owns" the data directory.

Each path can be overridden via an env var for tests, alternate
databases, or ad-hoc re-homing without touching code:

- ``DIGIMON_CARDS_JSON``
- ``DIGIMON_DECK_LIBRARY``
- ``DIGIMON_ARCHETYPE_ALIASES``
- ``DIGIMON_CARD_OVERRIDES``
- ``DIGIMON_CARD_SOURCES_DIR``
- ``DIGIMON_CARDS_CANONICAL``
- ``DIGIMON_CARD_CORRECTIONS``
- ``DIGIMON_CARD_ADJUDICATIONS``
- ``DIGIMON_CARD_REGISTRY``
- ``DIGIMON_CARD_DATA_REVISION``
- ``DIGIMON_TESTED_CARDS``
- ``DIGIMON_ORACLE_READINESS`` (the training gate's readiness artifact)
- ``DIGIMON_CARD_ADMISSIBILITY`` (the training gate's per-card admissibility artifact)
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

REPO_ROOT: Path = Path(__file__).resolve().parent.parent

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
ORACLE_READINESS: Path = _resolve("DIGIMON_ORACLE_READINESS", "oracle_readiness.json")
CARD_ADMISSIBILITY: Path = _resolve("DIGIMON_CARD_ADMISSIBILITY", "card_admissibility.json")

# Canonical card-data pipeline inputs and generated pool artifacts.
CARD_SOURCES_DIR: Path = Path(
    os.environ.get("DIGIMON_CARD_SOURCES_DIR", DATA_DIR / "card_sources")
)
CARD_SOURCES_MANIFEST: Path = CARD_SOURCES_DIR / "manifest.json"
CARDS_CANONICAL: Path = _resolve("DIGIMON_CARDS_CANONICAL", "cards_canonical.json")
CARD_CORRECTIONS: Path = _resolve("DIGIMON_CARD_CORRECTIONS", "card_corrections.json")
CARD_MIGRATION_ADJUDICATIONS: Path = _resolve(
    "DIGIMON_CARD_ADJUDICATIONS", "card_migration_adjudications.json"
)
CARD_REGISTRY: Path = _resolve("DIGIMON_CARD_REGISTRY", "card_registry.json")
CARD_DATA_REVISION: Path = _resolve(
    "DIGIMON_CARD_DATA_REVISION", "card_data_revision.json"
)
CARD_TRANSACTION_JOURNAL: Path = DATA_DIR / ".card_data_transaction.json"
RUNTIME_REGISTERED_CARDS: Path = _resolve(
    "DIGIMON_RUNTIME_REGISTERED_CARDS", "runtime_registered_cards.json"
)
VALIDATED_PLAYABLE_CARDS: Path = _resolve(
    "DIGIMON_VALIDATED_PLAYABLE_CARDS", "validated_playable_cards.json"
)

QA_REPORTS_DIR: Path = Path(
    os.environ.get("DIGIMON_QA_REPORTS_DIR", REPO_ROOT / "qa" / "qa-reports")
)
CARD_CAMPAIGN_MANIFEST: Path = Path(
    os.environ.get(
        "DIGIMON_CARD_CAMPAIGN_MANIFEST",
        QA_REPORTS_DIR / "card_implementation_manifest.json",
    )
)
