"""Behavioral tests for BT5-033 Cutemon (Lv.3 Yellow Digimon, DP 3000, Cost 3).

Card text (from cards.json):
    [Opponent's Turn] Your opponent can't reduce digivolution costs.

Implementation:
    - Declarative/continuous effect registers ModifierType.CANNOT_REDUCE_COST.
    - Condition restricts the modifier to opponent's turn only (i.e., when
      the Cutemon owner's turn is NOT active) and requires Cutemon on field.
    - Engine `Player.digivolve()` honours CANNOT_REDUCE_COST: when active for
      the digivolving player, both WhenWouldDigivolve `cost_reduction` and
      cost-decreasing CHANGE_DIGIVOLUTION_COST modifiers are skipped.

These tests exercise the full digivolve cost pipeline end-to-end through
`Player.digivolve()`, not just the condition function.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming
from engine_py_legacy.engine.interfaces.modifiers import ModifierType
from engine_py_legacy.engine.interfaces.card_effect import ICardEffect


def _set_turn(game, turn_pid: int) -> None:
    """Force the game to ``turn_pid``'s turn."""
    if turn_pid == 1:
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False
    else:
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True


def _trigger_cutemon_declarative(cutemon_perm, game) -> None:
    """Run BT5-033's declarative on-play process to register the modifier."""
    top = cutemon_perm.top_card
    for eff in top.effect_list(None):
        if eff.on_process_callback and (
            getattr(eff, "is_on_play", False)
            or getattr(eff, "is_declarative", False)
            or eff.timing in (EffectTiming.OnEnterFieldAnyone, EffectTiming.NoTiming)
        ):
            eff.on_process_callback({
                "player": cutemon_perm.top_card.owner,
                "game": game,
                "permanent": cutemon_perm,
            })


def _make_cost_reduction_effect(amount: int) -> ICardEffect:
    """Build a synthetic WhenWouldDigivolve non-inherited effect that reduces by amount.

    Non-inherited so Player.digivolve picks it up while the source card is the
    top card of the permanent (not buried).
    """
    eff = ICardEffect()
    eff.set_timing(EffectTiming.WhenWouldDigivolve)
    eff.set_effect_name(f"Test: synthetic cost reduction -{amount}")
    eff.set_effect_description(f"[When Digivolving] Cost -{amount}")
    eff.is_inherited_effect = False
    eff.cost_reduction = amount
    eff.set_can_use_condition(lambda ctx: True)
    eff.set_on_process_callback(lambda ctx: None)
    return eff


def _inject_top_card_cost_reduction(card_source, amount: int) -> None:
    """Append a synthetic non-inherited WhenWouldDigivolve cost_reduction effect
    to a card's cached effect list so `Player.digivolve` picks it up while this
    card is the top card of the digivolving permanent."""
    card_source.effect_list(None)  # prime cache
    card_source._cached_effects.append(_make_cost_reduction_effect(amount))


@pytest.mark.behavioral
class TestBT5033Cutemon:
    """Tests for BT5-033 Cutemon."""

    # --- Structural / declarative tests ------------------------------

    def test_has_cost_reduction_prevention_effect(self, debug_runner):
        """Card should have an effect describing digivolution cost reduction prevention."""
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
            "Should have a digivolution cost reduction prevention effect. "
            f"Effects: {[(e.effect_name, e.effect_description) for e in effects]}"
        )

    def test_registers_cannot_reduce_cost_modifier(self, debug_runner):
        """Declarative trigger should register a CANNOT_REDUCE_COST modifier."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT5-033"])
        game = runner.game

        _trigger_cutemon_declarative(perm, game)

        mods = game.modifiers._modifiers.get(ModifierType.CANNOT_REDUCE_COST, [])
        assert len(mods) >= 1, (
            "CANNOT_REDUCE_COST modifier should be registered. "
            f"All mods: {list(game.modifiers._modifiers.keys())}"
        )

    def test_effect_condition_requires_on_field(self, debug_runner):
        """Effect condition should be False when the card is not on the field."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT5-033", game.player1)
        effects = cs.effect_list(None)
        cost_prevent = [
            e for e in effects
            if "reduce" in (e.effect_description or "").lower()
        ]
        assert len(cost_prevent) >= 1
        assert cost_prevent[0].can_use_condition({}) is False

    def test_effect_condition_only_on_opponents_turn(self, debug_runner):
        """Effect condition should be False on own turn, True on opponent's turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT5-033"])
        game = runner.game
        top = perm.top_card
        cost_prevent = [
            e for e in top.effect_list(None)
            if "reduce" in (e.effect_description or "").lower()
        ]
        assert len(cost_prevent) >= 1
        eff = cost_prevent[0]

        _set_turn(game, 1)  # Cutemon's owner's turn
        assert eff.can_use_condition({}) is False

        _set_turn(game, 2)  # opponent's turn
        assert eff.can_use_condition({}) is True

    # --- End-to-end engine behaviour ---------------------------------

    def test_digivolve_baseline_without_cutemon_allows_reduction(self, debug_runner):
        """Sanity: without Cutemon, cost reductions apply during digivolve."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # P2 has a Lv.3 Yellow on field (BT5-034 Kotemon) to digivolve from.
        base = runner.place_on_field(2, ["BT5-034"])

        # Inject a synthetic inherited WhenWouldDigivolve cost_reduction=2 onto
        # Kotemon so it self-reduces its own digivolve cost.
        _inject_top_card_cost_reduction(base.top_card, 2)

        # Put BT5-037 Gladimon (cost 2 from Lv.3 Yellow) in P2's hand and
        # digivolve from Kotemon into Gladimon.
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        gladimon = db.create_card_source("BT5-037", game.player2)
        game.player2.hand_cards.append(gladimon)

        _set_turn(game, 2)
        paid = game.player2.digivolve(base, gladimon)

        # Base cost 2 - reduction 2 = 0
        assert paid == 0, f"Expected 0 (2 - 2) without Cutemon, got {paid}"

    def test_digivolve_with_cutemon_blocks_inherited_cost_reduction(self, debug_runner):
        """With Cutemon on field, opponent's WhenWouldDigivolve reductions don't apply."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # P1 has Cutemon on field.
        cute = runner.place_on_field(1, ["BT5-033"])
        _trigger_cutemon_declarative(cute, game)

        # P2 has Kotemon on field with a synthetic -2 reduction effect.
        base = runner.place_on_field(2, ["BT5-034"])
        _inject_top_card_cost_reduction(base.top_card, 2)

        # P2 digivolves Kotemon into Gladimon (cost 2) on P2's turn.
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        gladimon = db.create_card_source("BT5-037", game.player2)
        game.player2.hand_cards.append(gladimon)

        _set_turn(game, 2)
        paid = game.player2.digivolve(base, gladimon)

        # Reduction must be blocked — full 2 paid.
        assert paid == 2, (
            f"Expected full cost 2 (reduction blocked by Cutemon), got {paid}"
        )

    def test_digivolve_on_cutemon_owner_turn_reduction_still_applies(self, debug_runner):
        """On Cutemon owner's turn, Cutemon does NOT block the owner's own reductions."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # P1 has Cutemon on field.
        cute = runner.place_on_field(1, ["BT5-033"])
        _trigger_cutemon_declarative(cute, game)

        # P1 has Kotemon on field with a synthetic -2 reduction effect.
        base = runner.place_on_field(1, ["BT5-034"])
        _inject_top_card_cost_reduction(base.top_card, 2)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        gladimon = db.create_card_source("BT5-037", game.player1)
        game.player1.hand_cards.append(gladimon)

        _set_turn(game, 1)  # Cutemon owner's turn
        paid = game.player1.digivolve(base, gladimon)

        # Cutemon's effect is [Opponent's Turn] — it must NOT block the owner's
        # own reductions on their own turn.
        assert paid == 0, (
            f"Expected reduction to apply on Cutemon owner's turn, got cost {paid}"
        )

    def test_digivolve_with_cutemon_blocks_change_digivolution_cost_modifier(
        self, debug_runner
    ):
        """CHANGE_DIGIVOLUTION_COST modifiers that reduce cost are also blocked."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # P1 has Cutemon on field.
        cute = runner.place_on_field(1, ["BT5-033"])
        _trigger_cutemon_declarative(cute, game)

        # P2 has Kotemon on field (no inherited reductions).
        base = runner.place_on_field(2, ["BT5-034"])

        # Register a CHANGE_DIGIVOLUTION_COST modifier that would reduce by 2
        # targeted at Kotemon. Without Cutemon this would make cost 0.
        game.register_modifier(
            base,
            ModifierType.CHANGE_DIGIVOLUTION_COST,
            condition=lambda target, ctx: True,
            value_fn=lambda current, target, ctx: current - 2,
            expiry="permanent",
        )

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        gladimon = db.create_card_source("BT5-037", game.player2)
        game.player2.hand_cards.append(gladimon)

        _set_turn(game, 2)
        paid = game.player2.digivolve(base, gladimon)

        assert paid == 2, (
            f"CHANGE_DIGIVOLUTION_COST reduction should be blocked; got {paid}"
        )

    def test_digivolve_with_cutemon_does_not_block_cost_increase(self, debug_runner):
        """A CHANGE_DIGIVOLUTION_COST that *increases* cost must still apply."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        cute = runner.place_on_field(1, ["BT5-033"])
        _trigger_cutemon_declarative(cute, game)

        base = runner.place_on_field(2, ["BT5-034"])

        game.register_modifier(
            base,
            ModifierType.CHANGE_DIGIVOLUTION_COST,
            condition=lambda target, ctx: True,
            value_fn=lambda current, target, ctx: current + 3,
            expiry="permanent",
        )

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        gladimon = db.create_card_source("BT5-037", game.player2)
        game.player2.hand_cards.append(gladimon)

        _set_turn(game, 2)
        paid = game.player2.digivolve(base, gladimon)

        assert paid == 5, (
            f"Cost increase (2 + 3) should still apply; got {paid}"
        )

    def test_cutemon_leaves_field_modifier_cleaned_up(self, debug_runner):
        """When Cutemon leaves the field, the CANNOT_REDUCE_COST modifier is removed."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        cute = runner.place_on_field(1, ["BT5-033"])
        _trigger_cutemon_declarative(cute, game)
        assert len(
            game.modifiers._modifiers.get(ModifierType.CANNOT_REDUCE_COST, [])
        ) >= 1

        # Cutemon leaves field
        game.player1.battle_area.remove(cute)
        game.cleanup_modifiers_for_permanent(cute)

        assert len(
            game.modifiers._modifiers.get(ModifierType.CANNOT_REDUCE_COST, [])
        ) == 0, "Modifier should be cleaned up when Cutemon leaves the field"

        # Reduction should work again for P2.
        base = runner.place_on_field(2, ["BT5-034"])
        _inject_top_card_cost_reduction(base.top_card, 2)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        gladimon = db.create_card_source("BT5-037", game.player2)
        game.player2.hand_cards.append(gladimon)

        _set_turn(game, 2)
        paid = game.player2.digivolve(base, gladimon)
        assert paid == 0, f"Post-cleanup: expected 0, got {paid}"
