from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_071(CardScript):
    """BT10-071 Gazimon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: retaliation
        # Retaliation
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-071 Retaliation")
        effect0.set_effect_description("Retaliation")
        effect0.is_inherited_effect = True
        effect0._is_retaliation = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
