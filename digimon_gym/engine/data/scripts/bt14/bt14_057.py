from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_057(CardScript):
    """BT14-057 ChuuChuumon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: save
        # Save
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-057 Save")
        effect0.set_effect_description("Save")
        effect0._is_save = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-057 Blocker")
        effect1.set_effect_description("Blocker")
        effect1.is_inherited_effect = True
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
