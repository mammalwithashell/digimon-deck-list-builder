from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT20_005(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect = ICardEffect()
        effect.set_effect_name("BT20-005 Inherited Effect")
        effect.set_effect_description("[Your Turn] When this Digimon checks a face-up security card, this Digimon gains <Jamming> for the turn.")
        effect.is_inherited_effect = True

        # Custom flag for Security Check
        effect.is_on_security_check = True

        def condition(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent
            if permanent and permanent.top_card and permanent.top_card.owner:
                return permanent.top_card.owner.is_my_turn
            return False

        def on_process():
            print("BT20-005: Gained <Jamming> (Simulated)")
            # In a real implementation, we would apply a temporary effect here.

        effect.set_can_use_condition(condition)
        effect.set_on_process_callback(on_process)

        return [effect]
