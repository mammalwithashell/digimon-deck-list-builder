#!/usr/bin/env python3
"""One-time migration: refresh the P-239/P-240/P-244 pre-release stubs; drop the RB1-10 duplicate.

P-239 DemiDevimon, P-240 Arcturusmon and P-244 Unique Emblem: Ragnarok Attainer
(Official Store Tournament 2026 Vol.3) were ingested before release: play cost 0,
no DP, no digivolve circles, no text, `|applinkdp =` residue in the inherited field.
Production builds CardData from data/cards.json, so in every real game they were
free, DP-less, text-less cards. The official mirror held only empty "skeleton"
entries for them, because the official search found nothing when it was scraped.

The digimoncard.io API has since filled them in. This script:
  1. adds data/card_overrides.json corrections for what the API still gets wrong
     against the official Bandai DB (see the `_note_store_tournament_2026_vol3` key);
  2. refreshes their data/cards.json entries the way a re-ingest would --
     ingest_cards.convert_card over the API rows embedded below (as served on
     2026-10-03), keeping index/norm_id, then ingest_cards.apply_overrides;
  3. replaces their data/card_official.json skeletons with the official pages
     (embedded below, as build_card_bundles.parse_official read them on 2026-10-03)
     and rewrites their data/card_bundles/<ID>.md from the corrected stats;
  4. writes their per-card engine JSON and data/card_meta pages, which every
     neighbouring promo (P-220..P-238) has and these three never got.

RB1-10 is a stale duplicate of RB1-010 Siriusmon that the API still serves (rarity
"Unknown", no set, older wording). The official DB has no RB1-10. It is removed from
cards.json, the official mirror (a skeleton), the bundles, the per-card engine JSON
and data/card_meta; ingest_cards.API_PHANTOM_IDS keeps a re-ingest from restoring it.
data/rag/index.json is left alone: it is a stale text snapshot built by an archived
tool, not a card index.

Both generated trees (code/digimon-engine/cards/**/*.json, data/card_meta) already
drift from cards.json for well over a thousand cards, so only these cards' files are
regenerated, not the trees.

Usage (repo root, after the ingest_cards.py change):
    python code/tools/archive/refresh_promo_stubs_rb1_10.py           # dry run
    python code/tools/archive/refresh_promo_stubs_rb1_10.py --apply
"""
import argparse
import json
import sys
from pathlib import Path

_CODE_DIR = Path(__file__).resolve().parents[2]
if str(_CODE_DIR) not in sys.path:
    sys.path.insert(0, str(_CODE_DIR))

from tools import build_card_bundles, ingest_cards, split_cards_json  # noqa: E402

PROMOS = ("P-239", "P-240", "P-244")
DROPPED = "RB1-10"
NOTE_KEY = "_note_store_tournament_2026_vol3"

# The API rows (only the fields convert_card reads), as served on 2026-10-03.
API_ROWS = json.loads(r'''{
    "P-239": {
        "id": "P-239",
        "name": "DemiDevimon",
        "type": "Digimon",
        "rarity": "P",
        "play_cost": 4,
        "dp": 3000,
        "level": 3,
        "color": "Purple",
        "color2": null,
        "digi_type": "Evil",
        "digi_type2": null,
        "digi_type3": null,
        "digi_type4": null,
        "form": null,
        "attribute": "Virus",
        "main_effect": "＜Blocker＞ (This Digimon can block in the blocker timing.)\r\n[On Deletion] By placing this Digimon card as the bottom digivolution card of 1 of your Digimon with [Myotismon] in its text, that Digimon may digivolve into a Digimon card with [Myotismon] in its name in your hand without paying the cost.",
        "source_effect": "[On Deletion] By trashing 1 card in your hand, delete 1 of your opponent's level 4 or lower Digimon.",
        "xros_req": "",
        "evolution_cost": 0,
        "evolution_color": null,
        "evolution_level": null
    },
    "P-240": {
        "id": "P-240",
        "name": "Arcturusmon",
        "type": "Digimon",
        "rarity": "P",
        "play_cost": 13,
        "dp": 13000,
        "level": 6,
        "color": "Purple",
        "color2": "Black",
        "digi_type": "Dragonkin",
        "digi_type2": "VB",
        "digi_type3": null,
        "digi_type4": null,
        "form": null,
        "attribute": "Virus",
        "main_effect": "＜Collision＞ (During this Digimon's attack, all of your opponent's Digimon gain ＜Blocker＞, and must block if possible.)\r\n＜Piercing＞ (When this Digimon deletes your opponent's Digimon in battle while attacking, it checks security before the attack ends.)\r\n＜Reboot＞ (This Digimon also unsuspends in your opponent's unsuspend phase.)\r\n＜Blocker＞ (This Digimon can block in the blocker timing.)\r\n[On Play] [When Digivolving] ＜De-Digivolve 3＞ 1 of your opponent's Digimon. (Trash up to 3 cards from the top. You can't trash past level 3 cards.) Then, by placing 2 cards with [Gammamon] in its text or the [VB] trait from your trash as this Digimon's bottom digivolution cards, give 1 of your opponent's Digimon \"[Start of Your Main Phase] This Digimon attacks.\" until their turn ends.\r\n[On Deletion] You may play 1 [Proximamon] from your hand or trash without paying the cost.",
        "source_effect": "[Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks, you may change the attack target to this Digimon.",
        "xros_req": "Assembly Requirements [Assembly -6] Lv.5 x Lv.4 x Lv.3, all w/[Gammamon] in text or w/[VB] trait\r\nWhen this would be played, by placing the specified cards from the trash under it, reduce the play cost.",
        "evolution_cost": 5,
        "evolution_color": null,
        "evolution_level": null
    },
    "P-244": {
        "id": "P-244",
        "name": "Unique Emblem: Ragnarok Attainer",
        "type": "Option",
        "rarity": "P",
        "play_cost": 3,
        "dp": null,
        "level": null,
        "color": "Black",
        "color2": null,
        "digi_type": "LIBERATOR",
        "digi_type2": null,
        "digi_type3": null,
        "digi_type4": null,
        "form": null,
        "attribute": null,
        "main_effect": "[Main] You may play 1 [Vemmon] or [Zenith] from your hand or trash without paying the cost. Then, place this card in the battle area.\r\n[All Turns] When effects place [Vemmon] as any of your Digimon's digivolution cards, ＜Delay＞ (By trashing this card after the placing turn, activate the effect below.)\r\n・1 of your Digimon with [Vemmon] in its text may digivolve into a Digimon card with [Vemmon] in its text in the hand or trash with the cost reduced by 3.",
        "source_effect": "[Security] Activate this card's [Main] effects.",
        "xros_req": "",
        "evolution_cost": null,
        "evolution_color": null,
        "evolution_level": null
    }
}''')

# The official pages, as build_card_bundles.parse_official read them on 2026-10-03.
OFFICIAL = json.loads(r'''{
    "P-239": {
        "card_id": "P-239",
        "digivolve_costs": [
            {
                "card_color": 6,
                "level": 2,
                "memory_cost": 0
            }
        ],
        "colors": "Purple",
        "play_cost": "4",
        "dp": "3000",
        "form": "Rookie",
        "attribute": "Virus",
        "type": "Evil",
        "qa": [
            "It refers to a card that contains the specified text or icon in its name, traits, effects, inherited effects, (Rule), digivolution requirements, DNA digivolution, DigiXros requirements, burst digivolve, App Fusion, Link, or Assembly requirements. For example, a card with [Knightmon] in its text would include cards with the name [DarkKnightmon] and cards with the text [Knightmon] in their effects."
        ],
        "text_sections": [
            {
                "label": "Effect",
                "text": "＜Blocker＞ [On Deletion] By placing this card from your trash as the bottom digivolution card of 1 of your Digimon with [Myotismon] in its text, that Digimon may digivolve into a Digimon card with [Myotismon] in its name in the hand without paying the cost."
            },
            {
                "label": "Inherited Effect",
                "text": "[On Deletion] By trashing 1 card in your hand, delete 1 of your opponent's level 4 or lower Digimon."
            }
        ]
    },
    "P-240": {
        "card_id": "P-240",
        "digivolve_costs": [
            {
                "card_color": 6,
                "level": 5,
                "memory_cost": 5
            },
            {
                "card_color": 5,
                "level": 5,
                "memory_cost": 5
            }
        ],
        "colors": "Purple Black",
        "play_cost": "13",
        "dp": "13000",
        "form": "Mega",
        "attribute": "Virus",
        "type": "Dragonkin/VB",
        "qa": [
            "It refers to a card that contains the specified text or icon in its name, traits, effects, inherited effects, (Rule), digivolution requirements, DNA digivolution, DigiXros requirements, burst digivolve, App Fusion, Link, or Assembly requirements. For example, a card with [Knightmon] in its text would include cards with the name [DarkKnightmon] and cards with the text [Knightmon] in their effects."
        ],
        "text_sections": [
            {
                "label": "Special Digivolution Condition",
                "text": "[Digivolve] Lv.5 w/[Gammamon] in text or w/[VB] trait: Cost 4"
            },
            {
                "label": "Effect",
                "text": "＜Collision＞ ＜Piercing＞ ＜Reboot＞ ＜Blocker＞ [On Play] [When Digivolving] ＜De-Digivolve 3＞ 1 of your opponent's Digimon. Then, by placing 2 cards with [Gammamon] in their texts or the [VB] trait from your trash as this Digimon's bottom digivolution cards, give 1 of your opponent's Digimon \"[Start of Your Main Phase] This Digimon attacks.\" until their turn ends. [On Deletion] You may play 1 [Proximamon] from your hand or trash without paying the cost."
            },
            {
                "label": "Special Play Condition",
                "text": "Assembly -6: Lv.5 × Lv.4 × Lv.3, all w/[Gammamon] in text or w/[VB] trait"
            },
            {
                "label": "Inherited Effect",
                "text": "[Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks, you may change the attack target to this Digimon."
            }
        ],
        "special_digivolution_condition": "[Digivolve] Lv.5 w/[Gammamon] in text or w/[VB] trait: Cost 4"
    },
    "P-244": {
        "card_id": "P-244",
        "digivolve_costs": [],
        "colors": "Black",
        "play_cost": "3",
        "type": "LIBERATOR",
        "qa": [
            "It refers to a card that contains the specified text or icon in its name, traits, effects, inherited effects, (Rule), digivolution requirements, DNA digivolution, DigiXros requirements, burst digivolve, App Fusion, Link, or Assembly requirements. For example, a card with [Knightmon] in its text would include cards with the name [DarkKnightmon] and cards with the text [Knightmon] in their effects."
        ],
        "text_sections": [
            {
                "label": "Effect",
                "text": "[Main] You may play 1 [Vemmon] or [Zenith] from your hand or trash without paying the cost. Then, place this card in the battle area. [Your Turn] When effects place [Vemmon] as any of your Digimon's digivolution cards, ＜Delay＞. ・1 of your Digimon with [Vemmon] in its text may digivolve into a Digimon card with [Vemmon] in its text in the hand or trash with the cost reduced by 3."
            },
            {
                "label": "Security Effect",
                "text": "[Security] Activate this card's [Main] effects."
            }
        ]
    }
}''')

NOTE = "P-239 DemiDevimon, P-240 Arcturusmon and P-244 Unique Emblem: Ragnarok Attainer (Official Store Tournament 2026 Vol.3) were ingested as pre-release stubs: play cost 0, no DP, no digivolve circles and no text, with |applinkdp = residue in the inherited field. The digimoncard.io API has since filled them in, and on 2026-10-03 their cards.json entries were refreshed from it (code/tools/archive/refresh_promo_stubs_rb1_10.py). These overrides correct what the API still gets wrong against the official Bandai DB (world.digimoncard.com, mirrored in data/card_official.json): P-239's form (Rookie) and its [On Deletion] wording, where the API says \"this Digimon card ... in your hand\" for the card's \"this card from your trash ... in the hand\"; P-240's form (Mega), its Black Lv.5 digivolve circle (the API drops the second colour) and its xros_req, which gains the [Digivolve] special digivolution line the API omits; and P-244's <Delay> trigger, which the official DB gives as [Your Turn] where the API says [All Turns]. Keyword reminder text follows the API's rendering, as elsewhere in cards.json. Source: the official DB pages fetched 2026-10-03; not yet checked against the card images."

OVERRIDES = json.loads(r'''{
    "P-239": {
        "form_eng": [
            "Rookie"
        ],
        "effect_description_eng": "＜Blocker＞ (This Digimon can block in the blocker timing.)\r\n[On Deletion] By placing this card from your trash as the bottom digivolution card of 1 of your Digimon with [Myotismon] in its text, that Digimon may digivolve into a Digimon card with [Myotismon] in its name in the hand without paying the cost."
    },
    "P-240": {
        "form_eng": [
            "Mega"
        ],
        "evo_costs": [
            {
                "card_color": 6,
                "level": 5,
                "memory_cost": 5
            },
            {
                "card_color": 5,
                "level": 5,
                "memory_cost": 5
            }
        ],
        "xros_req": "[Digivolve] Lv.5 w/[Gammamon] in text or w/[VB] trait: Cost 4\r\nAssembly Requirements [Assembly -6] Lv.5 × Lv.4 × Lv.3, all w/[Gammamon] in text or w/[VB] trait\r\nWhen this would be played, by placing the specified cards from the trash under it, reduce the play cost."
    },
    "P-244": {
        "effect_description_eng": "[Main] You may play 1 [Vemmon] or [Zenith] from your hand or trash without paying the cost. Then, place this card in the battle area.\r\n[Your Turn] When effects place [Vemmon] as any of your Digimon's digivolution cards, ＜Delay＞ (By trashing this card after the placing turn, activate the effect below.)\r\n・1 of your Digimon with [Vemmon] in its text may digivolve into a Digimon card with [Vemmon] in its text in the hand or trash with the cost reduced by 3."
    }
}''')


def read_json(path: Path):
    """(data, writer) where writer(data) reproduces the file's exact serialisation."""
    raw = path.read_bytes()
    data = json.loads(raw.decode("utf-8"))
    eol = "\r\n" if b"\r\n" in raw else "\n"
    for ensure_ascii in (False, True):
        body = json.dumps(data, indent=2, ensure_ascii=ensure_ascii).replace("\n", eol)
        for tail in (eol, ""):
            if (body + tail).encode("utf-8") == raw:
                def write(new, ea=ensure_ascii, t=tail):
                    out = json.dumps(new, indent=2, ensure_ascii=ea).replace("\n", eol) + t
                    path.write_bytes(out.encode("utf-8"))
                return data, write
    raise SystemExit(f"{path}: serialisation is not json.dumps(indent=2); refusing to rewrite it")


def refreshed_entry(old: dict, cid: str) -> dict:
    """convert_card over the API row, keeping index/norm_id and the old key order."""
    new = ingest_cards.convert_card(API_ROWS[cid])
    for key in ("index", "norm_id"):
        if key in old:
            new[key] = old[key]
    ordered = {k: new[k] for k in old if k in new}
    ordered.update((k, v) for k, v in new.items() if k not in ordered)
    ingest_cards.apply_overrides({cid: ordered}, {cid: OVERRIDES[cid]})
    return ordered


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--apply", action="store_true", help="write the changes (default: dry run)")
    ap.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3], help="repo root")
    args = ap.parse_args(argv)
    root = args.root
    if DROPPED not in ingest_cards.API_PHANTOM_IDS:
        raise SystemExit("ingest_cards.API_PHANTOM_IDS does not list RB1-10 yet; apply that change first")

    cards_path = root / "data" / "cards.json"
    overrides_path = root / "data" / "card_overrides.json"
    official_path = root / "data" / "card_official.json"
    bundles = root / "data" / "card_bundles"
    engine_cards = root / "code" / "digimon-engine" / "cards"
    card_meta = root / "data" / "card_meta"

    cards, write_cards = read_json(cards_path)
    overrides, write_overrides = read_json(overrides_path)
    index, write_index = read_json(official_path)

    # 1. overrides: the note, then one entry per card (replaced in place on a re-run).
    overrides[NOTE_KEY] = NOTE
    for cid in PROMOS:
        overrides[cid] = OVERRIDES[cid]

    # 2. cards.json
    changes = []
    for cid in PROMOS:
        old = cards[cid]
        new = refreshed_entry(old, cid)
        diff = [k for k in sorted(set(old) | set(new)) if old.get(k) != new.get(k)]
        changes.append(f"cards.json  {cid}: {', '.join(diff) or 'no change'}")
        cards[cid] = new
    if cards.pop(DROPPED, None) is not None:
        changes.append(f"cards.json  {DROPPED}: removed")

    # 3. official mirror (set, not just replace: a cleanup may already have dropped a skeleton)
    for cid in PROMOS:
        index["cards"][cid] = OFFICIAL[cid]
    if index["cards"].pop(DROPPED, None) is not None:
        changes.append(f"official    {DROPPED}: skeleton removed")
    index["count"] = len(index["cards"])
    changes.append(f"official    {', '.join(PROMOS)}: page data set; count {index['count']}")

    for line in changes:
        print(line)
    deletions = [bundles / f"{DROPPED}.md", engine_cards / "rb1" / f"{DROPPED}.json",
                 card_meta / "rb1" / f"{DROPPED}.md"]
    for path in deletions:
        print(f"delete      {path.relative_to(root)}{'' if path.exists() else ' (already absent)'}")
    for cid in PROMOS:
        for path in (bundles / f"{cid}.md", engine_cards / "p" / f"{cid}.json", card_meta / "p" / f"{cid}.md"):
            print(f"{'rewrite' if path.exists() else 'create':11} {path.relative_to(root)}")
    if not args.apply:
        print("dry run: nothing written")
        return 0

    # Imported before the first write: resolve_deck needs the digimon_engine bindings, and a
    # missing build must fail here rather than after cards.json has been rewritten.
    from tools import build_card_meta, resolve_deck
    write_overrides(overrides)
    write_cards(cards)
    write_index(index)
    for cid in PROMOS:
        build_card_bundles.write_bundle(cid, OFFICIAL[cid], cards[cid], str(bundles))
        split_cards_json.write_card_json(cid, cards[cid], engine_cards)
    # build_card_meta renders from resolve_deck's cached cards.json; seed it with the
    # corrected data so --root works on a scratch copy too.
    resolve_deck._CARDS_JSON_CACHE = cards
    for cid in PROMOS:
        build_card_meta.write_card_meta(cid, card_meta)
    for path in deletions:
        if path.exists():
            path.unlink()
    print("applied")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
