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
4. Where the official mirror itself is wrong, PRINTED_OVER_MIRROR records what the card
   prints, and cards.json must hold that instead. Each entry must still disagree with
   the mirror, so a rebuilt or corrected mirror retires it.
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
# mirror. Add an id once its entry is fixed; the guard then keeps it fixed. The values were
# checked against the card image (and the DCGO card asset, where it has one) on 2026-10-03;
# the corrections live in data/card_overrides.json (see its _note_ keys).
REVIEWED = (
    # Pre-release stubs (cost 0, no DP, no circles, no text) until 2026-10-03.
    "P-239", "P-240", "P-244",
    # Never-refreshed stubs: cost 0, no DP, no text, no special digivolution condition.
    "BT24-028", "BT24-061", "BT24-068", "BT24-078",
    # Play cost / DP the API still gets wrong (ST24-04 and BT25-088 also had stale text).
    "BT17-042", "BT20-056", "BT20-060", "BT21-019", "BT23-005", "BT23-007", "BT23-010", "BT23-036",
    "BT23-044", "BT24-044", "BT25-088", "EX11-013", "EX11-033", "EX12-048", "EX4-047", "EX9-039",
    "P-136", "P-168", "P-169", "ST23-06", "ST24-04", "ST24-08",
    # Three-colour cards: the API carries two colours (BT16-102's override had the wrong third).
    "AD1-006", "AD1-020", "AD1-023", "BT16-102", "BT17-077", "BT18-018", "BT18-042", "BT18-095",
    "BT18-097", "BT19-014", "BT19-091", "BT19-101", "BT20-045", "BT20-102", "BT21-052", "BT22-015",
    "BT23-047", "BT25-084", "EX10-011", "EX7-037", "EX7-074", "EX8-029", "EX8-064", "EX8-073",
    "EX9-021", "EX9-045", "P-185",
    # `|applinkdp =` residue where the card has no inherited effect.
    "BT21-030", "BT21-035", "BT25-019", "BT25-020", "BT25-029", "BT25-037", "BT25-042", "BT25-058",
    "BT25-059", "BT25-060", "BT25-075", "BT25-076", "BT25-077", "BT25-103", "EX12-065", "ST23-05",
    "ST24-11",
    # Missing or misfiled text, a missing trait, or a missing special digivolution condition.
    "BT8-061", "BT13-006", "BT22-001", "BT24-020", "BT24-022", "EX10-069", "BT20-073", "BT24-025",
    # Its Virus attribute override (d3fb5686e) had never been applied to cards.json (2026-10-04).
    "BT25-044",
)

# What the card prints, where data/card_official.json is wrong (checked against the card
# image and the DCGO card asset on 2026-10-03). Two causes:
#   - build_card_bundles.parse_official keeps the LAST printing's cost/DP/colour from a search
#     page that lists every printing, and some alt-printing blocks on the official site carry
#     another card's stats (BT15-017_P1/_P3/_P4 list 12/12000; the card prints 11/11000).
#   - the official DB's own entry for the card is wrong (BT8-099 lists Blue Red; the card is
#     blue|green; BT22-001 is a Digi-Egg and prints no DP; BT18-086 prints 0 DP, listed blank).
PRINTED_OVER_MIRROR = {
    "BT11-012": {"play_cost": 7, "dp": 6000, "card_colors": [0, 2]},
    "BT11-015": {"dp": 8000},
    "BT12-042": {"card_colors": [2, 0]},
    "BT15-017": {"play_cost": 11, "dp": 11000},
    "BT15-053": {"play_cost": 12, "dp": 12000},
    "BT21-029": {"play_cost": 12},
    "EX4-052": {"dp": 2000},
    "EX6-029": {"play_cost": 7, "dp": 12000},
    "EX8-016": {"play_cost": 13},
    "EX8-065": {"play_cost": 3},
    "EX8-067": {"play_cost": 4},
    "EX9-033": {"play_cost": 12},
    "P-181": {"card_colors": [3, 5]},
    "BT8-099": {"card_colors": [1, 3]},
    "BT10-068": {"card_colors": [5]},
    "BT10-107": {"card_colors": [6]},
    "LM-006": {"card_colors": [1, 6]},
    "EX3-029": {"dp": 5000},
    "EX3-054": {"dp": 12000},
    "P-058": {"dp": 4000},
    "BT22-001": {"dp": None},
    "BT18-086": {"dp": 0},
}


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


# A (Rule) box, whose bracketed names may themselves hold full stops ([J.P. Shibayama]).
_RULE_BOX = re.compile(r"\(Rule\)\s*(?:[^\[\].]|\[[^\]]*\])*\.?")
# The classic stages ride in the official Form field, but reconcile_traits.py keeps them out
# of form_eng: they are not match-on traits. Forms that are traits (Hybrid, Appmon, ...) count.
_STAGES = {"Baby", "In-Training", "Rookie", "Champion", "Ultimate", "Mega"}


def _tokens(text):
    """Bracketed tokens, case-folded. The official DB prints a zone box as {Trash} where
    cards.json writes [Trash], and sometimes writes "Once per Turn". (Rule) boxes are dropped:
    the API never carries them, and tests/dsl official_rule_grants guards the trait grants."""
    text = _RULE_BOX.sub(" ", text or "")
    return sorted({t.casefold() for t in re.findall(r"[\[{]([^\]}]+)[\]}]", text)})


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


def stats_violations(cards, official, ids, printed=None):
    """Every printed-data disagreement for `ids`, as strings. Empty means clean.

    `printed` maps a card to cards.json-shaped values that replace a wrong mirror value
    (see PRINTED_OVER_MIRROR)."""
    out = []
    for cid in ids:
        card, page = cards.get(cid), official.get(cid) or {}
        fixed = (printed or {}).get(cid, {})
        if card is None:
            out.append(f"{cid}: missing from cards.json")
            continue
        if "colors" not in page:
            out.append(f"{cid}: the official mirror holds no page data for it")
            continue
        # cards.json stores 0 for a card that prints no play cost (Digi-Eggs).
        want = fixed.get("play_cost", _number(page.get("play_cost")) or 0)
        if (card.get("play_cost") or 0) != want:
            out.append(f"{cid}: play_cost {card.get('play_cost')!r} != official {page.get('play_cost')!r}")
        want = fixed["dp"] if "dp" in fixed else _number(page.get("dp"))
        if card.get("dp") != want:
            out.append(f"{cid}: dp {card.get('dp')!r} != official {page.get('dp')!r}")
        colors = fixed.get("card_colors") or [_COLOR[c.lower()] for c in page["colors"].split()]
        if sorted(card.get("card_colors") or []) != sorted(colors):
            out.append(f"{cid}: card_colors {card.get('card_colors')} != official {page['colors']!r}")
        circles = sorted((d["card_color"], d["level"], d["memory_cost"]) for d in page.get("digivolve_costs") or [])
        have = sorted((d["card_color"], d["level"], d["memory_cost"]) for d in card.get("evo_costs") or [])
        if have != circles:
            out.append(f"{cid}: evo_costs {have} != official circles {circles}")
        for field, key in (("type_eng", "type"), ("form_eng", "form"), ("attribute_eng", "attribute")):
            missing = [t.strip() for t in (page.get(key) or "").split("/")
                       if t.strip() and t.strip() not in (card.get(field) or [])
                       and not (field == "form_eng" and t.strip() in _STAGES)]
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
    violations = stats_violations(cards, official, REVIEWED, printed=PRINTED_OVER_MIRROR)
    assert not violations, "cards.json disagrees with the official Bandai DB:\n  " + "\n  ".join(violations)


def _mirror_value(page, field):
    if field == "play_cost":
        return _number(page.get("play_cost")) or 0
    if field == "dp":
        return _number(page.get("dp"))
    return sorted(_COLOR[c.lower()] for c in page.get("colors", "").split())


def test_cards_keep_printed_values_where_the_official_mirror_is_wrong():
    cards, official = _repo_inputs()
    problems = []
    for cid, fields in PRINTED_OVER_MIRROR.items():
        for field, value in fields.items():
            have = cards[cid].get(field)
            if field == "card_colors":
                have, value = sorted(have or []), sorted(value)
            if have != value:
                problems.append(f"{cid}: cards.json {field} {have!r}, the card prints {value!r}")
            if _mirror_value(official.get(cid) or {}, field) == value:
                problems.append(f"{cid}: the mirror now agrees on {field}; drop the PRINTED_OVER_MIRROR entry")
    assert not problems, "\n  ".join(problems)


def _synthetic(card, page):
    base_card = {"play_cost": 3, "dp": 4000, "card_colors": [0], "evo_costs": [], "type_eng": [],
                 "form_eng": [], "attribute_eng": [], "card_kind": 0}
    base_page = {"colors": "Red", "play_cost": "3", "dp": "4000", "text_sections": []}
    return {"X-1": {**base_card, **card}}, {"X-1": {**base_page, **page}}


def test_zone_braces_case_and_rule_lines_are_not_drift():
    # The official DB prints a zone box as {Trash}, sometimes writes "Once per Turn", and
    # carries (Rule) grants the API never has (guarded pool-wide by tests/dsl official_rule_grants).
    cards, official = _synthetic(
        {"effect_description_eng": "[Trash] [Main] Play this card.\r\n[All Turns] [Once Per Turn] Draw."},
        {"text_sections": [{"label": "Effect", "text": "{Trash} [Main] Play this card. [All Turns] [Once per Turn]"
                            " Draw. (Rule) Name: Also treated as [J.P. Shibayama]/[Koji Minamoto]."}]})
    assert stats_violations(cards, official, ["X-1"]) == []
    cards["X-1"]["effect_description_eng"] = "[Main] Play this card.\r\n[Your Turn] [Once Per Turn] Draw."
    assert len(stats_violations(cards, official, ["X-1"])) == 1


def test_stage_forms_are_not_required_but_trait_forms_are():
    # reconcile_traits.py keeps the stage (Rookie..Mega) out of form_eng: it is not a match-on trait.
    cards, official = _synthetic({}, {"form": "Mega"})
    assert stats_violations(cards, official, ["X-1"]) == []
    cards, official = _synthetic({}, {"form": "Hybrid"})
    assert stats_violations(cards, official, ["X-1"]) == ["X-1: form_eng [] lacks official ['Hybrid']"]


def test_no_printed_cost_matches_the_zero_convention():
    cards, official = _synthetic({"play_cost": 0, "dp": None, "card_kind": 3}, {"play_cost": None, "dp": None})
    assert stats_violations(cards, official, ["X-1"]) == []


def test_printed_values_stand_in_for_a_wrong_mirror():
    cards, official = _synthetic({"dp": None, "card_kind": 3, "play_cost": 0}, {"play_cost": None, "dp": "1000"})
    assert stats_violations(cards, official, ["X-1"]) == ["X-1: dp None != official '1000'"]
    assert stats_violations(cards, official, ["X-1"], printed={"X-1": {"dp": None}}) == []
