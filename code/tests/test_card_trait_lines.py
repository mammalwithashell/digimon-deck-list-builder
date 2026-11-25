"""Guard: the printed trait line of every implemented card reaches production.

Production builds `CardData.traits` from data/cards.json as form_eng + attribute_eng +
type_eng (`card_data.rs`), and every trait filter in the engine ([SW] Use Req., Alliance
grants, "trash 1 [SW] card", digivolve cost reductions, attribute filters) reads that list.
The digimoncard.io API behind cards.json drops or garbles parts of the printed trait line,
and `tests/dsl/trait_parity.rs` only checks the YAML against cards.json, so a trait missing
from BOTH (the YAML is usually authored from cards.json) passes every existing test.

Saneiketsu Megas Nezhamon (EX12-019), Erlangmon (EX12-034) and Takutoumon (EX12-057) print
[SW] on the card face; cards.json, the overrides and the YAML all left it out, so Saneiketsu
Invitation (EX12-071) <Use Req. ([SW] trait)> ignored them. Erlangmon also sat as Data; the
face says Vaccine. None of the three has an official-mirror page, so no mirror check saw it.

For each card with a YAML spec (an implemented card):

1. With an official-mirror page (data/card_official.json): every trait on the mirror's
   form / attribute / type line is in cards.json (the classic stage forms excepted, as in
   test_cards_json_integrity._STAGES), and every cards.json attribute is on that line or
   granted by a `(Rule) Trait: Has [X] attribute.` box. MIRROR_TRAIT_EXCEPTIONS
   holds the image-checked cases where the mirror is wrong or the attribute comes from an
   announcement / an "also treated as" box; each must still be needed.
2. Without a mirror page: IMAGE_CHECKED_TRAIT_LINES records the trait line read off the
   card image, and cards.json must match it. A newly implemented card without a mirror page
   fails until someone reads its face and adds it here; an entry whose card gained a mirror
   page must be dropped (check 1 then covers it).
3. Pool-wide: no trait value carries ingest residue (`}}`) or two traits glued by a slash
   (`Hudie/CS`). [Zip/Unzip] (BT24-056) is one app-name trait that contains a slash.
"""
import glob
import json
import os

from tools.audit_digivolve.reconcile_traits import rule_grants

_HERE = os.path.dirname(os.path.abspath(__file__))
_ROOT = os.path.abspath(os.path.join(_HERE, "..", ".."))

_STAGES = {"Baby", "In-Training", "Rookie", "Champion", "Ultimate", "Mega"}
# One trait whose printed name contains a slash.
_SLASHED_TRAITS = {"Zip/Unzip (App Name)"}


def _load_json(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def _norm(trait):
    # Bandai "Card Text and Information" (rule/card_text/, 2022-07-15): [X-Antibody] is
    # [X Antibody]. The mirror still prints the hyphen on older cards.
    return trait.strip().casefold().replace("x-antibody", "x antibody")


def _split(line):
    return [t.strip() for t in str(line or "").split("/") if t.strip()]


def implemented_card_ids(cards):
    out = set()
    pattern = os.path.join(_ROOT, "code", "digimon-engine", "cards", "*", "*.yaml")
    for path in glob.glob(pattern):
        if os.path.basename(os.path.dirname(path)).startswith("_"):
            continue
        cid = os.path.splitext(os.path.basename(path))[0]
        if cid in cards:
            out.add(cid)
    return out


# Checked against the card image on 2026-11-22. `missing` lists mirror traits the card does
# not print; `extra` lists attributes the card has beyond the mirror's line and its
# (Rule) Trait: Has grants.
MIRROR_TRAIT_EXCEPTIONS = {
    # Gabumon prints Rookie | Data | Reptile; the mirror lists Virus.
    "ST6-03": {"missing": ["Virus"], "extra": ["Data"]},
    # Angemon prints Champion | Vaccine | Angel; the mirror's attribute line says Angel.
    "ST3-05": {"extra": ["Vaccine"]},
    # Sistermon Ciel: Bandai "Japanese / English Character Changes" (rule/rule_change/,
    # 2022-03-18) gives BT6-084 the attributes Virus and Data.
    "BT6-084": {"extra": ["Virus"]},
    # Sistermon Ciel prints Data and "Also treat as if ... traits include [Virus]".
    "ST12-13": {"extra": ["Virus"]},
    # Sistermon Noir (Awakened) prints Virus and "(Rule) Also treated as ... Trait: [Data]
    # Attribute", a box the (Rule) Trait: Has grant parser does not read.
    "EX13-066": {"extra": ["Data"]},
}


# The printed trait line (form, attribute, type) of every implemented card the official
# mirror has no page for, read off DCGO_Application/Assets/Textures/Card/<ID>.webp on
# 2026-11-22. "" = the card prints no such field. P-241 is a Japanese-only print
# (アプリドライヴァー/アプモン = App Driver/Appmon).
IMAGE_CHECKED_TRAIT_LINES = {
    "EX12-001": ("In-Training", "", "Lesser/VB"),
    "EX12-002": ("In-Training", "", "Smoke/Shambala/SW"),
    "EX12-003": ("In-Training", "", "Lesser/ME"),
    "EX12-005": ("Rookie", "Vaccine", "Reptile/VB"),
    "EX12-006": ("Rookie", "Vaccine", "Beast/Shambala/SW"),
    "EX12-007": ("Rookie", "Virus", "Ceratopsian/VB"),
    "EX12-008": ("Rookie", "Vaccine", "Puppet/ME"),
    "EX12-010": ("Champion", "Vaccine", "Dinosaur/ME/VB"),
    "EX12-011": ("Champion", "Vaccine", "Holy Beast/Shambala/TB"),
    "EX12-012": ("Champion", "Vaccine", "Beastkin/Shambala/SW"),
    "EX12-013": ("Champion", "Vaccine", "Dragonkin/VB"),
    "EX12-014": ("Ultimate", "Vaccine", "Sky Dragon/VB"),
    "EX12-015": ("Ultimate", "Virus", "Beastkin/Shambala/SW"),
    "EX12-016": ("Ultimate", "Vaccine", "Cyborg/ME/VB"),
    "EX12-017": ("Mega", "Vaccine", "Dragonkin/ME/VB"),
    "EX12-019": ("Mega", "Data", "Cyborg/Saneiketsu/Tentei Hachibushu/Shambala/SW"),
    "EX12-021": ("Rookie", "Data", "Reptile/VB"),
    "EX12-022": ("Rookie", "Data", "Cyborg/Shambala/SW"),
    "EX12-024": ("Champion", "Vaccine", "Beast/NSo/VB"),
    "EX12-025": ("Champion", "Data", "Cyborg/Shambala/SW"),
    "EX12-029": ("Ultimate", "Virus", "Wizard/Shambala/SW"),
    "EX12-032": ("Ultimate", "Vaccine", "Beastkin/NSo/VB"),
    "EX12-034": ("Mega", "Vaccine", "Shaman/Saneiketsu/Tentei Hachibushu/Shambala/SW"),
    "EX12-035": ("Mega", "Data", "Cyborg/ME/VB"),
    "EX12-037": ("Mega", "Vaccine", "Holy Warrior/Royal Knight/ME/VB"),
    "EX12-038": ("Rookie", "Data", "Machine/ME"),
    "EX12-039": ("Rookie", "Vaccine", "Beast/Shambala/SW"),
    "EX12-040": ("Rookie", "Vaccine", "Mammal/VB"),
    "EX12-041": ("Champion", "Data", "Mutant/ME"),
    "EX12-042": ("Champion", "Vaccine", "Holy Beast/NSp/VB"),
    "EX12-043": ("Champion", "Vaccine", "Holy Beast/Shambala/SW"),
    "EX12-044": ("Ultimate", "Vaccine", "Archangel/NSp/VB"),
    "EX12-045": ("Ultimate", "Vaccine", "Monk/Shambala/SW"),
    "EX12-053": ("Rookie", "Virus", "Machine/ME"),
    "EX12-054": ("Champion", "Virus", "Machine/ME"),
    "EX12-055": ("Ultimate", "Vaccine", "Cyborg/ME"),
    "EX12-056": ("Ultimate", "Data", "Puppet/Shambala/SW"),
    "EX12-057": ("Mega", "Vaccine", "Shaman/Saneiketsu/Tentei Hachibushu/Shambala/SW"),
    "EX12-058": ("Mega", "Vaccine", "Cyborg/ME"),
    "EX12-059": ("Mega", "Virus", "Machine/ME"),
    "EX12-060": ("Mega", "Virus", "Machine/ME"),
    "EX12-064": ("Ultimate", "Virus", "Cyborg/ME"),
    "EX12-066": ("", "", "VB"),
    "EX12-069": ("", "", "VB"),
    "EX12-071": ("", "", "Shambala/SW"),
    "EX12-072": ("", "", "ME"),
    "EX12-073": ("", "", "NSp/DS/NSo/WG/ME/VB"),
    "EX12-075": ("", "", "Shambala/SW/TB"),
    "EX12-077": ("Mega", "Virus", "Unique/VB"),
    "LM-057": ("", "", ""),
    "LM-058": ("", "", ""),
    "LM-059": ("", "", ""),
    "LM-060": ("", "", ""),
    "LM-061": ("", "", ""),
    "LM-062": ("", "", ""),
    "LM-063": ("", "", ""),
    "LM-064": ("", "", ""),
    "LM-065": ("", "", ""),
    "P-241": ("", "", "App Driver/Appmon"),
    "P-247": ("In-Training", "", "Lesser/Titan/TS"),
    "P-248": ("Rookie", "Free", "Mini Dragon"),
    "P-249": ("Hybrid", "Variable", "Beastkin"),
}


def mirror_violations(cards, official, ids, exceptions):
    """Implemented cards whose cards.json trait line disagrees with the official mirror."""
    out, used = [], set()
    for cid in sorted(ids):
        page = official.get(cid) or {}
        if "colors" not in page:
            continue
        card = cards[cid]
        exc = exceptions.get(cid, {})
        have = {_norm(t) for f in ("form_eng", "attribute_eng", "type_eng") for t in card.get(f) or []}
        excused = {_norm(t) for t in exc.get("missing", [])}
        for key in ("form", "attribute", "type"):
            for trait in _split(page.get(key)):
                if key == "form" and trait in _STAGES:
                    continue
                if _norm(trait) in have:
                    continue
                if _norm(trait) in excused:
                    used.add((cid, "missing"))
                    continue
                out.append(f"{cid}: printed {key} [{trait}] missing from cards.json {card.get(key + '_eng')}")
        # An attribute cards.json carries that the card does not print is a wrong trait
        # (Erlangmon EX12-034 sat as Data; it prints Vaccine). The engine merges the three
        # fields into one list, so compare against the whole printed line.
        _, granted, _ = rule_grants(page)
        printed = {_norm(t) for key in ("form", "attribute", "type") for t in _split(page.get(key))}
        printed |= {_norm(t) for t in granted}
        allowed = {_norm(t) for t in exc.get("extra", [])}
        for trait in card.get("attribute_eng") or []:
            if _norm(trait) in printed:
                continue
            if _norm(trait) in allowed:
                used.add((cid, "extra"))
                continue
            out.append(f"{cid}: cards.json attribute [{trait}] is not on the printed line")
    for cid, exc in exceptions.items():
        for kind in exc:
            if (cid, kind) not in used:
                out.append(f"{cid}: MIRROR_TRAIT_EXCEPTIONS[{kind!r}] is no longer needed; drop it")
    return out


def image_line_violations(cards, official, ids, lines):
    out = []
    for cid in sorted(ids):
        if "colors" in (official.get(cid) or {}):
            continue
        if cid not in lines:
            out.append(f"{cid}: implemented, no official-mirror page, and no IMAGE_CHECKED_TRAIT_LINES "
                       f"entry; read its trait line off the card image and add it")
    for cid, (form, attribute, types) in sorted(lines.items()):
        if "colors" in (official.get(cid) or {}):
            out.append(f"{cid}: now has an official-mirror page; drop its IMAGE_CHECKED_TRAIT_LINES entry")
            continue
        card = cards.get(cid)
        if card is None:
            out.append(f"{cid}: missing from cards.json")
            continue
        want = {
            "form_eng": [t for t in _split(form) if t not in _STAGES],
            "attribute_eng": _split(attribute),
            "type_eng": _split(types),
        }
        for field, printed in want.items():
            have = [t for t in card.get(field) or [] if t not in _STAGES]
            if sorted(map(_norm, have)) != sorted(map(_norm, printed)):
                out.append(f"{cid}: cards.json {field} {card.get(field)} != printed {printed}")
    return out


def residue_violations(cards):
    out = []
    for cid, card in sorted(cards.items()):
        for field in ("form_eng", "attribute_eng", "type_eng"):
            for trait in card.get(field) or []:
                glued = "/" in trait and trait not in _SLASHED_TRAITS
                if glued or any(ch in trait for ch in "{}|[]=") or trait != trait.strip() or not trait:
                    out.append(f"{cid}: {field} value {trait!r} is not one clean trait")
    return out


def _repo():
    cards = _load_json(os.path.join(_ROOT, "data", "cards.json"))
    official = _load_json(os.path.join(_ROOT, "data", "card_official.json"))["cards"]
    return cards, official, implemented_card_ids(cards)


def test_implemented_cards_carry_the_official_mirror_trait_line():
    cards, official, ids = _repo()
    violations = mirror_violations(cards, official, ids, MIRROR_TRAIT_EXCEPTIONS)
    assert not violations, "trait line drift against the official mirror:\n  " + "\n  ".join(violations)


def test_implemented_cards_without_a_mirror_page_carry_the_image_checked_trait_line():
    cards, official, ids = _repo()
    violations = image_line_violations(cards, official, ids, IMAGE_CHECKED_TRAIT_LINES)
    assert not violations, "trait line drift against the card face:\n  " + "\n  ".join(violations)


def test_no_trait_value_carries_ingest_residue():
    cards, _, _ = _repo()
    violations = residue_violations(cards)
    assert not violations, "\n  ".join(violations)


def override_violations(cards, overrides):
    """A trait correction in card_overrides.json (re-applied by every ingest) that
    cards.json does not hold (BT25-044's Virus override sat unapplied until 2026-10-04)."""
    out = []
    for cid, entry in sorted(overrides.items()):
        if not isinstance(entry, dict):
            continue
        for field in ("form_eng", "attribute_eng", "type_eng"):
            if field in entry and (cards.get(cid) or {}).get(field) != entry[field]:
                out.append(f"{cid}: card_overrides {field} {entry[field]} != cards.json "
                           f"{(cards.get(cid) or {}).get(field)}")
    return out


def test_trait_overrides_are_applied_to_cards_json():
    cards, _, _ = _repo()
    overrides = _load_json(os.path.join(_ROOT, "data", "card_overrides.json"))
    violations = override_violations(cards, overrides)
    assert not violations, "\n  ".join(violations)


# Trait corrections made from the card face that must survive a re-ingest of the
# digimoncard.io API, which drops them: each must be in card_overrides.json.
FACE_TRAIT_CORRECTIONS = {
    "EX12-019": {"type_eng": "SW"},
    "EX12-034": {"type_eng": "SW", "attribute_eng": "Vaccine"},
    "EX12-057": {"type_eng": "SW", "attribute_eng": "Vaccine"},
    "AD1-010": {"attribute_eng": "Vaccine"},
    "BT23-014": {"attribute_eng": "Virus"},
    "BT23-058": {"attribute_eng": "Vaccine"},
    "BT24-072": {"attribute_eng": "Virus"},
    "BT24-075": {"attribute_eng": "Data"},
    "EX13-060": {"attribute_eng": "Vaccine"},
    "ST9-06": {"attribute_eng": "Free"},
    "BT9-040": {"type_eng": "X Antibody"},
    "BT23-090": {"type_eng": "CS"},
}


def test_face_trait_corrections_survive_a_reingest():
    overrides = _load_json(os.path.join(_ROOT, "data", "card_overrides.json"))
    missing = [f"{cid}: card_overrides {field} lacks {trait!r}"
               for cid, fields in sorted(FACE_TRAIT_CORRECTIONS.items())
               for field, trait in fields.items()
               if trait not in ((overrides.get(cid) or {}).get(field) or [])]
    assert not missing, "\n  ".join(missing)


def _synthetic(card, page=None):
    base = {"form_eng": [], "attribute_eng": [], "type_eng": []}
    official = {"X-1": {"colors": "Red", **page}} if page is not None else {}
    return {"X-1": {**base, **card}}, official


def test_a_trait_missing_from_both_the_yaml_and_cards_json_fails():
    # The Saneiketsu shape: the face prints [SW], cards.json does not.
    cards, official = _synthetic({"attribute_eng": ["Data"], "type_eng": ["Cyborg", "Shambala"]})
    lines = {"X-1": ("Mega", "Data", "Cyborg/Shambala/SW")}
    assert image_line_violations(cards, official, {"X-1"}, lines) == [
        "X-1: cards.json type_eng ['Cyborg', 'Shambala'] != printed ['Cyborg', 'Shambala', 'SW']"]
    cards["X-1"]["type_eng"].append("SW")
    assert image_line_violations(cards, official, {"X-1"}, lines) == []


def test_a_wrong_attribute_fails_against_the_mirror():
    cards, official = _synthetic({"attribute_eng": ["Data"]}, {"form": "Mega", "attribute": "Vaccine"})
    assert mirror_violations(cards, official, {"X-1"}, {}) == [
        "X-1: printed attribute [Vaccine] missing from cards.json ['Data']",
        "X-1: cards.json attribute [Data] is not on the printed line"]
    cards["X-1"]["attribute_eng"] = ["Vaccine"]
    assert mirror_violations(cards, official, {"X-1"}, {}) == []


def test_a_rule_granted_attribute_is_printed():
    cards, official = _synthetic(
        {"attribute_eng": ["Vaccine", "Free"]},
        {"attribute": "Vaccine", "text_sections": [{"label": "Effect", "text": "(Rule) Trait: Has [Free] attribute."}]})
    assert mirror_violations(cards, official, {"X-1"}, {}) == []


def test_an_unmirrored_implemented_card_needs_an_image_check():
    cards, official = _synthetic({})
    assert image_line_violations(cards, official, {"X-1"}, {}) == [
        "X-1: implemented, no official-mirror page, and no IMAGE_CHECKED_TRAIT_LINES entry; "
        "read its trait line off the card image and add it"]


def test_residue_and_glued_traits_fail():
    assert residue_violations({"X-1": {"attribute_eng": ["Free}}"]}, "X-2": {"type_eng": ["Hudie/CS"]},
                               "X-3": {"type_eng": ["Zip/Unzip (App Name)"]}}) == [
        "X-1: attribute_eng value 'Free}}' is not one clean trait",
        "X-2: type_eng value 'Hudie/CS' is not one clean trait"]
