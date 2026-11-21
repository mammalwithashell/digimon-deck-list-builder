"""The official mirror must keep the rainbow (any-colour) digivolve circle.

world.digimoncard.com writes a rainbow circle as
``<span class='cardColor_all'>Multicolor</span> 4 from Lv.4`` (the Japanese DB writes
the same circle 「全 Lv.4から4」: 全 = all colours). ``parse_evo_costs`` used to keep only
spans whose class named one colour, so the row vanished: ``data/card_official.json`` said
"no standard circles" for Kimeramon (BT8-084), both Ukkomon, the three Susanoomon,
BigUkkomon (BT16-083), Luminamon (Nene Version) (BT19-102), Kimeramon (EX9-074) and
Omnimon: Merciful Mode (EX13-077), and a reconcile against that mirror erased the
EX13-077 circle from cards.json outright.

The markup fixtures are the live pages' own lines (fetched 2026-11-21).
"""

import pytest

from tools import build_card_bundles as bundles
from tools import scrape_official_evo_costs as scraper


def _digivolve_block(body: str, slot: int = 1) -> str:
    return f"""
          <dl class="cardInfoBox col2">
            <dt class="cardInfoTit">Digivolve Cost {slot}</dt>
            <dd class="cardInfoData cardColor cardColorCost">
              {body}            </dd>
          </dl>"""


RAINBOW_LV4 = _digivolve_block("<span class='cardColor_all'>Multicolor</span> 4 from Lv.4")
# Dantemon (BT26-086): an Appmon grade circle, not a level circle.
RAINBOW_GRADE = _digivolve_block("<span class='cardColor_all'>Multicolor</span> 4 from 神")
SPLIT_LV5 = _digivolve_block(
    "<span class='cardColor_green'>Green</span> <span class='cardColor_red'>Red</span> 3 from Lv.5"
)


# Both tools parse the same block: the mirror builder and the standalone circle scraper.
PARSERS = pytest.mark.parametrize("parse", [bundles.parse_evo_costs, scraper.parse_evo_costs])


@PARSERS
def test_a_multicolor_level_circle_is_one_row_per_colour(parse):
    assert parse(RAINBOW_LV4) == [{"card_color": c, "level": 4, "memory_cost": 4} for c in range(7)]


@PARSERS
def test_a_repeated_printing_block_does_not_duplicate_the_rainbow_rows(parse):
    assert len(parse(RAINBOW_LV4 + RAINBOW_LV4)) == 7


@PARSERS
def test_a_multicolor_grade_circle_is_not_a_level_circle(parse):
    # "Multicolor 4 from Stnd./Sup./Ult./神" is a trait gate the evo_costs table cannot
    # express; it must not be invented as level rows.
    assert parse(RAINBOW_GRADE) == []


@PARSERS
def test_a_split_circle_still_lists_exactly_its_printed_colours(parse):
    assert parse(SPLIT_LV5) == [
        {"card_color": 3, "level": 5, "memory_cost": 3},
        {"card_color": 0, "level": 5, "memory_cost": 3},
    ]


def test_the_bundle_prints_a_rainbow_circle_as_one_any_colour_line(tmp_path):
    official = {"card_id": "BT8-084", "digivolve_costs": bundles.parse_evo_costs(RAINBOW_LV4)}
    bundles.write_bundle("BT8-084", official, {"card_name_eng": "Kimeramon", "level": 5}, str(tmp_path))
    text = (tmp_path / "BT8-084.md").read_text(encoding="utf-8")
    assert "  - Any colour (rainbow circle; official DB \"Multicolor\") Lv.4 / cost 4" in text
    # Seven per-colour lines would read like seven separate circles.
    assert "  - Red Lv.4 / cost 4" not in text
    assert "(none" not in text


def test_the_bundle_keeps_per_colour_lines_for_a_split_circle(tmp_path):
    official = {"card_id": "EX8-074", "digivolve_costs": bundles.parse_evo_costs(SPLIT_LV5)}
    bundles.write_bundle("EX8-074", official, {"card_name_eng": "MedievalGallantmon"}, str(tmp_path))
    text = (tmp_path / "EX8-074.md").read_text(encoding="utf-8")
    assert "  - Green Lv.5 / cost 3" in text
    assert "  - Red Lv.5 / cost 3" in text
    assert "Any colour" not in text
