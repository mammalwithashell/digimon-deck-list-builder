"""Behavioral tests for LM-028 Blue Scramble (Option, Blue, Cost 2).

Card text:
  [Main] 1 of your blue Digimon may digivolve into a blue Digimon card in
  the hand with the digivolution cost reduced by 3. Then, place this card
  in the battle area.

  [Start of Your Turn] If your opponent has a Digimon, <Delay>.
  Return 1 blue Digimon card from your trash to the top of the deck.
  Then, if you don't have a Digimon, you may play 1 blue Digimon card
  with 2000 DP or less from your trash without paying the cost.

  [Security] You may play 1 blue Digimon card with 2000 DP or less from
  your trash without paying the cost. Then, add this card to the hand.
"""

import pytest


@pytest.mark.behavioral
class TestLM028BlueScramble:
    """Tests for LM-028 Blue Scramble."""

    def test_main_places_card_in_battle_area(self, debug_runner):
        """Playing Blue Scramble should place it in the battle area (Delay)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a blue Digimon on field so there's a valid digivolve base
        runner.place_on_field(1, ["BT1-029"])  # Gabumon (Blue, Lv.3)
        runner.inject_card(1, "LM-028", "hand")

        action = runner.find_action("Play Blue Scramble")
        assert action is not None, "Should be able to play Blue Scramble"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Blue Scramble should be in P1's battle area as a Delay option
        in_battle = any(s.card_id == "LM-028" for s in snap.p1_field)
        assert in_battle, "Blue Scramble should be placed in the battle area"

    def test_delay_returns_blue_digimon_from_trash_to_deck_top(self, debug_runner):
        """Delay effect should return 1 blue Digimon from trash to top of deck."""
        runner = debug_runner(initial_memory=3, skip_shuffle=True)

        # Setup: Blue Scramble in battle area (placed on a prior turn)
        runner.place_on_field(1, ["LM-028"], turn_played=-1)
        # Opponent must have a Digimon for the Delay condition
        runner.place_on_field(2, ["BT1-029"])
        # Put a blue Digimon in P1's trash
        runner.inject_card(1, "BT1-029", "trash")  # Gabumon

        lib_size_before = len(runner.game.player1.library_cards)
        trash_size_before = len(runner.game.player1.trash_cards)

        runner.set_phase("Main")

        # Look for the Delay activation
        delay_action = runner.find_action("Delay")
        assert delay_action is not None, "Delay effect should be available"
        runner.execute(delay_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # LM-028 should have been trashed (Delay consumption)
        lm028_on_field = any(s.card_id == "LM-028" for s in snap.p1_field)
        assert not lm028_on_field, "LM-028 should be trashed after Delay activation"

        # Library should have gained one card (the returned Digimon)
        # Trash should have lost the returned Digimon but gained LM-028
        assert snap.p1_library_size == lib_size_before + 1, (
            "Deck should gain 1 card from the returned blue Digimon"
        )

    def test_delay_play_from_trash_when_no_digimon(self, debug_runner):
        """After returning card to deck, if no Digimon on field, may play blue DP<=2000 from trash."""
        runner = debug_runner(initial_memory=3, skip_shuffle=True)

        # Setup: Blue Scramble in battle area, no other Digimon for P1
        runner.place_on_field(1, ["LM-028"], turn_played=-1)
        # Opponent must have a Digimon
        runner.place_on_field(2, ["BT1-029"])
        # Put two blue Digimon in P1's trash:
        # one to return to deck, one with DP<=2000 to play
        runner.inject_card(1, "BT1-029", "trash")  # Gabumon (DP 1000, Blue Lv.3)
        runner.inject_card(1, "BT1-029", "trash")  # Another Gabumon

        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None, "Delay effect should be available"
        runner.execute(delay_action)

        # First selection: pick a blue Digimon from trash to return to deck
        # (SelectTrash phase — pick first legal action)
        legal = runner.action_mask()
        assert legal, "Should have trash selection options"
        runner.execute(legal[0])

        # Second selection: optional play from trash (since no Digimon on field)
        # Find the Gabumon selection (not the Decline/Pass)
        gabumon_action = runner.find_action("Gabumon")
        if gabumon_action is None:
            # Try to find any non-pass action
            actions = runner.actions()
            for aid, desc in actions.items():
                if "pass" not in desc.lower() and "decline" not in desc.lower():
                    gabumon_action = aid
                    break
        assert gabumon_action is not None, (
            "Should offer play-from-trash selection when P1 has no Digimon"
        )
        runner.execute(gabumon_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # P1 should have a Gabumon on field (played from trash)
        gabumon_on_field = any(
            s.card_id == "BT1-029" for s in snap.p1_field
        )
        assert gabumon_on_field, (
            "Should play a blue Digimon with DP<=2000 from trash when no Digimon on field"
        )

    def test_delay_no_play_when_has_digimon(self, debug_runner):
        """If P1 has a Digimon on field, should NOT get the play-from-trash sub-effect."""
        runner = debug_runner(initial_memory=3, skip_shuffle=True)

        # Setup: Blue Scramble + a Digimon in battle area
        runner.place_on_field(1, ["LM-028"], turn_played=-1)
        runner.place_on_field(1, ["BT1-029"])  # P1 has a Digimon
        # Opponent must have a Digimon
        runner.place_on_field(2, ["BT1-029"])
        # Put blue Digimon in P1's trash
        runner.inject_card(1, "BT1-029", "trash")
        runner.inject_card(1, "BT1-029", "trash")

        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None
        runner.execute(delay_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Only the original Gabumon should be on field, no additional play from trash
        gabumon_count = sum(1 for s in snap.p1_field if s.card_id == "BT1-029")
        assert gabumon_count == 1, (
            "Should NOT play from trash when P1 already has a Digimon on field"
        )

    def test_delay_effect_skipped_when_opponent_has_no_digimon(self, debug_runner):
        """When opponent has no Digimon, Delay action trashes card but effect doesn't fire."""
        runner = debug_runner(initial_memory=3, skip_shuffle=True)

        runner.place_on_field(1, ["LM-028"], turn_played=-1)
        # No opponent Digimon
        runner.inject_card(1, "BT1-029", "trash")

        lib_size_before = len(runner.game.player1.library_cards)

        runner.set_phase("Main")

        # Delay action is still available (engine doesn't gate on delay effect condition)
        delay_action = runner.find_action("Delay")
        assert delay_action is not None, "Delay action is available regardless"
        runner.execute(delay_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # LM-028 should be trashed
        assert not any(s.card_id == "LM-028" for s in snap.p1_field)
        # But the delay effect should NOT have fired — library unchanged
        assert snap.p1_library_size == lib_size_before, (
            "Delay effect should not fire when opponent has no Digimon"
        )
        # Trash Gabumon should still be in trash (not returned to deck)
        assert "BT1-029" in snap.p1_trash, "Gabumon should remain in trash"
