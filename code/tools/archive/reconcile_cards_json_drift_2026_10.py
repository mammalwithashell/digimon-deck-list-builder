#!/usr/bin/env python3
"""One-time migration: reconcile data/cards.json with the official Bandai DB (2026-10-03).

A survey of cards.json against data/card_official.json found 97 cards whose play cost, DP,
colours or text disagreed. Each was judged against the official base printing, the DCGO
card asset (DCGO/Assets/CardBaseEntity, in the base repo) and the card image, because both
sides had errors:

  cards.json wrong, corrected here (80 cards, 110 fields: 73 from the survey and 7 found
  while checking them; all as data/card_overrides.json entries so a re-ingest keeps them,
  see the `_note_cards_json_drift_2026_10` key):
    - never-refreshed pre-release stubs: BT24-028/061/068/078 had cost 0, no DP, no text
      and no special digivolution condition; BT24-044, ST23-06, ST24-04 cost 0 and no DP;
    - play cost or DP the API still serves wrong (BT17-042 4 for 3, BT20-056 0 for 12, ...);
    - the third colour of 28 three-colour cards (the API has two colour fields), and
      BT16-102, whose earlier override gave white for black;
    - `|applinkdp =` MediaWiki residue as the inherited text of 18 cards with no inherited
      effect;
    - stale or missing text: ST24-04 and EX11-033 (the cards print [When Moving] and newer
      wording), BT25-088 (the effect field held the security text), BT8-061, BT25-059, and
      the Digi-Eggs BT13-006 / BT22-001, whose inherited text sat in the effect field;
    - missing traits: AD1-006 [Data], BT24-020 / BT24-022 [Vaccine], EX10-069 [LIBERATOR]
      (P-169's [LIBERATOR] moves from attribute to type, as printed);
    - the special digivolution condition missing from xros_req: BT14-081, BT20-073, BT24-025
      and the four BT24/BT25 stubs.

  The DSL YAML of 25 of these cards (code/digimon-engine/cards/**) had copied the old
  colour, cost or DP; DebugRunner tests compile stats from it, so the same change
  corrects those lines by hand. This script does not touch YAML.

  The official mirror wrong, cards.json already right (22 cards, no change here): the
  builder keeps the last printing's cost/DP/colour from a page listing every printing, some
  alt-printing blocks carry another card's stats, and a few official entries are simply
  wrong. code/tests/test_cards_json_integrity.py records them in PRINTED_OVER_MIRROR.

  Left alone: BT10-007, BT10-063, BT12-058 and BT18-020 print no effect beyond a
  special digivolution/play condition or a (Rule) box, which cards.json files elsewhere.

Texts come from the DCGO asset (the API's clause breaks and NBSPs) unless it is stale, and
equal the official text up to whitespace, zone braces, reminder text and the (Rule) box.
An API refresh of these cards would change nothing beyond the overrides (checked against
the rows served on 2026-10-03), so the entries are patched in place, keeping index/norm_id.

Only these cards' existing per-card engine JSON and data/card_meta pages are regenerated;
both trees are partial and stale elsewhere (BT25 has 16 of 98 engine JSON files), so no new
files are created.

Usage (repo root):
    python code/tools/archive/reconcile_cards_json_drift_2026_10.py           # dry run
    python code/tools/archive/reconcile_cards_json_drift_2026_10.py --apply
"""
import argparse
import json
import sys
from pathlib import Path

_CODE_DIR = Path(__file__).resolve().parents[2]
if str(_CODE_DIR) not in sys.path:
    sys.path.insert(0, str(_CODE_DIR))

from tools import ingest_cards, split_cards_json  # noqa: E402

NOTE_KEY = "_note_cards_json_drift_2026_10"
NOTE = (
    "2026-10-03 reconciliation against the official Bandai DB; every value checked against the card "
    "image and the DCGO card asset (code/tools/archive/reconcile_cards_json_drift_2026_10.py lists "
    "them). The API still serves these wrong unless noted: pre-release stubs BT24-028/061/068/078 "
    "(cost 0, no DP, no text, no special digivolution condition), BT24-044 and ST23-06 (cost 0, no "
    "DP); ST24-04 (the API now has its cost/DP, but its text is still the stale pre-release text); "
    "play cost or DP; the third colour of three-colour cards (the API has two colour fields; "
    "BT16-102's earlier override said white for black); '|applinkdp =' residue as inherited text; "
    "stale or misfiled text (EX11-033 and ST24-04 print [When Moving]; BT25-088's effect held its "
    "security text; Digi-Eggs BT13-006/BT22-001 keep inherited text in the inherited field); missing "
    "traits (P-169's LIBERATOR is a type, as printed); special digivolution conditions missing from "
    "xros_req. Where the official mirror itself is wrong, cards.json keeps the printed value: see "
    "PRINTED_OVER_MIRROR in code/tests/test_cards_json_integrity.py."
)

# Field corrections per card, merged into data/card_overrides.json.
CORRECTIONS = json.loads(r'''
{
    "AD1-006": {
        "card_colors": [
            0,
            5,
            1
        ],
        "attribute_eng": [
            "Data"
        ]
    },
    "AD1-020": {
        "card_colors": [
            1,
            0,
            3
        ]
    },
    "AD1-023": {
        "card_colors": [
            5,
            2,
            6
        ]
    },
    "AD1-025": {
        "card_colors": [
            0,
            4,
            1
        ]
    },
    "BT13-006": {
        "inherited_effect_description_eng": "[On Deletion] By trashing 1 card in your hand, delete 1 of your opponent's level 3 Digimon.",
        "effect_description_eng": ""
    },
    "BT14-081": {
        "xros_req": "[Digivolve] [Seraphimon]: Cost 1"
    },
    "BT16-102": {
        "card_colors": [
            2,
            1,
            5
        ]
    },
    "BT17-042": {
        "play_cost": 3
    },
    "BT17-077": {
        "card_colors": [
            4,
            1,
            3
        ]
    },
    "BT18-018": {
        "card_colors": [
            0,
            3,
            1
        ]
    },
    "BT18-042": {
        "card_colors": [
            2,
            6,
            5
        ]
    },
    "BT18-095": {
        "card_colors": [
            0,
            3,
            1
        ]
    },
    "BT18-097": {
        "card_colors": [
            2,
            6,
            5
        ]
    },
    "BT19-014": {
        "card_colors": [
            0,
            2,
            5
        ]
    },
    "BT19-091": {
        "card_colors": [
            0,
            2,
            3
        ]
    },
    "BT19-101": {
        "card_colors": [
            0,
            6,
            5
        ]
    },
    "BT20-045": {
        "card_colors": [
            3,
            0,
            1
        ]
    },
    "BT20-056": {
        "play_cost": 12
    },
    "BT20-060": {
        "play_cost": 9
    },
    "BT20-073": {
        "xros_req": "[Digivolve] [Phantomon]: Cost 1"
    },
    "BT20-102": {
        "card_colors": [
            1,
            4,
            0
        ]
    },
    "BT21-019": {
        "play_cost": 5
    },
    "BT21-030": {
        "inherited_effect_description_eng": ""
    },
    "BT21-035": {
        "inherited_effect_description_eng": ""
    },
    "BT21-052": {
        "card_colors": [
            3,
            0,
            1
        ]
    },
    "BT22-001": {
        "inherited_effect_description_eng": "[Your Turn] [Once Per Turn] When effects place Digimon cards with [Aqua] or [Sea Animal] in any of their traits in this Digimon's digivolution cards, ＜Draw 1＞ (Draw 1 card from your deck.)",
        "effect_description_eng": ""
    },
    "BT22-015": {
        "card_colors": [
            0,
            4,
            1
        ]
    },
    "BT23-005": {
        "play_cost": 3
    },
    "BT23-007": {
        "dp": 1000
    },
    "BT23-010": {
        "dp": 5000
    },
    "BT23-036": {
        "dp": 12000
    },
    "BT23-044": {
        "play_cost": 7
    },
    "BT23-047": {
        "card_colors": [
            3,
            0,
            1
        ]
    },
    "BT24-020": {
        "attribute_eng": [
            "Vaccine"
        ]
    },
    "BT24-022": {
        "attribute_eng": [
            "Vaccine"
        ]
    },
    "BT24-025": {
        "xros_req": "[Digivolve] Lv.3 w/[TS] trait: Cost 2"
    },
    "BT24-028": {
        "play_cost": 6,
        "dp": 6000,
        "effect_description_eng": "[On Play] [When Digivolving] By placing 1 level 5 or lower blue [TS] trait Digimon card from your hand as this Digimon's bottom digivolution card, until your opponent's turn ends, this Digimon can't be deleted in battle and gains ＜Blocker＞.\r\n[Your Turn] When this Digimon unsuspends, it may digivolve into [Neptunemon] in the hand without paying the cost.",
        "inherited_effect_description_eng": "[When Attacking] [Once Per Turn] You may play 1 level 4 or lower blue Digimon card with the [TS] trait from this Digimon's digivolution cards without paying the cost.",
        "xros_req": "[Digivolve] Lv.4 w/[Aqua] or [Sea Animal] in any trait or w/[TS] trait: Cost 3"
    },
    "BT24-044": {
        "play_cost": 3,
        "dp": 1000
    },
    "BT24-061": {
        "play_cost": 6,
        "dp": 7000,
        "effect_description_eng": "[On Play] [When Digivolving] Return 1 of your opponent's play cost 3 or lower Digimon or Tamers to the top of the deck.",
        "inherited_effect_description_eng": "[When Attacking] [Once Per Turn] ＜De-Digivolve 1＞ 1 of your opponent's Digimon.",
        "xros_req": "[Digivolve] Lv.4 w/[TS] trait: Cost 3"
    },
    "BT24-068": {
        "play_cost": 3,
        "dp": 1000,
        "effect_description_eng": "[On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Evil] or [Fallen Angel] trait and 1 card with the [Seven Great Demon Lords] trait among them to the hand. Return the rest to the bottom of the deck. Then, trash 1 card in your hand.",
        "inherited_effect_description_eng": "[When Attacking] [Once Per Turn] Trash the top card of both players' decks."
    },
    "BT24-078": {
        "play_cost": 13,
        "dp": 13000,
        "effect_description_eng": "[Trash] [Your Turn] When one of your [Creepymon] attacks, if your opponent has 10 or more cards in their trash, by digivolving it into this card without paying the cost, trash your opponent's top security card.\r\n[When Digivolving] Delete all of your opponent's lowest level Digimon. Then, you may play up to 4 play cost's total worth of [Evil] or [Fallen Angel] trait cards from your trash without paying the cost. For every 10 cards in your opponent's trash, add 4 to this effect's play cost maximum.",
        "xros_req": "[Digivolve] [Creepymon]: Cost 2"
    },
    "BT25-019": {
        "inherited_effect_description_eng": ""
    },
    "BT25-020": {
        "inherited_effect_description_eng": ""
    },
    "BT25-029": {
        "inherited_effect_description_eng": ""
    },
    "BT25-037": {
        "inherited_effect_description_eng": ""
    },
    "BT25-042": {
        "inherited_effect_description_eng": ""
    },
    "BT25-058": {
        "inherited_effect_description_eng": ""
    },
    "BT25-059": {
        "inherited_effect_description_eng": "",
        "effect_description_eng": "When this card would be played, if there are 2 or more suspended Digimon, reduce the cost by 5.\r\n[On Play] [When Digivolving] You may suspend up to 2 Digimon. Then, none of your suspended [Vegetation] or [TS] trait Digimon are affected by your opponent's Digimon effects until their turn ends.\r\n[All Turns] [Once Per Turn] When any Digimon suspend, to 1 of your opponent's Digimon, give -3000 DP until their turn ends for each suspended Digimon.",
        "xros_req": "[Digivolve] Lv.5 w/[Vegetation]/[TS] trait: Cost 3"
    },
    "BT25-060": {
        "inherited_effect_description_eng": ""
    },
    "BT25-075": {
        "inherited_effect_description_eng": ""
    },
    "BT25-076": {
        "inherited_effect_description_eng": ""
    },
    "BT25-077": {
        "inherited_effect_description_eng": ""
    },
    "BT25-084": {
        "card_colors": [
            6,
            0,
            3
        ],
        "inherited_effect_description_eng": ""
    },
    "BT25-088": {
        "play_cost": 4,
        "effect_description_eng": "[Start of Your Turn] If you have 2 or less memory, set it to 3.\r\n[All Turns] When your security stack is removed from, by suspending this Tamer, you may place the top 2 cards of your deck face down under this Tamer.\r\n[Your Turn] [Once Per Turn] When any of your [Glowing Dawn] trait cards would be played, by trashing the bottom face-down card from under any of your Tamers, reduce the cost by 1."
    },
    "BT25-103": {
        "inherited_effect_description_eng": ""
    },
    "BT8-061": {
        "effect_description_eng": "The name of this card/Digimon is also treated as [Mamemon]."
    },
    "EX10-011": {
        "card_colors": [
            0,
            6,
            1
        ]
    },
    "EX10-069": {
        "type_eng": [
            "LIBERATOR"
        ]
    },
    "EX11-013": {
        "dp": 1000
    },
    "EX11-033": {
        "dp": 7000,
        "effect_description_eng": "[When Moving] [When Digivolving] You may play 1 [Maquinamon] from your hand or this Digimon's link cards without paying the cost.\r\n[Your Turn] [Once Per Turn] When this Digimon gets linked, suspend 1 of your opponent's Digimon. Then, 1 of their Digimon can't unsuspend until their turn ends."
    },
    "EX12-048": {
        "play_cost": 13,
        "dp": 13000,
        "card_colors": [
            2,
            0,
            1
        ]
    },
    "EX12-065": {
        "inherited_effect_description_eng": ""
    },
    "EX4-047": {
        "play_cost": 8
    },
    "EX7-037": {
        "card_colors": [
            3,
            2,
            5
        ]
    },
    "EX7-074": {
        "card_colors": [
            3,
            2,
            6
        ]
    },
    "EX8-029": {
        "card_colors": [
            1,
            5,
            2
        ]
    },
    "EX8-064": {
        "card_colors": [
            6,
            5,
            2
        ]
    },
    "EX8-073": {
        "card_colors": [
            0,
            1,
            2
        ]
    },
    "EX9-021": {
        "card_colors": [
            1,
            4,
            0
        ]
    },
    "EX9-039": {
        "play_cost": 5
    },
    "EX9-045": {
        "card_colors": [
            3,
            2,
            1
        ]
    },
    "P-136": {
        "play_cost": 3
    },
    "P-168": {
        "play_cost": 3
    },
    "P-169": {
        "play_cost": 3,
        "type_eng": [
            "LIBERATOR"
        ],
        "attribute_eng": []
    },
    "P-185": {
        "card_colors": [
            3,
            0,
            1
        ]
    },
    "ST23-05": {
        "inherited_effect_description_eng": ""
    },
    "ST23-06": {
        "play_cost": 3,
        "dp": 1000
    },
    "ST24-04": {
        "play_cost": 3,
        "dp": 1000,
        "effect_description_eng": "[When Moving] [On Play] Reveal the top 3 cards of your deck. Among them, add 1 [DATA SQUAD] trait card to the hand and place 1 such card face down under any of your [DATA SQUAD] trait Tamers. Return the rest to the bottom of the deck."
    },
    "ST24-08": {
        "dp": 2000
    },
    "ST24-11": {
        "inherited_effect_description_eng": ""
    }
}
''')


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


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--apply", action="store_true", help="write the changes (default: dry run)")
    ap.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3], help="repo root")
    args = ap.parse_args(argv)
    root = args.root
    engine_cards = root / "code" / "digimon-engine" / "cards"
    card_meta = root / "data" / "card_meta"

    cards, write_cards = read_json(root / "data" / "cards.json")
    overrides, write_overrides = read_json(root / "data" / "card_overrides.json")

    # 1. overrides: the note, then each card's fields merged into its entry (in place on a re-run).
    overrides[NOTE_KEY] = NOTE
    for cid, fields in CORRECTIONS.items():
        overrides.setdefault(cid, {}).update(fields)

    # 2. cards.json: re-apply each touched card's whole override entry, as a re-ingest would.
    touched = []
    for cid in CORRECTIONS:
        before = dict(cards[cid])
        ingest_cards.apply_overrides({cid: cards[cid]}, {cid: overrides[cid]})
        diff = [k for k in sorted(set(before) | set(cards[cid])) if before.get(k) != cards[cid].get(k)]
        if diff:
            touched.append(cid)
        print(f"cards.json  {cid}: {', '.join(diff) or 'no change'}")

    targets = []
    for cid in touched:
        folder = split_cards_json.set_id_from_card_id(cid)
        for path in (engine_cards / folder / f"{cid}.json", card_meta / folder / f"{cid}.md"):
            if path.exists():
                targets.append(path)
                print(f"rewrite     {path.relative_to(root)}")
    print(f"{len(touched)} cards change")
    if not args.apply:
        print("dry run: nothing written")
        return 0

    # Imported before the first write: resolve_deck needs the digimon_engine bindings, and a
    # missing build must fail here rather than after cards.json has been rewritten.
    from tools import build_card_meta, resolve_deck
    write_overrides(overrides)
    write_cards(cards)
    # build_card_meta renders from resolve_deck's cached cards.json; seed it with the
    # corrected data so --root works on a scratch copy too.
    resolve_deck._CARDS_JSON_CACHE = cards
    for path in targets:
        cid = path.stem
        if path.suffix == ".json":
            split_cards_json.write_card_json(cid, cards[cid], engine_cards)
        else:
            build_card_meta.write_card_meta(cid, card_meta)
    print("applied")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
