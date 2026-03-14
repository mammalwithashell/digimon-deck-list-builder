from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class ST1_07(CardScript):
    """ST1-07 Greymon | Inherited Effect: <Security Attack +1>"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Inherited Effect: <Security Attack +1>
        effect = ICardEffect()
        effect.set_effect_name("ST1-07 Security Attack +1")
        effect.set_effect_description("<Security Attack +1>")
        effect.is_inherited_effect = True
        effect._security_attack_modifier = 1

        def condition(context: Dict[str, Any]) -> bool:
            return True

        effect.set_can_use_condition(condition)
        effects.append(effect)

        return effects
