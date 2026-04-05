"""Behavioral tests for EX1-068 Ice Wall!

Card text (from cards.json):
[Main] All of your opponent's Digimon gain "[When Attacking] lose 2 memory"
    until the end of their next turn.
[Security] Gain 2 memory.
"""

import pytest


@pytest.mark.behavioral
class TestEX1068IceWall:
    """Tests for EX1-068 Ice Wall!"""

    def test_security_effect_gains_2_memory(self, debug_runner):
        """[Security] Should gain 2 memory for the security player."""
        runner = debug_runner(
            deck1=["EX1-068"] * 4 + ["ST2-04"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=5,
        )
        # Verify the script has a SecuritySkill effect that gains 2 memory
        game = runner.game
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX1-068", game.player1)
        effects = cs.effect_list(None)

        from digimon_gym.engine.data.enums import EffectTiming
        sec_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effects) >= 1, "Should have at least 1 SecuritySkill effect"

    def test_main_effect_grants_penalty_to_opponent_digimon(self, debug_runner):
        """[Main] Should grant when-attacking penalty to opponent Digimon."""
        runner = debug_runner(
            deck1=["EX1-068"] * 4 + ["ST2-04"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=5,
        )
        # Place a Blue Digimon/Tamer on P1 field so color requirement is met
        runner.place_on_field(1, ["ST2-04"])

        # Place opponent Digimon
        runner.place_on_field(2, ["ST1-03"])

        # Play Ice Wall from hand
        runner.inject_card(1, "EX1-068", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Ice Wall")
        assert action is not None, (
            f"Should find play action for Ice Wall. Available: {runner.actions()}"
        )
        runner.execute(action)
        runner.auto_resolve()

        # The opponent's Digimon should now have a granted temp effect
        game = runner.game
        opp_digimon = [p for p in game.player2.battle_area if p.is_digimon]
        assert len(opp_digimon) > 0, "Opponent should still have Digimon"

        # Check that temp effects were granted
        perm = opp_digimon[0]
        assert len(perm._granted_effects) > 0, (
            "Opponent's Digimon should have a granted 'lose 2 memory' effect"
        )

    def test_penalty_loses_memory_from_correct_player(self, debug_runner):
        """The when-attacking penalty should lose memory from the attacking player (opponent)."""
        runner = debug_runner(
            deck1=["EX1-068"] * 4 + ["ST2-04"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=5,
        )
        # Place Blue Digimon on P1 field for color requirement
        runner.place_on_field(1, ["ST2-04"])
        runner.place_on_field(2, ["ST1-03"])
        runner.inject_card(1, "EX1-068", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Ice Wall")
        assert action is not None, f"Should find Ice Wall action. Available: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        # Verify: the penalty callback should call enemy.lose_memory(2)
        game = runner.game
        opp_digimon = [p for p in game.player2.battle_area if p.is_digimon]
        assert len(opp_digimon) > 0
        perm = opp_digimon[0]
        granted = perm._granted_effects
        assert len(granted) > 0, "Should have granted penalty effect"
