from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class ST1_03(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect = ICardEffect()
        effect.set_effect_name("ST1-03 Inherited Effect")
        effect.set_effect_description("[Your Turn] This Digimon gets +1000 DP.")
        effect.is_inherited_effect = True

        def condition(context: Dict[str, Any]) -> bool:
            return True

        effect.set_can_use_condition(condition)

        return [effect]
