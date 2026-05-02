"""Behavioral tests for BT3-006 DemiMeramon (Lv.2 Purple Digi-Egg).

Card text:
  Inherited [On Deletion] <Draw 1>. Then, trash 1 card in your hand.

Key behaviors:
  - Inherited effect only (Digi-Egg)
  - On Deletion timing (OnDestroyedAnyone)
  - NOT optional (mandatory effect per C# reference: 4th arg = false)
  - Sequence: Draw 1 FIRST, then trash 1 from hand
  - Trash applies to any card (no type/level filter)
  - Trash is mandatory (canNoSelect: false in C#)
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT3006InheritedFlags:
    """Verify effect metadata: inherited, on_deletion, timing, mandatory."""

    def test_effect_is_inherited(self, debug_runner):
        """Effect must be flagged as inherited (Digi-Egg)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT3-006", "ST1-03"])

        egg_card = perm.card_sources[0]
        assert egg_card.c_entity_base.card_id == "BT3-006"
        effects = egg_card.effect_list(None)
        od_effects = [e for e in effects if getattr(e, 'is_on_deletion', False)]
        assert len(od_effects) >= 1, "BT3-006 should have an On Deletion effect"
        assert od_effects[0].is_inherited_effect, "Effect must be inherited"

    def test_effect_timing_on_destroyed(self, debug_runner):
        """Effect timing should be OnDestroyedAnyone."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT3-006", "ST1-03"])

        egg_card = perm.card_sources[0]
        effects = egg_card.effect_list(None)
        od_effects = [e for e in effects if getattr(e, 'is_on_deletion', False)]
        assert od_effects[0].timing == EffectTiming.OnDestroyedAnyone

    def test_effect_is_not_optional(self, debug_runner):
        """Effect should NOT be optional (mandatory per C# reference).

        The C# SetUpActivateClass 4th argument is false, meaning this is
        a mandatory trigger. The player must resolve it.
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT3-006", "ST1-03"])

        egg_card = perm.card_sources[0]
        effects = egg_card.effect_list(None)
        od_effects = [e for e in effects if getattr(e, 'is_on_deletion', False)]
        assert not od_effects[0].is_optional, (
            "Effect should NOT be optional (mandatory trigger)"
        )


@pytest.mark.behavioral
class TestBT3006DrawThenTrash:
    """Verify the correct sequence: Draw 1 first, then trash 1."""

    def test_draw_1_then_trash_1_sequence(self, debug_runner):
        """After deletion, should draw 1 then trash 1 from hand.

        Net effect on hand: +1 (draw) - 1 (trash) = 0 net change,
        but the hand composition changes (drew a new card, trashed a different one).
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT3-006", "ST1-03"])

        game = runner.game
        player = game.player1

        # Set up a known hand
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-05", "hand")  # Greymon in hand

        hand_before = len(player.hand_cards)
        deck_before = len(player.library_cards)

        # Delete the permanent to trigger the inherited On Deletion
        player.delete_permanent(perm)
        runner.auto_resolve()

        hand_after = len(player.hand_cards)
        deck_after = len(player.library_cards)

        # Draw 1 (+1 hand, -1 deck) then trash 1 (-1 hand): net hand = 0, deck = -1
        assert hand_after == hand_before, (
            f"Hand should be unchanged (draw 1 then trash 1). "
            f"Before: {hand_before}, After: {hand_after}"
        )
        assert deck_after == deck_before - 1, (
            f"Deck should decrease by 1 (drew 1 card). "
            f"Before: {deck_before}, After: {deck_after}"
        )

    def test_trash_card_goes_to_trash_zone(self, debug_runner):
        """The trashed card should end up in the trash zone."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT3-006", "ST1-03"])

        game = runner.game
        player = game.player1

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-05", "hand")

        trash_before = len(player.trash_cards)

        player.delete_permanent(perm)
        runner.auto_resolve()

        trash_after = len(player.trash_cards)

        # Trash should increase: the deleted Digimon stack + 1 card trashed from hand
        # The permanent stack (BT3-006 + ST1-03) goes to trash on deletion,
        # plus 1 card trashed from hand effect
        assert trash_after > trash_before, (
            f"Trash should grow after deletion + hand trash. "
            f"Before: {trash_before}, After: {trash_after}"
        )


@pytest.mark.behavioral
class TestBT3006TrashFilter:
    """Trash selection should accept ANY card in hand (no filter)."""

    def test_trash_accepts_any_card_type(self, debug_runner):
        """The trash selection should accept any card — Digimon, Tamer, Option."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT3-006", "ST1-03"])

        egg_card = perm.card_sources[0]
        effects = egg_card.effect_list(None)
        od_effect = [e for e in effects if getattr(e, 'is_on_deletion', False)][0]

        game = runner.game
        player = game.player1

        # Inject a Tamer into hand
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT1-085", "hand")  # Tai Kamiya Tamer

        selected_cards = []

        def mock_select(p, filter_fn, callback, is_optional=False, prompt=None):
            for card in p.hand_cards:
                if filter_fn(card):
                    selected_cards.append(card)
        game.effect_select_hand_card = mock_select

        od_effect.on_process_callback({
            'player': player, 'game': game, 'permanent': perm,
        })

        assert len(selected_cards) >= 1, (
            "Trash filter should accept Tamer cards (any card type)"
        )

    def test_trash_is_mandatory(self, debug_runner):
        """The trash selection should be mandatory (is_optional=False).

        C# reference: canNoSelect: false, meaning the player MUST trash a card.
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT3-006", "ST1-03"])

        egg_card = perm.card_sources[0]
        effects = egg_card.effect_list(None)
        od_effect = [e for e in effects if getattr(e, 'is_on_deletion', False)][0]

        game = runner.game
        player = game.player1

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-05", "hand")

        optional_flag = None

        def mock_select(p, filter_fn, callback, is_optional=False, prompt=None):
            nonlocal optional_flag
            optional_flag = is_optional
        game.effect_select_hand_card = mock_select

        od_effect.on_process_callback({
            'player': player, 'game': game, 'permanent': perm,
        })

        assert optional_flag is False, (
            "Trash selection should be mandatory (is_optional=False)"
        )


@pytest.mark.behavioral
class TestBT3006EdgeCases:
    """Edge cases for empty deck and empty hand scenarios."""

    def test_draw_with_empty_deck_causes_deckout(self, debug_runner):
        """If deck is empty, the draw attempt causes a deck-out game loss.

        When draw_cards is called with an empty library, the engine declares
        a deck-out loss before the trash step can run. This is correct behavior:
        the mandatory draw triggers the loss condition.
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT3-006", "ST1-03"])

        game = runner.game
        player = game.player1

        # Clear deck and set up hand
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-05", "hand")
        runner.inject_card(1, "ST1-09", "hand")
        player.library_cards.clear()

        player.delete_permanent(perm)
        runner.auto_resolve()

        # With empty deck, draw attempt causes deck-out loss
        assert game.game_over, (
            "Game should end via deck-out when draw is attempted with empty deck"
        )

    def test_no_trash_when_hand_empty_after_draw(self, debug_runner):
        """If hand is empty and draw fails (empty deck), trash is skipped.

        C# checks `card.Owner.HandCards.Count >= 1` before trash selection.
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT3-006", "ST1-03"])

        game = runner.game
        player = game.player1

        runner.clear_zone(1, "hand")
        player.library_cards.clear()

        # Both hand and deck empty
        assert len(player.hand_cards) == 0
        assert len(player.library_cards) == 0

        # This should not crash — effect runs but draw fails, hand still empty, skip trash
        player.delete_permanent(perm)
        runner.auto_resolve()

        assert len(player.hand_cards) == 0, "Hand should remain empty"

    def test_inherited_triggers_on_carrier_deletion(self, debug_runner):
        """When a Digimon carrying BT3-006 is deleted, the inherited
        On Deletion effect should trigger: draw 1, trash 1."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        # Build a stack: BT3-006 (egg) -> ST1-03 (Lv.3) -> ST1-05 (Lv.4)
        perm = runner.place_on_field(1, ["BT3-006", "ST1-03", "ST1-05"])

        game = runner.game
        player = game.player1

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-09", "hand")

        hand_before = len(player.hand_cards)
        deck_before = len(player.library_cards)

        player.delete_permanent(perm)
        runner.auto_resolve()

        hand_after = len(player.hand_cards)
        deck_after = len(player.library_cards)

        # Draw 1 then trash 1 => net hand = 0, deck = -1
        assert hand_after == hand_before, (
            f"Inherited should fire on carrier deletion. "
            f"Hand: {hand_before} -> {hand_after} (expected {hand_before})"
        )
        assert deck_after == deck_before - 1, (
            f"Deck should decrease by 1. "
            f"Before: {deck_before}, After: {deck_after}"
        )
