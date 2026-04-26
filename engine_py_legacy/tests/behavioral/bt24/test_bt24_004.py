"""Behavioral tests for BT24-004 Wanyamon (Lv.2 Digitama).

Card text:
Inherited Effect [Your Turn] [Once Per Turn] When any of your [Iliad] trait
Digimon are played, <Draw 1>.
"""

import pytest


@pytest.mark.behavioral
class TestBT24004Wanyamon:
    """Tests for BT24-004 Wanyamon."""

    def test_inherited_draw_on_iliad_play(self, debug_runner):
        """Inherited: playing an Iliad Digimon should trigger Draw 1."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a Digimon with Wanyamon inherited (Lv.3 Kamemon on Lv.2 Wanyamon)
        runner.place_on_field(1, ["BT24-004", "BT24-019"])

        # Put an Iliad Digimon in hand to play
        runner.inject_card(1, "BT24-019", "hand")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Play Kamemon")
        assert action is not None, "Should be able to play Kamemon from hand"

        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Kamemon was removed from hand (played) and Draw 1 should add 1
        # Net: hand_before - 1 (played) + 1 (draw) = hand_before
        assert len(snap_after.p1_hand) == hand_before, (
            f"Should draw 1 after playing Iliad Digimon; "
            f"hand went from {hand_before} to {len(snap_after.p1_hand)} "
            f"(expected {hand_before}: -1 played +1 drawn)"
        )

    def test_inherited_no_draw_on_non_iliad_play(self, debug_runner):
        """Inherited: playing a non-Iliad Digimon should NOT trigger Draw 1."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a Digimon with Wanyamon inherited
        runner.place_on_field(1, ["BT24-004", "BT24-019"])

        # Put a non-Iliad Digimon in hand
        runner.inject_card(1, "ST1-03", "hand")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Play Agumon")
        assert action is not None, "Should be able to play Agumon from hand"

        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Only lost 1 card from hand (played), no draw
        assert len(snap_after.p1_hand) == hand_before - 1, (
            f"Should NOT draw when non-Iliad Digimon is played; "
            f"hand went from {hand_before} to {len(snap_after.p1_hand)}"
        )

    def test_inherited_no_draw_on_opponent_turn(self, debug_runner):
        """Inherited: should NOT trigger on opponent's turn."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a Digimon with Wanyamon inherited on P1's field
        runner.place_on_field(1, ["BT24-004", "BT24-019"])

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        # Switch to P2's turn and have P2 play an Iliad Digimon
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True
        runner.game.turn_player = runner.game.player2
        runner.game.opponent_player = runner.game.player1

        runner.inject_card(2, "BT24-019", "hand")
        action = runner.find_action("Play Kamemon")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

        snap_after = runner.snapshot()
        assert len(snap_after.p1_hand) == hand_before, (
            "Should NOT draw on opponent's turn"
        )

    def test_inherited_once_per_turn(self, debug_runner):
        """Inherited: should only trigger once per turn."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a Digimon with Wanyamon inherited
        runner.place_on_field(1, ["BT24-004", "BT24-019"])

        # Put two Iliad Digimon in hand
        runner.inject_card(1, "BT24-019", "hand")
        runner.inject_card(1, "BT24-019", "hand")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        # Play first Iliad Digimon
        action = runner.find_action("Play Kamemon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Play second Iliad Digimon
        action2 = runner.find_action("Play Kamemon")
        if action2 is not None:
            runner.execute(action2)
            runner.auto_resolve()

        snap_after = runner.snapshot()
        # Started with hand_before, played 2 (-2), drew 1 (once per turn) = hand_before - 1
        assert len(snap_after.p1_hand) == hand_before - 1, (
            f"Should only draw once per turn; "
            f"hand went from {hand_before} to {len(snap_after.p1_hand)} "
            f"(expected {hand_before - 1}: -2 played +1 drawn)"
        )
