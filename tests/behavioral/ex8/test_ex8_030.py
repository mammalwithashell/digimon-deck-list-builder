"""Behavioral tests for EX8-030 Tapirmon (Lv.3 Yellow Digimon, DP 2000, Cost 3).

Card text (from cards.json):
[All Turns] Your opponent can't gain memory other than by Tamer effects.

This is a continuous/aura effect that registers CANNOT_ADD_MEMORY on the opponent.
Pattern reference: BT3-061 Chuumon (identical effect text, different color).
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming
from digimon_gym.engine.interfaces.modifiers import ModifierType


@pytest.mark.behavioral
class TestEX8030Tapirmon:
    """Tests for EX8-030 Tapirmon."""

    def test_has_cannot_add_memory_effect(self, debug_runner):
        """Should have an effect that registers CANNOT_ADD_MEMORY modifier."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-030"])

        # Should have an on_play or declarative effect that processes
        top = perm.top_card
        effects = top.effect_list(None)
        # At minimum there should be an effect describing memory blocking
        memory_effects = [
            e for e in effects
            if "memory" in (e.effect_description or "").lower()
            or "memory" in (e.effect_name or "").lower()
        ]
        assert len(memory_effects) >= 1, (
            f"Should have a memory-related effect. Effects: "
            f"{[(e.effect_name, e.effect_description) for e in effects]}"
        )

    def test_opponent_cannot_gain_memory_when_on_field(self, debug_runner):
        """Opponent should be blocked from gaining memory while Tapirmon is on field."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-030"])
        game = runner.game

        # Trigger the on-play effect to register the modifier
        top = perm.top_card
        effects = top.effect_list(None)
        for eff in effects:
            if eff.on_process_callback and (
                getattr(eff, 'is_on_play', False)
                or eff.timing == EffectTiming.OnEnterFieldAnyone
            ):
                eff.on_process_callback({
                    'player': game.player1,
                    'game': game,
                })

        # Now check that opponent can't gain memory
        mem_before = game.memory
        game.player2.add_memory(3)  # Opponent tries to gain 3 memory
        # Memory should NOT have changed (blocked by CANNOT_ADD_MEMORY)
        assert game.memory == mem_before, (
            f"Opponent should not be able to gain memory. "
            f"Before: {mem_before}, after: {game.memory}"
        )

    def test_owner_can_still_gain_memory(self, debug_runner):
        """Owner of Tapirmon should still be able to gain memory."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-030"])
        game = runner.game

        # Trigger the on-play effect
        top = perm.top_card
        effects = top.effect_list(None)
        for eff in effects:
            if eff.on_process_callback and (
                getattr(eff, 'is_on_play', False)
                or eff.timing == EffectTiming.OnEnterFieldAnyone
            ):
                eff.on_process_callback({
                    'player': game.player1,
                    'game': game,
                })

        # Owner should still be able to gain memory
        mem_before = game.memory
        game.player1.add_memory(2)
        assert game.memory == mem_before + 2, (
            f"Owner should still gain memory. "
            f"Before: {mem_before}, after: {game.memory}"
        )

    def test_effect_active_on_both_turns(self, debug_runner):
        """[All Turns] means the block is active on both your turn and opponent's turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-030"])
        game = runner.game

        # Trigger the on-play effect
        top = perm.top_card
        effects = top.effect_list(None)
        for eff in effects:
            if eff.on_process_callback and (
                getattr(eff, 'is_on_play', False)
                or eff.timing == EffectTiming.OnEnterFieldAnyone
            ):
                eff.on_process_callback({
                    'player': game.player1,
                    'game': game,
                })

        # On P1's turn: opponent can't gain
        mem_before = game.memory
        game.player2.add_memory(1)
        assert game.memory == mem_before, "Opponent blocked on P1's turn"

        # Switch turns
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        # On P2's turn: opponent (P2) still can't gain
        mem_before = game.memory
        game.player2.add_memory(1)
        assert game.memory == mem_before, "Opponent still blocked on P2's turn"
