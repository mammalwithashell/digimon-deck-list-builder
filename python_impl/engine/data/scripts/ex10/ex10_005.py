from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class EX10_005(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect = ICardEffect()
        effect.set_effect_name("EX10-005 ESS: Draw on Mill")
        effect.set_effect_description("[Your Turn] [Once Per Turn] When your opponent's deck is trashed from, <Draw 1>.")
        effect.is_inherited_effect = True
        effect.timing = EffectTiming.OnDiscardLibrary
        effect.max_count_per_turn = 1

        def condition(context: Dict[str, Any]) -> bool:
            return card.owner and card.owner.is_my_turn

        effect.set_can_use_condition(condition)

        def activate():
            if card.owner:
                card.owner.draw()
                print("Draw 1 from EX10-005")

        effect.set_on_process_callback(activate)

        return [effect]
