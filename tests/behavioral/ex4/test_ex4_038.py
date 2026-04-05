"""Behavioral tests for EX4-038 Agumon (Lv.3 Purple, Cost 3).

Card text:
[On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with [Greymon]
    in its name and 1 Digimon card with [Gabumon], [Garurumon] or [Omnimon] in
    its name among them to your hand. Place the remaining cards at the bottom of
    your deck in any order.

Inherited: [Your Turn] [Once Per Turn] When one of your other Digimon digivolves,
    gain 1 memory.
"""

import pytest


@pytest.mark.behavioral
class TestEX4038Agumon:
    """Tests for EX4-038 Agumon."""

    def test_on_play_adds_greymon_and_gabumon_to_hand(self, debug_runner):
        """[On Play] Should add 1 Greymon Digimon and 1 Gabumon/Garurumon/Omnimon
        Digimon from the revealed top 3 to hand."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Clear the deck so we control what's on top
        runner.clear_zone(1, "library")

        # Stack the top 3: Greymon (Lv.4), Gabumon (Lv.3), and a filler
        runner.inject_card(1, "EX4-044", "library_top")  # Greymon
        runner.inject_card(1, "EX4-039", "library_top")  # Gabumon
        runner.inject_card(1, "ST1-03", "library_top")   # Agumon (filler, no match)

        # Also need more cards in deck for draw etc.
        for _ in range(10):
            runner.inject_card(1, "ST1-03", "library_bottom")

        # Put EX4-038 in hand
        runner.inject_card(1, "EX4-038", "hand")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Play Agumon")
        assert action is not None, "Should find play action for EX4-038 Agumon"

        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Played 1 card from hand (-1), added Greymon (+1) and Gabumon (+1) = net +1
        assert len(snap_after.p1_hand) == hand_before + 1, (
            f"Hand should gain 1 net card (played 1, added 2); "
            f"was {hand_before}, now {len(snap_after.p1_hand)}"
        )
        # Greymon and Gabumon should be in hand
        assert "EX4-044" in snap_after.p1_hand, "Greymon should be added to hand"
        assert "EX4-039" in snap_after.p1_hand, "Gabumon should be added to hand"

    def test_on_play_remaining_go_to_bottom_of_deck(self, debug_runner):
        """[On Play] Remaining cards should go to the BOTTOM of the deck, not top."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Clear the deck
        runner.clear_zone(1, "library")

        # Stack deck: bottom-up order. We want specific cards on top.
        # inject_card with library_top puts them on top (index 0)
        # After reveal, remaining should go to bottom.
        runner.inject_card(1, "ST1-03", "library_bottom")  # "old" bottom card
        runner.inject_card(1, "EX4-044", "library_top")  # Greymon (will be added to hand)
        runner.inject_card(1, "EX4-039", "library_top")  # Gabumon (will be added to hand)
        runner.inject_card(1, "BT1-010", "library_top")  # Agumon (filler - no match)

        runner.inject_card(1, "EX4-038", "hand")

        runner.execute(runner.find_action("Play Agumon"))
        runner.auto_resolve()

        # The filler (BT1-010 Agumon) should now be at the bottom of the library,
        # BELOW the old bottom card (ST1-03)
        game = runner.game
        p1 = game.player1
        # Library bottom is index -1 (end of list) or index 0 depending on convention
        # library_cards[0] is the top, library_cards[-1] is the bottom
        # The remaining BT1-010 should be at the bottom
        bottom_card_id = p1.library_cards[-1].c_entity_base.card_id
        assert bottom_card_id == "BT1-010", (
            f"Remaining filler card should be at BOTTOM of deck, "
            f"but bottom is {bottom_card_id}"
        )

    def test_on_play_no_greymon_in_revealed(self, debug_runner):
        """[On Play] If no Greymon among revealed, only add Gabumon/Garurumon/Omnimon if present."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.clear_zone(1, "library")
        # Stack: no Greymon at all, just Gabumon and fillers
        runner.inject_card(1, "EX4-039", "library_top")  # Gabumon
        runner.inject_card(1, "ST1-03", "library_top")   # Agumon (filler)
        runner.inject_card(1, "BT1-010", "library_top")  # Agumon (filler)
        for _ in range(10):
            runner.inject_card(1, "ST1-03", "library_bottom")

        runner.inject_card(1, "EX4-038", "hand")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        runner.execute(runner.find_action("Play Agumon"))
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Played 1, added only Gabumon (+1) = net 0
        assert "EX4-039" in snap_after.p1_hand, "Gabumon should still be added"
        assert len(snap_after.p1_hand) == hand_before, (
            f"Hand should be net 0 (played 1, added 1); "
            f"was {hand_before}, now {len(snap_after.p1_hand)}"
        )

    def test_inherited_memory_gain_on_other_digivolve(self, debug_runner):
        """Inherited: gain 1 memory when another Digimon digivolves on your turn."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a Digimon with EX4-038 as inherited source
        # Stack: EX4-038 (Agumon Lv.3) under EX4-044 (Greymon Lv.4)
        runner.place_on_field(1, ["EX4-038", "EX4-044"])

        # Place a Lv.3 target that can digivolve
        runner.place_on_field(1, ["ST1-03"])  # Agumon Lv.3

        # Put a Lv.4 in hand to digivolve onto the other Digimon
        runner.inject_card(1, "ST1-07", "hand")  # Greymon Lv.4

        memory_before = runner.game.memory

        action = runner.find_action("Digivolve")
        assert action is not None, "Should be able to digivolve ST1-03 -> ST1-07"

        runner.execute(action)
        runner.auto_resolve()

        # Memory should have increased by 1 from the inherited effect
        # (minus digivolve cost)
        # ST1-07 Greymon costs 2 to digivolve from Lv.3
        snap_after = runner.snapshot()
        expected_memory = memory_before - 2 + 1  # -2 digivolve cost, +1 inherited
        assert snap_after.memory == expected_memory, (
            f"Memory should be {expected_memory} (started {memory_before}, "
            f"-2 digivolve, +1 inherited); got {snap_after.memory}"
        )

    def test_inherited_no_trigger_on_self_digivolve(self, debug_runner):
        """Inherited: should NOT trigger when the Digimon carrying it digivolves."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place EX4-038 under a Greymon
        perm = runner.place_on_field(1, ["EX4-038", "EX4-044"])

        # Put a Lv.5 in hand to digivolve the same Digimon
        runner.inject_card(1, "EX4-045", "hand")  # MetalGreymon Lv.5

        memory_before = runner.game.memory

        action = runner.find_action("Digivolve")
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # MetalGreymon costs 3 to digivolve from Lv.4
        # No +1 because it's the same Digimon
        expected_memory = memory_before - 3
        assert snap_after.memory == expected_memory, (
            f"Memory should be {expected_memory} (no inherited trigger on self); "
            f"got {snap_after.memory}"
        )

    def test_inherited_once_per_turn(self, debug_runner):
        """Inherited: should only trigger once per turn."""
        runner = debug_runner(initial_memory=20)
        runner.set_phase("Main")

        # Place a Digimon with EX4-038 as inherited
        runner.place_on_field(1, ["EX4-038", "EX4-044"])

        # Place two Lv.3 Digimon to digivolve
        runner.place_on_field(1, ["ST1-03"])
        runner.place_on_field(1, ["BT1-010"])

        # Two Lv.4 in hand
        runner.inject_card(1, "ST1-07", "hand")  # Greymon
        runner.inject_card(1, "BT1-015", "hand")  # Greymon

        memory_before = runner.game.memory

        # Digivolve first
        action1 = runner.find_action("Digivolve")
        assert action1 is not None
        runner.execute(action1)
        runner.auto_resolve()

        # Digivolve second
        action2 = runner.find_action("Digivolve")
        if action2 is not None:
            runner.execute(action2)
            runner.auto_resolve()

        snap_after = runner.snapshot()
        # Two digivolves at cost 2 each = -4, only 1 memory gain (OPT)
        expected_memory = memory_before - 4 + 1
        assert snap_after.memory == expected_memory, (
            f"Memory should be {expected_memory} (once per turn); "
            f"got {snap_after.memory}"
        )
