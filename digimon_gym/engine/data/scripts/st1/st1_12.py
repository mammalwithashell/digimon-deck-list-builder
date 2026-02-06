from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class ST1_12(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect = ICardEffect()
        effect.set_effect_name("ST1-12 Effect")
        effect.set_effect_description("[Your Turn] All of your Red Digimon gain +1000 DP.")
        effect.is_tamer_effect = True

        def condition(context: Dict[str, Any]) -> bool:
            return True # Logic: Your Turn

        effect.set_can_use_condition(condition)

        return [effect]
