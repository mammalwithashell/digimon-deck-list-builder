"""Behavioral tests for BT1-090 Gravity Crush (Option, Red, Cost 0).

Card text (from cards.json):
  [Main] Gain 2 memory. At end of turn, lose 2 memory.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT1090GravityCrush:
    """Tests for BT1-090 Gravity Crush."""

    def test_main_gains_2_memory(self, debug_runner):
        """Playing Gravity Crush should gain 2 memory."""
        runner = debug_runner(initial_memory=3)
        runner.set_phase("Main")
        # Options require a matching-color Digimon/Tamer on field
        runner.place_on_field(1, ["ST1-03"])  # Red Agumon
        runner.inject_card(1, "BT1-090", "hand")

        memory_before = runner.game.memory
        action = runner.find_action("Gravity Crush")
        assert action is not None, "Should be able to play Gravity Crush"
        runner.execute(action)
        runner.auto_resolve()

        # Memory should increase by 2 (minus cost 0 = net +2)
        assert runner.game.memory >= memory_before + 2, (
            f"Memory should increase by at least 2, was {memory_before}, now {runner.game.memory}"
        )

    def test_card_goes_to_trash_after_main(self, debug_runner):
        """Option should go to trash after resolving main effect."""
        runner = debug_runner(initial_memory=3)
        runner.set_phase("Main")
        runner.place_on_field(1, ["ST1-03"])
        runner.inject_card(1, "BT1-090", "hand")

        action = runner.find_action("Gravity Crush")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert "BT1-090" in snap.p1_trash, "Gravity Crush should be in trash after resolution"
        assert "BT1-090" not in snap.p1_hand, "Gravity Crush should not be in hand"

    def test_end_of_turn_loses_2_memory(self, debug_runner):
        """At end of turn after playing Gravity Crush, should lose 2 memory."""
        runner = debug_runner(initial_memory=3)
        runner.set_phase("Main")
        runner.place_on_field(1, ["ST1-03"])
        runner.inject_card(1, "BT1-090", "hand")

        action = runner.find_action("Gravity Crush")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        memory_after_play = runner.game.memory

        # Trigger end-of-turn effects manually
        runner.game.execute_effects(EffectTiming.OnEndTurn)

        memory_after_eot = runner.game.memory
        assert memory_after_eot == memory_after_play - 2, (
            f"Should lose 2 memory at end of turn. After play: {memory_after_play}, "
            f"after EOT: {memory_after_eot}"
        )

    def test_end_of_turn_effect_only_fires_once(self, debug_runner):
        """The -2 memory end-of-turn effect should only fire once (one-shot)."""
        runner = debug_runner(initial_memory=3)
        runner.set_phase("Main")
        runner.place_on_field(1, ["ST1-03"])
        runner.inject_card(1, "BT1-090", "hand")

        action = runner.find_action("Gravity Crush")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        memory_after_play = runner.game.memory

        # First end-of-turn
        runner.game.execute_effects(EffectTiming.OnEndTurn)
        memory_after_first = runner.game.memory
        assert memory_after_first == memory_after_play - 2

        # Second end-of-turn should NOT fire again
        runner.game.execute_effects(EffectTiming.OnEndTurn)
        memory_after_second = runner.game.memory
        assert memory_after_second == memory_after_first, (
            "End-of-turn memory loss should only fire once, not repeatedly"
        )

    def test_has_option_skill_timing(self, debug_runner):
        """Script should register an OptionSkill effect for the Main ability."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT1-090")
        effects = cs.effect_list(None)
        option_skill = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(option_skill) >= 1, "Should have at least 1 OptionSkill effect"

    def test_has_end_of_turn_timing(self, debug_runner):
        """Script should register an OnEndTurn effect for the memory loss."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT1-090")
        effects = cs.effect_list(None)
        eot = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(eot) >= 1, "Should have at least 1 OnEndTurn effect"
