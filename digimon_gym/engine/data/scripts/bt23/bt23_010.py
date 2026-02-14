from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_010(CardScript):
    """BT23-010"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-010 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 2
        effect0._alt_digi_cost = 2

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: security_play
        # Security: Play this card
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-010 Security: Play this card")
        effect1.set_effect_description("Security: Play this card")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: raid
        # Raid
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-010 Raid")
        effect2.set_effect_description("Raid")
        effect2._is_raid = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: blocker
        # Blocker
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-010 Blocker")
        effect3.set_effect_description("Blocker")
        effect3._is_blocker = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Factory effect: blocker
        # Blocker
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-010 Blocker")
        effect4.set_effect_description("Blocker")
        effect4.is_inherited_effect = True
        effect4._is_blocker = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
