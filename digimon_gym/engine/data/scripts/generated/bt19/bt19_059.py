from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_059(CardScript):
    """BT19-059 DeadlyAxemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: retaliation
        # Retaliation
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-059 Retaliation")
        effect0.set_effect_description("Retaliation")
        effect0._is_retaliation = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: save
        # Save
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-059 Save")
        effect1.set_effect_description("Save")
        effect1._is_save = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: reboot
        # Reboot
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-059 Reboot")
        effect2.set_effect_description("Reboot")
        effect2.is_inherited_effect = True
        effect2._is_reboot = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
