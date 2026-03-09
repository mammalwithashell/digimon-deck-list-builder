from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from .....core.card_script import CardScript
from .....interfaces.card_effect import ICardEffect
from .....data.enums import EffectTiming

if TYPE_CHECKING:
    from .....core.card_source import CardSource


class ST18_07(CardScript):
    """ST18-07 Kokatorimon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Blocker>
        effect0 = ICardEffect()
        effect0.set_effect_name("ST18-07 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Inherited: <Piercing>
        effect1 = ICardEffect()
        effect1.set_effect_name("ST18-07 Piercing")
        effect1.set_effect_description("Piercing")
        effect1._is_piercing = True
        effect1.is_inherited_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
