#!/usr/bin/env python3
"""Build a future-proof card registry from the DigimonCard.io API.

Fetches all cards, assigns stable integer indices using natural sort order,
and writes a dict-format cards.json with index/norm_id fields.

The registry is append-only: existing card indices are never changed.
New cards are assigned the next available index after all existing ones.

Usage:
    python tools/build_registry.py                  # Full build from API
    python tools/build_registry.py --dry-run        # Fetch + stats, no write
    python tools/build_registry.py --offline        # Rebuild norm_ids only
    python tools/build_registry.py --sets BT25 EX12 # Only fetch specific sets
"""

import argparse
import json
import os
import re
import sys
import time
import urllib.request

_PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if _PROJECT_ROOT not in sys.path:
    sys.path.insert(0, _PROJECT_ROOT)

from digimon_gym.data_paths import CARDS_JSON as _CARDS_JSON_PATH  # noqa: E402

REGISTRY_CAPACITY = 20_000

CARDS_JSON = str(_CARDS_JSON_PATH)

# All known set prefixes in the Digimon Card Game.
# Card IDs use non-zero-padded numbers: BT1 (not BT01), EX8 (not EX08).
KNOWN_SETS = [
    # Boosters (BT1-BT26)
    *[f"BT{i}" for i in range(1, 27)],
    # Extra Boosters (EX1-EX13)
    *[f"EX{i}" for i in range(1, 14)],
    # Starters (ST1-ST24)
    *[f"ST{i}" for i in range(1, 25)],
    # Advanced Boosters
    "AD1",
    # Resurgence Boosters
    "RB1",
    # Limited Card Packs (LM has no numbered sub-sets)
    "LM",
    # Promos
    "P",
]

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


def card_sort_key(card_id: str) -> tuple:
    """Natural sort key: 'BT14-001' -> ('BT', 14, 1)."""
    prefix, num_str = card_id.rsplit("-", 1)
    m = re.match(r'^([A-Za-z]+)(\d*)$', prefix)
    letters = m.group(1) if m else prefix
    set_num = int(m.group(2)) if m and m.group(2) else 0
    card_num = int(num_str)
    return (letters, set_num, card_num)


def parse_evo_costs(api_card):
    """Parse evolution costs from the API card data.

    Mirrors ``tools.ingest_cards`` so partial set refreshes do not drop
    inferred evo lines for newer sets whose API rows omit explicit
    evolution_color / evolution_level fields.
    """
    costs = []
    card_level = api_card.get("level")
    card_color = api_card.get("color")

    evo_cost = api_card.get("evolution_cost")
    evo_color = api_card.get("evolution_color")
    evo_level = api_card.get("evolution_level")

    if evo_cost is not None and card_level and card_level >= 3:
        if not evo_color and card_color:
            evo_color = card_color
        if not evo_level and card_level:
            evo_level = card_level - 1

        if evo_color and evo_level:
            costs.append({
                "card_color": COLOR_MAP.get(evo_color, 0),
                "level": evo_level,
                "memory_cost": evo_cost,
            })

    evo_cost2 = api_card.get("evolution_cost_2")
    evo_color2 = api_card.get("evolution_color_2")
    evo_level2 = api_card.get("evolution_level_2")

    if evo_cost2 is not None and card_level and card_level >= 3:
        if not evo_color2 and api_card.get("color2"):
            evo_color2 = api_card.get("color2")
        if not evo_level2 and card_level:
            evo_level2 = card_level - 1

        if evo_color2 and evo_level2:
            costs.append({
                "card_color": COLOR_MAP.get(evo_color2, 0),
                "level": evo_level2,
                "memory_cost": evo_cost2,
            })

    return costs


def convert_card(api_card, index, norm_id):
    """Convert a digimoncard.io API card to our cards.json format with index/norm_id."""
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
        "index": index,
        "norm_id": norm_id,
        "card_id": card_id,
        "card_index": card_index,
        "card_name_eng": api_card.get("name", ""),
        "card_name_jpn": "",
        "card_effect_class_name": class_name,
        "play_cost": api_card.get("play_cost") or 0,
        "dp": api_card.get("dp"),
        "level": api_card.get("level"),
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
        "xros_req": api_card.get("xros_req") or "",
    }


def fetch_set(set_id):
    """Fetch cards for a single set from the DigimonCard.io API.

    Uses card= prefix search, then filters to exact set prefix match
    (e.g. ?card=BT1 also returns BT10-BT19, so we filter).
    """
    url = f"https://digimoncard.io/api-public/search.php?card={set_id}"
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    try:
        resp = urllib.request.urlopen(req, timeout=30)
        api_data = json.loads(resp.read().decode())
    except Exception as e:
        print(f"    FAILED: {e}")
        return {}

    prefix = set_id + "-"
    seen = {}
    for card in api_data:
        cid = card["id"]
        if cid.startswith(prefix) and cid not in seen:
            seen[cid] = card
    return seen


def fetch_all_cards(sets):
    """Fetch all cards from the API, iterating over known sets."""
    all_cards = {}
    for i, set_id in enumerate(sets):
        print(f"  Fetching {set_id}...", end=" ", flush=True)
        cards = fetch_set(set_id)
        new_count = 0
        for cid, card in cards.items():
            if cid not in all_cards:
                all_cards[cid] = card
                new_count += 1
        print(f"{len(cards)} cards ({new_count} new)")

        # Rate limit between requests
        if i < len(sets) - 1:
            time.sleep(1)

    return all_cards


def load_existing_mappings(path):
    """Load existing card_id -> index mappings from cards.json.

    Returns empty dict if file missing or in old array format.
    """
    abs_path = os.path.abspath(path)
    if not os.path.exists(abs_path):
        return {}, {}

    with open(abs_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    if isinstance(data, dict):
        mappings = {}
        card_data = {}
        for card_id, entry in data.items():
            if "index" in entry:
                mappings[card_id] = entry["index"]
            card_data[card_id] = entry
        return mappings, card_data

    # Old array format — no stable indices to preserve
    return {}, {}


def build_registry(api_card_ids, existing_mappings, capacity=REGISTRY_CAPACITY):
    """Build the append-only registry mapping.

    - Existing card_id -> index mappings are preserved unchanged.
    - New cards are sorted by natural sort key and assigned sequential indices
      starting after the highest existing index.
    - Raises ValueError if total cards exceed capacity.

    Returns: dict mapping card_id -> index
    """
    # Start with all existing mappings
    registry = dict(existing_mappings)

    # Find the highest existing index
    max_index = max(registry.values(), default=0)

    # Identify genuinely new cards (not in existing registry)
    new_ids = sorted(
        [cid for cid in api_card_ids if cid not in registry],
        key=card_sort_key,
    )

    # Assign new indices sequentially
    for card_id in new_ids:
        max_index += 1
        registry[card_id] = max_index

    # Validate capacity
    if len(registry) > capacity:
        raise ValueError(
            f"Card count {len(registry)} exceeds registry capacity {capacity}. "
            f"Increase REGISTRY_CAPACITY."
        )

    return registry


def main():
    parser = argparse.ArgumentParser(
        description="Build a future-proof card registry from DigimonCard.io API"
    )
    parser.add_argument(
        "--dry-run", action="store_true",
        help="Fetch and compute stats without writing to disk",
    )
    parser.add_argument(
        "--offline", action="store_true",
        help="Skip API fetch; rebuild norm_ids from existing data only",
    )
    parser.add_argument(
        "--capacity", type=int, default=REGISTRY_CAPACITY,
        help=f"Registry capacity ceiling (default: {REGISTRY_CAPACITY})",
    )
    parser.add_argument(
        "--sets", nargs="+", default=None,
        help="Override KNOWN_SETS with specific set IDs to fetch",
    )
    parser.add_argument(
        "--force", action="store_true",
        help="Override reindex safety check (use only if you intend to reassign indices)",
    )
    args = parser.parse_args()

    capacity = args.capacity
    cards_path = os.path.abspath(CARDS_JSON)

    print(f"Registry capacity: {capacity}")
    print(f"Output: {cards_path}")

    # Step 1: Load existing registry
    existing_mappings, existing_card_data = load_existing_mappings(cards_path)
    print(f"Existing cards with stable indices: {len(existing_mappings)}")

    if args.offline:
        if not existing_mappings:
            print("ERROR: --offline requires an existing dict-format cards.json")
            sys.exit(1)

        # Rebuild norm_ids only
        registry = {}
        for card_id, entry in existing_card_data.items():
            idx = entry.get("index", 0)
            entry["norm_id"] = idx / capacity
            registry[card_id] = entry

        if not args.dry_run:
            with open(cards_path, "w", encoding="utf-8") as f:
                json.dump(registry, f, indent=2, ensure_ascii=False)
            print(f"Rebuilt norm_ids for {len(registry)} cards.")
        else:
            print(f"[DRY RUN] Would rebuild norm_ids for {len(registry)} cards.")
        return

    # Step 2: Fetch from API
    sets_to_fetch = args.sets or KNOWN_SETS
    print(f"\nFetching {len(sets_to_fetch)} sets from DigimonCard.io API...")
    api_cards = fetch_all_cards(sets_to_fetch)
    print(f"\nTotal unique cards from API: {len(api_cards)}")

    # Step 3: Build stable registry
    api_card_ids = list(api_cards.keys())
    registry = build_registry(api_card_ids, existing_mappings, capacity)

    new_count = len(registry) - len(existing_mappings)
    max_index = max(registry.values(), default=0)
    print(f"\nRegistry: {len(registry)} cards (index 1..{max_index})")
    print(f"  Preserved: {len(existing_mappings)}")
    print(f"  New: {new_count}")
    print(f"  Capacity used: {len(registry)}/{capacity} ({100*len(registry)/capacity:.1f}%)")

    # Safety check: detect cards that were in cards.json with an index but
    # are about to receive a DIFFERENT index (indicates index was stripped
    # by a prior ingest run and is being silently reassigned).
    reindexed = []
    for card_id, old_entry in existing_card_data.items():
        old_idx = old_entry.get("index")
        if old_idx is not None and card_id in registry and registry[card_id] != old_idx:
            reindexed.append((card_id, old_idx, registry[card_id]))
    if reindexed:
        print(f"\nERROR: {len(reindexed)} cards would be reindexed (breaks RL agent weights):")
        for cid, old_idx, new_idx in sorted(reindexed)[:20]:
            print(f"  {cid}: {old_idx} -> {new_idx}")
        if len(reindexed) > 20:
            print(f"  ... and {len(reindexed) - 20} more")
        print("\nThis likely means `ingest_cards.py` stripped index fields from these cards.")
        print("Fix the source data and re-run, or use --force to override this check.")
        if not getattr(args, 'force', False):
            sys.exit(1)

    # Step 4: Convert all cards to output format
    output = {}
    for card_id in sorted(registry.keys(), key=card_sort_key):
        idx = registry[card_id]
        norm_id = idx / capacity

        if card_id in api_cards:
            # Card data from API
            entry = convert_card(api_cards[card_id], idx, norm_id)
        elif card_id in existing_card_data:
            # Preserved from existing registry (card removed from API)
            entry = dict(existing_card_data[card_id])
            entry["index"] = idx
            entry["norm_id"] = norm_id
        else:
            # Shouldn't happen, but handle gracefully
            entry = {
                "index": idx,
                "norm_id": norm_id,
                "card_id": card_id,
                "card_name_eng": "",
                "card_name_jpn": "",
                "card_effect_class_name": card_id.replace("-", "_"),
                "play_cost": 0,
                "dp": None,
                "level": None,
                "card_kind": 0,
                "rarity": 0,
                "card_colors": [],
                "type_eng": [],
                "form_eng": [],
                "attribute_eng": [],
                "effect_description_eng": "",
                "inherited_effect_description_eng": "",
                "security_effect_description_eng": "",
                "evo_costs": [],
            }

        output[card_id] = entry

    # Step 5: Save
    if args.dry_run:
        print(f"\n[DRY RUN] Would write {len(output)} cards to {cards_path}")
    else:
        with open(cards_path, "w", encoding="utf-8") as f:
            json.dump(output, f, indent=2, ensure_ascii=False)
        print(f"\nWrote {len(output)} cards to {cards_path}")

    # Summary by set
    set_counts = {}
    for cid in output:
        m = re.match(r'^([A-Za-z]+\d*)-', cid)
        if m:
            sid = m.group(1)
            set_counts[sid] = set_counts.get(sid, 0) + 1
    print(f"\nSets ({len(set_counts)}):")
    for sid in sorted(set_counts.keys(), key=lambda s: card_sort_key(s + "-0")):
        print(f"  {sid}: {set_counts[sid]} cards")


if __name__ == "__main__":
    main()
