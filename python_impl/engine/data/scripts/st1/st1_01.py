from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class ST1_01(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect = ICardEffect()
        effect.set_effect_name("ST1-01 Inherited Effect")
        effect.set_effect_description("[Your Turn] While this Digimon has 4 or more digivolution cards, it gets +1000 DP.")
        effect.is_inherited_effect = True

        def condition(context: Dict[str, Any]) -> bool:
            is_my_turn = True # context.get("is_my_turn", False)

            permanent = effect.effect_source_permanent
            if permanent:
                # Check for 4 or more sources (including the top card? Usually sources are cards underneath)
                # In Digimon TCG, "digivolution cards" are the cards under the top card.
                # Permanent.digivolution_cards includes all cards?
                # Usually it includes the top one in some implementations, but let's assume strict definition.
                # If Permanent.digivolution_cards is all cards, sources = len - 1.
                # ST1-01 text: "4 or more digivolution cards".
                # If we assume permanent.digivolution_cards is the list of sources:
                if len(permanent.digivolution_cards) >= 4:
                    return is_my_turn
            return False

        effect.set_can_use_condition(condition)

        return [effect]
