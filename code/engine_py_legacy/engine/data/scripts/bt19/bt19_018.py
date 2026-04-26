from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_018(CardScript):
    """BT19-018 Swimmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: evade
        # Evade
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-018 Evade")
        effect0.set_effect_description("Evade")
        effect0._is_evade = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: jamming
        # Jamming
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-018 Jamming")
        effect1.set_effect_description("Jamming")
        effect1.is_inherited_effect = True
        effect1._is_jamming = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
