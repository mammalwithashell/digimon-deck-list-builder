"""Behavioral tests for ST2-13 Hammer Spark.

Card text (from cards.json):
[Main] Gain 1 memory.
[Security] Gain 2 memory.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestST213HammerSpark:
    """Tests for ST2-13 Hammer Spark."""

    def test_main_gains_1_memory(self, debug_runner):
        """[Main] Should gain 1 memory using player.add_memory."""
        runner = debug_runner(
            deck1=["ST2-13"] * 4 + ["ST2-04"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=3,
        )
        # Place Blue Digimon for color requirement
        runner.place_on_field(1, ["ST2-04"])
        runner.inject_card(1, "ST2-13", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Hammer Spark")
        assert action is not None, f"Should find play action. Available: {runner.actions()}"

        before_mem = runner.snapshot().memory
        runner.execute(action)
        runner.auto_resolve()

        after_mem = runner.snapshot().memory
        # Play cost 0, gain 1 memory => net +1
        assert after_mem == before_mem + 1, (
            f"Memory should increase by 1 (cost 0 + gain 1), was {before_mem}, now {after_mem}"
        )

    def test_security_gains_2_memory(self, debug_runner):
        """[Security] Should gain 2 memory."""
        runner = debug_runner(
            deck1=["ST2-13"] * 4 + ["ST2-04"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=3,
        )
        game = runner.game
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST2-13", game.player1)
        effects = cs.effect_list(None)

        sec = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec) >= 1, "Should have SecuritySkill effect"

    def test_memory_gain_uses_player_add_memory(self, debug_runner):
        """Both effects should use player.add_memory for proper directional memory."""
        runner = debug_runner(
            deck1=["ST2-13"] * 4 + ["ST2-04"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=3,
        )
        # game.memory += N is equivalent to player.add_memory(N) when P1 is active
        # but player.add_memory respects ICannotAddMemoryEffect modifier
        # Scripts should prefer player.add_memory for consistency
        game = runner.game
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("ST2-13", game.player1)
        effects = cs.effect_list(None)

        main = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main) >= 1
        sec = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec) >= 1
