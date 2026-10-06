#!/usr/bin/env python3
"""One-time migration: apply Bandai's card errata to data/cards.json (2026-10-05).

Bandai's errata page (world.digimoncard.com/rule/errata_card, read 2026-10-05) lists 93
errata, 2020-11-27 to 2026-05-29: "The Card Errata 'After' text is applied to all game formats
and takes precedence over the original wording of the card." The digimoncard.io API that
cards.json is ingested from serves many of these cards in their original wording, and the
official Bandai DB mirror (data/card_official.json) is itself pre-errata for several (ST10-06,
EX2-028, EX3-028, EX3-030, LM-013, P-060, ...). code/tests/test_cards_json_errata.py records
every errata and holds cards.json to it.

An errata'd passage must read as the After text, or as Bandai's card list words the same
post-errata text. Where cards.json held neither, it now holds the After text, keeping
cards.json's keyword reminder text (41 cards, 42 fields):

  pre-errata:
    - BT23-078 is named Gorou Matayoshi (2025-10-17); no other card names him;
    - ST10-06 searches the security stack, then "you may play 1 ... among it" (2025-08-01);
    - BT25-057's Final Judgment grants its buffs "for the turn" (2026-05-15). Its Option face
      lives in the override's `dual` block, which apply_overrides replaces whole. The API had
      also filed that face as inherited text, keywords dropped and pre-errata; cleared, as the
      BT25-085 override does;
    - "at the next end of your opponent's turn" (2025-04-25): EX3-069, EX4-063, EX4-071, LM-013
      and BT13-089. BT13-089 now reads as its post-errata card: "you may play" and "in one of
      its traits", which every printing carries and the API dropped;
  garbled or cut short: BT9-071 ("Place the rest at the bottom of your."), BT14-002 ("with with
  ... than"), BT14-091 ("Trash 2", "that Digimon");
  worded unlike both the After text and the card list: BT3-097, BT8-110, BT9-067, BT10-004,
  BT10-093, BT10-096, BT10-097, BT10-107, BT12-037, BT12-051, BT16-060, BT18-099, BT19-091,
  EX1-073, EX2-028, EX2-053, EX3-003, EX3-023, EX3-026, EX3-028, EX3-030, EX3-035, EX3-045,
  P-012, P-029, P-030, ST16-11. EX3-035's "-6000 for the turn" (no DP) is the After text and
  the post-errata card;
  the After text itself garbled (EX3-008 and EX3-058's second bullet, "... and one of your
  other Digimon may DNA digivolve into ..."; EX3-014's "3000 or less", "add 2000 DP to the
  maximum DP"): the card list's wording, which says what the post-errata cards on the errata
  page print (EX3-014's word for word).

  While there: BT18-099's [Main] read "1 of your their Digimon"; the card prints "give 1 of
  their Digimon".

  Not changed:
    - already the After text (43 cards; BT14-023 and EX3-001 against the post-errata cards,
      the page misprinting "(Once Per Turn)" and "+1000 for the turn");
    - the card list's wording of the After text: EX3-022, EX3-064 ("If you don't have
      [Trial ...]") and P-115 ("[Nene Amano] or [Yuu Amano]");
    - <Decoy> reminder text, which cards.json renders in its current form (P-045, BT6-059,
      BT6-064); both forms carry the errata's "by an opponent's effect";
    - errata without text: EX11-025 (emblem art), P-123 (its "Digivolve" label), BT21-023
      (a yellow segment in the cost ring) and BT3-111 (cost-5 digivolve circles), the last two
      already right in cards.json, as is BT4-041's Unknown attribute and type;
    - EX4-058 and EX4-072, corrected with the rest of their text by the 2026-10 names and traits
      pass (code/tools/archive/fix_card_names_drift_2026_10.py, not yet on this branch).

  The 16 errata'd cards with a DSL YAML already implement the After text, so no YAML changes:
  BT8-097, BT8-109, BT10-093, BT16-077, BT21-023, BT25-057, EX3-008, EX3-014, EX3-057, EX3-063,
  EX7-027, EX7-030, EX9-021, P-123, ST19-08 and ST19-12.

Usage (repo root):
    python code/tools/archive/apply_card_errata_2026_10.py           # dry run
    python code/tools/archive/apply_card_errata_2026_10.py --apply
"""
import argparse
import json
import sys
from pathlib import Path

_CODE_DIR = Path(__file__).resolve().parents[2]
if str(_CODE_DIR) not in sys.path:
    sys.path.insert(0, str(_CODE_DIR))

from tools import ingest_cards, split_cards_json  # noqa: E402

NOTE_KEY = "_note_card_errata_2026_10"
NOTE = (
    "2026-10-05: Bandai's card errata (world.digimoncard.com/rule/errata_card, 93 entries to "
    "2026-05-29) applied to 41 cards whose text was pre-errata, garbled, or worded unlike both the "
    "After text and the official card list: BT23-078's name, ST10-06, BT25-057's Final Judgment, "
    "'the next end of your opponent's turn' on five cards and 33 more "
    "(code/tools/archive/apply_card_errata_2026_10.py lists them). "
    "code/tests/test_cards_json_errata.py holds cards.json to every errata."
)

# Field corrections per card, merged into data/card_overrides.json.
CORRECTIONS = json.loads(r'''
{
    "BT3-097": {
        "effect_description_eng": "[Main] 1 of your Digimon gains \"This Digimon doesn't activate the [Security] effects of any Option cards it checks\" for the turn."
    },
    "BT8-110": {
        "inherited_effect_description_eng": "Security Effect [Security] You may play 1 level 3 Digimon card with [Free] in its traits from your hand or trash without paying its memory cost."
    },
    "BT9-067": {
        "effect_description_eng": "[On Play] [When Digivolving] Place 1 [Raijinmon], 1 [Fujinmon], and 1 [Suijinmon] from your trash under this Digimon in any order as its bottom digivolution cards. Gain 1 memory for each card placed.\r\n[When Attacking] If the level 6 cards in this Digimon's digivolution cards have 3 or more colors among them, this Digimon gain +3000 DP until the end of your opponent's next turn. If they have 4 or more colors, ＜De-Digivolve 1＞ 1 of your opponent's Digimon. (Trash the top card. You can't trash past level 3 cards.)"
    },
    "BT9-071": {
        "effect_description_eng": "[On Play] Reveal the top 3 cards of your deck. Among them, add 1 card with [Undead] or [Dark Animal] in one of its traits to the hand and trash 1 such card. Return the rest to the bottom of the deck."
    },
    "BT10-004": {
        "inherited_effect_description_eng": "[Your Turn] [Once Per Turn] When an effect suspends a Digimon, this Digimon gets +1000 DP for the turn."
    },
    "BT10-093": {
        "effect_description_eng": "[All Turns] [Once Per Turn] When a purple card is placed under this Tamer, ＜Draw 1＞ (Draw 1 card from your deck.) and memory +1.\r\n[Your Turn] [Once Per Turn] When you would play 1 level 4 or higher Digimon card with [Bagra Army] in its traits, by placing up to 3 purple Digimon cards from under your Tamers in the digivolution cards of the Digimon card played, reduce the memory cost of that Digimon by 2 for each card placed."
    },
    "BT10-096": {
        "inherited_effect_description_eng": "Security Effect [Security] Reveal the top 3 cards of your deck. You may add 1 Digimon card with [Xros Heart] in its traits among them to your hand and play 1 [Taiki Kudo] among them without paying its memory cost. Place the rest at the bottom of your deck in any order."
    },
    "BT10-097": {
        "effect_description_eng": "[Main] Reveal the top 6 cards of your deck. You may add 2 cards with [Blue Flare] in their traits among them to your hand, and play 1 [Kiriha Aonuma] among them without paying its memory cost. Place the rest at the bottom of your deck in any order. Then, place this card in your battle area.\r\n[Main] ＜Delay＞ (By trashing this card after the placing turn, activate the effect below.)\r\n・Gain 2 memory."
    },
    "BT10-107": {
        "inherited_effect_description_eng": "Security Effect [Security] You may play 1 [Yuu Amano] from your hand or trash without paying its play cost. Then, add this card to its owner's hand."
    },
    "BT12-037": {
        "effect_description_eng": "[On Play] [When Digivolving] Reveal the top 3 cards of your deck. You may play 1 [Airu Suzaki], [Ren Tobari], or [Ryoma Mogami] card among them without paying its cost. Place the remaining cards at the bottom of your deck in any order.\r\n[On Deletion] ＜Save＞. Then, place 1 Digimon card with ＜Save＞ in its text from your trash under 1 of your Tamers."
    },
    "BT12-051": {
        "effect_description_eng": "[On Play] [When Digivolving] You may play 1 [Airu Suzaki], [Ren Tobari], or [Ryoma Mogami] card from your hand without paying its cost.\r\n[On Deletion] ＜Save＞. Then, place 1 Digimon card with ＜Save＞ in its text from your trash under 1 of your Tamers."
    },
    "BT13-089": {
        "effect_description_eng": "[End of Your Turn] By deleting this Digimon that has a digivolution card with [Bird] or [Avian] in one of its traits, at the next end of your opponent's turn, you may play 1 [Ravemon] from your trash without paying the cost.\r\n[On Deletion] You may play 1 [Falcomon] or [Keenan Crier] from your hand or trash without paying the cost."
    },
    "BT14-002": {
        "inherited_effect_description_eng": "[Your Turn] While your opponent has no Digimon with as many or more digivolution cards as this Digimon, this Digimon gains ＜Jamming＞ (This Digimon can't be deleted in battles against Security Digimon.)."
    },
    "BT14-091": {
        "effect_description_eng": "[Main] Trash any 2 digivolution cards from your opponent's Digimon. Then, if you have a Tamer with [Joe Kido] in its name, choose 1 of your Digimon. If your opponent has no Digimon with as many or more digivolution cards as the chosen Digimon, unsuspend it."
    },
    "BT16-060": {
        "effect_description_eng": "[On Play] [When Digivolving] Reveal the top 3 cards of your deck. For each [D-Brigade] or [DigiPolice] trait card among them, reduce the play costs of all of your opponent's Digimon by 1 for the turn. Return the revealed cards to the top or the bottom of the deck. Then, delete 1 of your opponent's Digimon with a play cost of 4 or less."
    },
    "BT18-099": {
        "effect_description_eng": "While you have a Digimon with [Knightmon] in its text, you can ignore this card's color requirements.\r\n[Main] Until the end of your opponent's turn, give 1 of their Digimon \"[Start of Your Main Phase] This Digimon attacks.\" Then, place this card in the battle area.\r\n[All Turns] When attack targets change, ＜Delay＞ (By trashing this card after the placing turn, activate the effect below.)\r\n・1 of your Digimon gains ＜Piercing＞ (When this Digimon attacks and deletes an opponent's Digimon and survives the battle, it performs any security checks it normally would.) and ＜Security A. +1＞ (This Digimon checks 1 additional security card.) until the end of your turn."
    },
    "BT19-091": {
        "effect_description_eng": "While you have a level 5 [WarGrowlmon], [Taomon] or [Rapidmon], you may ignore this card's color requirements.\r\n[Main] Play 1 [WarGrowlmon] Token (Digimon/Red/6000 DP), 1 [Taomon] Token (Digimon/Yellow/6000 DP), and 1 [Rapidmon] Token (Digimon/Green/6000 DP). This effect can't play tokens with the same names as your Digimon. Then, 1 of your level 5 Digimon gains ＜Alliance＞ twice for the turn and attacks."
    },
    "BT23-078": {
        "card_name_eng": "Gorou Matayoshi"
    },
    "BT25-057": {
        "inherited_effect_description_eng": "",
        "dual": {
            "digimon": {
                "level": 5,
                "dp": 8000,
                "colors": [
                    "Green",
                    "Black"
                ],
                "traits": [
                    "Cyborg",
                    "Glowing Dawn",
                    "BEATBREAK"
                ],
                "evo_costs": [
                    {
                        "card_color": 3,
                        "level": 4,
                        "memory_cost": 4
                    },
                    {
                        "card_color": 5,
                        "level": 4,
                        "memory_cost": 4
                    }
                ],
                "effect_text": "[When Digivolving] [When Attacking] [Once Per Turn] By trashing the bottom face-down card under any of your Tamers, ＜De-Digivolve 1＞ 1 of your opponent's Digimon. [When Digivolving] This Digimon may battle 1 of your opponent's Digimon.",
                "inherited_text": "",
                "keywords": [
                    "ArtsDigivolve"
                ]
            },
            "option": {
                "use_cost": 4,
                "colors": [
                    "Green"
                ],
                "effect_text": "＜Use Req. ([Glowing Dawn] trait)＞ (Specified cards let you ignore color requirements.) [Main] 1 of your Digimon gains ＜Rush＞, ＜Security A. +1＞ and +5000 DP for the turn. Then, it may attack.",
                "security_text": "",
                "keywords": [
                    "ArtsDigivolve"
                ]
            }
        }
    },
    "EX1-073": {
        "effect_description_eng": "[On Play] You may place up to 5 level 5 red and/or black cards with [Cyborg] in their traits and different card numbers from your hand and/or trash in this Digimon's digivolution cards to gain 1 memory for each card placed.\r\n[All Turns] This Digimon's DP can't be reduced.\r\n[All Turns] When this Digimon would be deleted, you may trash 2 level 5 Digimon cards in this Digimon's digivolution cards to prevent this Digimon from being deleted."
    },
    "EX2-028": {
        "effect_description_eng": "[End of Attack] You may place this Digimon as 1 of your other Digimon's bottom digivolution card."
    },
    "EX2-053": {
        "effect_description_eng": "[On Play] [When Attacking] [Once Per Turn] If one of your [Mother D-Reaper]s has 5 or more digivolution cards, reveal the top 3 cards of your deck. You may play 1 card with [D-Reaper] in its traits and a play cost of 10 or less among them without paying its memory cost. Place the remaining cards at the top of your deck in any order."
    },
    "EX3-003": {
        "effect_description_eng": "[When Attacking] Reveal the top 3 cards of your deck. Add 1 Digimon card with [Dragon], [saur] or [Ceratopsian] in one of its traits among them to your hand. Place the rest at the bottom of your deck in any order."
    },
    "EX3-008": {
        "effect_description_eng": "[When Digivolving] Activate 1 of the effects below.\r\n・1 of your other Digimon may digivolve into a level 4 purple Digimon card with [Free] in its traits from your trash for the cost.\r\n・This Digimon and one of your other Digimon may DNA digivolve into a Digimon card in your hand for the cost."
    },
    "EX3-014": {
        "effect_description_eng": "＜Rush＞ (This Digimon can attack the turn it comes into play.)\r\n[On Play] Delete 1 of your opponent's Digimon with 3000 DP or less. For each card with [Dragon], [saur] or [Ceratopsian] in one of its traits in this Digimon's digivolution cards, add 2000 to the maximum DP you can choose with this effect."
    },
    "EX3-023": {
        "effect_description_eng": "[When Digivolving] You may play 1 blue level 3 Digimon card or 1 level 4 or lower Digimon card with [Aqua] or [Sea Animal] in one of its traits from one of your blue Digimon's digivolution cards without paying its memory cost. Then, you may place 1 blue Digimon card from your hand under this Digimon as its bottom digivolution card."
    },
    "EX3-026": {
        "effect_description_eng": "[When Digivolving] You may play 1 blue level 3 Digimon card or 1 Digimon card with [Seadramon] in its name or [Aqua] or [Sea Animal] in one of its traits from one of your blue Digimon's digivolution cards without paying its memory cost.\r\n[Opponent's Turn] [Once Per Turn] When your opponent plays a Digimon card, you may activate 1 of this Digimon's [When Digivolving] effects."
    },
    "EX3-028": {
        "effect_description_eng": "[On Play] Reveal the top 4 cards of your deck. Add 1 yellow card with [Angel], [Cherub], [Throne], [Authority], [Seraph] or [Virtue], other than [Three Great Angels], in one of its traits and 1 card with the [Four Great Dragons] trait among them to your hand. Place the rest at the bottom of your deck in any order."
    },
    "EX3-030": {
        "effect_description_eng": "[On Play] Reveal the top 4 cards of your deck. Add 1 yellow card with [Angel], [Cherub], [Throne], [Authority], [Seraph] or [Virtue], other than [Three Great Angels], in one of its traits and 1 card with the [Four Great Dragons] trait among them to your hand. Place the rest at the bottom of your deck in any order."
    },
    "EX3-035": {
        "effect_description_eng": "[When Digivolving] You may return 1 card with the [Four Great Dragons] trait from your trash to your hand.\r\n[When Attacking] 1 of your opponent's Digimon gets -6000 for the turn. Then, by returning 1 [Magnadramon], 1 [Azulongmon], and 1 [Megidramon] from your trash to the bottom of your deck in any order, trash the top 2 cards of your opponent's security stack."
    },
    "EX3-045": {
        "effect_description_eng": "[When Digivolving] You may suspend 1 Digimon.\r\n[All Turns] [Once Per Turn] When an opponent's Digimon becomes suspended, for each other suspended Digimon with [Vegetation], [Plant], or [Fairy] in one of their traits you have in play, gain 1 memory.\r\n[End of Your Turn] [Once Per Turn] If you have 2 or more suspended Digimon with [Vegetation], [Plant], or [Fairy] in one of their traits, return 1 of your opponent's suspended Digimon to the bottom of its owner's deck."
    },
    "EX3-058": {
        "effect_description_eng": "[When Digivolving] Activate 1 of the effects below.\r\n・1 of your other Digimon may digivolve into a red level 4 Digimon card with [Free] in its traits from your trash for the cost.\r\n・This Digimon and one of your other Digimon may DNA digivolve into a Digimon card in your hand for the cost."
    },
    "EX3-069": {
        "effect_description_eng": "[Main] ＜Draw 1＞ (Draw 1 card from your deck.) Then, place this card in your battle area.\r\n[Main] ＜Delay＞ (By trashing this card after the placing turn, activate the effect below.)\r\n・Play 1 Digimon card with the [Four Great Dragons] trait from your hand without paying the cost. The Digimon played by this effect can't digivolve to level 7, and at the next end of your opponent's turn, delete that Digimon."
    },
    "EX4-063": {
        "effect_description_eng": "[Start of Your Main Phase] If you have 1 or fewer Digimon in play, you may play 1 [Terriermon] or [Lopmon] from your hand without paying the cost. Digimon played by this effect can't digivolve and are deleted at the next end of your opponent's turn.\r\n[Your Turn] When one of your Digimon with [Terriermon] or [Lopmon] in its digivolution cards would digivolve, by suspending this Tamer, reduce the digivolution cost by 1."
    },
    "EX4-071": {
        "effect_description_eng": "[Main] By deleting 1 of your Digimon, delete 1 of your opponent's Digimon whose level is less than or equal to that Digimon. If one of your Digimon with [Ravemon] in its name was deleted by this effect, at the next end of your opponent's turn, play 1 [Ravemon] from your trash without paying the cost."
    },
    "LM-013": {
        "effect_description_eng": "[Hand] [Counter] ＜Blast Digivolve＞ (Your Digimon may digivolve into this card without paying the cost.)\r\n[On Play] [When Digivolving] Suspend 1 of your opponent's Digimon. Then, if they have no unsuspended Digimon, gain 2 memory.\r\n[When Attacking] You may play 1 Digimon card with [Angoramon] in its text from your hand without paying the cost. At the next end of your opponent's turn, return that Digimon to the hand."
    },
    "P-012": {
        "effect_description_eng": "[Main] If you have a Digimon with [Veedramon] in its name, you may suspend this Tamer to activate one of the following effects:\r\n・Trigger ＜Draw 1＞ (Draw 1 card from your deck.)\r\n・1 of your Digimon gets +1000 DP for the turn."
    },
    "P-029": {
        "inherited_effect_description_eng": "[Your Turn] When digivolving this Digimon into an [AncientGreymon] in your hand, reduce its digivolution cost by 2."
    },
    "P-030": {
        "inherited_effect_description_eng": "[Your Turn] When digivolving this Digimon into an [AncientGarurumon] in your hand, reduce its digivolution cost by 2."
    },
    "ST10-06": {
        "effect_description_eng": "[When Digivolving] Place 1 yellow or purple Digimon card from your trash on top of your security stack face down. When DNA digivolving, search your security stack, and you may play 1 level 5 or lower Digimon card among it without paying its cost. Then, shuffle your security stack.\r\n[All Turns] When you play another Digimon using an effect, delete 1 of your opponent's Digimon whose level is less than or equal to the played Digimon's level."
    },
    "ST16-11": {
        "inherited_effect_description_eng": "[When Attacking] [Once Per Turn] By trashing 1 card in your hand, delete 1 of your opponent's level 4 or lower Digimon."
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
