"""Behavioral tests for EX4-061 Matt Ishida & Tai Kamiya (Tamer Blue/Red).

Card text:
[Your Turn] When you play a [Gabumon] or [Agumon], you may suspend this Tamer
    to gain 1 memory.
[Your Turn] [Once Per Turn] When one of your Digimon digivolves, if it has
    [Omnimon] in its name, <Draw 2>.
[Security] Play this card without paying the cost.
"""

import pytest


@pytest.mark.behavioral
class TestEX4061MattTai:
    """Tests for EX4-061 Matt Ishida & Tai Kamiya."""

    def test_play_tamer_on_field(self, debug_runner):
        """Play the tamer from hand and verify it appears on field."""
        runner = debug_runner(initial_memory=10)
        runner.inject_card(1, "EX4-061", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Matt")
        assert action is not None, "Should find play action for EX4-061"

        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        tamer_on_field = any(
            s.card_id == "EX4-061" and s.is_tamer
            for s in snap.p1_field
        )
        assert tamer_on_field, "Matt Ishida & Tai Kamiya should be on field"

    def test_suspend_to_gain_memory_on_agumon_play(self, debug_runner):
        """[Your Turn] Playing an [Agumon] should allow suspending this tamer for +1 memory."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place tamer on field (unsuspended)
        runner.place_on_field(1, ["EX4-061"])

        # Put an Agumon in hand
        runner.inject_card(1, "EX4-038", "hand")  # EX4-038 Agumon

        memory_before = runner.game.memory

        action = runner.find_action("Play Agumon")
        assert action is not None, "Should be able to play EX4-038 Agumon"

        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Agumon costs 3 to play, tamer gives +1 memory
        expected_memory = memory_before - 3 + 1
        assert snap_after.memory == expected_memory, (
            f"Memory should be {expected_memory} (play cost 3, +1 from tamer); "
            f"got {snap_after.memory}"
        )

        # Tamer should be suspended
        tamer = None
        for s in snap_after.p1_field:
            if s.card_id == "EX4-061":
                tamer = s
                break
        assert tamer is not None
        assert tamer.is_suspended, "Tamer should be suspended after effect"

    def test_suspend_to_gain_memory_on_gabumon_play(self, debug_runner):
        """[Your Turn] Playing a [Gabumon] should allow suspending this tamer for +1 memory."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["EX4-061"])
        runner.inject_card(1, "EX4-039", "hand")  # EX4-039 Gabumon

        memory_before = runner.game.memory

        action = runner.find_action("Play Gabumon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        expected_memory = memory_before - 3 + 1
        assert snap_after.memory == expected_memory, (
            f"Memory should be {expected_memory}; got {snap_after.memory}"
        )

    def test_no_trigger_when_tamer_suspended(self, debug_runner):
        """[Your Turn] Should NOT trigger if tamer is already suspended."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place tamer already suspended
        runner.place_on_field(1, ["EX4-061"], is_suspended=True)
        runner.inject_card(1, "EX4-038", "hand")

        memory_before = runner.game.memory

        action = runner.find_action("Play Agumon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Only play cost, no tamer bonus
        expected_memory = memory_before - 3
        assert snap_after.memory == expected_memory, (
            f"Memory should be {expected_memory} (no tamer trigger when suspended); "
            f"got {snap_after.memory}"
        )

    def test_draw_2_when_omnimon_digivolves(self, debug_runner):
        """[Your Turn] [OPT] Draw 2 when an Omnimon Digimon digivolves."""
        runner = debug_runner(initial_memory=20)
        runner.set_phase("Main")

        # Place tamer on field
        runner.place_on_field(1, ["EX4-061"])

        # Place a Red Lv.6 Digimon that can digivolve into Omnimon
        # BT5-086 Omnimon needs Red Lv.6 base
        runner.place_on_field(1, ["ST1-11"])  # WarGreymon (Red Lv.6)

        # Put an Omnimon in hand
        runner.inject_card(1, "BT5-086", "hand")  # Omnimon Lv.7

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Digivolve")
        assert action is not None, "Should be able to digivolve into Omnimon"

        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Digivolve: used 1 from hand (-1), drew 1 (digivolve draw, +1),
        # Drew 2 from tamer effect (+2) = net +2
        assert len(snap_after.p1_hand) == hand_before + 2, (
            f"Should draw 2 when Omnimon digivolves; "
            f"hand was {hand_before}, now {len(snap_after.p1_hand)} "
            f"(expected {hand_before + 2}: -1 digivolve +1 draw +2 tamer)"
        )

    def test_no_draw_when_non_omnimon_digivolves(self, debug_runner):
        """[Your Turn] Should NOT draw when a non-Omnimon Digimon digivolves."""
        runner = debug_runner(initial_memory=20)
        runner.set_phase("Main")

        runner.place_on_field(1, ["EX4-061"])
        runner.place_on_field(1, ["ST1-03"])  # Agumon Lv.3

        runner.inject_card(1, "ST1-07", "hand")  # Greymon Lv.4

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Digivolve")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Digivolve: used 1 from hand (-1), drew 1 (digivolve draw, +1) = net 0
        # No Draw 2 since not Omnimon
        assert len(snap_after.p1_hand) == hand_before, (
            f"Should NOT draw 2 when non-Omnimon digivolves; "
            f"hand was {hand_before}, now {len(snap_after.p1_hand)} (expected {hand_before})"
        )

    def test_draw_2_once_per_turn(self, debug_runner):
        """[Your Turn] [OPT] Draw effect should only trigger once per turn."""
        runner = debug_runner(initial_memory=30)
        runner.set_phase("Main")

        runner.place_on_field(1, ["EX4-061"])

        # Two Red Lv.6 Digimon to digivolve into Omnimon
        runner.place_on_field(1, ["ST1-11"])    # WarGreymon (Red Lv.6)
        runner.place_on_field(1, ["BT1-025"])   # WarGreymon (Red Lv.6)

        runner.inject_card(1, "BT5-086", "hand")  # Omnimon
        runner.inject_card(1, "BT1-084", "hand")  # Omnimon

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        # First digivolve into Omnimon
        action1 = runner.find_action("Digivolve")
        assert action1 is not None
        runner.execute(action1)
        runner.auto_resolve()

        # Second digivolve into Omnimon
        action2 = runner.find_action("Digivolve")
        if action2 is not None:
            runner.execute(action2)
            runner.auto_resolve()

        snap_after = runner.snapshot()
        # Two digivolves: 2 * (-1 used +1 draw) = 0 net from digivolving
        # Draw 2 fires once (OPT) = +2
        # Total: hand_before + 2
        assert len(snap_after.p1_hand) == hand_before + 2, (
            f"Should only draw 2 once per turn (OPT); "
            f"hand was {hand_before}, now {len(snap_after.p1_hand)} "
            f"(expected {hand_before + 2}: 2 digivolves net 0 + 1x Draw 2)"
        )
