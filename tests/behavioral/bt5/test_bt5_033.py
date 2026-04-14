"""Behavioral tests for BT5-033 Cutemon (Lv.3 Yellow Digimon, DP 3000, Cost 3).

Card text (from cards.json):
[Opponent's Turn] Your opponent can't reduce digivolution costs.

This is a continuous/aura effect that registers CANNOT_REDUCE_COST on the
opponent during the opponent's turn. Uses the declarative pattern with
is_declarative=True and CANNOT_REDUCE_COST modifier.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming
from digimon_gym.engine.interfaces.modifiers import ModifierType


@pytest.mark.behavioral
class TestBT5033Cutemon:
    """Tests for BT5-033 Cutemon."""

    def test_has_cost_reduction_prevention_effect(self, debug_runner):
        """Should have an effect that prevents opponent cost reductions."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT5-033"])

        top = perm.top_card
        effects = top.effect_list(None)
        cost_prevent = [
            e for e in effects
            if "reduce" in (e.effect_description or "").lower()
            and "digivolution" in (e.effect_description or "").lower()
        ]
        assert len(cost_prevent) >= 1, (
            f"Should have digivolution cost reduction prevention effect. "
            f"Effects: {[(e.effect_name, e.effect_description) for e in effects]}"
        )

    def test_registers_cannot_reduce_cost_modifier(self, debug_runner):
        """Should register CANNOT_REDUCE_COST modifier when on field."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT5-033"])
        game = runner.game

        # Trigger the declarative/on-play effect
        top = perm.top_card
        effects = top.effect_list(None)
        for eff in effects:
            if eff.on_process_callback and (
                getattr(eff, 'is_on_play', False)
                or getattr(eff, 'is_declarative', False)
                or eff.timing in (EffectTiming.OnEnterFieldAnyone, EffectTiming.NoTiming)
            ):
                eff.on_process_callback({
                    'player': game.player1,
                    'game': game,
                })

        # Check that CANNOT_REDUCE_COST modifier is registered
        mods = game.modifiers._modifiers.get(ModifierType.CANNOT_REDUCE_COST, [])
        assert len(mods) >= 1, (
            f"Should have CANNOT_REDUCE_COST modifier registered. "
            f"All mods: {list(game.modifiers._modifiers.keys())}"
        )

    def test_effect_condition_requires_on_field(self, debug_runner):
        """Effect condition should require the card to be on the field."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT5-033", game.player1)
        effects = cs.effect_list(None)

        cost_prevent = [
            e for e in effects
            if "reduce" in (e.effect_description or "").lower()
        ]
        assert len(cost_prevent) >= 1

        # Card not on field -> condition should be False
        result = cost_prevent[0].can_use_condition({})
        assert result is False, "Condition should be False when card is not on field"

    def test_effect_condition_opponents_turn_only(self, debug_runner):
        """Effect should only be active during opponent's turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT5-033"])
        game = runner.game

        top = perm.top_card
        effects = top.effect_list(None)
        cost_prevent = [
            e for e in effects
            if "reduce" in (e.effect_description or "").lower()
        ]
        assert len(cost_prevent) >= 1

        # On your turn: condition should be False
        assert game.player1.is_my_turn
        result = cost_prevent[0].can_use_condition({})
        assert result is False, (
            "Effect should not be active on owner's turn"
        )

        # Switch to opponent's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        result = cost_prevent[0].can_use_condition({})
        assert result is True, (
            "Effect should be active on opponent's turn"
        )
