"""Behavioral tests for BT21-007 Agumon (Lv.3 Red Reptile/LIBERATOR, DP 1000, Cost 3).

Card text:
  [On Play] You may return 1 Digimon card with the [Reptile] or [Dragonkin]
  trait from your trash to the hand.

  Inherited: [Your Turn] This Digimon gets +2000 DP.
"""

import pytest


# Use Tentomon (Insectoid) as filler so BT21-007 Agumon is easily identifiable
_FILLER_DECK = ["BT1-066"] * 40 + ["BT1-001"] * 5  # 40 Tentomon + 5 Digi-Eggs


def _play_bt21_007(runner):
    """Helper: inject BT21-007 into hand, clear other hand cards, and play it.

    Returns the hand index used. Clears the hand first so BT21-007 is
    the only playable Agumon.
    """
    runner.clear_zone(1, "hand")
    runner.inject_card(1, "BT21-007", "hand")
    # BT21-007 is now the only card in hand at index 0
    action = runner.find_action("Play Agumon")
    assert action is not None, (
        f"Should be able to play BT21-007. Available actions: {runner.actions()}"
    )
    runner.execute(action)
    return action


@pytest.mark.behavioral
class TestBT21007OnPlay:
    """Tests for [On Play] trash recovery effect."""

    def test_on_play_returns_reptile_from_trash(self, debug_runner):
        """Playing BT21-007 should allow returning a Reptile Digimon from trash to hand."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )
        runner.set_phase("Main")

        # Put a Reptile Digimon in P1's trash
        runner.inject_card(1, "BT1-010", "trash")  # Agumon (Reptile)

        _play_bt21_007(runner)

        # Should be in a selection phase (SelectTrash) for the On Play effect
        snap = runner.snapshot()
        phase = snap.phase
        assert phase in ("SelectTrash", "SelectTarget"), (
            f"Expected selection phase after playing BT21-007, got {phase}. "
            f"Actions: {runner.actions()}"
        )

        # Select the Reptile Digimon from trash
        legal = runner.action_mask()
        # Filter out the pass/decline action (62)
        trash_selections = [a for a in legal if a != 62]
        assert len(trash_selections) >= 1, (
            f"Should have at least 1 trash selection option, got {legal}"
        )
        runner.execute(trash_selections[0])
        runner.auto_resolve()

        snap = runner.snapshot()
        # The Reptile Digimon (BT1-010) should now be in hand
        assert "BT1-010" in snap.p1_hand, (
            f"BT1-010 should be returned to hand. Hand: {snap.p1_hand}, Trash: {snap.p1_trash}"
        )
        # BT1-010 should no longer be in trash
        assert "BT1-010" not in snap.p1_trash, (
            f"BT1-010 should be removed from trash. Trash: {snap.p1_trash}"
        )

    def test_on_play_returns_dragonkin_from_trash(self, debug_runner):
        """On Play should also work with Dragonkin trait Digimon."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )
        runner.set_phase("Main")

        # Put a Dragonkin Digimon in P1's trash
        runner.inject_card(1, "BT1-025", "trash")  # WarGreymon (Dragonkin)

        _play_bt21_007(runner)

        # Select the Dragonkin from trash
        legal = runner.action_mask()
        trash_selections = [a for a in legal if a != 62]
        assert len(trash_selections) >= 1, (
            f"Should have Dragonkin selection option. Legal: {legal}, Actions: {runner.actions()}"
        )
        runner.execute(trash_selections[0])
        runner.auto_resolve()

        snap = runner.snapshot()
        assert "BT1-025" in snap.p1_hand, (
            f"BT1-025 WarGreymon should be returned to hand. Hand: {snap.p1_hand}, Trash: {snap.p1_trash}"
        )

    def test_on_play_filters_non_matching_traits(self, debug_runner):
        """Non-Reptile/non-Dragonkin Digimon in trash should NOT be selectable."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )
        runner.set_phase("Main")

        # Put an Insectoid Digimon in P1's trash (not Reptile or Dragonkin)
        runner.inject_card(1, "BT1-066", "trash")  # Tentomon (Insectoid)

        _play_bt21_007(runner)
        runner.auto_resolve()

        # Should skip the selection entirely (no valid targets) and go to Main
        snap = runner.snapshot()
        # Tentomon should still be in trash (not returned)
        assert "BT1-066" in snap.p1_trash, (
            f"Non-matching Tentomon should remain in trash. Trash: {snap.p1_trash}"
        )

    def test_on_play_is_optional(self, debug_runner):
        """The On Play effect is optional -- player may decline."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )
        runner.set_phase("Main")

        runner.inject_card(1, "BT1-010", "trash")  # Reptile Digimon

        _play_bt21_007(runner)

        # Should be in selection phase; action 62 should be available (decline)
        legal = runner.action_mask()
        assert 62 in legal, f"Decline (62) should be available. Legal: {legal}"
        runner.execute(62)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Reptile should still be in trash (we declined)
        assert "BT1-010" in snap.p1_trash, (
            f"BT1-010 should remain in trash after declining. Trash: {snap.p1_trash}"
        )

    def test_on_play_does_not_return_non_digimon(self, debug_runner):
        """Option/Tamer cards should NOT be selectable (only Digimon cards qualify)."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )
        runner.set_phase("Main")

        # Put a Tamer in trash
        runner.inject_card(1, "BT1-085", "trash")  # Tai Kamiya (Tamer)

        _play_bt21_007(runner)
        runner.auto_resolve()

        # No valid selection (Tamer is not a Digimon) -- should skip to Main
        snap = runner.snapshot()
        assert "BT1-085" in snap.p1_trash, (
            f"Tamer should remain in trash. Trash: {snap.p1_trash}"
        )

    def test_on_play_with_mixed_trash(self, debug_runner):
        """Only Reptile/Dragonkin Digimon should be selectable, not other cards in trash."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )
        runner.set_phase("Main")

        # Mix of cards in trash
        runner.inject_card(1, "BT1-066", "trash")  # Tentomon (Insectoid) - NOT valid
        runner.inject_card(1, "BT1-010", "trash")  # Agumon (Reptile) - valid
        runner.inject_card(1, "BT1-085", "trash")  # Tai Kamiya (Tamer) - NOT valid

        _play_bt21_007(runner)

        # Should be in selection phase
        snap = runner.snapshot()
        assert snap.phase in ("SelectTrash", "SelectTarget"), (
            f"Expected selection phase, got {snap.phase}"
        )

        legal = runner.action_mask()
        trash_selections = [a for a in legal if a != 62]
        # Only 1 valid target (BT1-010 Agumon) should be selectable
        assert len(trash_selections) == 1, (
            f"Expected exactly 1 selectable trash card (BT1-010), got {len(trash_selections)}. "
            f"Legal: {legal}, Actions: {runner.actions()}"
        )

        runner.execute(trash_selections[0])
        runner.auto_resolve()

        snap = runner.snapshot()
        assert "BT1-010" in snap.p1_hand, (
            f"BT1-010 should be returned to hand. Hand: {snap.p1_hand}"
        )
        # Others remain in trash
        assert "BT1-066" in snap.p1_trash, "Tentomon should remain in trash"
        assert "BT1-085" in snap.p1_trash, "Tamer should remain in trash"


@pytest.mark.behavioral
class TestBT21007InheritedDP:
    """Tests for inherited [Your Turn] +2000 DP effect."""

    def test_inherited_dp_boost_on_own_turn(self, debug_runner):
        """When inherited, this Digimon should get +2000 DP during your turn."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )
        runner.set_phase("Main")

        # Place a Digimon with BT21-007 as an inherited source
        # Stack: BT21-007 (bottom) + BT1-010 (top) -- simulating digivolution
        perm = runner.place_on_field(1, ["BT21-007", "BT1-010"])

        snap = runner.snapshot()
        # BT1-010 has 2000 base DP; with +2000 inherited = 4000 on your turn
        field_slot = snap.p1_field[0]
        assert field_slot.dp == 4000, (
            f"Expected 4000 DP (2000 base + 2000 inherited), got {field_slot.dp}"
        )

    def test_inherited_dp_no_boost_on_opponent_turn(self, debug_runner):
        """Inherited DP boost should NOT apply during opponent's turn."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
            first_player=2,  # P2 goes first -> P1's is_my_turn = False
        )
        runner.set_phase("Main")

        # Place a Digimon with BT21-007 inherited on P1's side
        perm = runner.place_on_field(1, ["BT21-007", "BT1-010"])

        snap = runner.snapshot()
        # On P2's turn, P1's inherited +2000 should NOT apply
        field_slot = snap.p1_field[0]
        assert field_slot.dp == 2000, (
            f"Expected 2000 DP (no inherited boost on opponent's turn), got {field_slot.dp}"
        )
