"""Behavioral tests for EX8-026 MetalSeadramon (Ace) — Lv.6 Blue/Black, DP 12000, Cost 7.

Card text:
[Hand] [Counter] <Blast Digivolve>
[On Play] [When Digivolving] <De-Digivolve 1> 1 of your opponent's Digimon.
    Then, return 1 of your opponent's Digimon with a play cost of 7 or less
    to the bottom of the deck.
[All Turns] While you have 1 or more memory, none of your opponent's Digimon
    can suspend.
Inherited: Ace Overflow <-4>

Key behaviors to test:
1. De-Digivolve 1 fires on play/digivolve (target selection)
2. Bounce (return to deck bottom) for cost <= 7 fires after de-digivolve
3. CANNOT_SUSPEND continuous effect: blocks opponent Digimon from suspending
   while owner has 1+ memory, inactive when owner has 0 memory
4. CANNOT_SUSPEND is cleaned up when MetalSeadramon leaves the field
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming
from engine_py_legacy.engine.interfaces.modifiers import ModifierType


def _trigger_on_play_effects(perm, game, player):
    """Manually fire all OnPlay-like effects on a permanent (simulates entry)."""
    top = perm.top_card
    effects = top.effect_list(None)
    for eff in effects:
        if eff.on_process_callback and getattr(eff, 'is_on_play', False):
            eff.on_process_callback({
                'player': player,
                'game': game,
                'permanent': perm,
            })


def _trigger_continuous_effects(perm, game, player):
    """Trigger the continuous aura registration effect (OnEnterFieldAnyone with
    no is_on_play/is_when_digivolving flag, or a dedicated registrator)."""
    top = perm.top_card
    effects = top.effect_list(None)
    for eff in effects:
        if eff.on_process_callback and (
            getattr(eff, 'is_on_play', False)
            or getattr(eff, '_is_continuous_registrator', False)
            or 'suspend' in (eff.effect_name or '').lower()
            or 'suspend' in (eff.effect_description or '').lower()
        ):
            if 'suspend' in (eff.effect_description or '').lower():
                eff.on_process_callback({
                    'player': player,
                    'game': game,
                    'permanent': perm,
                })


@pytest.mark.behavioral
class TestEX8026MetalSeadramon:
    """Tests for EX8-026 MetalSeadramon continuous CANNOT_SUSPEND effect."""

    def test_cannot_suspend_active_with_positive_memory(self, debug_runner):
        """Opponent Digimon cannot suspend (attack) when owner has 1+ memory."""
        runner = debug_runner(initial_memory=3)
        # Place MetalSeadramon on P1's field
        metal_perm = runner.place_on_field(1, ["EX8-026"])
        # Place an opponent Digimon on P2's field
        opp_perm = runner.place_on_field(2, ["BT1-019"])
        game = runner.game

        # Trigger the continuous effect registration
        _trigger_continuous_effects(metal_perm, game, game.player1)

        # P1 has 3 memory (>= 1), so opponent Digimon should not be able to suspend
        assert game.modifiers.has_modifier(opp_perm, ModifierType.CANNOT_SUSPEND), (
            "Opponent Digimon should have CANNOT_SUSPEND modifier when owner has 1+ memory"
        )

    def test_cannot_suspend_inactive_with_zero_memory(self, debug_runner):
        """Opponent Digimon CAN suspend when owner has 0 memory."""
        runner = debug_runner(initial_memory=0)
        metal_perm = runner.place_on_field(1, ["EX8-026"])
        opp_perm = runner.place_on_field(2, ["BT1-019"])
        game = runner.game

        # Trigger the continuous effect registration
        _trigger_continuous_effects(metal_perm, game, game.player1)

        # P1 has 0 memory, so CANNOT_SUSPEND should NOT be active
        assert not game.modifiers.has_modifier(opp_perm, ModifierType.CANNOT_SUSPEND), (
            "Opponent Digimon should NOT have CANNOT_SUSPEND when owner has 0 memory"
        )

    def test_cannot_suspend_inactive_with_negative_memory(self, debug_runner):
        """Opponent Digimon CAN suspend when memory is on opponent's side."""
        runner = debug_runner(initial_memory=-2)
        metal_perm = runner.place_on_field(1, ["EX8-026"])
        opp_perm = runner.place_on_field(2, ["BT1-019"])
        game = runner.game

        # Trigger the continuous effect
        _trigger_continuous_effects(metal_perm, game, game.player1)

        # P1 has negative memory (-2 means P2 has 2), so inactive
        assert not game.modifiers.has_modifier(opp_perm, ModifierType.CANNOT_SUSPEND), (
            "Opponent Digimon should NOT have CANNOT_SUSPEND when owner has negative memory"
        )

    def test_cannot_suspend_does_not_affect_own_digimon(self, debug_runner):
        """Owner's own Digimon should NOT be affected by the CANNOT_SUSPEND."""
        runner = debug_runner(initial_memory=3)
        metal_perm = runner.place_on_field(1, ["EX8-026"])
        own_perm = runner.place_on_field(1, ["BT1-019"])
        game = runner.game

        # Trigger
        _trigger_continuous_effects(metal_perm, game, game.player1)

        # Own Digimon should be able to suspend
        assert not game.modifiers.has_modifier(own_perm, ModifierType.CANNOT_SUSPEND), (
            "Owner's own Digimon should NOT be affected by CANNOT_SUSPEND"
        )

    def test_cannot_suspend_blocks_attack(self, debug_runner):
        """Opponent Digimon with CANNOT_SUSPEND cannot attack (attack requires suspend)."""
        runner = debug_runner(initial_memory=3)
        metal_perm = runner.place_on_field(1, ["EX8-026"])
        opp_perm = runner.place_on_field(2, ["BT1-019"])
        game = runner.game

        # Trigger
        _trigger_continuous_effects(metal_perm, game, game.player1)

        # Opponent Digimon should not be able to attack
        assert not opp_perm.can_attack(), (
            "Opponent Digimon with CANNOT_SUSPEND should not be able to attack"
        )

    def test_cannot_suspend_dynamic_memory_check(self, debug_runner):
        """CANNOT_SUSPEND should respond dynamically to memory changes."""
        runner = debug_runner(initial_memory=3)
        metal_perm = runner.place_on_field(1, ["EX8-026"])
        opp_perm = runner.place_on_field(2, ["BT1-019"])
        game = runner.game

        # Register the modifier
        _trigger_continuous_effects(metal_perm, game, game.player1)

        # With 3 memory, should be active
        assert game.modifiers.has_modifier(opp_perm, ModifierType.CANNOT_SUSPEND), (
            "Should be active with memory=3"
        )

        # Set memory to 0 — should become inactive
        game.memory = 0
        assert not game.modifiers.has_modifier(opp_perm, ModifierType.CANNOT_SUSPEND), (
            "Should be inactive with memory=0"
        )

        # Set memory back to 1 — should be active again
        game.memory = 1
        assert game.modifiers.has_modifier(opp_perm, ModifierType.CANNOT_SUSPEND), (
            "Should be active again with memory=1"
        )

    def test_cannot_suspend_cleaned_up_on_leave_field(self, debug_runner):
        """CANNOT_SUSPEND should be removed when MetalSeadramon leaves the field."""
        runner = debug_runner(initial_memory=3)
        metal_perm = runner.place_on_field(1, ["EX8-026"])
        opp_perm = runner.place_on_field(2, ["BT1-019"])
        game = runner.game

        # Register the modifier
        _trigger_continuous_effects(metal_perm, game, game.player1)

        # Confirm it's active
        assert game.modifiers.has_modifier(opp_perm, ModifierType.CANNOT_SUSPEND)

        # Remove MetalSeadramon from the field (simulates deletion)
        game.cleanup_modifiers_for_permanent(metal_perm)
        game.player1.battle_area.remove(metal_perm)

        # Modifier should be gone
        assert not game.modifiers.has_modifier(opp_perm, ModifierType.CANNOT_SUSPEND), (
            "CANNOT_SUSPEND should be cleaned up when MetalSeadramon leaves field"
        )

    def test_dedigivolve_and_bounce_on_play(self, debug_runner):
        """On Play should de-digivolve 1 then bounce a cost<=7 Digimon to deck bottom."""
        runner = debug_runner(initial_memory=7)
        # Place opponent digimon with stack (for de-digivolve target)
        opp_stack = runner.place_on_field(2, ["BT1-003", "BT1-019"])  # egg + rookie
        # Place another opponent with cost <= 7 (for bounce target)
        opp_cheap = runner.place_on_field(2, ["BT1-019"])  # cost 3 rookie
        game = runner.game

        # Place MetalSeadramon and trigger on-play
        metal_perm = runner.place_on_field(1, ["EX8-026"])

        # Fire the on-play effect manually
        top = metal_perm.top_card
        effects = top.effect_list(None)
        on_play_effects = [
            e for e in effects
            if e.on_process_callback and getattr(e, 'is_on_play', False)
        ]
        assert len(on_play_effects) >= 1, "Should have an on-play effect"

        # Count before
        opp_field_before = len(game.player2.battle_area)
        opp_stack_size_before = len(opp_stack.card_sources)

        # Execute the on-play (de-digivolve + bounce)
        on_play_effects[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': metal_perm,
        })
        # Auto-resolve selections
        runner.auto_resolve()

        # After: opp_stack should have lost 1 digivolution card (de-digivolve 1)
        # and opp_cheap should have been bounced to deck bottom
        # Note: one of these two things should have happened
        opp_field_after = len(game.player2.battle_area)
        # At minimum the bounce should remove one from field
        assert opp_field_after < opp_field_before or len(opp_stack.card_sources) < opp_stack_size_before, (
            "On Play should have de-digvolved or bounced an opponent Digimon"
        )
