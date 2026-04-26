from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_054(CardScript):
    """BT8-054 Pistmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.BeforePayCost
        # <Digisorption -2> (When one of your Digimon digivolves into this card from your hand, you may suspend 1 of your Digimon to reduce the memory cost of the digivolution by 2.)
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT8-054 Digisorption -2")
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

        # Factory effect: dp_modifier
        # DP modifier
        effect1 = ICardEffect()
        effect1.set_effect_name("BT8-054 DP modifier")
        effect1.set_effect_description("DP modifier")
        effect1.is_inherited_effect = True
        effect1.dp_modifier = 0

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
