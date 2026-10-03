#!/usr/bin/env python3
"""Generate DCGO's DeckFormats.json from this repo's data/deck_formats.json.

Implements the design D8 mapping. The DCGO asset must be readable by Unity's
JsonUtility, which cannot deserialize dictionaries, top-level arrays, or nested
generic collections -- so `limited_to` (an object) and `choice_groups` (a list of
lists of lists) are flattened:

    banned:      [id, ...]        -> restrictions: [{id, limit: 0}, ...]
    limited:     [id, ...]        -> restrictions: [{id, limit: 1}, ...]
    limited_to:  {id: n}          -> restrictions: [{id, limit: n}, ...]
    choice_groups: [[[a],[b]]]    -> bannedPair:   [{id: a, pairs: [b]}]

Run from the repo root:
    python openspec/changes/add-dcgo-format-selection/gen_deck_formats.py

Re-run whenever data/deck_formats.json changes; the drift check (task 4.6)
compares the regenerated output against the committed DCGO asset.
"""

import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
SRC = REPO / "data" / "deck_formats.json"

# The formats DCGO ships. EDEN Singleton is deliberately absent (see design non-goals).
#
# roomToken / randomMatchKey are the on-the-wire matchmaking encodings, carried as DATA so no code
# branches on a format id (design D1/D2). The legacy values are load-bearing compatibility:
#
#   standard        roomToken "True"  -> room name "12345-True"   (byte-identical to today)
#   no_restriction  roomToken "False" -> room name "12345-False"  (byte-identical to today)
#   both share randomMatchKey "randomMatchRoom", which deliberately preserves the existing mixed
#   random-match pool rather than splitting it (design D4).
#
# New formats get keys that are NOT substrings/suffixes of the legacy one, so an unmodded client
# scanning for "randomMatchRoom" can never match them.
FORMATS = [
    # id,             displayName,  rarityPolicy,   banlistId,      copies, roomToken,   randomMatchKey,        enforce
    ("standard",       "Standard",   "all",          "official_eng", 4, "True",      "randomMatchRoom",     False),
    ("no_restriction", "No Banlist", "all",          "",             4, "False",     "randomMatchRoom",     False),
    ("singleton",      "Singleton",  "all",          "",             1, "singleton", "singletonMatchRoom",  True),
    ("eden",           "EDEN",       "eden_anomaly", "eden",         4, "eden",      "edenMatchRoom",       True),
]

# Banlists DCGO carries as shipped data. `official_eng` is intentionally NOT here:
# it keeps coming from the existing dcgo.online fetch (design D8).
SHIPPED_BANLISTS = ["eden"]


def build_banlist(name: str, block: dict) -> dict:
    """Flatten one restriction block into DCGO's BanList shape."""
    restrictions = []
    seen = {}

    def add(card_id: str, limit: int, origin: str) -> None:
        if card_id in seen and seen[card_id][0] != limit:
            raise SystemExit(
                f"conflicting limits for {card_id} in '{name}': "
                f"{seen[card_id][0]} (from {seen[card_id][1]}) vs {limit} (from {origin})"
            )
        if card_id in seen:
            return
        seen[card_id] = (limit, origin)
        restrictions.append({"id": card_id, "limit": limit})

    for cid in block.get("banned", []):
        add(cid, 0, "banned")
    for cid in block.get("limited", []):
        add(cid, 1, "limited")
    for cid, lim in (block.get("limited_to") or {}).items():
        add(cid, int(lim), "limited_to")

    banned_pair = []
    for group in block.get("choice_groups", []):
        flat = [cid for side in group for cid in side]
        if len(flat) < 2:
            continue
        banned_pair.append({"id": flat[0], "pairs": flat[1:]})

    restrictions.sort(key=lambda r: r["id"])
    banned_pair.sort(key=lambda p: p["id"])
    return {"id": name, "restrictions": restrictions, "bannedPair": banned_pair}


def build_anomaly(block: dict) -> dict:
    """Map the anomaly protocol onto DCGO's CardKind / Rarity vocabulary."""
    kind_map = {"digimon": "Digimon", "tamer": "Tamer", "option": "Option", "digi-egg": "DigiEgg"}
    categories = []
    for cat in block.get("categories", []):
        kind = cat.get("card_kind", "")
        if kind not in kind_map:
            raise SystemExit(f"unknown card_kind {kind!r} in anomaly_protocol")
        for r in cat.get("rarities", []):
            if r not in ("C", "U", "R", "SR", "UR", "SEC", "P"):
                raise SystemExit(f"unknown rarity {r!r} in anomaly_protocol")
        categories.append({
            "cardKind": kind_map[kind],
            "nameContains": cat.get("name_contains", ""),
            "rarities": list(cat.get("rarities", [])),
        })
    return {
        "maxTotal": int(block.get("max_total", 0)),
        "categories": categories,
        "extraCardIds": list(block.get("extra_card_ids", [])),
    }


def main() -> int:
    if not SRC.is_file():
        raise SystemExit(f"source registry not found: {SRC}")
    src = json.loads(SRC.read_text(encoding="utf-8"))

    out = {
        "_comment": (
            "GENERATED from data/deck_formats.json by gen_deck_formats.py -- do not hand-edit. "
            "Flattened for Unity JsonUtility (no dicts, no nested generic collections). "
            "official_eng is intentionally absent: it still comes from the existing online fetch."
        ),
        "formats": [
            {
                "id": fid,
                "displayName": name,
                "rarityPolicy": policy,
                "banlistId": banlist,
                "maxCopies": copies,
                "roomToken": token,
                "randomMatchKey": key,
                "enforceDeckValidation": enforce,
            }
            for fid, name, policy, banlist, copies, token, key, enforce in FORMATS
        ],
        "banlists": [
            build_banlist(n, src["restrictions"][n]) for n in SHIPPED_BANLISTS
        ],
        "anomalyProtocol": build_anomaly(src["anomaly_protocol"]),
    }

    text = json.dumps(out, indent=2, ensure_ascii=False) + "\n"
    dest = Path(__file__).resolve().parent / "assets" / "DeckFormats.json"
    dest.parent.mkdir(parents=True, exist_ok=True)

    # --check: verify a target asset still matches what the source registry generates,
    # so drift between the DCGO copy and data/deck_formats.json is detectable (task 4.6).
    if "--check" in sys.argv:
        targets = [a for a in sys.argv[1:] if not a.startswith("--")]
        targets = [Path(t) for t in targets] or [dest]
        failed = False
        for target in targets:
            if not target.is_file():
                print(f"DRIFT: {target} does not exist")
                failed = True
                continue
            actual = target.read_text(encoding="utf-8")
            if actual != text:
                print(f"DRIFT: {target} differs from data/deck_formats.json")
                failed = True
            else:
                print(f"ok: {target} matches data/deck_formats.json")
        return 1 if failed else 0

    dest.write_text(text, encoding="utf-8")

    eden = out["banlists"][0]
    limits = {}
    for r in eden["restrictions"]:
        limits[r["limit"]] = limits.get(r["limit"], 0) + 1
    print(f"wrote {dest}")
    print(f"  formats:            {len(out['formats'])}")
    print(f"  eden restrictions:  {len(eden['restrictions'])} " f"(by limit: {dict(sorted(limits.items()))})")
    print(f"  eden bannedPair:    {len(eden['bannedPair'])}")
    print(f"  anomaly categories: {len(out['anomalyProtocol']['categories'])}, "
          f"maxTotal={out['anomalyProtocol']['maxTotal']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
