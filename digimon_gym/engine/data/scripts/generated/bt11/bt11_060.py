from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_060(CardScript):
    """BT11-060 Monmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: cannot_return_to_hand
        # Cannot Return to Hand
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-060 Cannot Return to Hand")
        effect0.set_effect_description("Cannot Return to Hand")
        effect0._grant_bounce_immunity = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: cannot_return_to_deck
        # Cannot Return to Deck
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-060 Cannot Return to Deck")
        effect1.set_effect_description("Cannot Return to Deck")
        effect1._grant_return_to_deck_immunity = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
