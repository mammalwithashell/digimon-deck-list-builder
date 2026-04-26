"""Utility to find competitive decklists where all cards have frozen script implementations.

Cross-references deck_library.json with _frozen_manifest.json to identify
playable decks sorted by meta share or coverage.
"""

from pathlib import Path
from dataclasses import dataclass, asdict, field
from typing import List
import json
import argparse
import sys

from digimon_gym.data_paths import DECK_LIBRARY as _DECK_LIBRARY_PATH
from digimon_gym.engine.data.card_database import CardDatabase
from digimon_gym.engine.data.enums import CardKind

# Scripts still live alongside the Python engine.
_ENGINE_DATA_DIR = Path(__file__).parent
_MANIFEST_PATH = _ENGINE_DATA_DIR / "scripts" / "_frozen_manifest.json"


@dataclass
class PlayableDeck:
    archetype_name: str
    deck_id: str
    source: str
    card_ids: List[str]
    egg_deck: List[str]
    main_deck: List[str]
    meta_share: float
    missing_card_ids: List[str]
    coverage_pct: float


def load_implemented_card_ids() -> set:
    """Load the set of card IDs that have frozen script implementations."""
    with open(_MANIFEST_PATH, "r", encoding="utf-8") as f:
        manifest = json.load(f)
    return set(manifest.get("cards", {}).keys())


def find_playable_decks(
    min_coverage: float = 1.0,
    max_results: int = 50,
    sort_by: str = "meta_share",
) -> List[PlayableDeck]:
    """Find competitive decklists where cards meet the minimum coverage threshold.

    Args:
        min_coverage: Minimum fraction of unique card IDs that must have frozen
            scripts (1.0 = all cards implemented).
        max_results: Maximum number of decks to return.
        sort_by: Sort key -- "meta_share" (descending) or "coverage" (descending).

    Returns:
        List of PlayableDeck objects matching the criteria.
    """
    implemented = load_implemented_card_ids()

    with open(_DECK_LIBRARY_PATH, "r", encoding="utf-8") as f:
        library = json.load(f)

    db = CardDatabase()
    results: List[PlayableDeck] = []

    archetypes = library.get("archetypes", {})
    for archetype_name, archetype_data in archetypes.items():
        meta_share = archetype_data.get("stats", {}).get("meta_share", 0.0)

        for deck_entry in archetype_data.get("decklists", []):
            deck_id = deck_entry.get("deck_id", "")
            source = deck_entry.get("source", "")
            decklist_raw = deck_entry.get("decklist", "[]")

            # Parse the decklist JSON string into a list of card IDs
            try:
                card_ids: List[str] = json.loads(decklist_raw)
            except (json.JSONDecodeError, TypeError):
                continue

            if not card_ids:
                continue

            unique_ids = set(card_ids)
            implemented_ids = unique_ids & implemented
            coverage = len(implemented_ids) / len(unique_ids) if unique_ids else 0.0
            missing = sorted(unique_ids - implemented)

            if coverage < min_coverage:
                continue

            # Classify cards into egg_deck vs main_deck
            egg_deck: List[str] = []
            main_deck: List[str] = []
            for cid in card_ids:
                card = db.get_card(cid)
                if card and card.card_kind == CardKind.DigiEgg:
                    egg_deck.append(cid)
                else:
                    main_deck.append(cid)

            results.append(PlayableDeck(
                archetype_name=archetype_name,
                deck_id=deck_id,
                source=source,
                card_ids=card_ids,
                egg_deck=egg_deck,
                main_deck=main_deck,
                meta_share=meta_share,
                missing_card_ids=missing,
                coverage_pct=round(coverage, 4),
            ))

    # Sort results
    if sort_by == "coverage":
        results.sort(key=lambda d: (-d.coverage_pct, -d.meta_share))
    else:
        results.sort(key=lambda d: (-d.meta_share, -d.coverage_pct))

    return results[:max_results]


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Find competitive decklists with frozen script coverage."
    )
    parser.add_argument(
        "--min-coverage",
        type=float,
        default=1.0,
        help="Minimum coverage ratio (0.0-1.0). Default: 1.0",
    )
    parser.add_argument(
        "--max-results",
        type=int,
        default=20,
        help="Maximum number of results. Default: 20",
    )
    args = parser.parse_args()

    decks = find_playable_decks(
        min_coverage=args.min_coverage,
        max_results=args.max_results,
    )

    output = [
        {
            "archetype_name": d.archetype_name,
            "deck_id": d.deck_id,
            "card_count": len(d.card_ids),
            "coverage": d.coverage_pct,
            "meta_share": d.meta_share,
            "missing_card_ids": d.missing_card_ids,
        }
        for d in decks
    ]

    json.dump(output, sys.stdout, indent=2)
    print()  # trailing newline
