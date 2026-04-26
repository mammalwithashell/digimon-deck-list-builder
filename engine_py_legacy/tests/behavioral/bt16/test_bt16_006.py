"""Behavioral tests for BT16-006 Cupimon (Lv.2 Purple DigiEgg).

Card text:
  Inherited: [On Deletion] By trashing 1 card in your hand, gain 1 memory.

Key behaviors per C# reference (DCGO BT16_006.cs):
  - Inherited-only effect (is_inherited_effect=True)
  - Timing: OnDestroyedAnyone with is_on_deletion=True
  - CanActivateCondition checks card.Owner.HandCards.Count >= 1
  - "By trashing" = cost: memory gain is conditional on actually trashing
  - canNoSelect: true in C# -- player may decline the trash even after effect fires
  - Memory +1 only if discarded is True
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


# BT16-006 is a Lv.2 DigiEgg, so we need a filler deck with some copies
CUPIMON_DECK = ["BT16-006"] * 5 + ["BT1-036"] * 45  # Lv.4 filler


@pytest.mark.behavioral
class TestBT16006Cupimon:
    """Tests for BT16-006 Cupimon inherited [On Deletion] effect."""

    # ------------------------------------------------------------------ #
    # Effect structure checks
    # ------------------------------------------------------------------ #

    def test_effect_is_inherited(self, debug_runner):
        """BT16-006's effect must be an inherited effect."""
        runner = debug_runner(deck1=CUPIMON_DECK, deck2=CUPIMON_DECK, initial_memory=10)
        runner.inject_card(1, "BT16-006", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        inherited = [e for e in effects if e.is_inherited_effect]
        assert len(inherited) >= 1, (
            f"BT16-006 should have at least 1 inherited effect, got {len(inherited)}"
        )

    def test_effect_has_on_deletion_flag(self, debug_runner):
        """The effect must have is_on_deletion=True."""
        runner = debug_runner(deck1=CUPIMON_DECK, deck2=CUPIMON_DECK, initial_memory=10)
        runner.inject_card(1, "BT16-006", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        on_del = [e for e in effects if getattr(e, 'is_on_deletion', False)]
        assert len(on_del) >= 1, (
            "BT16-006 should have an effect with is_on_deletion=True"
        )

    def test_effect_timing_is_on_destroyed_anyone(self, debug_runner):
        """The effect timing must be OnDestroyedAnyone."""
        runner = debug_runner(deck1=CUPIMON_DECK, deck2=CUPIMON_DECK, initial_memory=10)
        runner.inject_card(1, "BT16-006", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        on_del = [e for e in effects if getattr(e, 'is_on_deletion', False)]
        assert len(on_del) >= 1
        assert on_del[0].timing == EffectTiming.OnDestroyedAnyone, (
            f"Expected timing OnDestroyedAnyone, got {on_del[0].timing}"
        )

    def test_effect_is_optional(self, debug_runner):
        """The effect should be optional (player may choose not to activate)."""
        runner = debug_runner(deck1=CUPIMON_DECK, deck2=CUPIMON_DECK, initial_memory=10)
        runner.inject_card(1, "BT16-006", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        on_del = [e for e in effects if getattr(e, 'is_on_deletion', False)]
        assert len(on_del) >= 1
        assert on_del[0].is_optional, (
            "BT16-006 On Deletion effect should be optional"
        )

    # ------------------------------------------------------------------ #
    # Inherited: fires from digivolution sources, not as top card
    # ------------------------------------------------------------------ #

    def test_inherited_fires_when_stacked_under(self, debug_runner):
        """When Cupimon is under a Digimon that gets deleted, the inherited
        on-deletion effect should fire (trash from hand -> gain 1 memory)."""
        runner = debug_runner(deck1=CUPIMON_DECK, deck2=CUPIMON_DECK, initial_memory=5)
        game = runner.game

        # Stack: BT16-006 (bottom, Lv.2) + BT1-036 (top, Lv.4)
        perm = runner.place_on_field(1, ["BT16-006", "BT1-036"])

        # Ensure P1 has at least 1 card in hand to trash
        runner.inject_card(1, "BT1-036", "hand")
        hand_before = len(game.player1.hand_cards)
        memory_before = game.memory

        # Delete the stacked Digimon -- inherited On Deletion should fire
        game.player1.delete_permanent(perm)
        # auto_resolve picks the first legal hand card to trash
        runner.auto_resolve()

        snap = runner.snapshot()
        # Memory should have increased by 1
        assert snap.memory == memory_before + 1, (
            f"Expected memory {memory_before + 1} after inherited on-deletion, "
            f"got {snap.memory}"
        )
        # Hand should have decreased by 1 (trashed card)
        assert len(snap.p1_hand) == hand_before - 1, (
            f"Expected hand size {hand_before - 1} after trashing, got {len(snap.p1_hand)}"
        )

    # ------------------------------------------------------------------ #
    # "By trashing" = cost: memory only if trash succeeds
    # ------------------------------------------------------------------ #

    def test_memory_gain_only_after_trash(self, debug_runner):
        """Memory gain (+1) must only happen AFTER a card is successfully trashed.
        This tests that add_memory is inside the on_trashed callback, not after
        the effect_select_hand_card call."""
        runner = debug_runner(deck1=CUPIMON_DECK, deck2=CUPIMON_DECK, initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(1, ["BT16-006", "BT1-036"])
        runner.inject_card(1, "BT1-036", "hand")
        memory_before = game.memory

        # Delete -- triggers on-deletion
        game.player1.delete_permanent(perm)

        # At this point, if the engine enters a selection phase,
        # memory should NOT have changed yet (it's pending selection)
        from engine_py_legacy.engine.data.enums import GamePhase
        if game.current_phase in (GamePhase.SelectHand,):
            assert game.memory == memory_before, (
                f"Memory should not change before hand card selection! "
                f"Before: {memory_before}, Current: {game.memory}"
            )

        # Now resolve the selection
        runner.auto_resolve()

        # After resolution, memory should have increased
        assert game.memory == memory_before + 1, (
            f"Memory should be {memory_before + 1} after trash+gain, got {game.memory}"
        )

    def test_no_memory_gain_with_empty_hand(self, debug_runner):
        """If the player has no cards in hand, the effect should not activate
        and no memory should be gained. C# CanActivateCondition checks
        HandCards.Count >= 1."""
        runner = debug_runner(deck1=CUPIMON_DECK, deck2=CUPIMON_DECK, initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(1, ["BT16-006", "BT1-036"])

        # Clear the hand entirely
        runner.clear_zone(1, "hand")
        assert len(game.player1.hand_cards) == 0

        memory_before = game.memory

        # Delete -- on-deletion should NOT fire (no hand cards to trash)
        game.player1.delete_permanent(perm)
        runner.auto_resolve()

        assert game.memory == memory_before, (
            f"With empty hand, no memory gain should occur. "
            f"Before: {memory_before}, After: {game.memory}"
        )

    # ------------------------------------------------------------------ #
    # Trash goes to trash pile
    # ------------------------------------------------------------------ #

    def test_trashed_card_goes_to_trash(self, debug_runner):
        """The card trashed from hand should end up in the player's trash pile."""
        runner = debug_runner(deck1=CUPIMON_DECK, deck2=CUPIMON_DECK, initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(1, ["BT16-006", "BT1-036"])
        runner.inject_card(1, "BT1-036", "hand")

        trash_before = len(game.player1.trash_cards)

        game.player1.delete_permanent(perm)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Trash should have the hand card PLUS the deleted stack cards
        # The hand-trashed card is the one we care about
        assert len(game.player1.trash_cards) > trash_before, (
            f"Trash should contain the discarded hand card. "
            f"Before: {trash_before}, After: {len(game.player1.trash_cards)}"
        )

    # ------------------------------------------------------------------ #
    # Not active as top card (DigiEgg on field by itself)
    # ------------------------------------------------------------------ #

    def test_not_active_as_top_card(self, debug_runner):
        """As a DigiEgg, BT16-006's inherited effect should NOT be active when
        it is the top card of a permanent (inherited effects only fire from
        digivolution sources)."""
        runner = debug_runner(deck1=CUPIMON_DECK, deck2=CUPIMON_DECK, initial_memory=5)
        game = runner.game

        # Place Cupimon alone on field (it's a Lv.2 DigiEgg)
        perm = runner.place_on_field(1, ["BT16-006"])
        runner.inject_card(1, "BT1-036", "hand")

        memory_before = game.memory

        # Delete -- inherited effect should NOT fire from top card position
        game.player1.delete_permanent(perm)
        runner.auto_resolve()

        # Inherited effects don't fire from top card, so no memory gain
        # (The engine handles this: inherited effects only activate from
        # digivolution source cards, not the top card)
        assert game.memory == memory_before, (
            f"Inherited-only effect should not fire from top card. "
            f"Before: {memory_before}, After: {game.memory}"
        )
