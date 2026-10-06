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
5. Every card in the mirror holds the traits it prints, pool-wide: its form, attribute
   and type lines (stages aside) plus the traits its text grants, or what Bandai has
   announced instead (ANNOUNCED_TRAITS). Production matches on form_eng + attribute_eng
   + type_eng as one list, so that union must agree, and a Digimon or Digi-Egg must file
   each trait in the field it is printed in. Until 2026-10-05, 44 cards held another
   attribute, a stage as their attribute, a renamed, invented or split trait, or a trait
   in the wrong field.
"""
import html
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
# image and the DCGO card asset on 2026-10-03; traits, against the card's own block on the
# official card list too, on 2026-10-05). Two causes:
#   - build_card_bundles.parse_official keeps the LAST printing's cost/DP/colour and traits
#     from a search page that lists every printing, and some alt-printing blocks on the
#     official site carry another card's stats (BT15-017_P1/_P3/_P4 list 12/12000; the card
#     prints 11/11000) or traits (BT11-012_P1 lists Hybrid/Variable/Wizard; BT2-012_P1 files
#     its type as the attribute, ST4-03_P1 its attribute as the type).
#   - the official DB's own entry for the card is wrong (BT8-099 lists Blue Red; the card is
#     blue|green; BT22-001 is a Digi-Egg and prints no DP; BT18-086 prints 0 DP, listed blank;
#     EX3-051 and EX3-052 have each other's attribute; BT6-024 prints [Rare Animal], as does
#     BT26-023, where the DB says [Elusive Beast]; P-217 and BT14-098 are listed with no traits).
# Trait fields hold what the card prints on that line, stages included as printed.
PRINTED_OVER_MIRROR = {
    "BT11-012": {"play_cost": 7, "dp": 6000, "card_colors": [0, 2],
                 "form_eng": ["Champion"], "attribute_eng": ["Data"], "type_eng": ["Composite", "Xros Heart"]},
    "BT11-015": {"dp": 8000},
    "BT12-042": {"card_colors": [2, 0]},
    "BT15-017": {"play_cost": 11, "dp": 11000, "attribute_eng": ["Vaccine"], "type_eng": ["Holy Beast"]},
    "BT15-053": {"play_cost": 12, "dp": 12000, "type_eng": ["Insectoid"]},
    "BT15-042": {"type_eng": ["Holy Dragon", "Four Great Dragons"]},
    "BT15-067": {"type_eng": ["Beast Dragon", "X Antibody", "DigiPolice"]},
    "EX4-050": {"attribute_eng": ["Virus"], "type_eng": ["Seraph"]},
    "P-060": {"attribute_eng": ["Vaccine"], "type_eng": ["Beast"]},
    "BT2-012": {"attribute_eng": ["Vaccine"]},
    "BT2-015": {"attribute_eng": ["Vaccine"]},
    "BT2-025": {"attribute_eng": ["Vaccine"]},
    "BT2-027": {"attribute_eng": ["Vaccine"]},
    "ST3-05": {"attribute_eng": ["Vaccine"]},
    "ST4-03": {"type_eng": ["Insectoid"]},
    "ST4-10": {"type_eng": ["Fairy"]},
    "ST5-03": {"type_eng": ["Reptile"]},
    "EX3-051": {"attribute_eng": ["Virus"]},
    "EX3-052": {"attribute_eng": ["Data"]},
    "ST6-03": {"attribute_eng": ["Data"]},
    "BT6-024": {"type_eng": ["Rare Animal"]},
    "P-217": {"type_eng": ["App Driver", "Appmon"]},
    "BT14-098": {"type_eng": ["D-Brigade"]},
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

# Traits Bandai changed by announcement (world.digimoncard.com/rule/), beyond what the card
# prints and the mirror lists. Like PRINTED_OVER_MIRROR, cards.json must hold these, and an
# entry retires once the mirror agrees.
#   - "Digimon Type Changes" (2022-04-22): these print [Sea Animal]; "revised printings of
#     these cards are not available". [Sea Animal] is still BT19-066 Gizamon's trait.
#   - "Japanese / English Character Changes" (2022-03-18): BT6-084 and BT7-083 Sistermon Ciel
#     have the attributes Virus and Data (later printings print it, e.g. ST12-13, BT23-077).
#   ("Card Text and Information", 2022-07-15: [X-Antibody] is [X Antibody]; _trait_key
#   compares them alike.)
ANNOUNCED_TRAITS = {
    "BT2-024": {"type_eng": ["Aquatic"]},
    "BT2-029": {"type_eng": ["Aquatic"]},
    "BT4-024": {"type_eng": ["Aquatic"]},
    "BT4-028": {"type_eng": ["Aquatic"]},
    "BT4-034": {"type_eng": ["Aquatic"]},
    "BT6-084": {"attribute_eng": ["Data", "Virus"]},
    "BT7-083": {"attribute_eng": ["Data", "Virus"]},
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


_TRAIT_FIELDS = (("form_eng", "form"), ("attribute_eng", "attribute"), ("type_eng", "type"))
_MIRROR_KEY = dict(_TRAIT_FIELDS)
# Trait names holding a "/" themselves. The official DB prints a card's traits as one
# "/"-joined line, so these stay whole: BT24-056 Dezipmon's app name is [Zip/Unzip].
_SLASHED_TRAITS = ("Zip/Unzip",)
# Traits a card's text grants, which the API never carries: (Rule) boxes ("Trait: Has
# [Ice-Snow] Type.", "Also has Name:[Sistermon Noir] and Trait: [Virus].", "... and has
# Trait: [DigiPolice].") and two printed lines, P-101's "also treated as having the [Cyborg]
# trait" and ST12-13's "traits include [Virus]". A digivolve condition's "w/[X] trait: Cost 3"
# is lower case and never matches.
_TRAIT_GRANT = re.compile(r"Trait:\s*(?:Has\s*)?((?:\[[^\]]+\]\s*(?:and\s*)?)+)(?i:(attribute|form))?")
_TRAIT_LINE = re.compile(r"treated as having the \[([^\]]+)\] trait|traits include \[([^\]]+)\]")
# reconcile_traits.py keeps the API's "Search (App Name)" beside the printed [Search].
_APP_NAME = re.compile(r"\s*\(App Name\)$")


def _trait_key(name):
    """Case, hyphens, colons and spacing don't tell traits apart: old printings spell
    [X-Antibody] for [X Antibody]. HTML entities do: "Copy &amp; Paste" is ingest residue."""
    return " ".join(name.replace("-", " ").replace(":", " ").split()).casefold()


_STAGE_KEYS = {_trait_key(stage) for stage in _STAGES}


def _trait_line(line):
    """The traits on one official form/attribute/type line, split on "/" (but not inside a
    _SLASHED_TRAITS name)."""
    parts, out = [p.strip() for p in html.unescape(line or "").split("/")], []
    while parts:
        if "/".join(parts[:2]) in _SLASHED_TRAITS:
            out.append("/".join(parts[:2]))
            del parts[:2]
        else:
            out.append(parts.pop(0))
    return [t for t in out if t]


def _trait_set(names, field):
    """Comparable keys of a field's traits; stages are not traits in the form field."""
    return sorted({_trait_key(t) for t in names or []} - (_STAGE_KEYS if field == "form_eng" else set()))


def _filed(card, field):
    """The keys a cards.json field files, an "(App Name)" copy counted as its trait."""
    return set(_trait_set([_APP_NAME.sub("", t) for t in card.get(field) or []], field))


def trait_grants(page):
    """The traits the card's printed text grants (see _TRAIT_GRANT)."""
    text = " ".join(s.get("text", "") for s in page.get("text_sections") or [])
    names = [n for m in _TRAIT_GRANT.finditer(text) for n in re.findall(r"\[([^\]]+)\]", m.group(1))]
    return names + [m.group(1) or m.group(2) for m in _TRAIT_LINE.finditer(text)]


def trait_violations(cards, official, ids, printed=None):
    """Cards whose traits differ from what they print, as strings. Empty means clean.

    A card prints its form, attribute and type lines (stages aside) and the traits its text
    grants. Production matches on form_eng + attribute_eng + type_eng as one list, so that
    union must equal the printed traits; an "(App Name)" copy of an app-name trait may ride
    along. A Digimon or Digi-Egg must also file each printed trait under its own field: the
    deck builder shows the fields apart, and an attribute filed as a type hid EX11-011's
    missing attribute. `printed` replaces a wrong mirror line (see PRINTED_OVER_MIRROR)."""
    out = []
    for cid in ids:
        card, page = cards.get(cid), official.get(cid) or {}
        if card is None or "colors" not in page:
            continue
        fixed = (printed or {}).get(cid, {})
        lines = {f: fixed[f] if f in fixed else _trait_line(page.get(key)) for f, key in _TRAIT_FIELDS}
        grants = trait_grants(page)
        want = {f: set(_trait_set(lines[f], f)) for f, _ in _TRAIT_FIELDS}
        grant_keys = set(_trait_set(grants, "type_eng"))
        want_all = set().union(*want.values(), grant_keys)
        prints = ", ".join(f"{key} {'/'.join(lines[f])}" for f, key in _TRAIT_FIELDS if lines[f])
        prints += f", and its text grants {grants}" if grants else ""
        # A trait is held only under its own name; the "(App Name)" copy may ride along.
        held = set().union(*(_trait_set(card.get(f), f) for f, _ in _TRAIT_FIELDS))
        filed = set().union(*(_filed(card, f) for f, _ in _TRAIT_FIELDS))
        printed_name = {_trait_key(t): t for names in (*lines.values(), grants) for t in names}
        held_name = {_trait_key(_APP_NAME.sub("", t)): t for f, _ in _TRAIT_FIELDS for t in card.get(f) or []}
        missing = sorted(printed_name[k] for k in want_all - held)
        extra = sorted(held_name[k] for k in filed - want_all)
        if missing or extra:
            out.append(f"{cid}: lacks {missing} and holds {extra}; the card prints {prints}")
            continue
        if card.get("card_kind") not in (0, 3):  # only a Digimon or Digi-Egg prints the three fields
            continue
        for field, key in _TRAIT_FIELDS:
            here = _filed(card, field)
            if not want[field] <= here or here - want[field] - grant_keys:
                out.append(f"{cid}: {field} {card.get(field)}, but the card prints {key} "
                           f"{'/'.join(lines[field]) or '(none)'}")
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
    if field in _MIRROR_KEY:
        return _trait_set(_trait_line(page.get(_MIRROR_KEY[field])), field)
    return sorted(_COLOR[c.lower()] for c in page.get("colors", "").split())


def test_cards_keep_printed_values_where_the_official_mirror_is_wrong():
    cards, official = _repo_inputs()
    problems = []
    for cid, fields in [*PRINTED_OVER_MIRROR.items(), *ANNOUNCED_TRAITS.items()]:
        for field, value in fields.items():
            have = cards[cid].get(field)
            if field == "card_colors":
                have, value = sorted(have or []), sorted(value)
            elif field in _MIRROR_KEY:
                have, value = _trait_set(have, field), _trait_set(value, field)
            if have != value:
                problems.append(f"{cid}: cards.json {field} {have!r}, the card has {value!r}")
            if _mirror_value(official.get(cid) or {}, field) == value:
                problems.append(f"{cid}: the mirror now agrees on {field}; drop the entry")
    assert not problems, "\n  ".join(problems)


def test_every_card_holds_the_traits_it_prints():
    # Until 2026-10-05, 44 cards disagreed (code/tools/archive/fix_card_traits_drift_2026_10.py):
    # the other attribute, a stage as the attribute, the API's [Avian] for BT10-060's printed
    # [Bird], a bogus "3", "Hudie/CS" as one trait, a dropped grant.
    cards, official = _repo_inputs()
    governing = {cid: {**PRINTED_OVER_MIRROR.get(cid, {}), **ANNOUNCED_TRAITS.get(cid, {})}
                 for cid in PRINTED_OVER_MIRROR.keys() | ANNOUNCED_TRAITS.keys()}
    violations = trait_violations(cards, official, sorted(official), printed=governing)
    assert not violations, (f"{len(violations)} trait disagreements with what the cards print (fix "
                            "cards.json through data/card_overrides.json, or record a wrong mirror "
                            "line in PRINTED_OVER_MIRROR):\n  " + "\n  ".join(violations))


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


def test_traits_compare_as_one_list_and_by_field():
    cards, official = _synthetic({"form_eng": ["Mega"], "attribute_eng": ["Vaccine"], "type_eng": ["Dinosaur", "LIBERATOR"]},
                                 {"form": "Mega", "attribute": "Vaccine", "type": "Dinosaur/LIBERATOR"})
    assert trait_violations(cards, official, ["X-1"]) == []
    # EX11-011: the right traits, the attribute filed as a type.
    cards["X-1"].update(attribute_eng=[], type_eng=["Vaccine", "Dinosaur", "LIBERATOR"])
    assert trait_violations(cards, official, ["X-1"]) == [
        "X-1: attribute_eng [], but the card prints attribute Vaccine",
        "X-1: type_eng ['Vaccine', 'Dinosaur', 'LIBERATOR'], but the card prints type Dinosaur/LIBERATOR"]
    # An Option prints one trait line; the DB files BT9-104's [X Antibody] as its attribute.
    cards["X-1"]["card_kind"] = 2
    assert trait_violations(cards, official, ["X-1"]) == []
    # BT24-072: the stage "Ultimate" as the attribute.
    cards, official = _synthetic({"attribute_eng": ["Ultimate"]}, {"form": "Ultimate", "attribute": "Virus"})
    assert trait_violations(cards, official, ["X-1"]) == [
        "X-1: lacks ['Virus'] and holds ['Ultimate']; the card prints form Ultimate, attribute Virus"]


def test_trait_grants_spellings_and_app_names():
    page = {"type": "Puppet/CS", "attribute": "Data", "text_sections": [
        {"label": "Effect", "text": "[On Play] Draw 1. (Rule) Also has Name:[Sistermon Noir] and Trait: [Virus]."}]}
    cards, official = _synthetic({"attribute_eng": ["Data", "Virus"], "type_eng": ["Puppet", "CS"]}, page)
    assert trait_violations(cards, official, ["X-1"]) == []
    cards["X-1"]["attribute_eng"] = ["Data"]
    assert len(trait_violations(cards, official, ["X-1"])) == 1
    for text in ("(Rule) Trait: Has [Ice-Snow] Type.", "(Rule) Also treated as Name: [Shuu Yulin] and has Trait: [Ice-Snow].",
                 "This card/Digimon is also treated as having the [Ice-Snow] trait.",
                 "*Also treat as if name is [Agumon] and traits include [Ice-Snow]."):
        assert trait_grants({"text_sections": [{"label": "Effect", "text": text}]}) == ["Ice-Snow"], text
    assert trait_grants({"text_sections": [{"label": "Special Digivolution Condition",
                                            "text": "[Digivolve] Lv.4 w/[Demon]/[TS] trait: Cost 3"}]}) == []
    # [X-Antibody] on old printings is [X Antibody]; an app name keeps its "/" and its "(App Name)" copy.
    cards, official = _synthetic({"form_eng": ["Sup.", "Appmon"], "type_eng": ["Zip/Unzip (App Name)", "Zip/Unzip", "X Antibody"]},
                                 {"form": "Sup./Appmon", "type": "Zip/Unzip/X-Antibody"})
    assert trait_violations(cards, official, ["X-1"]) == []
    cards["X-1"]["type_eng"] = ["Zip/Unzip (App Name)", "Zip", "Unzip", "X Antibody"]
    assert len(trait_violations(cards, official, ["X-1"])) == 1
    # Ingest residue is not the trait: "&amp;" and "}}" never match what the card prints.
    for residue in ("Copy &amp; Paste", "Copy & Paste}}"):
        cards, official = _synthetic({"type_eng": [residue]}, {"type": "Copy &amp; Paste"})
        assert len(trait_violations(cards, official, ["X-1"])) == 1, residue


def test_no_printed_cost_matches_the_zero_convention():
    cards, official = _synthetic({"play_cost": 0, "dp": None, "card_kind": 3}, {"play_cost": None, "dp": None})
    assert stats_violations(cards, official, ["X-1"]) == []


def test_printed_values_stand_in_for_a_wrong_mirror():
    cards, official = _synthetic({"dp": None, "card_kind": 3, "play_cost": 0}, {"play_cost": None, "dp": "1000"})
    assert stats_violations(cards, official, ["X-1"]) == ["X-1: dp None != official '1000'"]
    assert stats_violations(cards, official, ["X-1"], printed={"X-1": {"dp": None}}) == []
