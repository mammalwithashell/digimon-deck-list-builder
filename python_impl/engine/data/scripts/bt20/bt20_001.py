from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT20_001(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect = ICardEffect()
        effect.set_effect_name("BT20-001 Inherited Effect")
        effect.set_effect_description("[Your Turn] This Digimon with 4 or more digivolution cards gets +2000 DP.")
        effect.is_inherited_effect = True
        effect.dp_modifier = 2000

        def condition(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent
            if permanent and permanent.top_card and permanent.top_card.owner:
                # Check turn
                if not permanent.top_card.owner.is_my_turn:
                    return False
                # Check sources count
                return len(permanent.digivolution_cards) >= 4
            return False

        effect.set_can_use_condition(condition)

        return [effect]
