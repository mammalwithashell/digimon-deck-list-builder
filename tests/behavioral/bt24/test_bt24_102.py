"""Behavioral tests for BT24-102 Homeros (Tamer).

Card text:
[Start of Your Main Phase] Gain 1 memory. Then, if you have 5 or more memory,
suspend this Tamer and <Draw 1>.
[All Turns] All of your [TS] trait Digimon get +1000 DP.
[End of Your Turn] By suspending this Tamer, you may activate 1 [On Play] or
[When Digivolving] effect of 1 of your [Olympos XII] trait Digimon.
[Security] Play this card without paying the cost.
"""

import pytest


@pytest.mark.behavioral
class TestBT24102Homeros:
    """Tests for BT24-102 Homeros."""

    def test_play_homeros_on_field(self, debug_runner):
        """Play Homeros from hand and verify it appears on field as a tamer."""
        runner = debug_runner(archetype1="TS Jupitermon", initial_memory=10)
        runner.inject_card(1, "BT24-102", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Homeros")
        assert action is not None, "Should find play action for Homeros"

        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        tamer_on_field = any(
            s.card_id == "BT24-102" and s.is_tamer
            for s in snap.p1_field
        )
        assert tamer_on_field, "Homeros should be on field as a tamer"

    def test_dp_boost_ts_digimon(self, debug_runner):
        """[All Turns] TS trait Digimon get +1000 DP when Homeros is on field."""
        runner = debug_runner(archetype1="TS Jupitermon", initial_memory=10)

        # Place Homeros as a tamer on field
        runner.place_on_field(1, ["BT24-102"])

        # Place a TS Digimon (Shamanmon, base DP 1000, has [TS] trait)
        runner.place_on_field(1, ["BT24-009"])

        snap = runner.snapshot()

        # Find the Shamanmon on field
        shamanmon = None
        for s in snap.p1_field:
            if s.card_id == "BT24-009":
                shamanmon = s
                break

        assert shamanmon is not None, "Shamanmon should be on field"
        # Base DP 1000 + 1000 from Homeros aura = 2000
        assert shamanmon.dp == 2000, (
            f"Shamanmon should have 2000 DP (1000 base + 1000 Homeros aura), got {shamanmon.dp}"
        )

    def test_dp_boost_not_applied_to_non_ts(self, debug_runner):
        """[All Turns] Non-TS Digimon should NOT get the +1000 DP boost."""
        runner = debug_runner(archetype1="TS Jupitermon", initial_memory=10)

        # Place Homeros on field
        runner.place_on_field(1, ["BT24-102"])

        # Place a non-TS Digimon (ST1-03 Agumon, no TS trait, base DP 2000)
        runner.place_on_field(1, ["ST1-03"])

        snap = runner.snapshot()

        agumon = None
        for s in snap.p1_field:
            if s.card_id == "ST1-03":
                agumon = s
                break

        assert agumon is not None, "Agumon should be on field"
        # Should NOT have TS boost — base DP only
        assert agumon.dp == 2000, (
            f"Non-TS Agumon should have base 2000 DP (no Homeros boost), got {agumon.dp}"
        )

    def test_dp_boost_not_applied_to_opponent_ts(self, debug_runner):
        """[All Turns] Opponent's TS Digimon should NOT get the boost."""
        runner = debug_runner(archetype1="TS Jupitermon", initial_memory=10)

        # Place Homeros on P1's field
        runner.place_on_field(1, ["BT24-102"])

        # Place a TS Digimon on P2's field (opponent)
        runner.place_on_field(2, ["BT24-009"])

        snap = runner.snapshot()

        opp_shamanmon = None
        for s in snap.p2_field:
            if s.card_id == "BT24-009":
                opp_shamanmon = s
                break

        assert opp_shamanmon is not None, "Opponent's Shamanmon should be on field"
        # Should NOT have boost — base DP 1000 only
        assert opp_shamanmon.dp == 1000, (
            f"Opponent's TS Digimon should not get Homeros boost, got {opp_shamanmon.dp}"
        )

    def test_olympos_xii_trait_checks_top_card_only(self, debug_runner):
        """[End of Turn] Olympos XII trait filter should only check the top card.

        A Digimon with Olympos XII on a digivolution card but NOT on the top
        card should not be eligible for the reactivation effect.
        """
        runner = debug_runner(archetype1="TS Jupitermon", initial_memory=10)

        # Place Homeros on field (unsuspended)
        runner.place_on_field(1, ["BT24-102"])

        # Place a Digimon with Olympos XII under a non-Olympos XII top card.
        # BT10-042 Venusmon (Olympos XII) under BT24-010 Greymon (TS but NOT Olympos XII)
        perm = runner.place_on_field(1, ["BT10-042", "BT24-010"])

        # Verify top card is Greymon (no Olympos XII trait)
        assert perm.top_card.c_entity_base.card_id == "BT24-010"
        assert not perm.has_trait("Olympos XII"), (
            "Greymon on top should not have Olympos XII trait"
        )

        # The condition2 for the End of Turn effect checks for Olympos XII Digimon.
        # With our fix, this Digimon should NOT qualify because Olympos XII is only
        # on the digivolution card, not the top card.
        # We verify by directly checking the script's condition logic via the game.
        game = runner.game
        player = game.player1
        homeros_card = None
        for p in player.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT24-102":
                homeros_card = p.top_card
                break

        assert homeros_card is not None
        effects = homeros_card.effect_list(None)
        # Find the End of Turn effect (effect2)
        eot_effect = None
        for eff in effects:
            from digimon_gym.engine.data.enums import EffectTiming
            if eff.timing == EffectTiming.OnEndTurn:
                eot_effect = eff
                break

        assert eot_effect is not None, "Should have End of Turn effect"
        # The condition should return False because no eligible Olympos XII Digimon
        # (the only one has Olympos XII buried under a non-Olympos top card)
        result = eot_effect.can_use_condition({})
        assert result is False, (
            "End of Turn condition should be False when no top-card Olympos XII Digimon exists"
        )

    def test_olympos_xii_top_card_trait_accepted(self, debug_runner):
        """[End of Turn] Digimon with Olympos XII on top card SHOULD be eligible."""
        runner = debug_runner(archetype1="TS Jupitermon", initial_memory=10)

        # Place Homeros on field (unsuspended)
        runner.place_on_field(1, ["BT24-102"])

        # Place an Olympos XII Digimon with trait on top card
        # BT24-101 Jupitermon has Olympos XII trait
        runner.place_on_field(1, ["BT24-101"])

        game = runner.game
        player = game.player1
        homeros_card = None
        for p in player.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT24-102":
                homeros_card = p.top_card
                break

        assert homeros_card is not None
        effects = homeros_card.effect_list(None)
        eot_effect = None
        for eff in effects:
            from digimon_gym.engine.data.enums import EffectTiming
            if eff.timing == EffectTiming.OnEndTurn:
                eot_effect = eff
                break

        assert eot_effect is not None
        # Condition should return True — Jupitermon has Olympos XII on top card
        result = eot_effect.can_use_condition({})
        assert result is True, (
            "End of Turn condition should be True when an Olympos XII Digimon is on field"
        )

    def test_play_cost(self, debug_runner):
        """Homeros costs 5 memory to play."""
        runner = debug_runner(archetype1="TS Jupitermon", initial_memory=10)
        runner.inject_card(1, "BT24-102", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Homeros")
        assert action is not None, "Should be able to play Homeros"

        result = runner.execute(action)

        # Play cost is 5, so memory should decrease by 5
        assert result.after.memory == result.before.memory - 5, (
            f"Memory should decrease by 5 (play cost), "
            f"was {result.before.memory}, now {result.after.memory}"
        )
