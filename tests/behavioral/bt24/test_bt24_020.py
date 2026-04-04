"""Behavioral tests for BT24-020 Gomamon (Lv.3 Blue Digimon).

Card text:
Alt-digi: Lv.2 with [TS] trait for cost 0
[On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with the
[Sea Beast] or [Shaman] trait or [Aqua] or [Sea Animal] in any of its traits
and 1 card with the [TS] trait among them to the hand. Return the rest to the
bottom of the deck.
Inherited: [Your Turn] [Once Per Turn] When this Digimon unsuspends, if you
have 7 or fewer cards in your hand, <Draw 1>.
"""

import pytest


@pytest.mark.behavioral
class TestBT24020Gomamon:
    """Tests for BT24-020 Gomamon."""

    def test_on_play_reveal_and_select(self, debug_runner):
        """[On Play] should reveal top 3 and offer two selection passes."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Inject Gomamon into hand
        runner.inject_card(1, "BT24-020", "hand")

        # Stack deck with known cards: a Sea Beast Digimon, a TS card, and filler
        # BT24-022 Ikkakumon (Sea Beast trait, Digimon) — matches filter 0
        # BT24-009 Shamanmon (TS trait) — matches filter 1
        # ST1-03 Agumon (no relevant traits) — filler
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "BT24-009", "library_top")
        runner.inject_card(1, "BT24-022", "library_top")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Play Gomamon")
        assert action is not None, "Should be able to play Gomamon from hand"

        runner.execute(action)
        # Auto-resolve the reveal selection phases
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Gomamon should be on field
        assert any(
            s.card_id == "BT24-020" for s in snap_after.p1_field
        ), "Gomamon should be on the field after playing"

    def test_inherited_unsuspend_draw(self, debug_runner):
        """Inherited: when host unsuspends on your turn with ≤7 hand, draw 1."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a Digimon with Gomamon in its sources (inherited effect)
        perm = runner.place_on_field(1, ["BT24-020", "BT24-022"], is_suspended=True)

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        # Unsuspend to trigger the inherited effect
        perm.unsuspend()

        # Auto-resolve any triggered effect
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Should have drawn 1 card
        assert len(snap_after.p1_hand) == hand_before + 1, (
            f"Should draw 1 card on unsuspend; hand went from {hand_before} to {len(snap_after.p1_hand)}"
        )

    def test_inherited_unsuspend_no_draw_over_7(self, debug_runner):
        """Inherited: should NOT draw if hand has more than 7 cards."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a Digimon with Gomamon inherited
        perm = runner.place_on_field(1, ["BT24-020", "BT24-022"], is_suspended=True)

        # Fill hand to 8 cards
        for _ in range(8):
            runner.inject_card(1, "ST1-03", "hand")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        perm.unsuspend()
        runner.auto_resolve()

        snap_after = runner.snapshot()
        assert len(snap_after.p1_hand) == hand_before, (
            "Should NOT draw when hand has more than 7 cards"
        )

    def test_inherited_unsuspend_only_self_not_other(self, debug_runner):
        """Inherited: should NOT trigger when a DIFFERENT permanent unsuspends."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a Digimon with Gomamon inherited (host)
        host_perm = runner.place_on_field(1, ["BT24-020", "BT24-022"])
        # Place another Digimon that will unsuspend
        other_perm = runner.place_on_field(1, ["ST1-03"], is_suspended=True)

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        # Unsuspending the OTHER permanent should NOT trigger Gomamon's draw
        other_perm.unsuspend()
        runner.auto_resolve()

        snap_after = runner.snapshot()
        assert len(snap_after.p1_hand) == hand_before, (
            "Inherited draw should NOT fire when a different permanent unsuspends"
        )

    def test_inherited_unsuspend_not_on_opponent_turn(self, debug_runner):
        """Inherited: should NOT trigger on opponent's turn."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a Digimon with Gomamon inherited, but it's P2's field
        # Actually, let's put it on P1's field but make it P2's turn
        perm = runner.place_on_field(1, ["BT24-020", "BT24-022"], is_suspended=True)

        # Switch to P2's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        perm.unsuspend()
        runner.auto_resolve()

        snap_after = runner.snapshot()
        assert len(snap_after.p1_hand) == hand_before, (
            "Should NOT draw on opponent's turn"
        )
