"""Behavioral tests for BT24-095 Sonic Shot (Option, Green, Cost 3).

Card text:
While you have an [TS] trait Digimon or Tamer on the field, you can ignore
this card's color requirements.
[Security] Activate this card's [Main] effects.
[Main] Suspend 1 of your opponent's Digimon or Tamers. It can't unsuspend
in their next unsuspend phase. Then, you may link this card to 1 of your
Digimon on the field without paying the cost.
[Link] [TS] trait: Cost 3
[When Attacking] [Once Per Turn] Return 1 of your opponent's suspended
Digimon to the hand.
"""

import pytest


# BT24-031 Elecmon: Yellow, [TS], Lv3
TS_LV3 = "BT24-031"
# BT24-034 Aegiomon: Blue, [TS], Lv4
TS_LV4 = "BT24-034"
# BT24-030 Neptunemon: Blue, [TS], Lv6
TS_LV6 = "BT24-030"
# ST1-03 Agumon: Red, no TS, Lv3
NON_TS_LV3 = "ST1-03"
# ST1-07 Greymon: Red, no TS, Lv4
NON_TS_LV4 = "ST1-07"


@pytest.mark.behavioral
class TestBT24095SonicShot:
    """Tests for BT24-095 Sonic Shot."""

    def test_color_bypass_with_ts_digimon_on_field(self, debug_runner):
        """Color requirements bypassed when a [TS] Digimon is on the field."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, [TS_LV3])
        card = runner.inject_card(1, "BT24-095", "hand")
        card.effect_list(None)

        assert card.match_color_requirement is False, (
            "Should bypass color requirement when TS-trait Digimon is on field"
        )

    def test_color_enforced_without_ts_on_field(self, debug_runner):
        """Color requirements enforced when no [TS] Digimon/Tamer is on field."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, [NON_TS_LV3])
        card = runner.inject_card(1, "BT24-095", "hand")
        card.effect_list(None)

        assert card.match_color_requirement is True, (
            "Should enforce color requirement when no TS-trait permanent on field"
        )

    def test_main_suspends_opponent_digimon(self, debug_runner):
        """[Main] Suspend 1 of opponent's Digimon or Tamers."""
        runner = debug_runner(initial_memory=3)
        game = runner.game

        # Place TS Digimon on P1 (green to match color)
        runner.place_on_field(1, [TS_LV3])
        # Place unsuspended Digimon on opponent's field
        opp_perm = runner.place_on_field(2, [NON_TS_LV3])
        assert not opp_perm.is_suspended

        runner.inject_card(1, "BT24-095", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Sonic Shot")
        assert action is not None, f"Should find Sonic Shot action. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        assert opp_perm.is_suspended, (
            "Opponent's Digimon should be suspended after Sonic Shot"
        )

    def test_linked_wa_bounces_suspended_digimon(self, debug_runner):
        """[When Attacking] bounces 1 opponent's suspended Digimon."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place an attacking TS Digimon with this card linked
        attacker = runner.place_on_field(1, [TS_LV6])
        link_card = runner.inject_card(1, "BT24-095", "hand")
        attacker.link_card(link_card)

        # Place a suspended Digimon on opponent's field
        opp_perm = runner.place_on_field(2, [NON_TS_LV3])
        opp_perm.suspend()
        assert opp_perm.is_suspended

        opp_before = len(game.player2.battle_area)

        runner.set_phase("Main")

        action = runner.find_action("attack")
        if action is None:
            action = runner.find_action("Neptunemon")
        assert action is not None, f"Should find attack action. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        assert len(game.player2.battle_area) < opp_before, (
            "Opponent's suspended Digimon should have been bounced by linked WA effect"
        )

    def test_linked_wa_does_not_bounce_unsuspended(self, debug_runner):
        """[When Attacking] does not target unsuspended Digimon."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        attacker = runner.place_on_field(1, [TS_LV6])
        link_card = runner.inject_card(1, "BT24-095", "hand")
        attacker.link_card(link_card)

        # Place only unsuspended Digimon on opponent's field
        opp_perm = runner.place_on_field(2, [NON_TS_LV3])
        assert not opp_perm.is_suspended

        opp_before = len(game.player2.battle_area)

        runner.set_phase("Main")

        action = runner.find_action("attack")
        if action is None:
            action = runner.find_action("Neptunemon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        assert len(game.player2.battle_area) == opp_before, (
            "Unsuspended Digimon should not be bounced by linked WA effect"
        )
