"""Behavioral tests for EX4-039 Gabumon (Lv.3 Purple, Cost 3).

Card text:
[On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with [Garurumon]
    in its name and 1 Digimon card with [Agumon], [Greymon] or [Omnimon] in its
    name among them to your hand. Place the remaining cards at the bottom of your
    deck in any order.

Inherited: [Your Turn] [Once Per Turn] When one of your other Digimon digivolves,
    gain 1 memory.
"""

import pytest


@pytest.mark.behavioral
class TestEX4039Gabumon:
    """Tests for EX4-039 Gabumon."""

    def test_on_play_adds_garurumon_and_agumon_to_hand(self, debug_runner):
        """[On Play] Should add 1 Garurumon and 1 Agumon/Greymon/Omnimon to hand."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.clear_zone(1, "library")

        # Stack top 3: Garurumon, Agumon, filler
        runner.inject_card(1, "EX4-043", "library_top")  # Garurumon
        runner.inject_card(1, "ST1-03", "library_top")   # Agumon (matches group 2)
        runner.inject_card(1, "BT1-029", "library_top")  # Gabumon (filler, no match)

        for _ in range(10):
            runner.inject_card(1, "ST1-03", "library_bottom")

        runner.inject_card(1, "EX4-039", "hand")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Play Gabumon")
        assert action is not None, "Should find play action for EX4-039 Gabumon"

        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Played 1, added Garurumon (+1) and Agumon (+1) = net +1
        assert len(snap_after.p1_hand) == hand_before + 1, (
            f"Hand should gain 1 net card; was {hand_before}, now {len(snap_after.p1_hand)}"
        )
        assert "EX4-043" in snap_after.p1_hand, "Garurumon should be added to hand"
        assert "ST1-03" in snap_after.p1_hand, "Agumon should be added to hand"

    def test_on_play_remaining_go_to_bottom_of_deck(self, debug_runner):
        """[On Play] Remaining cards should go to BOTTOM of deck, not top."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.clear_zone(1, "library")

        # Stack: old bottom card, then top 3
        runner.inject_card(1, "ST1-03", "library_bottom")       # old bottom
        runner.inject_card(1, "EX4-043", "library_top")  # Garurumon (added to hand)
        runner.inject_card(1, "BT1-010", "library_top")  # Agumon (matches group 2, added)
        runner.inject_card(1, "BT1-029", "library_top")  # Gabumon (filler, no match)

        runner.inject_card(1, "EX4-039", "hand")

        runner.execute(runner.find_action("Play Gabumon"))
        runner.auto_resolve()

        game = runner.game
        p1 = game.player1
        # The remaining filler (BT1-029 Gabumon) should be at bottom of deck
        bottom_id = p1.library_cards[-1].c_entity_base.card_id
        assert bottom_id == "BT1-029", (
            f"Remaining card should be at BOTTOM of deck, but bottom is {bottom_id}"
        )

    def test_on_play_omnimon_matches_second_group(self, debug_runner):
        """[On Play] An Omnimon card should match the second group
        (Agumon/Greymon/Omnimon)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.clear_zone(1, "library")

        # Stack: Garurumon, Omnimon, filler
        runner.inject_card(1, "EX4-043", "library_top")  # Garurumon
        runner.inject_card(1, "BT5-086", "library_top")  # Omnimon (Lv.7 Digimon)
        runner.inject_card(1, "BT1-029", "library_top")  # Gabumon (filler)

        for _ in range(10):
            runner.inject_card(1, "ST1-03", "library_bottom")

        runner.inject_card(1, "EX4-039", "hand")

        runner.execute(runner.find_action("Play Gabumon"))
        runner.auto_resolve()

        snap_after = runner.snapshot()
        assert "EX4-043" in snap_after.p1_hand, "Garurumon should be added to hand"
        assert "BT5-086" in snap_after.p1_hand, "Omnimon should match second group"

    def test_inherited_memory_gain_on_other_digivolve(self, debug_runner):
        """Inherited: gain 1 memory when another Digimon digivolves on your turn."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a Digimon with EX4-039 as inherited source
        runner.place_on_field(1, ["EX4-039", "EX4-043"])

        # Place another Lv.3 that can digivolve
        runner.place_on_field(1, ["ST1-03"])
        runner.inject_card(1, "ST1-07", "hand")  # Greymon Lv.4 for digivolve

        memory_before = runner.game.memory

        action = runner.find_action("Digivolve")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # -2 digivolve cost + 1 inherited = -1
        expected_memory = memory_before - 2 + 1
        assert snap_after.memory == expected_memory, (
            f"Memory should be {expected_memory}; got {snap_after.memory}"
        )
