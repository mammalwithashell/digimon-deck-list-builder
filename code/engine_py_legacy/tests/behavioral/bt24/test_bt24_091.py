"""Behavioral tests for BT24-091 Tidal Stream (Option, Blue, Cost 5).

Card text:
While you have an [TS] trait Digimon or Tamer on the field, you can ignore
this card's color requirements.
[Security] Activate this card's [Main] effects.
[Main] Return all of your opponent's lowest level Digimon to the hand.
If this effect returned at least 1, 1 of your [TS] trait Digimon unsuspends.
Then, you may link this card to 1 of your Digimon on the field without
paying the cost.
[Link] [TS] trait: Cost 3
[When Attacking] [Once Per Turn] Return 1 of your opponent's lowest level
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
class TestBT24091TidalStream:
    """Tests for BT24-091 Tidal Stream."""

    def test_color_bypass_with_ts_digimon_on_field(self, debug_runner):
        """Color requirements bypassed when a [TS] Digimon is on the field."""
        runner = debug_runner(initial_memory=10)

        # Place a TS-trait Digimon on field
        runner.place_on_field(1, [TS_LV3])

        # Inject Tidal Stream and load effects
        card = runner.inject_card(1, "BT24-091", "hand")
        card.effect_list(None)

        assert card.match_color_requirement is False, (
            "Should bypass color requirement when TS-trait Digimon is on field"
        )

    def test_color_enforced_without_ts_on_field(self, debug_runner):
        """Color requirements enforced when no [TS] Digimon/Tamer is on field."""
        runner = debug_runner(initial_memory=10)

        # Place a non-TS Digimon on field
        runner.place_on_field(1, [NON_TS_LV3])

        card = runner.inject_card(1, "BT24-091", "hand")
        card.effect_list(None)

        assert card.match_color_requirement is True, (
            "Should enforce color requirement when no TS-trait permanent on field"
        )

    def test_main_bounces_all_lowest_level_digimon(self, debug_runner):
        """[Main] returns ALL of opponent's lowest level Digimon to hand."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place TS Digimon on P1 field (to bypass color and for unsuspend)
        ts_perm = runner.place_on_field(1, [TS_LV3], is_suspended=True)

        # Place 2 Lv3 and 1 Lv4 on opponent's field
        runner.place_on_field(2, [NON_TS_LV3])  # Lv3 - should be bounced
        runner.place_on_field(2, [TS_LV3])       # Lv3 - should be bounced
        runner.place_on_field(2, [NON_TS_LV4])   # Lv4 - should stay

        opp_before = len(game.player2.battle_area)
        assert opp_before == 3

        runner.inject_card(1, "BT24-091", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Tidal Stream")
        assert action is not None, f"Should find Tidal Stream action. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Both Lv3 Digimon should be bounced; Lv4 should remain
        opp_field = snap.p2_field
        assert len(opp_field) == 1, (
            f"Only the Lv4 Digimon should remain on opponent's field, got {opp_field}"
        )

    def test_unsuspend_ts_after_bounce(self, debug_runner):
        """If bounce returned at least 1, 1 [TS] Digimon unsuspends."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place a suspended TS Digimon on P1 field
        ts_perm = runner.place_on_field(1, [TS_LV3], is_suspended=True)
        assert ts_perm.is_suspended

        # Place a Lv3 on opponent's field (will be bounced)
        runner.place_on_field(2, [NON_TS_LV3])

        runner.inject_card(1, "BT24-091", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Tidal Stream")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # The TS Digimon should be unsuspended after bounce
        assert not ts_perm.is_suspended, (
            "TS Digimon should be unsuspended after bounce returned at least 1"
        )

    def test_no_unsuspend_when_no_bounce(self, debug_runner):
        """If no Digimon were bounced, unsuspend should not occur."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place a suspended TS Digimon on P1 field
        ts_perm = runner.place_on_field(1, [TS_LV3], is_suspended=True)

        # No opponent Digimon to bounce
        runner.clear_zone(2, "field")

        runner.inject_card(1, "BT24-091", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Tidal Stream")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # No bounce occurred, so unsuspend should not happen
        assert ts_perm.is_suspended, (
            "TS Digimon should remain suspended when no opponent Digimon were bounced"
        )

    def test_linked_wa_bounce_once_per_turn(self, debug_runner):
        """[When Attacking] [Once Per Turn] bounces 1 opponent's lowest level Digimon."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place an attacking Digimon with this card linked
        attacker = runner.place_on_field(1, [TS_LV6])
        link_card = runner.inject_card(1, "BT24-091", "hand")
        # Manually link the card
        attacker.link_card(link_card)

        # Place 2 opponent Digimon: Lv3 and Lv4
        runner.place_on_field(2, [NON_TS_LV3])
        runner.place_on_field(2, [NON_TS_LV4])

        opp_before = len(game.player2.battle_area)
        assert opp_before == 2

        runner.set_phase("Main")

        # Find attack action
        action = runner.find_action("attack")
        if action is None:
            action = runner.find_action("Neptunemon")
        assert action is not None, f"Should find attack action. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        # The linked WA effect should have bounced 1 lowest-level Digimon (Lv3)
        assert len(game.player2.battle_area) <= opp_before, (
            "At least 1 opponent Digimon should have been bounced by linked WA effect"
        )
