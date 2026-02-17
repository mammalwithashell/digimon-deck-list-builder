#!/usr/bin/env python3
"""Fetch card data from digimoncard.io API and merge into cards.json.

Usage:
    python tools/ingest_cards.py BT14 "Booster Blast Ace"
    python tools/ingest_cards.py --set BT22
    python tools/ingest_cards.py --bulk
"""

import json
import os
import re
import sys
import time
import urllib.request

CARDS_JSON = os.path.join(os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "cards.json")
PRIORITY_SETS_TXT = os.path.join(os.path.dirname(__file__), "..", "digimon_gym", "scraper", "priority_sets.txt")

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

# Known set names for convenience (used by legacy positional args mode)
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


def load_cards_json():
    """Load existing cards.json as a dict keyed by card_id, returning (dict, abs_path).

    Supports both new dict format and legacy array format.
    """
    cards_path = os.path.abspath(CARDS_JSON)
    if os.path.exists(cards_path):
        with open(cards_path, "r", encoding="utf-8") as f:
            data = json.load(f)
        if isinstance(data, dict):
            return data, cards_path
        # Legacy array format: convert to dict
        return {c["card_id"]: c for c in data}, cards_path
    return {}, cards_path


def save_cards_json(cards, cards_path):
    """Write cards dict to cards.json."""
    with open(cards_path, "w", encoding="utf-8") as f:
        json.dump(cards, f, indent=2, ensure_ascii=False)


def get_existing_set_ids(cards):
    """Return set of set IDs already in cards.json (e.g. {'BT14', 'BT20', 'P'})."""
    set_ids = set()
    for cid in cards.keys():
        # Extract set ID: BT14-001 -> BT14, EX8-001 -> EX8, P-001 -> P, ST1-01 -> ST1
        m = re.match(r'^([A-Z]+\d*)-', cid)
        if m:
            set_ids.add(m.group(1))
    return set_ids


def set_id_to_card_prefix(set_id):
    """Return the card_id prefix for a set. Card IDs use SET_ID-NNN format.

    BT14 -> BT14, EX10 -> EX10, ST1 -> ST1, P -> P, LM -> LM
    """
    return set_id


def fetch_set_by_card_prefix(set_id):
    """Fetch cards from digimoncard.io using the card= prefix search.

    Uses: https://digimoncard.io/api-public/search.php?card=BT22
    This works for any set without needing the set name.

    Note: The API matches card ID prefixes, so ?card=BT1 also returns
    BT10, BT11, etc. We filter results to the exact set prefix.
    """
    api_prefix = set_id_to_card_prefix(set_id)
    url = f"https://digimoncard.io/api-public/search.php?card={set_id}"
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    resp = urllib.request.urlopen(req, timeout=30)
    api_data = json.loads(resp.read().decode())

    # Deduplicate by card_id, filtering to exact set prefix
    # e.g. ?card=BT1 also returns BT10, BT11, etc.
    seen = {}
    for card in api_data:
        cid = card["id"]
        if not cid.startswith(api_prefix + "-"):
            continue
        if cid not in seen:
            seen[cid] = card

    new_cards = []
    for cid in sorted(seen.keys()):
        new_cards.append(convert_card(seen[cid]))

    return new_cards


def read_priority_sets():
    """Read set IDs from priority_sets.txt."""
    path = os.path.abspath(PRIORITY_SETS_TXT)
    if not os.path.exists(path):
        print(f"Warning: {path} not found")
        return []
    set_ids = []
    with open(path, "r") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("Processed") or line.startswith("Set Priority"):
                continue
            # Lines like "BT23: 1288" or just "BT24"
            set_id = line.split(":")[0].strip()
            if set_id and re.match(r'^[A-Z]+\d*$', set_id):
                set_ids.append(set_id)
    return set_ids


def merge_set_into_cards(existing, new_cards, set_id):
    """Remove old cards for this set and add new ones.

    existing: dict keyed by card_id
    new_cards: list of card dicts from convert_card()
    """
    prefix = set_id + "-"
    filtered = {cid: c for cid, c in existing.items() if not cid.startswith(prefix)}
    for card in new_cards:
        filtered[card["card_id"]] = card
    return filtered


def ingest_single_set(set_id, existing, cards_path):
    """Fetch and merge a single set. Returns updated card list."""
    print(f"  Fetching {set_id}...", end=" ", flush=True)
    try:
        new_cards = fetch_set_by_card_prefix(set_id)
        print(f"{len(new_cards)} cards")
        if new_cards:
            existing = merge_set_into_cards(existing, new_cards, set_id)
        return existing
    except Exception as e:
        print(f"FAILED: {e}")
        return existing


def bulk_ingest():
    """Ingest all priority sets that are missing from cards.json."""
    priority_sets = read_priority_sets()
    if not priority_sets:
        print("No priority sets found. Ensure digimon_gym/scraper/priority_sets.txt exists.")
        sys.exit(1)

    existing, cards_path = load_cards_json()
    existing_set_ids = get_existing_set_ids(existing)

    missing = [s for s in priority_sets if s not in existing_set_ids]
    print(f"Priority sets: {len(priority_sets)}")
    print(f"Already in cards.json: {len(existing_set_ids)} ({', '.join(sorted(existing_set_ids))})")
    print(f"Missing: {len(missing)} ({', '.join(missing)})")

    if not missing:
        print("All priority sets already ingested.")
        return

    print(f"\nIngesting {len(missing)} sets...")
    total_new = 0
    for i, set_id in enumerate(missing):
        old_count = len(existing)
        existing = ingest_single_set(set_id, existing, cards_path)
        added = len(existing) - old_count
        total_new += max(0, added)

        # Rate limit between requests
        if i < len(missing) - 1:
            time.sleep(1)

    # Save once at the end
    save_cards_json(existing, cards_path)

    # Summary
    set_counts = {}
    for cid in existing:
        m = re.match(r'^([A-Z]+\d*)-', cid)
        if m:
            sid = m.group(1)
            set_counts[sid] = set_counts.get(sid, 0) + 1
    print(f"\nDone. {len(existing)} total cards across {len(set_counts)} sets.")
    print(f"Added {total_new} new cards.")


def main():
    # --bulk mode: ingest all priority sets
    if len(sys.argv) >= 2 and sys.argv[1] == "--bulk":
        bulk_ingest()
        return

    # --set mode: ingest a single set by ID (no set name needed)
    if len(sys.argv) >= 3 and sys.argv[1] == "--set":
        set_id = sys.argv[2].upper()
        existing, cards_path = load_cards_json()
        existing = ingest_single_set(set_id, existing, cards_path)
        save_cards_json(existing, cards_path)

        set_counts = {}
        for cid in existing:
            m = re.match(r'^([A-Z]+\d*)-', cid)
            if m:
                sid = m.group(1)
                set_counts[sid] = set_counts.get(sid, 0) + 1
        print(f"Wrote {len(existing)} total cards across {len(set_counts)} sets to {cards_path}")
        return

    # Legacy positional args mode: SET_ID SET_NAME
    if len(sys.argv) < 2:
        print("Usage:")
        print("  python ingest_cards.py --bulk                   # Ingest all priority sets")
        print("  python ingest_cards.py --set BT22               # Ingest single set by ID")
        print("  python ingest_cards.py BT14 'Booster Blast Ace' # Legacy: set ID + name")
        sys.exit(1)

    set_id = sys.argv[1].upper()
    set_name = sys.argv[2] if len(sys.argv) > 2 else SET_NAMES.get(set_id, "")

    if not set_name:
        print(f"Unknown set '{set_id}'. Please provide the set name as second argument,")
        print(f"or use --set {set_id} to fetch by card prefix instead.")
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
    existing, cards_path = load_cards_json()
    # Remove any existing cards from this set and add new ones
    merged = {cid: c for cid, c in existing.items() if not cid.startswith(api_set_id)}
    for card in new_cards:
        merged[card["card_id"]] = card
    save_cards_json(merged, cards_path)

    prev_count = len(existing) - (len(existing) - len(merged) + len(new_cards))
    print(f"Wrote {len(merged)} total cards ({len(new_cards)} {set_id}) to {cards_path}")


if __name__ == "__main__":
    main()
