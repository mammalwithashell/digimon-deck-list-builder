#!/usr/bin/env python3
"""Fetch card data from digimoncard.io API and merge into cards.json.

Usage:
    python tools/ingest_cards.py BT14 "Booster Blast Ace"
    python tools/ingest_cards.py BT24 "Booster Time Stranger"
"""

import json
import os
import sys
import urllib.request

CARDS_JSON = os.path.join(os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "cards.json")

COLOR_MAP = {
    "Red": 0, "Blue": 1, "Yellow": 2, "Green": 3,
    "White": 4, "Black": 5, "Purple": 6,
}

KIND_MAP = {
    "Digimon": 0, "Tamer": 1, "Option": 2, "Digi-Egg": 3,
}

RARITY_MAP = {
    "C": 0, "U": 1, "R": 2, "SR": 3, "SEC": 4, "P": 5,
}

# Known set names for convenience
SET_NAMES = {
    "BT14": "Booster Blast Ace",
    "BT20": "Booster Over the X",
    "BT24": "Booster Time Stranger",
}


def parse_evo_costs(api_card):
    """Parse evolution costs from the API card data.

    Handles multiple evo costs (e.g. X-Antibody cards with 2+ evo lines).
    The API provides evolution_cost, evolution_cost_2, etc.
    """
    costs = []
    # Primary evo cost
    evo_cost = api_card.get("evolution_cost")
    evo_color = api_card.get("evolution_color")
    evo_level = api_card.get("evolution_level")

    if evo_cost and evo_color and evo_level:
        color_val = COLOR_MAP.get(evo_color, 0)
        costs.append({
            "card_color": color_val,
            "level": evo_level,
            "memory_cost": evo_cost,
        })

    # Secondary evo cost (common for X-Antibody and dual-color cards)
    evo_cost2 = api_card.get("evolution_cost_2")
    evo_color2 = api_card.get("evolution_color_2")
    evo_level2 = api_card.get("evolution_level_2")

    if evo_cost2 and evo_color2 and evo_level2:
        color_val2 = COLOR_MAP.get(evo_color2, 0)
        costs.append({
            "card_color": color_val2,
            "level": evo_level2,
            "memory_cost": evo_cost2,
        })

    return costs


def convert_card(api_card):
    """Convert a digimoncard.io API card to our cards.json format."""
    card_id = api_card["id"]
    try:
        card_index = int(card_id.split("-")[1])
    except (IndexError, ValueError):
        card_index = 0

    colors = []
    if api_card.get("color"):
        c = COLOR_MAP.get(api_card["color"])
        if c is not None:
            colors.append(c)
    if api_card.get("color2"):
        c = COLOR_MAP.get(api_card["color2"])
        if c is not None and c not in colors:
            colors.append(c)

    card_kind = KIND_MAP.get(api_card.get("type", "Digimon"), 0)
    rarity = RARITY_MAP.get(api_card.get("rarity", "C"), 0)

    traits = []
    for key in ["digi_type", "digi_type2", "digi_type3", "digi_type4"]:
        val = api_card.get(key)
        if val:
            traits.append(val)

    class_name = card_id.replace("-", "_")

    return {
        "card_id": card_id,
        "card_index": card_index,
        "card_name_eng": api_card.get("name", ""),
        "card_name_jpn": "",
        "card_effect_class_name": class_name,
        "play_cost": api_card.get("play_cost") or 0,
        "dp": api_card.get("dp"),  # None for eggs/tamers/options, 0+ for digimon
        "level": api_card.get("level"),  # None for options/tamers without level
        "card_kind": card_kind,
        "rarity": rarity,
        "card_colors": colors,
        "type_eng": traits,
        "form_eng": [api_card.get("form", "")] if api_card.get("form") else [],
        "attribute_eng": [api_card.get("attribute", "")] if api_card.get("attribute") else [],
        "effect_description_eng": api_card.get("main_effect") or "",
        "inherited_effect_description_eng": api_card.get("source_effect") or "",
        "security_effect_description_eng": "",
        "evo_costs": parse_evo_costs(api_card),
    }


def main():
    if len(sys.argv) < 2:
        print("Usage: python ingest_cards.py <SET_ID> [SET_NAME]")
        print("  e.g. python ingest_cards.py BT24 'Booster Time Stranger'")
        sys.exit(1)

    set_id = sys.argv[1].upper()
    set_name = sys.argv[2] if len(sys.argv) > 2 else SET_NAMES.get(set_id, "")

    if not set_name:
        print(f"Unknown set '{set_id}'. Please provide the set name as second argument.")
        sys.exit(1)

    # Build API URL: BT24 -> BT-24
    api_set_id = f"{set_id[:2]}-{set_id[2:]}"
    api_url = f"https://digimoncard.io/index.php/api-public/search?pack={api_set_id}:%20{set_name.replace(' ', '%20')}"

    print(f"Fetching {set_id} ({set_name}) data from API...")
    print(f"  URL: {api_url}")
    req = urllib.request.Request(api_url, headers={"User-Agent": "Mozilla/5.0"})
    resp = urllib.request.urlopen(req, timeout=30)
    api_data = json.loads(resp.read().decode())
    print(f"Got {len(api_data)} entries from API")

    # Deduplicate by card_id (API returns alt arts as separate entries)
    seen = {}
    for card in api_data:
        cid = card["id"]
        if cid not in seen:
            seen[cid] = card
    print(f"Unique cards: {len(seen)}")

    # Convert
    new_cards = []
    for cid in sorted(seen.keys()):
        new_cards.append(convert_card(seen[cid]))

    # Load existing cards.json
    cards_path = os.path.abspath(CARDS_JSON)
    if os.path.exists(cards_path):
        with open(cards_path, "r", encoding="utf-8") as f:
            existing = json.load(f)
        # Remove any existing cards from this set
        existing = [c for c in existing if not c["card_id"].startswith(api_set_id)]
    else:
        existing = []

    # Merge
    merged = existing + new_cards
    with open(cards_path, "w", encoding="utf-8") as f:
        json.dump(merged, f, indent=2, ensure_ascii=False)

    print(f"Wrote {len(merged)} total cards ({len(existing)} existing + {len(new_cards)} {set_id}) to {cards_path}")


if __name__ == "__main__":
    main()
