"""A link card's link effect lives in cards.json's `link_effect_description_eng`, not in its inherited text.

general_rule.pdf 2-3-12: link requirements, link DP and link effects are card information
of their own; 4-2-4: a Digimon gains the inherited effects of its digivolution cards; 4-2-6:
it gains the link effects of its link card; 2-3-4-2: a card without an inherited effect
section can't be referenced as having one. The digimoncard.io API returns a link card's
link effect as `source_effect`, the field it uses for inherited effects, usually without
its timing ("All of your opponent's Security Digimon get -3000 DP." for BT21-041's
"[Your Turn] ...") and on nine cards as a copy of the link requirement instead. Until
2026-10-05 cards.json held that text in `inherited_effect_description_eng`, so production
(card_data.rs reads the field as `CardData.inherited_text`) gave a Digimon the link effect
of every link card among its digivolution cards (BT21-009 Gatchmon: <Raid>), and the DSL's
`has_inherited` predicate counted all 77 link cards as cards "with inherited effects".
`ingest_cards.normalize_link_box` now moves the text when the API marks a card as a link
card; data/card_overrides.json restores what the API drops.

The printed text comes from the official Bandai DB mirror (data/card_official.json), which
files a link box two ways: BT21's cards as an "Inherited Effect" section that starts with
the "＜Link＞ ... (Plug this card ...)" requirement, later sets as "Link Condition" plus
"Link Effect" sections.
"""
import json
import os
import re
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
_ROOT = os.path.abspath(os.path.join(_HERE, "..", ".."))

INHERITED = "inherited_effect_description_eng"
LINK = "link_effect_description_eng"
_TEXT_FIELDS = ("effect_description_eng", INHERITED, "security_effect_description_eng")

# A link requirement as the official mirror prints it, or as the API's `source_effect`
# copies it ("Link Requirements [Link] [Appmon] trait: Cost 2 (Plug this card ...)").
_OFFICIAL_LINK_REQUIREMENT = re.compile(r"^＜Link＞.*?\(Plug this card[^)]*\)\s*", re.S)
_LINK_REQUIREMENT_TEXT = re.compile(r"Link Requirements|Plug this card")

# Link cards whose official-mirror link box is another card's (checked against the card
# image on 2026-10-05). An entry must still disagree with the mirror, so a corrected mirror
# retires it.
#   EX10-016 Mirrormon: the mirror lists EX10-014 Weatherdramon's link box (Cost 2,
#   +3000 DP, "-6000 DP"); the card prints Cost 1, +2000 DP and the effect below, as do
#   DCGO's card asset and the API.
LINK_EFFECT_OVER_MIRROR = {
    "EX10-016": "[When Attacking] By trashing 1 of this Digimon's link cards, suspend 2 of your opponent's Digimon.",
}


def _load(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def _inputs():
    cards = _load(os.path.join(_ROOT, "data", "cards.json"))
    official = _load(os.path.join(_ROOT, "data", "card_official.json"))["cards"]
    return cards, official


def _norm(text):
    text = (text or "").replace("\xa0", " ").replace("’", "'")
    return re.sub(r"\s+", " ", text).strip()


def official_link_effect(page):
    """The link effect an official page prints, or None for a card without a link box."""
    for section in page.get("text_sections") or []:
        if section.get("label") == "Link Effect":
            return section.get("text") or ""
    for section in page.get("text_sections") or []:
        text = (section.get("text") or "").strip()
        if section.get("label") == "Inherited Effect" and text.startswith("＜Link＞"):
            return _OFFICIAL_LINK_REQUIREMENT.sub("", text, count=1)
    return None


def official_inherited_effect(page):
    """The inherited effect an official page prints (a link box filed under that label is not one)."""
    for section in page.get("text_sections") or []:
        text = (section.get("text") or "").strip()
        if section.get("label") == "Inherited Effect" and not text.startswith("＜Link＞"):
            return text
    return ""


def printed_link_effects(official):
    out = {cid: official_link_effect(page) for cid, page in official.items()}
    out = {cid: text for cid, text in out.items() if text is not None}
    out.update(LINK_EFFECT_OVER_MIRROR)
    return out


def _ingest():
    sys.path.insert(0, os.path.join(_ROOT, "code", "tools"))
    try:
        import ingest_cards
    finally:
        sys.path.pop(0)
    return ingest_cards


def test_the_mirror_prints_77_link_boxes():
    """Pins the population the other tests range over, so a mirror parse that stops seeing
    link boxes cannot pass them vacuously."""
    _, official = _inputs()
    assert len(printed_link_effects(official)) == 77


def test_no_link_card_holds_its_link_box_in_the_inherited_field():
    cards, official = _inputs()
    problems = []
    for cid in sorted(printed_link_effects(official)):
        have = (cards.get(cid) or {}).get(INHERITED) or ""
        want = official_inherited_effect(official.get(cid) or {})
        if _norm(have) != _norm(want):
            problems.append(f"{cid}: inherited text {have!r}, the card prints {want!r} as its inherited effect")
    assert not problems, "link box text in the inherited field:\n  " + "\n  ".join(problems)


def test_link_effect_matches_what_the_card_prints():
    cards, official = _inputs()
    problems = []
    for cid, printed in sorted(printed_link_effects(official).items()):
        have = (cards.get(cid) or {}).get(LINK)
        if have is None or _norm(have) != _norm(printed):
            problems.append(f"{cid}: {LINK} {have!r}, the card prints {printed!r}")
    assert not problems, "\n  ".join(problems)


def test_only_link_cards_have_a_link_effect():
    cards, official = _inputs()
    link_cards = printed_link_effects(official)
    stray = sorted(cid for cid, card in cards.items() if LINK in card and cid not in link_cards)
    assert not stray, f"{LINK} on cards that print no link box: {stray}"


def test_no_text_field_holds_a_link_requirement():
    cards, _ = _inputs()
    found = [f"{cid}: {field}" for cid, card in sorted(cards.items()) for field in _TEXT_FIELDS + (LINK,)
             if _LINK_REQUIREMENT_TEXT.search(card.get(field) or "")]
    assert not found, "link requirement text left in a text field:\n  " + "\n  ".join(found)


def test_link_effect_over_mirror_entries_still_disagree_with_the_mirror():
    _, official = _inputs()
    stale = [cid for cid, text in LINK_EFFECT_OVER_MIRROR.items()
             if _norm(official_link_effect(official.get(cid) or {})) == _norm(text)]
    assert not stale, f"the mirror now prints these link effects; drop them from LINK_EFFECT_OVER_MIRROR: {stale}"


_REQUIREMENT = ("Link Requirements [Link] [Appmon] trait: Cost 1\r\n"
                "(Plug this card from the hand or battle area sideways into the specified Digimon in the battle area.)")


def test_normalize_link_box_moves_a_link_cards_source_effect():
    ingest = _ingest()
    api = {"source_effect": "＜Raid＞", "link_requirements": _REQUIREMENT, "link_dp": 2000}
    card = {INHERITED: "＜Raid＞"}
    assert ingest.normalize_link_box(card, api) is True
    assert card == {INHERITED: "", LINK: "＜Raid＞"}
    assert ingest.normalize_link_box(card, api) is False  # idempotent


def test_normalize_link_box_drops_a_copied_link_requirement():
    ingest = _ingest()
    copied = _REQUIREMENT.replace("Cost 1", "Cost 1 ")
    api = {"source_effect": copied, "link_requirements": _REQUIREMENT, "link_dp": 2000}
    card = {INHERITED: copied}
    assert ingest.normalize_link_box(card, api) is True
    assert card == {INHERITED: "", LINK: ""}


def test_normalize_link_box_leaves_other_cards_alone():
    ingest = _ingest()
    api = {"source_effect": "[Your Turn] This Digimon gets +1000 DP.", "link_requirements": None, "link_dp": None}
    card = {INHERITED: "[Your Turn] This Digimon gets +1000 DP."}
    assert ingest.normalize_link_box(card, api) is False
    assert card == {INHERITED: "[Your Turn] This Digimon gets +1000 DP."}


def test_convert_card_files_the_link_box_as_a_link_effect():
    ingest = _ingest()
    api = {
        "name": "Calendamon", "type": "Digimon", "id": "BT21-041", "level": 3, "play_cost": 3,
        "color": "Yellow", "digi_type": "Calendar (App Name)", "form": "Stnd.", "dp": 1000,
        "attribute": "Tool", "rarity": "C",
        "xros_req": "[Digivolve] Lv.2 w/[Appmon] trait: Cost 0",
        "main_effect": "[Security] At the end of the battle, play this card without paying the cost.",
        "source_effect": "All of your opponent's Security Digimon get -3000 DP.",
        "link_requirements": _REQUIREMENT, "link_dp": 2000,
    }
    card = ingest.convert_card(api)
    assert card[INHERITED] == ""
    assert card[LINK] == "All of your opponent's Security Digimon get -3000 DP."
