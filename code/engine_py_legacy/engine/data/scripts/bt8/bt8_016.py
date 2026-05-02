from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_016(CardScript):
    """BT8-016 MasterTyrannomon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-016 Security Attack +1")
        effect0.set_effect_description("Security Attack +1")
        effect0.is_inherited_effect = True
        effect0._security_attack_modifier = 1

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
