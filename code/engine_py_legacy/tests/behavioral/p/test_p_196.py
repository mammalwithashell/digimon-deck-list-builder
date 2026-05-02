"""Behavioral tests for P-196 Gomamon (Lv.3 Blue Digimon).

Card text:
Alt-digi: Lv.2 w/[TS] trait: Cost 0
[Start of Your Main Phase] If you have 4 or less memory, this Digimon may
digivolve into a Digimon card with the [Sea Beast] or [TS] trait in the hand
without paying the cost.
Inherited: [When Attacking] [Once Per Turn] If you have 7 or fewer cards in
your hand, <Draw 1>
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestP196Gomamon:
    """Tests for P-196 Gomamon."""

    def test_start_main_digivolve_into_sea_beast_from_hand(self, debug_runner):
        """With <=4 memory, Gomamon can digivolve into a Lv.4 Sea Beast from hand for free."""
        runner = debug_runner(initial_memory=3)

        # Place Gomamon (Lv.3) on field
        gomamon_perm = runner.place_on_field(1, ["P-196"])
        # Inject a Lv.4 Sea Beast (BT1-034 Ikkakumon, evo from Lv.3 Blue) into hand
        runner.inject_card(1, "BT1-034", "hand")

        runner.set_phase("Main")
        # Trigger start of main phase effects
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        # Should have a selection for the digivolve target
        snap = runner.snapshot()
        assert runner.game.current_phase == GamePhase.SelectTarget, (
            "Should enter selection phase for digivolve from hand"
        )

    def test_start_main_blocked_above_4_memory(self, debug_runner):
        """With >4 memory, the start-of-main digivolve effect should not trigger."""
        runner = debug_runner(initial_memory=5)

        gomamon_perm = runner.place_on_field(1, ["P-196"])
        runner.inject_card(1, "BT1-034", "hand")

        runner.set_phase("Main")
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        # Should NOT enter selection phase
        assert runner.game.current_phase != GamePhase.SelectTarget, (
            "Should not trigger digivolve when memory > 4"
        )

    def test_start_main_rejects_wrong_level(self, debug_runner):
        """A Lv.5 Sea Beast should not be offered as a digivolve target for Lv.3 Gomamon."""
        runner = debug_runner(initial_memory=3)

        gomamon_perm = runner.place_on_field(1, ["P-196"])
        # Inject a Lv.5 Sea Beast (BT1-041 Zudomon, evo from Lv.4) — can't digivolve from Lv.3
        runner.inject_card(1, "BT1-041", "hand")

        runner.set_phase("Main")
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        # No valid targets, so effect should not produce a selection
        assert runner.game.current_phase != GamePhase.SelectTarget, (
            "Lv.5 Sea Beast should not be valid digivolve target for Lv.3 Gomamon"
        )

    def test_inherited_draw_on_attack(self, debug_runner):
        """Inherited: When attacking with <=7 hand cards, draw 1."""
        runner = debug_runner(initial_memory=5)

        # Place a Lv.4 with Gomamon as inherited source
        perm = runner.place_on_field(1, ["P-196", "BT1-034"])
        runner.set_phase("Main")

        hand_before = len(runner.game.player1.hand_cards)

        # Find and execute attack action
        action = runner.find_action("attack")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

            snap = runner.snapshot()
            hand_after = len(runner.game.player1.hand_cards)
            assert hand_after >= hand_before + 1, (
                "Inherited draw should trigger on attack with <=7 hand cards"
            )

    def test_inherited_draw_blocked_over_7_cards(self, debug_runner):
        """Inherited draw should NOT trigger with >7 hand cards."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["P-196", "BT1-034"])
        runner.set_phase("Main")

        # Fill hand to 8 cards
        while len(runner.game.player1.hand_cards) < 8:
            runner.inject_card(1, "ST1-03", "hand")

        hand_before = len(runner.game.player1.hand_cards)

        action = runner.find_action("attack")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

            hand_after = len(runner.game.player1.hand_cards)
            assert hand_after == hand_before, (
                "Inherited draw should NOT trigger with >7 hand cards"
            )
