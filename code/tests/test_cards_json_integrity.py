"""Data guards on data/cards.json: card identity, and cards reconciled against the official DB.

Production builds CardData from data/cards.json (`deck_tools.rs` include_str!s it), so a
wrong entry there is wrong in every real game whatever the card's YAML says.

1. No two card ids may name the same (set, number). The digimoncard.io API served
   RB1-10 beside RB1-010 Siriusmon, and it sat in cards.json as a second, stale copy.
2. For REVIEWED cards, the printed data the official Bandai DB mirror records
   (data/card_official.json) must match cards.json: play cost, DP, colours, digivolve
   circles, traits, and the bracketed tokens of each text section. Token sets are
   compared rather than strings because cards.json keeps the API's keyword reminder
   text; tokens still catch a wrong timing ([All Turns] for [Your Turn]) or a dropped
   name/trait reference.
3. Every card has a unique registry index, with norm_id = index / capacity. The
   engine's CardRegistry maps a card without one to PADDING_ID, so the RL observation
   cannot tell it from an empty slot. All 77 EX12 cards sat there from their
   2026-07-05 ingest until 2026-10-03; the Rust twin of this check
   (tests/mask_and_tensor/card_registry_parity.rs) runs in no CI workflow.
"""
import json
import os
import re
from collections import defaultdict

_HERE = os.path.dirname(os.path.abspath(__file__))
_ROOT = os.path.abspath(os.path.join(_HERE, "..", ".."))

_COLOR = {"red": 0, "blue": 1, "yellow": 2, "green": 3, "white": 4, "black": 5, "purple": 6}
_RESIDUE = re.compile(r"\|\s*\w+\s*=")  # `|applinkdp =`: MediaWiki residue from pre-release ingests

# Cards whose cards.json entry has been reconciled field by field against the official
# mirror. Add an id once its entry is fixed; the guard then keeps it fixed.
#   P-239 / P-240 / P-244 were pre-release stubs (cost 0, no DP, no circles, no text)
#   until 2026-10-03.
REVIEWED = ("P-239", "P-240", "P-244")


def _load_json(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def id_collisions(card_ids):
    """Groups of card ids naming the same (set, number), e.g. ['RB1-010', 'RB1-10']."""
    groups = defaultdict(list)
    for cid in card_ids:
        m = re.fullmatch(r"([A-Z]+\d*)-(\d+)", cid)
        if m:
            groups[(m.group(1), int(m.group(2)))].append(cid)
    return sorted(sorted(ids) for ids in groups.values() if len(ids) > 1)


def _number(value):
    try:
        return int(str(value).strip())
    except (TypeError, ValueError):
        return None


def _tokens(text):
    return sorted(set(re.findall(r"\[([^\]]+)\]", text or "")))


def _section_field(label, card):
    if label == "Effect":
        return "effect_description_eng"
    if label == "Inherited Effect":
        return "inherited_effect_description_eng"
    if label == "Security Effect":
        # cards.json files a Tamer's or Option's security text under the inherited key
        # (the API's convention, 460 of 485 Options with a security box).
        if card.get("card_kind") in (1, 2):
            return "inherited_effect_description_eng"
        return "security_effect_description_eng"
    return None


def stats_violations(cards, official, ids):
    """Every printed-data disagreement for `ids`, as strings. Empty means clean."""
    out = []
    for cid in ids:
        card, page = cards.get(cid), official.get(cid) or {}
        if card is None:
            out.append(f"{cid}: missing from cards.json")
            continue
        if "colors" not in page:
            out.append(f"{cid}: the official mirror holds no page data for it")
            continue
        for field, key in (("play_cost", "play_cost"), ("dp", "dp")):
            if card.get(field) != _number(page.get(key)):
                out.append(f"{cid}: {field} {card.get(field)!r} != official {page.get(key)!r}")
        colors = sorted(_COLOR[c.lower()] for c in page["colors"].split())
        if sorted(card.get("card_colors") or []) != colors:
            out.append(f"{cid}: card_colors {card.get('card_colors')} != official {page['colors']!r}")
        circles = sorted((d["card_color"], d["level"], d["memory_cost"]) for d in page.get("digivolve_costs") or [])
        have = sorted((d["card_color"], d["level"], d["memory_cost"]) for d in card.get("evo_costs") or [])
        if have != circles:
            out.append(f"{cid}: evo_costs {have} != official circles {circles}")
        for field, key in (("type_eng", "type"), ("form_eng", "form"), ("attribute_eng", "attribute")):
            missing = [t.strip() for t in (page.get(key) or "").split("/")
                       if t.strip() and t.strip() not in (card.get(field) or [])]
            if missing:
                out.append(f"{cid}: {field} {card.get(field)} lacks official {missing}")
        for section in page.get("text_sections") or []:
            field = _section_field(section["label"], card)
            if field is None:
                continue
            if _tokens(card.get(field)) != _tokens(section["text"]):
                out.append(f"{cid}: {field} tokens {_tokens(card.get(field))} != official "
                           f"{section['label']!r} tokens {_tokens(section['text'])}")
        for field in ("effect_description_eng", "inherited_effect_description_eng", "security_effect_description_eng"):
            if _RESIDUE.search(card.get(field) or ""):
                out.append(f"{cid}: {field} holds ingest residue {card.get(field)!r}")
    return out


def registry_violations(cards):
    """Cards without a usable, unique registry index, or with a norm_id that disagrees."""
    from tools.build_registry import REGISTRY_CAPACITY

    out, owner = [], {}
    for cid, card in cards.items():
        idx = card.get("index")
        if not isinstance(idx, int) or idx <= 0:
            out.append(f"{cid}: no registry index (the engine maps it to PADDING_ID)")
            continue
        if idx in owner:
            out.append(f"{cid}: index {idx} already belongs to {owner[idx]}")
        owner[idx] = cid
        if card.get("norm_id") != idx / REGISTRY_CAPACITY:
            out.append(f"{cid}: norm_id {card.get('norm_id')!r} != {idx} / {REGISTRY_CAPACITY}")
    return out


def _repo_inputs():
    cards = _load_json(os.path.join(_ROOT, "data", "cards.json"))
    official = _load_json(os.path.join(_ROOT, "data", "card_official.json"))["cards"]
    return cards, official


def test_no_two_cards_share_a_set_and_number():
    cards, _ = _repo_inputs()
    collisions = id_collisions(cards)
    assert not collisions, f"card ids naming the same card: {collisions}"


def test_every_card_has_a_unique_registry_index():
    cards, _ = _repo_inputs()
    violations = registry_violations(cards)
    assert not violations, f"{len(violations)} registry problems (first 10):\n  " + "\n  ".join(violations[:10])


def test_reviewed_cards_match_the_official_printed_data():
    cards, official = _repo_inputs()
    violations = stats_violations(cards, official, REVIEWED)
    assert not violations, "cards.json disagrees with the official Bandai DB:\n  " + "\n  ".join(violations)
