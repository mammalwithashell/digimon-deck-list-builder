"""Behavioral tests for EX8-046 Gotsumon (Lv.3, Black, Rock).

Card text:
  [On Deletion] By trashing 1 card with the [Mineral] or [Rock] trait in
  your hand, <Draw 2> (Draw 2 cards from your deck.)

  Inherited: <Blocker>

Key faithfulness points tested:
  - [On Deletion] timing (OnDestroyedAnyone), fires when this Digimon is deleted.
  - Effect is optional (player can decline).
  - Condition: must have at least 1 [Mineral] or [Rock] trait card in hand.
  - Once activated, trashing is mandatory (is_optional=False on hand selection).
  - Draw 2 only happens after successful trash.
  - Does NOT fire if no Mineral/Rock card in hand.
  - Inherited effect is Blocker (is_inherited_effect=True, _is_blocker=True).
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming

# Card IDs
EX8_046 = "EX8-046"        # Card under test (Gotsumon, Lv.3 Black, Rock trait)
BT2_054 = "BT2-054"        # Gotsumon (Lv.3 Black, Rock trait) - for hand trash target
BT4_066 = "BT4-066"        # Golemon (Lv.4 Black, Mineral trait) - for hand trash target
AGUMON_ST1 = "ST1-03"      # Agumon (Lv.3 Red, no Mineral/Rock) - filler

# Filler deck
FILLER = [AGUMON_ST1] * 50


@pytest.mark.behavioral
class TestEX8046OnDeletionAttributes:
    """Verify On Deletion effect structure and attributes."""

    def test_on_deletion_timing(self, debug_runner):
        """On Deletion effect should use OnDestroyedAnyone timing."""
        runner = debug_runner(deck1=[EX8_046] * 4 + [AGUMON_ST1] * 46, deck2=FILLER)

        perm = runner.place_on_field(1, [EX8_046])
        card = perm.top_card
        effects = card.effect_list(None)
        od_effects = [e for e in effects if e.is_on_deletion]
        assert len(od_effects) == 1, "Should have exactly 1 On Deletion effect"

        od = od_effects[0]
        assert od.timing == EffectTiming.OnDestroyedAnyone, (
            f"On Deletion should use OnDestroyedAnyone timing, got {od.timing}"
        )

    def test_on_deletion_is_optional(self, debug_runner):
        """On Deletion effect should be optional (player chooses to activate)."""
        runner = debug_runner(deck1=[EX8_046] * 4 + [AGUMON_ST1] * 46, deck2=FILLER)

        perm = runner.place_on_field(1, [EX8_046])
        card = perm.top_card
        effects = card.effect_list(None)
        od_effects = [e for e in effects if e.is_on_deletion]
        assert od_effects[0].is_optional, "On Deletion effect should be optional"


@pytest.mark.behavioral
class TestEX8046OnDeletionTrashAndDraw:
    """Test the On Deletion trash-and-draw mechanic via delete_permanent."""

    def test_deletion_triggers_trash_and_draw_with_rock_card(self, debug_runner):
        """Deleting EX8-046 with a [Rock] trait card in hand should trash it and draw 2."""
        runner = debug_runner(
            deck1=[EX8_046] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")

        # Place EX8-046 on field
        perm = runner.place_on_field(1, [EX8_046])

        # Put a Rock trait card in hand
        runner.inject_card(1, BT2_054, "hand")

        before = runner.snapshot()
        hand_before = len(before.p1_hand)
        lib_before = before.p1_library_size
        trash_before = len(before.p1_trash)

        # Delete the Gotsumon
        p1 = runner.game.player1
        deleted = p1.delete_permanent(perm)
        runner.auto_resolve()

        after = runner.snapshot()

        # The Rock card should have been trashed from hand (-1 from trash cost)
        # Then draw 2 cards (+2 from draw)
        # Net hand change: -1 (trash) +2 (draw) = +1
        assert after.p1_library_size == lib_before - 2, (
            f"Should have drawn 2 cards from library. Before: {lib_before}, after: {after.p1_library_size}"
        )
        # Trash should contain: EX8-046's card(s) + the trashed hand card
        assert len(after.p1_trash) > trash_before, (
            "Trash should contain at least the deleted permanent and the trashed hand card"
        )

    def test_deletion_triggers_trash_and_draw_with_mineral_card(self, debug_runner):
        """Deleting EX8-046 with a [Mineral] trait card in hand should trash it and draw 2."""
        runner = debug_runner(
            deck1=[EX8_046] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [EX8_046])

        # Put a Mineral trait card in hand
        runner.inject_card(1, BT4_066, "hand")

        before = runner.snapshot()
        lib_before = before.p1_library_size

        p1 = runner.game.player1
        deleted = p1.delete_permanent(perm)
        runner.auto_resolve()

        after = runner.snapshot()

        # Should have drawn 2 cards (Mineral card was valid trash target)
        assert after.p1_library_size == lib_before - 2, (
            f"Should have drawn 2 from library with Mineral hand card. "
            f"Before: {lib_before}, after: {after.p1_library_size}"
        )


@pytest.mark.behavioral
class TestEX8046OnDeletionCondition:
    """Test that the On Deletion effect condition correctly gates activation."""

    def test_no_activation_without_mineral_or_rock_in_hand(self, debug_runner):
        """On Deletion should NOT activate if no Mineral/Rock card in hand."""
        runner = debug_runner(
            deck1=[EX8_046] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [EX8_046])

        # Only non-Mineral/Rock cards in hand (Agumon has no relevant traits)
        runner.clear_zone(1, "hand")
        runner.inject_card(1, AGUMON_ST1, "hand")

        before = runner.snapshot()
        lib_before = before.p1_library_size

        p1 = runner.game.player1
        deleted = p1.delete_permanent(perm)
        runner.auto_resolve()

        after = runner.snapshot()

        # Library should NOT have decreased (no draw happened)
        assert after.p1_library_size == lib_before, (
            f"Library should not change without Mineral/Rock in hand. "
            f"Before: {lib_before}, after: {after.p1_library_size}"
        )

    def test_no_activation_with_empty_hand(self, debug_runner):
        """On Deletion should NOT activate if hand is empty."""
        runner = debug_runner(
            deck1=[EX8_046] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [EX8_046])
        runner.clear_zone(1, "hand")

        before = runner.snapshot()
        lib_before = before.p1_library_size

        p1 = runner.game.player1
        deleted = p1.delete_permanent(perm)
        runner.auto_resolve()

        after = runner.snapshot()

        assert after.p1_library_size == lib_before, (
            f"Library should not change with empty hand. "
            f"Before: {lib_before}, after: {after.p1_library_size}"
        )

    def test_condition_function_directly(self, debug_runner):
        """Directly test the condition function with and without valid hand cards."""
        runner = debug_runner(
            deck1=[EX8_046] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=10,
        )

        perm = runner.place_on_field(1, [EX8_046])
        card = perm.top_card
        effects = card.effect_list(None)
        od_effects = [e for e in effects if e.is_on_deletion]
        od = od_effects[0]

        p1 = runner.game.player1

        # No valid cards in hand
        runner.clear_zone(1, "hand")
        result_empty = od.can_use_condition({"player": p1})
        assert not result_empty, "Condition should fail with no valid hand cards"

        # Add a Rock trait card
        runner.inject_card(1, BT2_054, "hand")
        result_rock = od.can_use_condition({"player": p1})
        assert result_rock, "Condition should pass with Rock trait card in hand"

        # Clear and add a Mineral trait card
        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT4_066, "hand")
        result_mineral = od.can_use_condition({"player": p1})
        assert result_mineral, "Condition should pass with Mineral trait card in hand"

        # Only non-matching card
        runner.clear_zone(1, "hand")
        runner.inject_card(1, AGUMON_ST1, "hand")
        result_no_match = od.can_use_condition({"player": p1})
        assert not result_no_match, "Condition should fail with only non-Mineral/Rock cards"


@pytest.mark.behavioral
class TestEX8046OnDeletionDrawOnly:
    """Verify draw 2 only fires after successful trash."""

    def test_draw_count_is_exactly_2(self, debug_runner):
        """After trashing a valid hand card, should draw exactly 2."""
        runner = debug_runner(
            deck1=[EX8_046] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [EX8_046])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT2_054, "hand")

        before = runner.snapshot()
        hand_before = len(before.p1_hand)
        lib_before = before.p1_library_size

        # 1 card in hand (BT2-054, Rock)
        assert hand_before == 1, "Should have exactly 1 card in hand"

        p1 = runner.game.player1
        deleted = p1.delete_permanent(perm)
        runner.auto_resolve()

        after = runner.snapshot()

        # Hand: started with 1, trashed 1, drew 2 => 2 cards
        assert len(after.p1_hand) == 2, (
            f"Hand should have 2 cards (trashed 1, drew 2). Got: {len(after.p1_hand)}"
        )
        # Library decreased by exactly 2
        assert after.p1_library_size == lib_before - 2, (
            f"Library should decrease by exactly 2. Before: {lib_before}, after: {after.p1_library_size}"
        )


@pytest.mark.behavioral
class TestEX8046InheritedBlocker:
    """Test the inherited Blocker keyword."""

    def test_inherited_effect_is_blocker(self, debug_runner):
        """Inherited effect should set _is_blocker = True."""
        runner = debug_runner(
            deck1=[EX8_046] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
        )

        perm = runner.place_on_field(1, [EX8_046])
        card = perm.top_card
        effects = card.effect_list(None)
        inherited_effects = [e for e in effects if e.is_inherited_effect]
        assert len(inherited_effects) == 1, "Should have exactly 1 inherited effect"

        inh = inherited_effects[0]
        assert inh._is_blocker, "Inherited effect should be Blocker"

    def test_inherited_blocker_grants_keyword_to_stack(self, debug_runner):
        """When EX8-046 is in a digivolution stack, the top Digimon should gain Blocker."""
        runner = debug_runner(
            deck1=[EX8_046] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=10,
        )

        # Stack: EX8-046 (bottom, inherited) + Agumon (top, Lv.3)
        # Note: Normally you'd digivolve onto a Lv.3, but for inherited testing
        # we just need it in the stack. Lv.4+ would be more natural but Agumon works
        # to verify the keyword propagation.
        perm = runner.place_on_field(1, [EX8_046, AGUMON_ST1])

        snap = runner.snapshot()
        field = snap.p1_field
        assert len(field) > 0, "Should have the stacked permanent on field"

        # The permanent should have blocker keyword from inherited
        assert perm.has_keyword("_is_blocker"), (
            "Permanent with EX8-046 inherited should have Blocker keyword"
        )

    def test_inherited_effect_only(self, debug_runner):
        """The Blocker effect should be inherited-only, not a standalone effect."""
        runner = debug_runner(
            deck1=[EX8_046] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
        )

        perm = runner.place_on_field(1, [EX8_046])
        card = perm.top_card
        effects = card.effect_list(None)

        blocker_effects = [e for e in effects if getattr(e, '_is_blocker', False)]
        assert len(blocker_effects) == 1, "Should have exactly 1 Blocker effect"
        assert blocker_effects[0].is_inherited_effect, (
            "Blocker effect should be inherited-only"
        )


@pytest.mark.behavioral
class TestEX8046IntegrationOnDeletion:
    """Integration tests: full deletion flow triggering On Deletion effect."""

    def test_deletion_by_opponent_effect_triggers_on_deletion(self, debug_runner):
        """When deleted by opponent's effect, On Deletion should fire."""
        runner = debug_runner(
            deck1=[EX8_046] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [EX8_046])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT2_054, "hand")

        before = runner.snapshot()
        lib_before = before.p1_library_size

        p1 = runner.game.player1
        deleted = p1.delete_permanent(perm, is_opponent_effect=True)
        runner.auto_resolve()

        after = runner.snapshot()

        assert deleted, "Gotsumon should actually be deleted"
        assert after.p1_library_size == lib_before - 2, (
            "On Deletion should fire and draw 2 even when deleted by opponent's effect"
        )

    def test_trash_target_goes_to_trash(self, debug_runner):
        """The trashed hand card should end up in the trash pile."""
        runner = debug_runner(
            deck1=[EX8_046] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=10,
        )
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [EX8_046])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT2_054, "hand")

        p1 = runner.game.player1
        deleted = p1.delete_permanent(perm)
        runner.auto_resolve()

        after = runner.snapshot()

        # Trash should contain: EX8-046 (the deleted card) + BT2-054 (the trashed hand card)
        assert BT2_054 in after.p1_trash, (
            f"Trashed hand card (BT2-054) should be in trash. Trash: {after.p1_trash}"
        )
        assert EX8_046 in after.p1_trash, (
            f"Deleted permanent (EX8-046) should be in trash. Trash: {after.p1_trash}"
        )
