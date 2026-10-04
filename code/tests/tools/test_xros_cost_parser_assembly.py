"""EX13 Assembly requirement shapes in tools.xros_cost_parser.

The new element fields are serialized only when set, so these tests also pin
that legacy shapes keep their old JSON.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "tools"))

import ingest_cards as ic  # noqa: E402
from tools.xros_cost_parser import CardColor, parse_digixros_req  # noqa: E402


def _assembly(n, body):
    return (
        f"Assembly Requirements [Assembly\xa0-{n}] {body}\r\n"
        "When this would be played, by placing the specified cards from the trash under it, "
        "reduce the play cost."
    )


def _one(text):
    costs = parse_digixros_req(text)
    assert len(costs) == 1
    return costs[0]


def test_one_slot_per_level_with_name_alternatives():
    c = _one(_assembly(5, "Lv.5 × Lv.4 × Lv.3, all w/[Guilmon]/[Growlmon]\xa0in name"))
    assert [e.level_exact for e in c.elements] == [5, 4, 3]
    assert all(e.name_any == ["Guilmon", "Growlmon"] and e.count == 1 for e in c.elements)
    assert c.max_materials == 3 and c.reduce_cost_per_card == 5
    assert c.source_zones == ["trash"]


def test_level_slots_with_colour_and_trait_alternatives():
    c = _one(_assembly(5, "Lv.5 × Lv.4 × Lv.3, all green w/[Mammal]/[Beast]/[Beastkin] trait"))
    e = c.elements[0]
    assert e.color == CardColor.Green
    assert (e.trait_match, e.trait_alternatives) == ("Mammal", ["Beast", "Beastkin"])


def test_level_slots_with_keyword_and_text():
    kw = _one(_assembly(5, "Lv.5 × Lv.4 × Lv.3, all Black w/＜Blocker＞")).elements[0]
    assert (kw.keyword, kw.color) == ("Blocker", CardColor.Black)
    txt = _one(_assembly(5, "Lv.5 × Lv.4 × Lv.3, all w/[Huckmon] in text")).elements[0]
    assert txt.text_any == ["Huckmon"]


def test_multiplication_sign_named_materials():
    c = _one(_assembly(7, "[WarGreymon]×[MetalGarurumon]×[Agumon]×[Gabumon]"))
    assert [e.name_contains for e in c.elements] == [
        "WarGreymon", "MetalGarurumon", "Agumon", "Gabumon"]


def test_counted_shapes_and_distinctness_flags():
    c = _one(_assembly(5, "3 [Huckmon] text Digimon cards w/different names"))
    assert c.different_names and c.elements[0].text_any == ["Huckmon"] and c.elements[0].count == 3
    c = _one(_assembly(4, "3 Lv.5 or lower [Mamemon]\xa0text cards w/different names"))
    e = c.elements[0]
    assert (e.level_max, e.text_any, e.is_digimon_only) == (5, ["Mamemon"], False)
    c = _one(_assembly(4, "3 Lv.4 or lower Digimon cards w/[Sukamon]\xa0in name"))
    e = c.elements[0]
    assert (e.level_max, e.name_any, e.is_digimon_only) == (4, ["Sukamon"], True)
    c = _one(_assembly(8, "6 [ADVENTURE] trait Digimon cards w/different colors"))
    assert c.different_colors and not c.different_names
    assert c.elements[0].trait_match == "ADVENTURE"


def test_trait_count_is_a_trait_not_a_name():
    # BT26-086: the legacy parser read "[Seven Code] trait" as a NAME.
    e = _one(_assembly(7, "7 [Seven Code]\xa0trait Digimon cards w/different names")).elements[0]
    assert (e.trait_match, e.name_contains, e.is_digimon_only, e.count) == ("Seven Code", "", True, 7)


def test_legacy_shapes_keep_their_json():
    legacy = ic._parse_xros_costs(
        "DigiXros Requirements [DigiXros\xa0-2] 1 Digimon card w/[Bagra Army]\xa0trait\r\n"
        "When this would be played, you may place specified cards")[1]
    assert legacy == [{
        "elements": [{"name_contains": "", "trait_match": "Bagra Army", "trait_alternatives": [],
                      "level_max": None, "count": 1, "is_digimon_only": True, "color": None}],
        "reduce_cost_per_card": 2, "max_materials": 1, "different_card_numbers": False,
        "different_names": False, "has_text": "", "source_zones": ["hand", "field"],
    }]


def test_new_fields_serialize_only_when_set():
    out = ic._parse_xros_costs(_assembly(8, "6 [ADVENTURE] trait Digimon cards w/different colors"))[1][0]
    assert out["different_colors"] is True
    assert "level_exact" not in out["elements"][0]
    lv = ic._parse_xros_costs(_assembly(5, "Lv.5 × Lv.4 × Lv.3, all w/[Witchelny] in text"))[1][0]
    assert lv["elements"][0]["level_exact"] == 5 and lv["elements"][0]["text_any"] == ["Witchelny"]
    assert "different_colors" not in lv


_GAMMAMON_OR_VB = [{"text_any": ["Gammamon"]}, {"trait_match": "VB", "trait_alternatives": []}]


def test_or_qualifier_is_one_slot_that_matches_either_side():
    # P-240 Arcturusmon. Without OR support the legacy fallback read the whole
    # qualifier as a name group: ONE material named Gammamon or "VB", levels lost.
    c = _one(_assembly(6, "Lv.5 × Lv.4 × Lv.3, all w/[Gammamon] in text or w/[VB] trait"))
    assert [e.level_exact for e in c.elements] == [5, 4, 3]
    assert all(e.any_of == _GAMMAMON_OR_VB for e in c.elements)
    assert all((e.name_contains, e.text_any, e.trait_match) == ("", [], "") for e in c.elements)
    assert c.max_materials == 3 and c.reduce_cost_per_card == 6


def test_different_level_cards_with_an_or_qualifier():
    # BT26-085 Giant Slayer.
    c = _one(_assembly(5, "5 different-level cards w/[Chronomon]\xa0in text or w/[Shaman]\xa0trait"))
    assert c.different_levels and not c.different_names
    e = c.elements[0]
    assert (e.count, e.is_digimon_only) == (5, False)
    assert e.any_of == [{"text_any": ["Chronomon"]}, {"trait_match": "Shaman", "trait_alternatives": []}]


def test_uncounted_level_capped_card_is_one_material():
    # BT26-073 Aegiochusmon: Dark prints no count: one material.
    c = _one(_assembly(2, "Lv.4 or lower Digimon card w/[Chronomon] in text or w/[TS] trait"))
    e = c.elements[0]
    assert (e.count, e.level_max, e.is_digimon_only) == (1, 4, True)
    assert e.any_of == [{"text_any": ["Chronomon"]}, {"trait_match": "TS", "trait_alternatives": []}]
    assert c.max_materials == 1


def test_or_fields_serialize_only_when_set():
    out = ic._parse_xros_costs(_assembly(6, "Lv.5 × Lv.4 × Lv.3, all w/[Gammamon] in text or w/[VB] trait"))[1][0]
    assert out["elements"][0]["any_of"] == _GAMMAMON_OR_VB
    assert "different_levels" not in out
    dl = ic._parse_xros_costs(_assembly(5, "5 different-level cards w/[Chronomon] in text or w/[Shaman] trait"))[1][0]
    assert dl["different_levels"] is True
    plain = ic._parse_xros_costs(_assembly(5, "Lv.5 × Lv.4 × Lv.3, all w/[Witchelny] in text"))[1][0]
    assert "any_of" not in plain["elements"][0]


def test_overridden_xros_req_rederives_parsed_costs():
    cards = {"X-1": {"xros_req": "[Digivolve] [Omnimon]: Cost 2"}}
    patch = {"xros_req": "[Digivolve] [Omnimon]: Cost 2 \r\n"
                         + _assembly(8, "6 [ADVENTURE] trait Digimon cards w/different colors")}
    ic.apply_overrides(cards, overrides={"X-1": patch})
    assert cards["X-1"]["digixros_costs"][0]["different_colors"] is True
