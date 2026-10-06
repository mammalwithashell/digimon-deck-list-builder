#!/usr/bin/env python3
"""One-time migration: correct the traits of 44 cards in data/cards.json (2026-10-05).

Production builds CardData.traits from cards.json (form_eng + attribute_eng + type_eng), so
a wrong trait there decides every trait predicate in real games. A pool-wide comparison
with the official Bandai DB mirror (data/card_official.json; its form/attribute/type lines
split on "/", stages Baby..Mega ignored, trait grants in the card text added) found 68
cards whose traits disagree, as one list or, for a Digimon, field by field. Each was
judged against the card image, the card's own block on the official card list
(world.digimoncard.com lists every printing of a card; the mirror keeps the last block's
traits), the DCGO card asset (DCGO/Assets/CardBaseEntity, in the base repo) and Bandai's
rule announcements (world.digimoncard.com/rule/), which change some cards' traits beyond
what they print:

  cards.json wrong, corrected here (44 cards, all as data/card_overrides.json entries so a
  re-ingest keeps them; see the `_note_card_traits_drift_2026_10` key):
    - the attribute: AD1-010, BT6-054, BT6-057, BT9-020, BT9-072, BT12-045, BT12-052,
      BT14-054, BT18-016, BT22-011, BT23-014, BT23-058, BT23-061, EX11-010 and EX13-060
      print the other attribute (DCGO agrees with the print except EX11-010, where it says
      Data; EX13-060 has no asset). BT24-072 and BT24-075 held the stage "Ultimate" as
      their attribute and had none (BT24-075 prints Data; DCGO says Virus);
    - traits the API got wrong: BT10-060 prints [Bird] (API [Avian]; [Bird] is a trait of
      its own, which "[Avian] or [Bird] in one of its traits" texts name, see Bandai's
      "Effects Which Reference Traits", rule/effect_text/effects_reference.php), BT20-010
      [Beast] (API [Beast Dragon]), BT22-023 [Holy Dragon] (API [Mythical Dragon]), EX6-010
      [Holy Sword] (API [Weapon]); BT17-063, BT17-066, BT17-071 and BT17-072 carried a
      bogus trait "3" (the API's digi_type3); BT19-061 does not print [Twilight] (the API
      and DCGO add it); BT24-081 does not print [Olympos XII] (an earlier override copied
      it from the API);
    - traits the API dropped: EX11-059 [NSo], P-234 [Leviathan]; BT23-090's "Hudie/CS"
      is two traits, [Hudie] and [CS];
    - trait grants the API drops: BT23-077 "Also has Name:[Sistermon Noir] and Trait:
      [Virus]", BT24-086 "... and has Trait: [DigiPolice]", P-101 "also treated as having
      the [Cyborg] trait", ST12-13 "traits include [Virus]"; and BT6-084 and BT7-083
      Sistermon Ciel, which Bandai's "Japanese / English Character Changes" (2022-03-18,
      rule/rule_change/) treat as having the attributes Virus and Data;
    - ingest residue: BT9-040 [X Antibody}}], ST9-06 [Free}}], EX10-038 [Copy &amp; Paste];
      BT24-056's app name [Zip/Unzip] had been split into [Zip] and [Unzip];
    - right trait, wrong field: EX11-011's attribute and EX6-036's [Unknown] attribute sat
      in type_eng, BT19-001's [Minor] type in attribute_eng, and EX3-011's attribute in
      form_eng.

  cards.json right, the mirror shows the print (5 cards, no change here): Bandai's "Digimon
  Type Changes" (2022-04-22, rule/digimon_type_changes/) made BT2-024, BT2-029, BT4-024,
  BT4-028 and BT4-034 [Aquatic]; they print [Sea Animal], and "revised printings of these
  cards are not available". The API and DCGO already say Aquatic. [Sea Animal] is still
  another card's trait (BT19-066 Gizamon), which is why later texts name "[Aqua] or [Sea
  Animal]".

  The official mirror wrong, cards.json already right (21 cards, no change here): the
  mirror keeps the last printing's traits, and some alternate-printing blocks carry another
  card's (BT11-012, BT15-017, BT15-042, BT15-053, BT15-067, EX4-050, P-060) or misfile the
  attribute (BT2-012, BT2-015, BT2-025, BT2-027, ST3-05, ST4-03, ST4-10, ST5-03); a few
  official entries are simply wrong (EX3-051 and EX3-052 have swapped attributes, ST6-03
  says Virus, BT6-024 [Elusive Beast] for the printed [Rare Animal], and P-217 and BT14-098
  list no traits).

  code/tests/test_cards_json_integrity.py records the last two groups in ANNOUNCED_TRAITS
  and PRINTED_OVER_MIRROR, and its pool-wide trait test keeps every other card at the
  official traits.

The DSL YAML of six of these cards had copied the old data (the AD1-010 and BT23-058
attributes, BT19-061's [Twilight]) or lacked a grant (BT6-084 and BT23-077 [Virus], P-101
[Cyborg]). The same change corrects those by hand, and 14 YAMLs whose cards.json was right:
a wrong attribute (BT19-033, BT25-002, BT25-027, BT25-028), a grant (EX13-066 [Data]), or a
printed type left out of `traits:` (AD1-019, BT13-007, BT13-083, BT20-060, BT22-009,
BT22-084, EX10-069, EX9-066, P-094); tests/dsl/trait_parity.rs now holds every DSL YAML to
the printed type line and attribute. This script does not touch YAML.

Only these cards' existing per-card engine JSON and data/card_meta pages are regenerated;
both trees are partial and stale elsewhere, so no new files are created.

Usage (repo root):
    python code/tools/archive/fix_card_traits_drift_2026_10.py           # dry run
    python code/tools/archive/fix_card_traits_drift_2026_10.py --apply
"""
import argparse
import json
import sys
from pathlib import Path

_CODE_DIR = Path(__file__).resolve().parents[2]
if str(_CODE_DIR) not in sys.path:
    sys.path.insert(0, str(_CODE_DIR))

from tools import ingest_cards, split_cards_json  # noqa: E402

NOTE_KEY = "_note_card_traits_drift_2026_10"
NOTE = (
    "2026-10-05: traits of 44 cards corrected against the card image, the card's own block on the "
    "official card list, the DCGO card asset and Bandai's rule announcements "
    "(code/tools/archive/fix_card_traits_drift_2026_10.py lists them): wrong attributes, a stage filed "
    "as the attribute, traits the API renamed ([Avian] for BT10-060's printed [Bird]), invented ('3') "
    "or dropped, trait grants (including the Virus attribute Bandai gave BT6-084 and BT7-083 on "
    "2022-03-18), ingest residue, and traits filed in the wrong field. Where the official mirror itself "
    "is wrong, or Bandai changed a card's traits beyond its print (BT2-024 and four others are "
    "[Aquatic], 2022-04-22), cards.json keeps the governing traits: see PRINTED_OVER_MIRROR and "
    "ANNOUNCED_TRAITS in code/tests/test_cards_json_integrity.py."
)


# Field corrections per card, merged into data/card_overrides.json.
CORRECTIONS = {
    # The attribute the card prints.
    "AD1-010": {"attribute_eng": ["Vaccine"]},
    "BT6-054": {"attribute_eng": ["Data"]},
    "BT6-057": {"attribute_eng": ["Virus"]},
    "BT9-020": {"attribute_eng": ["Data"]},
    "BT9-072": {"attribute_eng": ["Vaccine"]},
    "BT12-045": {"attribute_eng": ["Data"]},
    "BT12-052": {"attribute_eng": ["Vaccine"]},
    "BT14-054": {"attribute_eng": ["Data"]},
    "BT18-016": {"attribute_eng": ["Data"]},
    "BT22-011": {"attribute_eng": ["Virus"]},
    "BT23-014": {"attribute_eng": ["Virus"]},
    "BT23-058": {"attribute_eng": ["Vaccine"]},
    "BT23-061": {"attribute_eng": ["Data"]},
    "EX11-010": {"attribute_eng": ["Vaccine"]},
    "EX13-060": {"attribute_eng": ["Vaccine"]},
    # The stage "Ultimate" sat in the attribute field.
    "BT24-072": {"attribute_eng": ["Virus"]},
    "BT24-075": {"attribute_eng": ["Data"]},
    # Traits the API renamed, invented or dropped.
    "BT10-060": {"type_eng": ["Bird", "Twilight", "Xros Heart"]},
    "BT17-063": {"type_eng": ["Angel"]},
    "BT17-066": {"type_eng": ["Mythical Beast"]},
    "BT17-071": {"type_eng": ["Demon Lord"]},
    "BT17-072": {"type_eng": ["Ancient Bird"]},
    "BT19-061": {"type_eng": ["Cyborg", "Xros Heart"]},
    "BT20-010": {"type_eng": ["Beast", "X Antibody", "Chronicle"]},
    "BT22-023": {"type_eng": ["Holy Dragon", "CS"]},
    "BT23-090": {"type_eng": ["Hudie", "CS"]},
    "BT24-081": {"type_eng": ["Shaman", "Titan", "TS", "Demon"]},
    "EX6-010": {"type_eng": ["Holy Sword", "Legend-Arms"]},
    "EX11-059": {"type_eng": ["NSo", "LIBERATOR"]},
    "P-234": {"type_eng": ["App Driver", "Appmon", "Leviathan"]},
    # Trait grants printed outside a "(Rule) Trait: Has" box, or announced by Bandai.
    "BT6-084": {"attribute_eng": ["Data", "Virus"]},
    "BT7-083": {"attribute_eng": ["Data", "Virus"]},
    "BT23-077": {"attribute_eng": ["Data", "Virus"]},
    "BT24-086": {"type_eng": ["SEEKERS", "DigiPolice"]},
    "P-101": {"type_eng": ["Undead", "Cyborg"]},
    "ST12-13": {"attribute_eng": ["Data", "Virus"]},
    # Ingest residue.
    "BT9-040": {"type_eng": ["Archangel", "X Antibody"]},
    "ST9-06": {"attribute_eng": ["Free"]},
    "EX10-038": {"type_eng": ["Copy & Paste (App Name)", "Leviathan", "Copy & Paste"]},
    "BT24-056": {"type_eng": ["Zip/Unzip (App Name)", "Zip/Unzip"]},
    # The right trait in the wrong field.
    "BT19-001": {"attribute_eng": [], "type_eng": ["Minor", "Xros Heart"]},
    "EX3-011": {"form_eng": []},
    "EX6-036": {"attribute_eng": ["Unknown"], "type_eng": ["Unidentified"]},
    "EX11-011": {"attribute_eng": ["Vaccine"], "type_eng": ["Dinosaur", "LIBERATOR"]},
}


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
