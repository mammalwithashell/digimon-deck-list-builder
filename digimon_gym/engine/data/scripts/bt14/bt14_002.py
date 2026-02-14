from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_002(CardScript):
    """BT14-002 Bukamon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: jamming
        # Jamming
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-002 Jamming")
        effect0.set_effect_description("Jamming")
        effect0.is_inherited_effect = True
        effect0._is_jamming = True

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
