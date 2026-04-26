"""Backfill xros_req field in cards.json from DigimonCard.io API.

Only fetches cards that have Jogress Condition scripts but missing xros_req data.

Usage:
    python tools/backfill_xros_req.py [--dry-run]
"""
import json
import os
import sys
import time
import urllib.request

_REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CARDS_JSON = os.path.join(_REPO_ROOT, "data", "cards.json")
SCRIPTS_DIR = os.path.join(
    _REPO_ROOT, "digimon_gym", "engine", "data", "scripts"
)
API_URL = "https://digimoncard.io/index.php/api-public/search?card={card_id}"


def find_jogress_cards():
    """Find card IDs that have Jogress Condition in their scripts."""
    jogress_ids = []
    for root, dirs, files in os.walk(SCRIPTS_DIR):
        dirs[:] = [d for d in dirs if d not in ("generated", "__pycache__")]
        for fname in files:
            if not fname.endswith(".py"):
                continue
            fpath = os.path.join(root, fname)
            with open(fpath, "r", encoding="utf-8", errors="ignore") as f:
                if "Jogress Condition" in f.read():
                    card_id = fname.replace(".py", "").replace("_", "-").upper()
                    jogress_ids.append(card_id)
    return sorted(jogress_ids)


def fetch_xros_req(card_id):
    """Fetch xros_req from DigimonCard.io API."""
    url = API_URL.format(card_id=card_id)
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            if isinstance(data, list) and data:
                return data[0].get("xros_req", "")
            elif isinstance(data, dict):
                return data.get("xros_req", "")
    except Exception as e:
        print(f"  ERROR fetching {card_id}: {e}")
    return ""


def main():
    dry_run = "--dry-run" in sys.argv

    with open(CARDS_JSON, "r", encoding="utf-8") as f:
        cards = json.load(f)

    jogress_ids = find_jogress_cards()
    print(f"Found {len(jogress_ids)} scripts with Jogress Condition")

    # Filter to those missing xros_req
    need_fetch = [cid for cid in jogress_ids if not cards.get(cid, {}).get("xros_req")]
    print(f"Missing xros_req: {len(need_fetch)}")

    if not need_fetch:
        print("Nothing to do.")
        return

    updated = 0
    for i, card_id in enumerate(need_fetch):
        if dry_run:
            print(f"  WOULD FETCH: {card_id}")
            continue

        xros_req = fetch_xros_req(card_id)
        if xros_req:
            if card_id in cards:
                cards[card_id]["xros_req"] = xros_req
                print(f"  [{i+1}/{len(need_fetch)}] {card_id}: {xros_req[:80]}...")
                updated += 1
            else:
                print(f"  [{i+1}/{len(need_fetch)}] {card_id}: not in cards.json, skipping")
        else:
            print(f"  [{i+1}/{len(need_fetch)}] {card_id}: no xros_req from API")

        # Rate limit: 500ms between requests
        if i < len(need_fetch) - 1:
            time.sleep(0.5)

    if not dry_run and updated > 0:
        with open(CARDS_JSON, "w", encoding="utf-8") as f:
            json.dump(cards, f, ensure_ascii=False, indent=2)
        print(f"\nUpdated {updated} cards in cards.json")
    elif dry_run:
        print(f"\nDry run: would fetch {len(need_fetch)} cards")


if __name__ == "__main__":
    main()
