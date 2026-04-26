from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_056(CardScript):
    """P-056 Rosemon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.BeforePayCost
        # <Digisorption -2> (When one of your Digimon digivolves into this card from your hand, you may suspend 1 of your Digimon to reduce the memory cost of the digivolution by 2.)
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("P-056 Digisorption -2")
        effect0.set_effect_description("<Digisorption -2> (When one of your Digimon digivolves into this card from your hand, you may suspend 1 of your Digimon to reduce the memory cost of the digivolution by 2.)")
        effect0.is_optional = True
        effect0.set_hash_string("Digisorption-3_BT2_045")
        effect0.cost_reduction = 2

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Cost -2, Suspend, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 2 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)
            # Grant effect immunity via modifier system
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If you have a Tamer in play, 1 of your opponent's Digimon can't attack or block until the end of their next turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("P-056 Opponent's 1 Digimon can't Attack or Block")
        effect1.set_effect_description("[When Digivolving] If you have a Tamer in play, 1 of your opponent's Digimon can't attack or block until the end of their next turn.")
        effect1.is_when_digivolving = True
        effect1._is_cannot_attack = True
        effect1._is_cannot_block = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain Keyword Cannot Attack, Gain Keyword Cannot Block, Grant Cannot Block"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_cannot_attack')
                perm.grant_keyword('_is_cannot_block')
            # Prevent target from blocking
            if not (player and game):
                return
            from engine_py_legacy.engine.interfaces.modifiers import ModifierType
            def on_restrict(target_perm):
                game.register_modifier(
                    ModifierType.CANNOT_BLOCK, target_perm,
                    value_fn=lambda: True, expiry='end_of_turn')
            game.effect_select_opponent_permanent(
                player, on_restrict, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
