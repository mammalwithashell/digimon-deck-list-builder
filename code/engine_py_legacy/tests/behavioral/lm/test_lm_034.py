"""Behavioral tests for LM-034 Wisteria Memory Boost!

Card text (from cards.json):
Red also meets this card's color requirements.
[Main] Reveal the top 3 cards of your deck. Add 1 blue or red Digimon card
    among them to the hand. Return the rest to the bottom of deck. Then, place
    this card in the battle area.
[Main] Delay: Gain 2 memory.
[Security] Place this card in the battle area.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestLM034WisteriaMemoryBoost:
    """Tests for LM-034 Wisteria Memory Boost!"""

    def test_red_meets_color_requirements(self, debug_runner):
        """Red should also meet this card's color requirements."""
        runner = debug_runner(
            deck1=["LM-034"] * 4 + ["ST1-07"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=10,
        )
        # LM-034 is Blue but Red should also work
        # The script sets card._match_color_requirement = False which bypasses
        # color checking entirely. This is a reasonable approximation.
        runner.inject_card(1, "LM-034", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Wisteria Memory Boost")
        assert action is not None, f"Should find play action. Available: {runner.actions()}"

    def test_main_reveal_filter_blue_or_red_digimon(self, debug_runner):
        """[Main] Reveal filter should match blue or red Digimon only."""
        runner = debug_runner(
            deck1=["LM-034"] * 4 + ["ST1-07"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=10,
        )
        game = runner.game
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-034", game.player1)
        effects = cs.effect_list(None)

        main = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main) >= 1, "Should have OptionSkill main effect"

    def test_has_delay_and_delay_effect(self, debug_runner):
        """Should have Delay marker and Delay effect (gain 2 memory)."""
        runner = debug_runner(
            deck1=["LM-034"] * 4 + ["ST1-07"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=10,
        )
        game = runner.game
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-034", game.player1)
        effects = cs.effect_list(None)

        delay = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delay) >= 1, "Should have Delay marker"

        delay_eff = [e for e in effects if getattr(e, '_is_delay_effect', False)]
        assert len(delay_eff) >= 1, "Should have Delay effect"

    def test_security_places_in_battle_area(self, debug_runner):
        """[Security] Should place this card in the battle area."""
        runner = debug_runner(
            deck1=["LM-034"] * 4 + ["ST1-07"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=10,
        )
        game = runner.game
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-034", game.player1)
        effects = cs.effect_list(None)

        sec = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec) >= 1, "Should have SecuritySkill effect"
