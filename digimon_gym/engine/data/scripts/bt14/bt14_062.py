from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_062(CardScript):
    """BT14-062 Datamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: cannot_destroyed_by_skill
        # Cannot Be Destroyed by Effects
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-062 Cannot Be Destroyed by Effects")
        effect0.set_effect_description("Cannot Be Destroyed by Effects")
        effect0._grant_destruction_immunity = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
