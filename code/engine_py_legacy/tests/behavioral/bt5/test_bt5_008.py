"""Behavioral tests for BT5-008 Gaossmon (Lv.3 Red Ceratopsian, DP 2000, Cost 3).

Card text:
  [Your Turn] Your other [Gaossmon] all get +3000 DP.
  [Opponent's Turn] Your opponent can't reduce digivolution costs.

C# reference:
  - ChangeDPStaticEffect: +3000 to other owner's Gaossmon (Your Turn only)
  - CannotReduceCostClass: opponent can't reduce evo costs (Opponent's Turn)
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT5008Gaossmon:
    """Tests for BT5-008 Gaossmon."""

    def test_dp_aura_applies_to_other_gaossmon(self, debug_runner):
        """Another Gaossmon should get +3000 DP from this card's aura."""
        runner = debug_runner(initial_memory=5)
        perm1 = runner.place_on_field(1, ["BT5-008"])  # This Gaossmon
        perm2 = runner.place_on_field(1, ["BT5-008"])  # Another Gaossmon
        game = runner.game

        # It's player 1's turn
        assert game.player1.is_my_turn

        # perm2 should get +3000 DP from perm1 (and vice versa)
        # Base DP for BT5-008 is 2000
        # Each should get +3000 from the other = 5000
        assert perm2.dp == 5000, f"Other Gaossmon should have 2000+3000=5000 DP, got {perm2.dp}"
        assert perm1.dp == 5000, f"This Gaossmon should also have 2000+3000=5000 DP from the other, got {perm1.dp}"

    def test_dp_aura_does_not_apply_to_self(self, debug_runner):
        """A lone Gaossmon should not boost itself."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT5-008"])

        # Base DP 2000, no self-boost
        assert perm.dp == 2000, f"Lone Gaossmon should have 2000 DP, got {perm.dp}"

    def test_dp_aura_does_not_apply_to_non_gaossmon(self, debug_runner):
        """A non-Gaossmon Digimon should not get the +3000 DP."""
        runner = debug_runner(initial_memory=5)
        perm1 = runner.place_on_field(1, ["BT5-008"])  # Gaossmon
        perm2 = runner.place_on_field(1, ["ST1-07"])   # Greymon (not Gaossmon)
        game = runner.game

        # ST1-07 Greymon has base DP 4000, should NOT get boost
        assert perm2.dp == 4000, f"Greymon should have 4000 DP (no boost), got {perm2.dp}"

    def test_dp_aura_only_on_your_turn(self, debug_runner):
        """Aura should only apply during [Your Turn]."""
        runner = debug_runner(initial_memory=5)
        perm1 = runner.place_on_field(1, ["BT5-008"])
        perm2 = runner.place_on_field(1, ["BT5-008"])
        game = runner.game

        # Verify boost on your turn
        assert game.player1.is_my_turn
        assert perm2.dp == 5000

        # Switch turns: it's now opponent's turn for P1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        # The aura condition checks card.owner.is_my_turn
        # Should no longer apply
        assert perm2.dp == 2000, (
            f"Gaossmon DP boost should not apply on opponent's turn, got {perm2.dp}"
        )

    def test_opponent_cost_reduction_prevention_tagged(self, debug_runner):
        """The opponent cost reduction prevention effect should exist (descriptive-tagged)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT5-008"])

        card = perm.top_card
        effects = card.effect_list(None)
        # Look for the cost reduction prevention effect
        cost_prevent = [e for e in effects
                        if "reduce" in e.effect_description.lower()
                        and "digivolution" in e.effect_description.lower()]
        assert len(cost_prevent) >= 1, "Should have cost reduction prevention effect"
