#!/usr/bin/env python
"""Generate scenario YAML stubs for an archetype's unique cards.

Creates one scenario file per card with appropriate setup (hand, memory,
field targets). Assertions are left empty for Claude to fill in based
on card text.

Usage:
    python tools/generate_scenarios.py --archetype "medusamon" --output qa/scenarios/medusamon/
    python tools/generate_scenarios.py --archetype "royal-knights" -o qa/scenarios/royal-knights/
"""

import argparse
import json
import sys
from pathlib import Path

import yaml

sys.path.insert(0, str(Path(__file__).parent.parent))

from data_paths import DECK_LIBRARY
from digimon_engine import CardDatabase, CardKind, CardRegistry


def load_archetype_cards(archetype_name: str) -> set[str]:
    """Load all unique card IDs across all decklists for an archetype."""
    with open(DECK_LIBRARY, "r", encoding="utf-8") as f:
        library = json.load(f)
    arch = library["archetypes"].get(archetype_name)
    if not arch or not arch.get("decklists"):
        raise ValueError(f"No decklists for archetype: {archetype_name}")
    all_cards = set()
    for dl in arch.get("decklists", []):
        all_cards.update(json.loads(dl["decklist"]))
    return all_cards


def generate_scenario(card_id: str, archetype: str, db: CardDatabase) -> dict:
    """Generate a scenario skeleton for a single card."""
    entity = db.get_card(card_id)
    if not entity:
        return None

    name = entity.card_name
    kind = entity.card_kind
    level = entity.level
    play_cost = entity.play_cost
    effect_text = entity.effect_text or ""

    # Determine appropriate memory
    memory = max(play_cost + 2, 3) if play_cost else 5

    scenario = {
        "name": f"{name} ({card_id}) — basic play test",
        "description": f"Test playing {name} and verify its effects fire correctly.",
    }

    setup = {
        "deck1_archetype": archetype,
        "deck2_archetype": archetype,
        "memory": memory,
        "first_player": 1,
    }

    # Setup player zones based on card type
    if kind == CardKind.DigiEgg:
        # Egg cards — place in breeding instead
        setup["player1"] = {"breeding": [card_id]}
        scenario["actions"] = [
            {"find": "Move"},
            {"auto_resolve": True},
        ]
    elif kind == CardKind.Digimon and level and level >= 4:
        # Digimon that needs a digivolution base
        setup["player1"] = {"hand": [card_id]}
        # Add a generic target for opponent
        setup["player2"] = {
            "field": [{"card_ids": ["ST1-03"], "turn_played": -1}],
        }
        scenario["actions"] = [
            {"find": f"Play {name}"},
            {"auto_resolve": True},
        ]
    elif kind == CardKind.Option:
        # Option cards — need valid targets usually
        setup["player1"] = {"hand": [card_id]}
        setup["player2"] = {
            "field": [{"card_ids": ["ST1-03"], "turn_played": -1}],
        }
        scenario["actions"] = [
            {"find": card_id},
            {"auto_resolve": True},
        ]
    else:
        # Tamer or low-level Digimon
        setup["player1"] = {"hand": [card_id]}
        scenario["actions"] = [
            {"find": f"Play"},
            {"auto_resolve": True},
        ]

    scenario["setup"] = setup

    # Empty assertions — to be filled by Claude
    scenario["assertions"] = [
        {"zone_contains": {"player": 1, "zone": "field", "card": card_id}},
    ]

    # Add card text as comment
    if effect_text:
        scenario["_card_text"] = effect_text

    return scenario


def main():
    parser = argparse.ArgumentParser(description="Generate scenario stubs for an archetype")
    parser.add_argument("--archetype", "-a", required=True, help="Archetype name from deck_library.json")
    parser.add_argument("--output", "-o", type=Path, required=True, help="Output directory for scenario files")
    parser.add_argument("--overwrite", action="store_true", help="Overwrite existing scenario files")
    args = parser.parse_args()

    CardRegistry.ensure_initialized()
    db = CardDatabase()

    try:
        card_ids = load_archetype_cards(args.archetype)
    except ValueError as e:
        print(f"Error: {e}")
        sys.exit(1)

    args.output.mkdir(parents=True, exist_ok=True)

    generated = 0
    skipped = 0
    for card_id in sorted(card_ids):
        filename = f"{card_id.lower().replace('-', '_')}.yaml"
        filepath = args.output / filename

        if filepath.exists() and not args.overwrite:
            skipped += 1
            continue

        scenario = generate_scenario(card_id, args.archetype, db)
        if scenario is None:
            print(f"  SKIP {card_id}: not found in database")
            continue

        with open(filepath, "w", encoding="utf-8") as f:
            yaml.dump(scenario, f, default_flow_style=False, sort_keys=False, allow_unicode=True)

        generated += 1

    print(f"Generated {generated} scenarios, skipped {skipped} existing")
    print(f"Output: {args.output}")


if __name__ == "__main__":
    main()
