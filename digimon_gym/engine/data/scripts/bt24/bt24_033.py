from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_033(CardScript):
    """BT24-033 Salamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-033 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 0
        effect0._alt_digi_cost = 0

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: change_digi_cost
        # Change digivolution cost
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-033 Change digivolution cost")
        effect1.set_effect_description("Change digivolution cost")
        # Reduce digivolution cost by 1 for [Iliad] trait
        effect1.cost_reduction = 1

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: barrier
        # Barrier
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-033 Barrier")
        effect2.set_effect_description("Barrier")
        effect2.is_inherited_effect = True
        effect2._is_barrier = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
