"""Behavioral tests for ST19-05 PawnChessmon.

Card text:
  <Blocker>
  [On Deletion] By trashing 1 card with the [Puppet] trait in your hand,
  <Draw 2>.

Clauses:
  C1: <Blocker> keyword
  C2: [On Deletion] triggers when this Digimon is deleted
  C3: Trash selection targets only [Puppet] trait cards in hand
  C4: Trashing is optional (canNoSelect: true in C#) — player can decline
  C5: Draw 2 only happens if a card was actually trashed
"""

import pytest


# ── Helpers ──────────────────────────────────────────────────────────────


def _make_runner(debug_runner, hand1=None, hand2=None):
    """Create a runner with ST19-05 on P1's field, some Puppet cards in hand."""
    # Use a minimal deck: lots of filler + the cards we need
    filler = ["BT1-010"] * 40  # Agumon (non-Puppet Lv3)
    deck1 = ["ST19-05"] * 4 + ["ST19-02"] * 4 + filler  # ST19-02 = Junkmon (Puppet)
    deck2 = ["BT1-010"] * 4 + filler

    runner = debug_runner(
        deck1=deck1,
        deck2=deck2,
        skip_shuffle=True,
        auto_mulligan=True,
        initial_memory=10,
        starting_hand1=hand1,
        starting_hand2=hand2,
    )
    return runner


class TestST19_05Blocker:
    """C1: <Blocker> keyword."""

    def test_blocker_keyword_present(self, debug_runner):
        """ST19-05 placed on field should have the blocker keyword."""
        runner = _make_runner(debug_runner)
        perm = runner.place_on_field(1, ["ST19-05"])
        assert perm.has_keyword("_is_blocker"), "PawnChessmon should have Blocker"


class TestST19_05OnDeletion:
    """C2-C5: [On Deletion] trash Puppet -> Draw 2."""

    def test_on_deletion_trash_puppet_draw_2(self, debug_runner):
        """C2+C3+C5: When deleted with a Puppet in hand, trash it and draw 2."""
        runner = _make_runner(debug_runner, hand1=["ST19-02", "BT1-010"])
        perm = runner.place_on_field(1, ["ST19-05"])

        p1 = runner.game.player1
        hand_before = len(p1.hand_cards)
        deck_before = len(p1.library_cards)
        trash_before = len(p1.trash_cards)

        # Delete the permanent directly
        p1.delete_permanent(perm)

        # The on-deletion effect should trigger and enter a selection phase
        # We need to resolve the hand selection (pick the Puppet card)
        snap = runner.snapshot()
        if runner.game.current_phase.name == "SelectHand":
            # Find and select the Puppet card (ST19-02)
            actions = runner.actions()
            puppet_action = None
            for aid, desc in actions.items():
                if "ST19-02" in desc or "Junkmon" in desc:
                    puppet_action = aid
                    break
            # If no direct match, just pick first valid action
            if puppet_action is None:
                legal = runner.action_mask()
                puppet_action = legal[0]
            runner.execute(puppet_action)

        # After resolution:
        # - 1 Puppet trashed from hand (hand_before - 1)
        # - 2 cards drawn (hand_before - 1 + 2 = hand_before + 1)
        # - trash should have the ST19-05 stack + the trashed Puppet
        assert len(p1.hand_cards) == hand_before + 1, (
            f"Expected hand size {hand_before + 1} (lost 1 Puppet, drew 2), "
            f"got {len(p1.hand_cards)}"
        )
        # Trash: deleted perm's cards + trashed Puppet from hand
        puppet_in_trash = any(
            c.c_entity_base and c.c_entity_base.card_id == "ST19-02"
            for c in p1.trash_cards
        )
        assert puppet_in_trash, "The trashed Puppet (ST19-02) should be in trash"

    def test_on_deletion_no_puppet_in_hand_no_trigger(self, debug_runner):
        """C3: If no Puppet trait cards in hand, the effect should not trigger."""
        runner = _make_runner(debug_runner, hand1=["BT1-010", "BT1-010"])
        perm = runner.place_on_field(1, ["ST19-05"])

        p1 = runner.game.player1
        hand_before = len(p1.hand_cards)
        deck_before = len(p1.library_cards)

        # Delete the permanent
        p1.delete_permanent(perm)

        # No Puppet in hand -> condition fails -> no selection phase
        assert runner.game.current_phase.name != "SelectHand", (
            "Should not enter SelectHand phase when no Puppet in hand"
        )
        # Hand size unchanged (no draw)
        assert len(p1.hand_cards) == hand_before, (
            f"Hand size should be unchanged ({hand_before}), got {len(p1.hand_cards)}"
        )

    def test_on_deletion_only_puppet_cards_selectable(self, debug_runner):
        """C3: Only [Puppet] trait cards should be selectable for trashing."""
        runner = _make_runner(debug_runner, hand1=["ST19-02", "BT1-010", "BT1-010"])
        perm = runner.place_on_field(1, ["ST19-05"])

        p1 = runner.game.player1

        # Delete the permanent
        p1.delete_permanent(perm)

        # Should be in SelectHand phase
        assert runner.game.current_phase.name == "SelectHand", (
            f"Expected SelectHand phase, got {runner.game.current_phase.name}"
        )

        # Check that only the Puppet card (ST19-02) is in the valid actions
        actions = runner.actions()
        # Count how many hand selection actions are available
        from digimon_gym.engine.game.constants import SEL_HAND_START
        hand_actions = {aid: desc for aid, desc in actions.items()
                        if aid >= SEL_HAND_START and aid < SEL_HAND_START + 50}

        # Should have exactly 1 selectable card (the Puppet) + possibly a pass/decline
        selectable_count = sum(1 for aid in hand_actions if "Pass" not in str(hand_actions[aid]))
        assert selectable_count == 1, (
            f"Expected exactly 1 selectable Puppet card, got {selectable_count}. "
            f"Actions: {hand_actions}"
        )

    def test_on_deletion_can_decline_trash(self, debug_runner):
        """C4: Player can choose not to trash (canNoSelect: true in C#).

        The 'By trashing' is an optional cost. If declined, no draw happens.
        """
        runner = _make_runner(debug_runner, hand1=["ST19-02", "BT1-010"])
        perm = runner.place_on_field(1, ["ST19-05"])

        p1 = runner.game.player1
        hand_before = len(p1.hand_cards)
        deck_before = len(p1.library_cards)

        # Delete the permanent
        p1.delete_permanent(perm)

        # Should be in SelectHand phase with is_optional=True
        assert runner.game.current_phase.name == "SelectHand", (
            f"Expected SelectHand phase, got {runner.game.current_phase.name}"
        )

        # Find the pass/decline action
        pass_action = runner.find_action("pass")
        if pass_action is None:
            pass_action = runner.find_action("decline")
        if pass_action is None:
            # Action 0 is typically the pass action
            pass_action = 0

        runner.execute(pass_action)

        # After declining: no trash, no draw -> hand unchanged
        assert len(p1.hand_cards) == hand_before, (
            f"Expected hand size {hand_before} (declined trash), "
            f"got {len(p1.hand_cards)}"
        )
        assert len(p1.library_cards) == deck_before, (
            f"Expected deck size {deck_before} (no draw), "
            f"got {len(p1.library_cards)}"
        )

    def test_on_deletion_draw_only_when_trashed(self, debug_runner):
        """C5: Draw 2 only happens if a Puppet was actually trashed."""
        runner = _make_runner(debug_runner, hand1=["ST19-02", "BT1-010"])
        perm = runner.place_on_field(1, ["ST19-05"])

        p1 = runner.game.player1
        deck_before = len(p1.library_cards)

        # Delete and select the Puppet to trash
        p1.delete_permanent(perm)

        # Resolve the selection
        if runner.game.current_phase.name == "SelectHand":
            legal = runner.action_mask()
            # Pick first non-pass action (should be the Puppet card)
            from digimon_gym.engine.game.constants import SEL_HAND_START
            puppet_actions = [a for a in legal if a >= SEL_HAND_START]
            if puppet_actions:
                runner.execute(puppet_actions[0])

        # 2 cards should have been drawn
        assert len(p1.library_cards) == deck_before - 2, (
            f"Expected deck to lose 2 cards from draw, "
            f"got {deck_before} -> {len(p1.library_cards)}"
        )
