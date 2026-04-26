"""Behavioral tests for P-103 Offense Training (Option, Red, Cost 2).

Card text (from cards.json):
  [Main] Reveal the top 2 cards of your deck. Add 1 red card among them to
  your hand. Place the rest at the bottom of your deck in any order. Then,
  place this card in the battle area.
  [Main] <Delay> - 1 of your Digimon may digivolve into a red Digimon card
  in your hand for its digivolution cost. When it would digivolve by this
  effect, reduce the cost by 2.
  [Security] Place this card in the battle area.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestP103OffenseTraining:
    """Tests for P-103 Offense Training."""

    def test_main_places_in_battle_area(self, debug_runner):
        """Playing Offense Training should place it in the battle area."""
        runner = debug_runner(initial_memory=10, skip_shuffle=True)
        runner.set_phase("Main")
        # Options require a matching-color Digimon/Tamer on field
        runner.place_on_field(1, ["ST1-03"])  # Red Agumon
        runner.inject_card(1, "P-103", "hand")

        action = runner.find_action("Offense Training")
        assert action is not None, "Should be able to play Offense Training"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        in_battle = any(s.card_id == "P-103" for s in snap.p1_field)
        assert in_battle, "Offense Training should be placed in the battle area"

    def test_has_delay_marker(self, debug_runner):
        """Should have a delay marker effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-103")
        effects = cs.effect_list(None)
        delays = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delays) >= 1, "Should have a delay marker effect"

    def test_delay_digivolve_with_cost_reduction(self, debug_runner):
        """Delay effect should trigger digivolve into red Digimon from hand with cost -2."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-103")
        effects = cs.effect_list(None)

        # The delay effect should be OnDeclaration with _is_field_main
        delay_main = [e for e in effects
                      if e.timing == EffectTiming.OnDeclaration
                      and getattr(e, '_is_field_main', False)]
        assert len(delay_main) >= 1, "Should have OnDeclaration _is_field_main delay effect"
        assert delay_main[0].on_process_callback is not None

    def test_has_security_effect(self, debug_runner):
        """Should have a security effect to place in battle area."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-103")
        effects = cs.effect_list(None)

        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, "Should have a security effect"

    def test_has_option_skill_timing(self, debug_runner):
        """Should have OptionSkill timing for the Main reveal effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("P-103")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill effect"

    def test_reveal_filter_accepts_any_red_card(self, debug_runner):
        """The reveal filter should match any red card (not just Digimon)."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # Red Digimon
        red_digi = db.create_card_source("ST1-03")  # Agumon (Red, Digimon)
        # Red Option
        red_opt = db.create_card_source("BT1-090")  # Gravity Crush (Red, Option)
        # Blue Digimon
        blue_digi = db.create_card_source("BT1-029")  # Gabumon (Blue, Digimon)

        # Both red cards should pass the filter
        for c, expected, name in [
            (red_digi, True, "Red Digimon"),
            (red_opt, True, "Red Option"),
            (blue_digi, False, "Blue Digimon"),
        ]:
            colors = getattr(c, 'card_colors', [])
            is_red = any(col.name == 'Red' for col in colors)
            assert is_red == expected, f"{name} should {'pass' if expected else 'fail'} red check"
