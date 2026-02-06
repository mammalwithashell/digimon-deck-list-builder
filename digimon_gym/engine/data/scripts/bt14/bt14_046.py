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
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            # Check color: CardColor.Green
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Cost -3"""
            # reduce_cost(3)

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
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            # Check: it's the owner's turn
            # card.owner and card.owner.is_my_turn
            # Check color: CardColor.Green
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)

        def process1():
            """Action: Cost -1"""
            # reduce_cost(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
