"""Behavioral tests for ST9-06 Imperialdramon Dragon Mode.

Card text:
  Lv.6 | Blue/Green | Ancient Dragon | DP:12000 | Cost:13
  Evo: Blue Lv.5 for cost 4
  [When Digivolving] You may play 1 level 4 or lower blue Digimon card and
  1 level 4 or lower green Digimon card from this Digimon's digivolution cards
  without paying their memory costs.
  Inherited: (none)

Key faithfulness points tested:
  - When Digivolving triggers on digivolving (not on play).
  - Effect is optional ("You may play").
  - Plays 1 blue Lv4-or-lower Digimon from digi-stack (selection phase).
  - Plays 1 green Lv4-or-lower Digimon from digi-stack (selection phase).
  - Played cards enter the field as new Permanents.
  - Played cards are removed from the digi-stack.
  - No memory cost is paid for the played cards.
  - Condition: must have at least 1 qualifying blue or green card in digi-stack.
  - Does not play the top card (Imperialdramon itself) from the stack.
"""

import pytest

# Card IDs used in tests
ST9_06 = "ST9-06"    # Imperialdramon Dragon Mode (Lv.6 Blue/Green)
ST9_05 = "ST9-05"    # Paildramon (Lv.5 Blue/Green, evo base)
ST9_04 = "ST9-04"    # ExVeemon (Lv.4 Blue)
ST9_09 = "ST9-09"    # Stingmon (Lv.4 Green)
ST9_02 = "ST9-02"    # Veemon (Lv.3 Blue)
ST9_08 = "ST9-08"    # Wormmon (Lv.3 Green)
FILLER = "ST1-03"    # Agumon (generic filler for decks)


@pytest.mark.behavioral
class TestST9_06WhenDigivolving:
    """[When Digivolving] Play blue Lv4 + green Lv4 from digi-stack."""

    def test_plays_blue_and_green_from_digistack(self, debug_runner):
        """Digivolving ST9-06 onto a stack with blue+green Lv4 underneath should
        play both cards onto the field as new Permanents."""
        # Build deck with the cards we need
        deck = [ST9_06, ST9_05, ST9_04, ST9_09] + [FILLER] * 46
        runner = debug_runner(deck1=deck, deck2=[FILLER] * 50, initial_memory=10)

        # Place Paildramon on field with ExVeemon (blue Lv4) and Stingmon (green Lv4) underneath
        # Stack bottom-to-top: ExVeemon, Stingmon, Paildramon
        perm = runner.place_on_field(1, [ST9_04, ST9_09, ST9_05])

        # Put Imperialdramon in hand for digivolving
        runner.inject_card(1, ST9_06, "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        field_count_before = len(snap_before.p1_field)

        # Digivolve Imperialdramon onto Paildramon
        action = runner.find_action("Digivolve Imperialdramon")
        assert action is not None, f"Should find digivolve action. Actions: {runner.actions()}"
        runner.execute(action)

        # Auto-resolve the Yes/selection phases
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # ExVeemon and Stingmon should now be separate Permanents on the field
        field_ids = [s.card_id for s in snap_after.p1_field]
        assert ST9_04 in field_ids, (
            f"ExVeemon (blue Lv4) should have been played onto the field. "
            f"Field: {field_ids}"
        )
        assert ST9_09 in field_ids, (
            f"Stingmon (green Lv4) should have been played onto the field. "
            f"Field: {field_ids}"
        )

        # Imperialdramon should still be on field (as the digivolved stack)
        assert ST9_06 in field_ids, (
            f"Imperialdramon should still be on field. Field: {field_ids}"
        )

        # The played cards should be removed from the Imperialdramon's digi-stack
        imperial_slot = next(
            (s for s in snap_after.p1_field if s.card_id == ST9_06), None
        )
        assert imperial_slot is not None
        assert ST9_04 not in imperial_slot.stack_ids, (
            f"ExVeemon should no longer be in Imperialdramon's digi-stack. "
            f"Stack: {imperial_slot.stack_ids}"
        )
        assert ST9_09 not in imperial_slot.stack_ids, (
            f"Stingmon should no longer be in Imperialdramon's digi-stack. "
            f"Stack: {imperial_slot.stack_ids}"
        )

    def test_no_memory_cost_for_played_cards(self, debug_runner):
        """Playing cards from digi-stack should not cost any memory beyond the evo cost."""
        deck = [ST9_06, ST9_05, ST9_04, ST9_09] + [FILLER] * 46
        runner = debug_runner(deck1=deck, deck2=[FILLER] * 50, initial_memory=10)

        perm = runner.place_on_field(1, [ST9_04, ST9_09, ST9_05])
        runner.inject_card(1, ST9_06, "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        memory_before = snap_before.memory

        action = runner.find_action("Digivolve Imperialdramon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Only the evo cost (4) should be spent, not the play costs of the cards
        memory_spent = memory_before - snap_after.memory
        assert memory_spent == 4, (
            f"Only evo cost 4 should be spent (not play costs of ExVeemon/Stingmon). "
            f"Memory before: {memory_before}, after: {snap_after.memory}, spent: {memory_spent}"
        )

    def test_only_blue_available_plays_only_blue(self, debug_runner):
        """If only blue Lv4 cards in digi-stack, only blue is played."""
        deck = [ST9_06, ST9_05, ST9_04] + [FILLER] * 47
        runner = debug_runner(deck1=deck, deck2=[FILLER] * 50, initial_memory=10)

        # Stack with only ExVeemon (blue) underneath, no green
        perm = runner.place_on_field(1, [ST9_04, ST9_05])
        runner.inject_card(1, ST9_06, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Digivolve Imperialdramon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        field_ids = [s.card_id for s in snap.p1_field]
        assert ST9_04 in field_ids, (
            f"ExVeemon (blue Lv4) should have been played. Field: {field_ids}"
        )

    def test_only_green_available_plays_only_green(self, debug_runner):
        """If only green Lv4 cards in digi-stack, only green is played."""
        deck = [ST9_06, ST9_05, ST9_09] + [FILLER] * 47
        runner = debug_runner(deck1=deck, deck2=[FILLER] * 50, initial_memory=10)

        # Stack with only Stingmon (green) underneath, no blue Lv4
        perm = runner.place_on_field(1, [ST9_09, ST9_05])
        runner.inject_card(1, ST9_06, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Digivolve Imperialdramon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        field_ids = [s.card_id for s in snap.p1_field]
        assert ST9_09 in field_ids, (
            f"Stingmon (green Lv4) should have been played. Field: {field_ids}"
        )

    def test_no_qualifying_cards_effect_skipped(self, debug_runner):
        """If no qualifying Digimon in digi-stack, effect condition fails and no cards are played.
        Paildramon (Lv.5) is too high to qualify; the effect requires Lv.4 or lower."""
        deck = [ST9_06, ST9_05] + [FILLER] * 48
        runner = debug_runner(deck1=deck, deck2=[FILLER] * 50, initial_memory=10)

        # Stack with just Paildramon (Lv.5 Blue/Green - too high for either selection)
        perm = runner.place_on_field(1, [ST9_05])
        runner.inject_card(1, ST9_06, "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        field_count_before = len(snap_before.p1_field)

        action = runner.find_action("Digivolve Imperialdramon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Only Imperialdramon's stack on field, no new Permanents
        assert len(snap_after.p1_field) == field_count_before, (
            f"No new Permanents should appear since no Lv4-or-lower blue/green in stack. "
            f"Before: {field_count_before}, After: {len(snap_after.p1_field)}"
        )

    def test_does_not_play_lv5_blue(self, debug_runner):
        """Level 5 blue Digimon in digi-stack should NOT be eligible."""
        deck = [ST9_06, ST9_05, ST9_09] + [FILLER] * 47
        runner = debug_runner(deck1=deck, deck2=[FILLER] * 50, initial_memory=10)

        # Stack: Stingmon (green Lv4), Paildramon (blue/green Lv5, too high), top=Paildramon
        # Use a second Paildramon as a "source" card (it's Lv5, should not be eligible as blue play)
        perm = runner.place_on_field(1, [ST9_09, ST9_05])
        runner.inject_card(1, ST9_06, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Digivolve Imperialdramon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        field_ids = [s.card_id for s in snap.p1_field]
        # Stingmon (green Lv4) should be played; Paildramon (Lv5) should NOT
        assert ST9_09 in field_ids, (
            f"Stingmon (green Lv4) should have been played. Field: {field_ids}"
        )
        # Paildramon in the stack was Lv5, shouldn't be treated as a blue Lv4
        # It should remain in the digi-stack
        imperial_slot = next(
            (s for s in snap.p1_field if s.card_id == ST9_06), None
        )
        assert imperial_slot is not None
        assert ST9_05 in imperial_slot.stack_ids, (
            f"Paildramon (Lv5) should remain in digi-stack. Stack: {imperial_slot.stack_ids}"
        )

    def test_effect_is_optional(self, debug_runner):
        """The effect says 'You may play', so it should be skippable."""
        deck = [ST9_06, ST9_05, ST9_04, ST9_09] + [FILLER] * 46
        runner = debug_runner(deck1=deck, deck2=[FILLER] * 50, initial_memory=10)

        perm = runner.place_on_field(1, [ST9_04, ST9_09, ST9_05])
        runner.inject_card(1, ST9_06, "hand")
        runner.set_phase("Main")

        # Check that the effect is declared as optional
        card = perm.top_card  # This is Paildramon currently, but we want to check ST9-06's effect
        # We need to check ST9-06's effect attributes directly
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(ST9_06, runner.game.player1)
        effects = cs.effect_list(None)
        wd_effect = None
        for eff in effects:
            if eff.is_when_digivolving:
                wd_effect = eff
                break
        assert wd_effect is not None, "Should have a When Digivolving effect"
        assert wd_effect.is_optional is True, (
            f"When Digivolving effect should be optional (card says 'You may'). "
            f"Got is_optional={wd_effect.is_optional}"
        )
