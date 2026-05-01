"""Tests for `tools/ingest_cards.py` — tri-color ingest + override merging."""
from data_paths import CARDS_JSON
from tools.ingest_cards import (
    COLOR_MAP,
    apply_overrides,
    convert_card,
    parse_evo_costs,
)


def _fake_api_card(**overrides):
    """Minimal API-shaped dict with everything `convert_card` reads."""
    base = {
        "id": "TST-001",
        "name": "Test",
        "type": "Digimon",
        "rarity": "C",
        "play_cost": 3,
        "level": 5,
        "dp": 5000,
        "color": "Red",
    }
    base.update(overrides)
    return base


class TestConvertCardTriColor:
    def test_third_color_ingested(self):
        """`color3` joins `color` / `color2` in `card_colors`."""
        card = convert_card(_fake_api_card(
            color="Yellow", color2="Blue", color3="White",
        ))
        assert card["card_colors"] == [
            COLOR_MAP["Yellow"], COLOR_MAP["Blue"], COLOR_MAP["White"],
        ]

    def test_missing_color3_falls_back_to_two(self):
        """Cards without `color3` stay dual-color — no regression."""
        card = convert_card(_fake_api_card(color="Yellow", color2="Blue"))
        assert card["card_colors"] == [COLOR_MAP["Yellow"], COLOR_MAP["Blue"]]

    def test_duplicate_colors_deduped(self):
        """Same color across slots doesn't appear twice."""
        card = convert_card(_fake_api_card(
            color="Red", color2="Red", color3="Blue",
        ))
        assert card["card_colors"] == [COLOR_MAP["Red"], COLOR_MAP["Blue"]]

    def test_digimoncard_io_dual_row_maps_to_dual_payload(self):
        row = {
            "id": "ST24-07",
            "name": "ShineGreymon",
            "type": "Dual",
            "level": 6,
            "play_cost": 5,
            "evolution_cost": 4,
            "color": "Yellow",
            "color2": "Red",
            "digi_type": "Light Dragon",
            "digi_type2": "DATA SQUAD",
            "dp": 12000,
            "main_effect": "[When Digivolving] Do the Digimon side.",
            "source_effect": "Use Requirement: DATA SQUAD trait\n[Main] Do the Option side.",
            "alt_effect": "[Digivolve] Lv.5 w/[RizeGreymon] in name or w/[DATA SQUAD] trait: Cost 3",
        }

        card = convert_card(row)

        assert card["card_kind"] == 4
        assert card["play_cost"] == 5
        assert card["level"] == 6
        assert card["dp"] == 12000
        assert card["effect_description_eng"] == "[When Digivolving] Do the Digimon side."
        assert card["inherited_effect_description_eng"] == ""
        assert card["dual"]["option"]["use_cost"] == 5
        assert card["dual"]["option"]["effect_text"].startswith("Use Requirement")
        assert card["dual"]["digimon"]["effect_text"].startswith("[When Digivolving]")
        assert card["dual"]["option"]["colors"] == ["Yellow"]
        assert card["dual"]["digimon"]["colors"] == ["Yellow", "Red"]


class TestParseEvoCostsThirdPath:
    def test_three_evo_costs_emitted(self):
        """`evolution_cost_3` produces a third entry in `evo_costs`."""
        costs = parse_evo_costs(_fake_api_card(
            level=6,
            evolution_cost=3, evolution_color="Red", evolution_level=5,
            evolution_cost_2=3, evolution_color_2="Blue", evolution_level_2=5,
            evolution_cost_3=3, evolution_color_3="Yellow", evolution_level_3=5,
        ))
        assert len(costs) == 3
        assert [c["card_color"] for c in costs] == [
            COLOR_MAP["Red"], COLOR_MAP["Blue"], COLOR_MAP["Yellow"],
        ]

    def test_third_cost_infers_color_from_color3(self):
        """Missing `evolution_color_3` falls back to `color3`."""
        costs = parse_evo_costs(_fake_api_card(
            level=6,
            color="Red", color2="Blue", color3="Yellow",
            evolution_cost=3, evolution_color="Red", evolution_level=5,
            evolution_cost_3=3,  # no color_3 / level_3 — should infer
        ))
        # Two costs: primary (Red) + inferred tertiary (Yellow, level=5).
        assert len(costs) == 2
        assert costs[1]["card_color"] == COLOR_MAP["Yellow"]
        assert costs[1]["level"] == 5

    def test_no_evolution_cost_on_low_level_card(self):
        """Level 1/2 cards never emit evo_costs (printed-rule invariant)."""
        costs = parse_evo_costs(_fake_api_card(
            level=2,
            evolution_cost=3, evolution_color="Red", evolution_level=1,
        ))
        assert costs == []


class TestApplyOverrides:
    def test_override_replaces_card_colors(self):
        cards = {
            "BT20-060": {"card_id": "BT20-060", "card_colors": [5, 2]},
        }
        overrides = {"BT20-060": {"card_colors": [5, 2, 0]}}
        applied = apply_overrides(cards, overrides)
        assert applied == 1
        assert cards["BT20-060"]["card_colors"] == [5, 2, 0]

    def test_override_leaves_other_fields_intact(self):
        cards = {
            "BT20-060": {
                "card_id": "BT20-060",
                "card_colors": [5, 2],
                "play_cost": 6,
                "dp": 12000,
            },
        }
        overrides = {"BT20-060": {"card_colors": [5, 2, 0]}}
        apply_overrides(cards, overrides)
        assert cards["BT20-060"]["play_cost"] == 6
        assert cards["BT20-060"]["dp"] == 12000

    def test_meta_keys_skipped(self):
        """`_comment` / other `_`-prefixed keys aren't treated as card ids."""
        cards = {"BT20-060": {"card_id": "BT20-060", "card_colors": [5]}}
        overrides = {
            "_comment": "explanation",
            "_version": 1,
            "BT20-060": {"card_colors": [5, 2, 0]},
        }
        applied = apply_overrides(cards, overrides)
        assert applied == 1
        assert cards["BT20-060"]["card_colors"] == [5, 2, 0]

    def test_unknown_card_id_skipped_with_warning(self, capsys):
        cards = {"BT1-001": {"card_id": "BT1-001", "card_colors": [0]}}
        overrides = {"DOES-NOT-EXIST": {"card_colors": [0]}}
        applied = apply_overrides(cards, overrides)
        assert applied == 0
        assert "DOES-NOT-EXIST" in capsys.readouterr().out


class TestProductionCardsJsonTriColor:
    """Integration: the two seeded tri-color overrides are live in cards.json."""

    def test_magnamon_x_antibody_tricolor(self):
        import json as _json
        d = _json.load(open(CARDS_JSON, encoding="utf-8"))
        cards = d if isinstance(d, list) else list(d.values())
        c = next(x for x in cards if x.get("card_id") == "BT16-102")
        assert set(c["card_colors"]) == {
            COLOR_MAP["Yellow"], COLOR_MAP["Blue"], COLOR_MAP["White"],
        }

    def test_alphamon_ouryuken_tricolor(self):
        import json as _json
        d = _json.load(open(CARDS_JSON, encoding="utf-8"))
        cards = d if isinstance(d, list) else list(d.values())
        c = next(x for x in cards if x.get("card_id") == "BT20-060")
        assert set(c["card_colors"]) == {
            COLOR_MAP["Black"], COLOR_MAP["Yellow"], COLOR_MAP["Red"],
        }
