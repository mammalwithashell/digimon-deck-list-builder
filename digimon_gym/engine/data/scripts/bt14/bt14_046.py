from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_046(CardScript):
    """Auto-transpiled from DCGO BT14_046.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Cost -3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-046 Play Cost -3")
        effect0.set_effect_description("Cost -3")
        effect0.cost_reduction = 3

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Cost -3"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Cost reduction handled via cost_reduction property

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Cost -1
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-046 Cost -1")
        effect1.set_effect_description("Cost -1")
        effect1.is_inherited_effect = True
        effect1.cost_reduction = 1

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Cost -1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Cost reduction handled via cost_reduction property

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
