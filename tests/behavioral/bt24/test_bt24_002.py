"""Behavioral tests for BT24-002 Bukamon (Lv.2 Digitama).

Card text:
Inherited Effect [End of Your Turn] [Once Per Turn] By paying 1 cost,
this blue Digimon with the [TS] trait unsuspends.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT24002Bukamon:
    """Tests for BT24-002 Bukamon inherited effect."""

    def test_inherited_unsuspend_blue_ts_digimon(self, debug_runner):
        """A suspended blue TS Digimon with Bukamon in sources should unsuspend
        when the end-of-turn effect activates, paying 1 cost."""
        runner = debug_runner(initial_memory=5)

        # Place a blue TS Digimon with Bukamon as digivolution source
        # BT24-019 Kamemon is a blue TS Lv.3
        perm = runner.place_on_field(1, ["BT24-002", "BT24-019"], is_suspended=True)

        # Record memory before
        game = runner.game
        memory_before = game.memory

        # Find the inherited end-of-turn effect
        card = perm.card_sources[0]  # Bukamon (bottom of stack)
        effects = card.effect_list(None)
        eot_effects = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(eot_effects) == 1, "Bukamon should have exactly 1 OnEndTurn inherited effect"

        eot = eot_effects[0]
        assert eot.is_inherited_effect, "Effect must be inherited"
        assert eot.is_optional, "Effect must be optional"

        # Condition should pass: on field, owner's turn
        assert eot.can_use_condition({}), "Condition should pass for blue TS Digimon on field"

        # Execute the effect
        eot.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Should have unsuspended
        assert not perm.is_suspended, "Blue TS Digimon should unsuspend after effect"
        # Should have paid 1 cost
        assert game.memory == memory_before - 1, "Should pay 1 memory cost"

    def test_inherited_no_unsuspend_non_blue(self, debug_runner):
        """A non-blue TS Digimon should NOT unsuspend (pays cost but fails check)."""
        runner = debug_runner(initial_memory=5)

        # Place a green TS Digimon with Bukamon as source
        # BT24-037 Silphymon is green with TS trait
        perm = runner.place_on_field(1, ["BT24-002", "BT24-037"], is_suspended=True)

        game = runner.game
        memory_before = game.memory

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        eot = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        eot.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Cost is paid (per C# behavior: pay first, then check)
        assert game.memory == memory_before - 1, "Cost should still be paid"
        # But should NOT unsuspend (not blue)
        assert perm.is_suspended, "Non-blue Digimon should remain suspended"

    def test_inherited_no_unsuspend_non_ts(self, debug_runner):
        """A blue non-TS Digimon should NOT unsuspend (pays cost but fails check)."""
        runner = debug_runner(initial_memory=5)

        # Place a blue non-TS Digimon with Bukamon
        # ST2-04 Gabumon is blue but has no TS trait
        perm = runner.place_on_field(1, ["BT24-002", "ST2-04"], is_suspended=True)

        game = runner.game

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        eot = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        eot.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert perm.is_suspended, "Blue non-TS Digimon should remain suspended"

    def test_inherited_not_suspended_noop(self, debug_runner):
        """If the Digimon is already unsuspended, cost is paid but nothing happens."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT24-002", "BT24-019"], is_suspended=False)

        game = runner.game
        memory_before = game.memory

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        eot = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        eot.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Cost paid, Digimon stays unsuspended
        assert not perm.is_suspended
        assert game.memory == memory_before - 1, "Cost is still paid even if already unsuspended"
