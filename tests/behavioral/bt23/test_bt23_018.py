"""Behavioral tests for BT23-018 Garurumon (Lv.4 Blue, Cost 6).

Card text (per cards.json / C# source of truth):
Alt digivolve: Lv.3 w/[Gabumon] in name or w/[CS] trait: Cost 2
Jamming.
[Main] [Once Per Turn] By placing this Digimon's top stacked card as its bottom
    digivolution card, you may play 1 [Agumon] or [Nokia Shiramine] from your
    hand with the play cost reduced by 2.
Inherited: [Opponent's Turn] This Digimon gets +2000 DP.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT23018Garurumon:
    """Tests for BT23-018 Garurumon."""

    # ---- Jamming keyword ----

    def test_has_jamming(self, debug_runner):
        """BT23-018 should have the Jamming keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT23-018"])

        effects = perm.top_card.effect_list(None)
        jamming_effects = [e for e in effects if getattr(e, '_is_jamming', False)]
        assert len(jamming_effects) >= 1, "BT23-018 should have Jamming keyword"

    # ---- [Main][OPT] stack shift + play effect ----

    def test_main_condition_requires_stacked_cards(self, debug_runner):
        """The main effect should require at least one stacked card (2+ card_sources)."""
        runner = debug_runner(initial_memory=5)
        # Single card on field (no sources beneath)
        perm = runner.place_on_field(1, ["BT23-018"])

        effects = perm.top_card.effect_list(None)
        main_effects = [e for e in effects if getattr(e, '_is_field_main', False)]
        assert len(main_effects) == 1, "Should have exactly 1 field_main effect"

        main_eff = main_effects[0]
        ctx = {'game': runner.game, 'player': runner.game.player1, 'permanent': perm}
        assert not main_eff.can_use_condition(ctx), (
            "Main effect should NOT be usable with only 1 card in stack"
        )

    def test_main_condition_passes_with_stacked_cards(self, debug_runner):
        """The main effect should be usable when there are digivolution sources."""
        runner = debug_runner(initial_memory=5)
        # Place with a digivolution source (Lv.3 + Lv.4)
        perm = runner.place_on_field(1, ["ST2-04", "BT23-018"])

        effects = perm.top_card.effect_list(None)
        main_effects = [e for e in effects if getattr(e, '_is_field_main', False)]
        assert len(main_effects) == 1
        main_eff = main_effects[0]
        ctx = {'game': runner.game, 'player': runner.game.player1, 'permanent': perm}
        assert main_eff.can_use_condition(ctx), (
            "Main effect should be usable with 2+ cards in stack"
        )

    def test_main_effect_moves_top_to_bottom(self, debug_runner):
        """The main effect should move the top stacked card to the bottom
        of the digivolution cards (stack shift)."""
        runner = debug_runner(initial_memory=5)
        # Stack: ST2-04 (bottom), BT23-018 (top)
        perm = runner.place_on_field(1, ["ST2-04", "BT23-018"])

        # Record initial stack order
        initial_top_id = perm.card_sources[-1].card_id
        initial_bottom_id = perm.card_sources[0].card_id
        assert initial_top_id == "BT23-018"
        assert initial_bottom_id == "ST2-04"

        effects = perm.top_card.effect_list(None)
        main_eff = [e for e in effects if getattr(e, '_is_field_main', False)][0]

        # Execute the process callback directly
        main_eff.on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        })

        # After stack shift: the old top (BT23-018) should have moved to bottom
        # Note: the process shifts then enters selection phase for play.
        # Since no valid targets in hand, it should end without playing.
        # But the stack shift is the cost and should always happen.
        assert perm.card_sources[0].card_id == "BT23-018", (
            "Top card should now be at bottom after stack shift"
        )

    def test_main_effect_once_per_turn(self, debug_runner):
        """The main effect should have Once Per Turn enforcement."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST2-04", "BT23-018"])

        effects = perm.top_card.effect_list(None)
        main_eff = [e for e in effects if getattr(e, '_is_field_main', False)][0]
        assert main_eff.max_count_per_turn == 1, "Main effect should be Once Per Turn"

    # ---- Inherited [Opponent's Turn] +2000 DP ----

    def test_inherited_dp_boost_opponents_turn(self, debug_runner):
        """Inherited: +2000 DP during opponent's turn."""
        runner = debug_runner(initial_memory=5)
        # Place a Lv.5 with BT23-018 as inherited source
        perm = runner.place_on_field(1, ["BT23-018", "ST2-08"])

        bt23_card = perm.card_sources[0]  # BT23-018 at bottom
        effects = bt23_card.effect_list(None)
        dp_effects = [e for e in effects if e.is_inherited_effect and getattr(e, 'dp_modifier', 0) > 0]
        assert len(dp_effects) == 1, "Should have 1 inherited DP modifier"

        dp_eff = dp_effects[0]
        assert dp_eff.dp_modifier == 2000, "DP modifier should be +2000"

        # Should be active on opponent's turn (not my turn)
        runner.game.player1.is_my_turn = False
        assert dp_eff.can_use_condition({}), "DP boost should be active on opponent's turn"

        # Should NOT be active on owner's turn
        runner.game.player1.is_my_turn = True
        assert not dp_eff.can_use_condition({}), "DP boost should NOT be active on owner's turn"
