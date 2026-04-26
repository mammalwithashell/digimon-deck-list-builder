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
from dataclasses import asdict

# Import the existing parsers and dataclasses so on-disk JSON matches the
# shapes the runtime loader expects. These imports are local to avoid
# pulling the engine at API-fetch time when card_database.py isn't needed.

# Add project root to path so the shared `data_paths` module
# is importable when this script is run via `python -m tools.ingest_cards`
# or `python tools/ingest_cards.py`.
_PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if _PROJECT_ROOT not in sys.path:
    sys.path.insert(0, _PROJECT_ROOT)

from data_paths import CARDS_JSON as _CARDS_JSON_PATH
from data_paths import CARD_OVERRIDES as _CARD_OVERRIDES_PATH

CARDS_JSON = str(_CARDS_JSON_PATH)
CARD_OVERRIDES_JSON = str(_CARD_OVERRIDES_PATH)
PRIORITY_SETS_TXT = os.path.join(os.path.dirname(__file__), "scraper", "priority_sets.txt")

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
    The API provides evolution_cost, evolution_cost_2, evolution_cost_3.

    When evolution_color or evolution_level are missing from the API
    (common for newer sets), infers them from the card's own color and
    level using standard Digimon TCG rules (digivolve from level - 1
    of the same color).
    """
    costs = []
    card_level = api_card.get("level")

    # Primary + secondary + tertiary evo paths. Some tri-color cards
    # (e.g. Ouryuken, Magnamon X-Antibody variants) have three.
    for suffix, color_fallback_key in (
        ("", "color"),
        ("_2", "color2"),
        ("_3", "color3"),
    ):
        evo_cost = api_card.get(f"evolution_cost{suffix}")
        evo_color = api_card.get(f"evolution_color{suffix}")
        evo_level = api_card.get(f"evolution_level{suffix}")

        if evo_cost is None or not card_level or card_level < 3:
            continue

        if not evo_color and api_card.get(color_fallback_key):
            evo_color = api_card.get(color_fallback_key)
        if not evo_level and card_level:
            evo_level = card_level - 1

        if evo_color and evo_level:
            color_val = COLOR_MAP.get(evo_color, 0)
            costs.append({
                "card_color": color_val,
                "level": evo_level,
                "memory_cost": evo_cost,
            })

    return costs


def _card_color_to_json(color):
    """Serialize a CardColor enum to its variant-name string (e.g. "Red").

    Rust's `DnaRequirement.card_color: Option<CardColor>` uses default
    serde derivation, which expects variant-name strings. `NoColor` and
    `None` both emit JSON null.
    """
    if color is None:
        return None
    name = color.name
    if name == "NoColor":
        return None
    return name


def _dna_requirement_to_json(req):
    # `card_colors` is a list of variant-name strings because printed
    # DNA reqs can be slash-color (e.g. "Blue/Purple Lv.6"). An empty
    # list means "any color" — level/name gated only.
    return {
        "level": int(req.level),
        "card_colors": [_card_color_to_json(c) for c in req.card_colors if c is not None],
        "name_contains": req.name_contains,
        "text_contains": req.text_contains,
    }


def _dna_cost_to_json(dc):
    return {
        "requirement1": _dna_requirement_to_json(dc.requirement1),
        "requirement2": _dna_requirement_to_json(dc.requirement2),
        "memory_cost": int(dc.memory_cost),
    }


def _digixros_element_to_json(el):
    return {
        "name_contains": el.name_contains,
        "trait_match": el.trait_match,
        "trait_alternatives": list(el.trait_alternatives),
        "level_max": el.level_max,
        "count": int(el.count),
        "is_digimon_only": bool(el.is_digimon_only),
        "color": _card_color_to_json(el.color),
    }


def _digixros_cost_to_json(dxc):
    return {
        "elements": [_digixros_element_to_json(e) for e in dxc.elements],
        "reduce_cost_per_card": int(dxc.reduce_cost_per_card),
        "max_materials": int(dxc.max_materials),
        "different_card_numbers": bool(dxc.different_card_numbers),
        "different_names": bool(dxc.different_names),
        "has_text": dxc.has_text,
        "source_zones": list(dxc.source_zones),
    }


def _parse_xros_costs(xros_req):
    """Run the runtime parsers over a raw `xros_req` string.

    Returns `(dna_costs_json, digixros_costs_json)` — each a list of dicts
    ready for cards.json, or empty when the string has no matching block.
    Imports are deferred because the runtime engine pulls heavy deps.
    """
    if not xros_req:
        return [], []
    from engine_py_legacy.engine.data.card_database import parse_xros_req, parse_digixros_req  # noqa: E402
    dna_costs = parse_xros_req(xros_req)
    digixros_costs = parse_digixros_req(xros_req)
    return (
        [_dna_cost_to_json(dc) for dc in dna_costs],
        [_digixros_cost_to_json(dxc) for dxc in digixros_costs],
    )


def convert_card(api_card):
    """Convert a digimoncard.io API card to our cards.json format."""
    card_id = api_card["id"]
    try:
        card_index = int(card_id.split("-")[1])
    except (IndexError, ValueError):
        card_index = 0

    colors = []
    for key in ("color", "color2", "color3"):
        name = api_card.get(key)
        if not name:
            continue
        c = COLOR_MAP.get(name)
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

    xros_req = api_card.get("xros_req") or ""
    dna_costs_json, digixros_costs_json = _parse_xros_costs(xros_req)

    out = {
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
        "xros_req": xros_req,
    }
    if dna_costs_json:
        out["dna_costs"] = dna_costs_json
    if digixros_costs_json:
        out["digixros_costs"] = digixros_costs_json
    return out


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


def load_card_overrides(path=None):
    """Load the hand-maintained `card_overrides.json` sidecar.

    The digimoncard.io API doesn't expose `color3` / `evolution_cost_3`
    for every tri-color card (or gets them wrong for some sets), so this
    file exists to correct known cases. Schema is
    `{card_id: {field: value, ...}}` — any top-level cards.json field can
    be overridden; the override values replace the ingest-produced ones
    outright (no deep merge).
    """
    if path is None:
        path = os.path.abspath(CARD_OVERRIDES_JSON)
    if not os.path.exists(path):
        return {}
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def apply_overrides(cards, overrides=None):
    """Merge overrides into the in-memory cards dict.

    Only fields present in each override entry are replaced; everything
    else is left as-is. Overrides for unknown card_ids are skipped with
    a warning (most likely a typo; surface the error rather than
    silently ingesting an orphan entry).
    """
    if overrides is None:
        overrides = load_card_overrides()
    applied = 0
    for cid, patch in overrides.items():
        # Underscore-prefixed keys are meta/comments (e.g. `_comment`),
        # not card IDs.
        if cid.startswith("_"):
            continue
        if cid not in cards:
            print(f"  WARNING: override for unknown card_id {cid!r} skipped")
            continue
        cards[cid].update(patch)
        applied += 1
    return applied


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

    Preserves ``index`` and ``norm_id`` from the old entry when a card is
    being re-fetched (so that stable tensor encoding is not corrupted).
    Genuinely new cards will lack these fields until ``build_registry.py``
    is run to assign indices.
    """
    prefix = set_id + "-"
    # Save old entries so we can carry over index/norm_id
    old_entries = {cid: c for cid, c in existing.items() if cid.startswith(prefix)}
    filtered = {cid: c for cid, c in existing.items() if not cid.startswith(prefix)}
    new_ids = set()
    for card in new_cards:
        cid = card["card_id"]
        new_ids.add(cid)
        old = old_entries.get(cid, {})
        if "index" in old:
            card["index"] = old["index"]
        if "norm_id" in old:
            card["norm_id"] = old["norm_id"]
        filtered[cid] = card

    # Safety check: warn about cards that had indices but are missing from
    # the API response (would lose their index on save)
    dropped = [cid for cid in old_entries if cid not in new_ids and "index" in old_entries[cid]]
    if dropped:
        print(f"  WARNING: {len(dropped)} cards with stable indices missing from API response:")
        for cid in sorted(dropped)[:10]:
            print(f"    {cid} (index={old_entries[cid]['index']})")
        if len(dropped) > 10:
            print(f"    ... and {len(dropped) - 10} more")
        print("  These cards will be REMOVED from cards.json.")
        print("  If this is unexpected, the API may be incomplete. Consider aborting.")

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
        print("No priority sets found. Ensure tools/scraper/priority_sets.txt exists.")
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

    # Apply manual overrides (tri-color cards, API corrections) before save.
    overrides_applied = apply_overrides(existing)

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
    print(f"Added {total_new} new cards. Applied {overrides_applied} manual overrides.")

    # Warn about missing indices for new cards
    missing_indices = sum(1 for v in existing.values() if "index" not in v)
    if missing_indices:
        print(f"\nWARNING: {missing_indices} cards are missing index/norm_id fields.")
        print("Run `python tools/build_registry.py` to assign stable indices.")


def backfill_xros_costs(cards_path=None):
    """Rewrite cards.json with parsed `dna_costs` / `digixros_costs` fields.

    Reads each card's existing `xros_req` string and re-runs the runtime
    parsers to emit structured fields alongside. Used to migrate cards.json
    in place without re-fetching from the API. Returns `(dna_count,
    digixros_count, total)` for caller logging.
    """
    if cards_path is None:
        cards, cards_path = load_cards_json()
    else:
        with open(cards_path, "r", encoding="utf-8") as f:
            cards = json.load(f)
        if isinstance(cards, list):
            cards = {c["card_id"]: c for c in cards}

    dna_count = 0
    digixros_count = 0
    for entry in cards.values():
        xros_req = entry.get("xros_req") or ""
        dna_json, digixros_json = _parse_xros_costs(xros_req)
        if dna_json:
            entry["dna_costs"] = dna_json
            dna_count += 1
        else:
            entry.pop("dna_costs", None)
        if digixros_json:
            entry["digixros_costs"] = digixros_json
            digixros_count += 1
        else:
            entry.pop("digixros_costs", None)

    # Overrides run LAST so manual corrections win over both the API
    # payload and the parser-derived fields above.
    overrides_applied = apply_overrides(cards)

    save_cards_json(cards, cards_path)
    return dna_count, digixros_count, len(cards), overrides_applied


def main():
    # --backfill mode: regenerate dna_costs / digixros_costs from existing xros_req
    if len(sys.argv) >= 2 and sys.argv[1] == "--backfill":
        dna_count, digixros_count, total, overrides_applied = backfill_xros_costs()
        print(
            f"Backfilled {total} cards: {dna_count} with dna_costs, "
            f"{digixros_count} with digixros_costs, {overrides_applied} overrides applied"
        )
        return

    # --bulk mode: ingest all priority sets
    if len(sys.argv) >= 2 and sys.argv[1] == "--bulk":
        bulk_ingest()
        return

    # --set mode: ingest a single set by ID (no set name needed)
    if len(sys.argv) >= 3 and sys.argv[1] == "--set":
        set_id = sys.argv[2].upper()
        existing, cards_path = load_cards_json()
        existing = ingest_single_set(set_id, existing, cards_path)
        apply_overrides(existing)
        save_cards_json(existing, cards_path)

        set_counts = {}
        for cid in existing:
            m = re.match(r'^([A-Z]+\d*)-', cid)
            if m:
                sid = m.group(1)
                set_counts[sid] = set_counts.get(sid, 0) + 1
        print(f"Wrote {len(existing)} total cards across {len(set_counts)} sets to {cards_path}")

        missing_indices = sum(1 for v in existing.values() if "index" not in v)
        if missing_indices:
            print(f"\nWARNING: {missing_indices} cards are missing index/norm_id fields.")
            print("Run `python tools/build_registry.py` to assign stable indices.")
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

    # Load existing cards.json and merge (preserving index/norm_id)
    existing, cards_path = load_cards_json()
    merged = merge_set_into_cards(existing, new_cards, set_id)
    apply_overrides(merged)
    save_cards_json(merged, cards_path)

    print(f"Wrote {len(merged)} total cards ({len(new_cards)} {set_id}) to {cards_path}")

    missing_indices = sum(1 for v in merged.values() if "index" not in v)
    if missing_indices:
        print(f"\nWARNING: {missing_indices} cards are missing index/norm_id fields.")
        print("Run `python tools/build_registry.py` to assign stable indices.")


if __name__ == "__main__":
    main()
