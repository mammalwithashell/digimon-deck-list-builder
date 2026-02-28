from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_050(CardScript):
    """BT16-050 Commandramon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: dp_modifier_all
        # All your Digimon DP modifier
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-050 All your Digimon DP modifier")
        effect0.set_effect_description("All your Digimon DP modifier")
        effect0.dp_modifier = 1000
        effect0._applies_to_all_own_digimon = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: dp_modifier_all
        # All your Digimon DP modifier
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-050 All your Digimon DP modifier")
        effect1.set_effect_description("All your Digimon DP modifier")
        effect1.is_inherited_effect = True
        effect1.dp_modifier = 1000
        effect1._applies_to_all_own_digimon = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
