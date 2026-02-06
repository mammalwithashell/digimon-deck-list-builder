from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class ST1_09(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect = ICardEffect()
        effect.set_effect_name("ST1-09 Inherited Effect")
        effect.is_inherited_effect = True
        effect.set_effect_description("[Your Turn] When this Digimon is blocked, gain 3 memory.")

        def condition(context: Dict[str, Any]) -> bool:
            # Trigger check: is_blocked
            return True

        effect.set_can_use_condition(condition)

        return [effect]
