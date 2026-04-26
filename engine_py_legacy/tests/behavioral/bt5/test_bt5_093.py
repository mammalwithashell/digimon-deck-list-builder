"""Behavioral tests for BT5-093 Tai Kamiya & Matt Ishida (Tamer White, Cost 4).

Card text:
[Start of Your Turn] If your opponent has a level 6 or higher Digimon in play,
    gain 2 memory.
[Your Turn] All of your Digimon with [Omnimon] in their names get +2000 DP.
[Security] Play this card without paying the cost.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestBT5093TaiAndMatt:
    """Tests for BT5-093 Tai Kamiya & Matt Ishida."""

    def test_play_on_field(self, debug_runner):
        """Play tamer from hand and verify it appears on field."""
        runner = debug_runner(initial_memory=10)
        runner.inject_card(1, "BT5-093", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Tai")
        assert action is not None, "Should find play action for Tai Kamiya & Matt Ishida"

        result = runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        tamer_on_field = any(
            s.card_id == "BT5-093" and s.is_tamer
            for s in snap.p1_field
        )
        assert tamer_on_field, "Tai & Matt should be on field as a tamer"

    def test_play_cost_is_4(self, debug_runner):
        """Costs 4 memory to play."""
        runner = debug_runner(initial_memory=10)
        runner.inject_card(1, "BT5-093", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Tai")
        assert action is not None
        result = runner.execute(action)
        runner.auto_resolve()

        assert result.after.memory == result.before.memory - 4, (
            f"Should cost 4 to play, was {result.before.memory}, now {result.after.memory}"
        )

    def test_start_turn_gain_2_memory_with_lv6_opponent(self, debug_runner):
        """[Start of Your Turn] Gain 2 memory when opponent has Lv.6+ Digimon."""
        runner = debug_runner(initial_memory=3)

        # Place tamer on field
        runner.place_on_field(1, ["BT5-093"])

        # Place a Lv.6 Digimon on opponent's field
        runner.place_on_field(2, ["AD1-004"])  # WarGreymon Lv.6

        memory_before = runner.game.memory

        # Execute start of turn effects
        runner.game.execute_effects(EffectTiming.OnStartTurn)

        # Memory should increase by 2
        assert runner.game.memory == memory_before + 2, (
            f"Should gain 2 memory when opponent has Lv.6+ Digimon, "
            f"was {memory_before}, now {runner.game.memory}"
        )

    def test_start_turn_no_gain_without_lv6_opponent(self, debug_runner):
        """[Start of Your Turn] No memory gain when opponent lacks Lv.6+ Digimon."""
        runner = debug_runner(initial_memory=3)

        runner.place_on_field(1, ["BT5-093"])
        # Place only Lv.3 on opponent's field
        runner.place_on_field(2, ["ST1-03"])  # Agumon Lv.3

        memory_before = runner.game.memory

        runner.game.execute_effects(EffectTiming.OnStartTurn)

        assert runner.game.memory == memory_before, (
            f"Should NOT gain memory when opponent lacks Lv.6+ Digimon, "
            f"was {memory_before}, now {runner.game.memory}"
        )

    def test_dp_boost_omnimon_plus_2000(self, debug_runner):
        """[Your Turn] Omnimon gets +2000 DP from this tamer's aura."""
        runner = debug_runner(initial_memory=10)

        # Place tamer on field
        runner.place_on_field(1, ["BT5-093"])

        # Place Omnimon on field (BT1-084: base DP 15000)
        runner.place_on_field(1, ["BT1-084"])

        snap = runner.snapshot()
        omnimon = None
        for s in snap.p1_field:
            if s.card_id == "BT1-084":
                omnimon = s
                break

        assert omnimon is not None, "Omnimon should be on field"
        assert omnimon.dp == 17000, (
            f"Omnimon should have 17000 DP (15000 base + 2000 aura), got {omnimon.dp}"
        )

    def test_dp_boost_not_applied_to_non_omnimon(self, debug_runner):
        """[Your Turn] Non-Omnimon Digimon should NOT get +2000 DP boost."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["BT5-093"])
        runner.place_on_field(1, ["ST1-03"])  # Agumon, base 2000 DP

        snap = runner.snapshot()
        agumon = None
        for s in snap.p1_field:
            if s.card_id == "ST1-03":
                agumon = s
                break

        assert agumon is not None
        assert agumon.dp == 2000, (
            f"Non-Omnimon Agumon should have base 2000 DP (no boost), got {agumon.dp}"
        )

    def test_dp_boost_not_applied_to_opponent_omnimon(self, debug_runner):
        """[Your Turn] Opponent's Omnimon should NOT get the boost."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["BT5-093"])
        runner.place_on_field(2, ["BT1-084"])  # Opponent's Omnimon

        snap = runner.snapshot()
        opp_omnimon = None
        for s in snap.p2_field:
            if s.card_id == "BT1-084":
                opp_omnimon = s
                break

        assert opp_omnimon is not None
        assert opp_omnimon.dp == 15000, (
            f"Opponent's Omnimon should not get boost, got {opp_omnimon.dp}"
        )

    def test_dp_boost_applies_to_omnimon_variants(self, debug_runner):
        """[Your Turn] Any Digimon with 'Omnimon' in its name gets the boost."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["BT5-093"])
        # BT5-087 Omnimon Zwart (base DP 15000)
        runner.place_on_field(1, ["BT5-087"])

        snap = runner.snapshot()
        zwart = None
        for s in snap.p1_field:
            if s.card_id == "BT5-087":
                zwart = s
                break

        assert zwart is not None
        assert zwart.dp == 17000, (
            f"Omnimon Zwart should have 17000 DP (15000 + 2000), got {zwart.dp}"
        )

    def test_security_play_free(self, debug_runner):
        """[Security] Should have security play effect."""
        runner = debug_runner(initial_memory=10)

        game = runner.game
        player = game.player1

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT5-093", player)
        effects = cs.effect_list(None)
        security_effect = None
        for eff in effects:
            if eff.timing == EffectTiming.SecuritySkill:
                security_effect = eff
                break

        assert security_effect is not None, "Should have Security effect"
        assert security_effect.is_security_effect is True
