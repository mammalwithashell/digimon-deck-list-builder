#!/usr/bin/env python3
"""
Fetch card effect data from digimoncard.io API for supported sets,
deduplicate, save to JSON, and analyze decision types in effect text.
"""

import json
import re
import time
import urllib.request
from collections import defaultdict
from pathlib import Path

API_URLS = {
    "BT14": "https://digimoncard.io/api-public/search.php?card=BT14&sort=cardnumber&sortdirection=asc",
    "BT24": "https://digimoncard.io/api-public/search.php?card=BT24&sort=cardnumber&sortdirection=asc",
    "ST1": "https://digimoncard.io/api-public/search.php?card=ST1&sort=cardnumber&sortdirection=asc",
    "BT23": "https://digimoncard.io/api-public/search.php?card=BT23&sort=cardnumber&sortdirection=asc",
}

# Only keep cards whose IDs start with these exact prefixes
VALID_SET_PREFIXES = ("BT14-", "BT24-", "ST1-", "BT23-")

OUTPUT_PATH = Path("/home/user/digimon-deck-list-builder/digimon_gym/engine/data/card_effects_api.json")


def fetch_set(set_name, url):
    """Fetch all cards for a set from the API."""
    print(f"Fetching {set_name} from {url} ...")
    req = urllib.request.Request(url, headers={"User-Agent": "DigimonDeckBuilder/1.0"})
    with urllib.request.urlopen(req, timeout=30) as resp:
        data = json.loads(resp.read().decode("utf-8"))
    print(f"  -> Got {len(data)} raw entries for {set_name}")
    return data


def extract_card(raw):
    """Extract relevant fields from a raw API card entry."""
    return {
        "id": raw.get("id", ""),
        "name": raw.get("name", ""),
        "type": raw.get("type", ""),
        "main_effect": raw.get("main_effect", "") or "",
        "source_effect": raw.get("source_effect", "") or "",
        "dp": raw.get("dp", None),
    }


def deduplicate(cards):
    """Keep first occurrence of each card ID (alt-arts share the same ID)."""
    seen = set()
    unique = []
    for c in cards:
        cid = c["id"]
        if cid and cid not in seen:
            seen.add(cid)
            unique.append(c)
    return unique


# ---------------------------------------------------------------------------
# Angle bracket helper: match both regular <> and full-width ＜＞
# ---------------------------------------------------------------------------

def _ab(inner):
    """Build a pattern that matches <inner> OR ＜inner＞."""
    return rf"(?:<{inner}>|＜{inner}＞)"


# ---------------------------------------------------------------------------
# Decision-type patterns
# ---------------------------------------------------------------------------

DECISION_PATTERNS = []

def _p(name, description, pattern):
    DECISION_PATTERNS.append((name, description, re.compile(pattern, re.IGNORECASE)))

# Play from hand/trash/reveal
_p("play", "play 1 / play up to (play from hand/trash/reveal)",
   r"play\s+(?:1|up\s+to\s+\d+|it\b)")

# Reveal top cards
_p("reveal_top", "reveal the top X cards (reveal and select)",
   r"reveal\s+(?:the\s+)?top\s+\d+\s+cards?")

# Digivolve selection
_p("digivolve_into", "digivolve into / may digivolve (digivolve selection)",
   r"(?:digivolve\s+into|may\s+digivolve|can\s+digivolve)")

# Mind Link
_p("mind_link", "<Mind Link>",
   _ab(r"Mind\s*Link"))

# Save
_p("save", "<Save> (save under tamer)",
   _ab(r"Save"))

# Delete opponent's digimon
_p("delete_opponent", "delete 1 of your opponent's (target opponent digimon)",
   r"delete\s+\d+\s+of\s+your\s+opponent")

# Suspend target
_p("suspend_target", "suspend 1 (target to suspend)",
   r"suspend\s+\d+")

# Return / bounce
_p("return_bounce", "return 1 / return to hand/deck (bounce)",
   r"return\s+\d+|return\s+(?:it|them|all)\s+to\s+(?:the\s+|their\s+)?(?:hand|deck|bottom|owner)")

# Trash selection
_p("trash_selection", "trash 1 / trash X cards (trash selection)",
   r"trash\s+(?:\d+\s+(?:card|of)|(?:its|the|any)\s+(?:top|bottom)?\s*\d*\s*digivolution)")

# Choose / select
_p("choose_select", "choose 1 / select (generic choice)",
   r"(?:choose|select)\s+\d+")

# Place at bottom/top of security
_p("security_place", "place X at the bottom/top of security (security manipulation)",
   r"place\s+.*(?:bottom|top)\s+of\s+.*security")

# Search security
_p("search_security", "search your security (security search)",
   r"search\s+(?:your\s+)?security")

# De-Digivolve
_p("de_digivolve", "<De-Digivolve>",
   _ab(r"De-Digivolve\s*\d*"))

# Draw keyword
_p("draw_keyword", "<Draw> (draw)",
   _ab(r"Draw\s*\d*"))

# Recovery
_p("recovery", "<Recovery>",
   _ab(r"Recovery\s*\+?\d*"))

# Memory gain
_p("memory_gain", "gain X memory (memory gain)",
   r"gain\s+\d+\s+memory")

# Blocker
_p("blocker", "<Blocker>",
   _ab(r"Blocker"))

# Jamming
_p("jamming", "<Jamming>",
   _ab(r"Jamming"))

# Retaliation
_p("retaliation", "<Retaliation>",
   _ab(r"Retaliation"))

# Rush
_p("rush", "<Rush>",
   _ab(r"Rush"))

# Security Attack +/-
_p("security_attack_mod", "<Security Attack +X> / <Security Attack -X>",
   _ab(r"Security\s+Attack\s+[+\-]\d+"))

# Piercing
_p("piercing", "<Piercing>",
   _ab(r"Piercing"))

# Multi-choice effects
_p("multi_choice", "activate 1 of the effects below (multi-choice effects)",
   r"(?:activate|use)\s+\d+\s+of\s+(?:the\s+)?(?:effects?|following)")

# Sacrifice / cost by deleting own
_p("sacrifice_cost", "by deleting 1 of your (sacrifice/cost)",
   r"by\s+(?:deleting|trashing)\s+\d+\s+of\s+your")

# Protection / substitution
_p("protection", "prevent it from leaving / protection / substitution",
   r"(?:prevent\s+(?:it|that|them)\s+from\s+(?:leaving|being)|prevent\s+that\s+deletion|isn't\s+(?:deleted|destroyed))")

# Can't be deleted / can't attack / can't be blocked (restriction)
_p("restriction", "can't be deleted / can't attack / can't be blocked (restriction)",
   r"can'?t\s+(?:be\s+)?(?:deleted|attack|blocked|suspend|play)")

# DP modification
_p("dp_modification", "gets +XXXX DP / gets -XXXX DP (DP modification)",
   r"(?:get|gain)s?\s+[+\-]\d+\s*DP")

# Stack manipulation (place cards from trash as digivolution cards)
_p("stack_manipulation", "place X cards from trash as digivolution cards (stack manipulation)",
   r"(?:place|add)\s+.*(?:as\s+(?:a\s+)?digivolution\s+cards?|(?:under|to)\s+(?:this|it|one\s+of).*digivolution)")

# Tokens
_p("token", "Token",
   r"\bToken\b")

# Armor Purge
_p("armor_purge", "<Armor Purge>",
   _ab(r"Armor\s*Purge"))

# Alliance
_p("alliance", "<Alliance>",
   _ab(r"Alliance"))

# Blast Digivolve / Blast DNA Digivolve
_p("blast_digivolve", "<Blast Digivolve> / <Blast DNA Digivolve>",
   _ab(r"Blast\s+(?:DNA\s+)?Digivolve"))

# Decoy
_p("decoy", "<Decoy>",
   _ab(r"Decoy"))

# Fortitude
_p("fortitude", "<Fortitude>",
   _ab(r"Fortitude"))

# Ice Clad
_p("ice_clad", "<Ice Clad>",
   _ab(r"Ice\s*Clad"))

# Material Save
_p("material_save", "<Material Save>",
   _ab(r"Material\s*Save"))

# Barrier
_p("barrier", "<Barrier>",
   _ab(r"Barrier"))

# Evade
_p("evade", "<Evade>",
   _ab(r"Evade"))

# Raid
_p("raid", "<Raid>",
   _ab(r"Raid"))

# Reboot
_p("reboot", "<Reboot>",
   _ab(r"Reboot"))

# Collision
_p("collision", "<Collision>",
   _ab(r"Collision"))

# Blitz
_p("blitz", "<Blitz>",
   _ab(r"Blitz"))

# Overclock
_p("overclock", "<Overclock>",
   _ab(r"Overclock"))

# Return to hand/deck (broader: "return ... to hand/deck")
_p("return_to_hand_deck", "return to hand/deck (broader bounce)",
   r"return\s+.*to\s+(?:the\s+)?(?:hand|deck|bottom\s+of\s+(?:the\s+)?deck|owner)")

# Doesn't unsuspend
_p("doesnt_unsuspend", "doesn't unsuspend (stun effect)",
   r"doesn't\s+unsuspend")

# Reduce play/digivolve cost
_p("cost_reduction", "reduce play/digivolve cost",
   r"reduce\s+(?:the\s+)?(?:play|digivolve|digivolution|evolution)\s+cost")

# Lose memory (opponent)
_p("memory_loss", "lose X memory",
   r"lose\s+\d+\s+memory")


def analyze_decisions(cards):
    """Analyze all effect text for decision-type patterns."""
    results = {}

    for pname, pdesc, pregex in DECISION_PATTERNS:
        matching_ids = []
        for c in cards:
            combined = c["main_effect"] + " " + c["source_effect"]
            if pregex.search(combined):
                matching_ids.append(c["id"])
        if matching_ids:
            results[pname] = {
                "description": pdesc,
                "count": len(matching_ids),
                "examples": matching_ids[:3],
                "all_ids": matching_ids,
            }

    return results


def main():
    all_raw = []
    for set_name, url in API_URLS.items():
        try:
            entries = fetch_set(set_name, url)
            all_raw.extend(entries)
        except Exception as e:
            print(f"  ERROR fetching {set_name}: {e}")
        time.sleep(0.5)  # polite delay

    print(f"\nTotal raw entries fetched: {len(all_raw)}")

    # Extract and deduplicate
    extracted = [extract_card(r) for r in all_raw]
    unique = deduplicate(extracted)
    print(f"Unique cards after deduplication (before set filter): {len(unique)}")

    # Filter to only our supported sets
    unique = [c for c in unique if c["id"].startswith(VALID_SET_PREFIXES)]
    print(f"Unique cards after filtering to supported sets: {len(unique)}")

    # Sort by ID for readability
    unique.sort(key=lambda c: c["id"])

    # Save to JSON
    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(OUTPUT_PATH, "w", encoding="utf-8") as f:
        json.dump(unique, f, indent=2, ensure_ascii=False)
    print(f"Saved to {OUTPUT_PATH}\n")

    # Per-set summary
    set_counts = defaultdict(int)
    for c in unique:
        prefix = c["id"].split("-")[0] if "-" in c["id"] else "unknown"
        set_counts[prefix] += 1
    print("Cards per set:")
    for s, cnt in sorted(set_counts.items()):
        print(f"  {s}: {cnt}")

    # Effect text stats
    with_main = sum(1 for c in unique if c["main_effect"].strip())
    with_source = sum(1 for c in unique if c["source_effect"].strip())
    with_any = sum(1 for c in unique if c["main_effect"].strip() or c["source_effect"].strip())
    print(f"\nCards with main_effect: {with_main}")
    print(f"Cards with source_effect: {with_source}")
    print(f"Cards with any effect text: {with_any}")
    print()

    # Analyze decision types
    analysis = analyze_decisions(unique)

    print("=" * 80)
    print("DECISION TYPE ANALYSIS")
    print("=" * 80)

    # Sort by count descending
    sorted_types = sorted(analysis.items(), key=lambda x: x[1]["count"], reverse=True)

    for pname, info in sorted_types:
        examples_str = ", ".join(info["examples"])
        print(f"\n{pname}")
        print(f"  Description : {info['description']}")
        print(f"  Card count  : {info['count']}")
        print(f"  Examples    : {examples_str}")

    # Summary table
    print("\n" + "=" * 80)
    print(f"{'Decision Type':<30} {'Count':>6}  {'Examples'}")
    print("-" * 80)
    for pname, info in sorted_types:
        examples_str = ", ".join(info["examples"][:3])
        print(f"{pname:<30} {info['count']:>6}  {examples_str}")
    print("=" * 80)
    print(f"Total unique decision types found: {len(analysis)}")

    # Cards with NO recognized decision patterns
    cards_with_effects = set()
    for info in analysis.values():
        cards_with_effects.update(info["all_ids"])
    cards_with_any_effect_text = [
        c for c in unique if c["main_effect"].strip() or c["source_effect"].strip()
    ]
    cards_no_pattern = [
        c["id"] for c in cards_with_any_effect_text if c["id"] not in cards_with_effects
    ]
    print(f"\nCards with effect text but NO recognized decision pattern: {len(cards_no_pattern)}")
    if cards_no_pattern:
        for cid in cards_no_pattern[:20]:
            card = next(c for c in unique if c["id"] == cid)
            eff = (card["main_effect"] + " | " + card["source_effect"]).strip()
            truncated = eff[:150]
            print(f"  {cid} ({card['name']}): {truncated}")


if __name__ == "__main__":
    main()
