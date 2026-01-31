from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class EX10_001(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect = ICardEffect()
        effect.set_effect_name("EX10-001 ESS: Gain Memory")
        effect.set_effect_description("[Your Turn] [Once Per Turn] When effects trash any of this Digimon's link cards, gain 1 memory.")
        effect.is_inherited_effect = True
        effect.timing = EffectTiming.OnLinkCardDiscarded

        def condition(context: Dict[str, Any]) -> bool:
            permanent = context.get("permanent")
            if permanent and permanent.top_card.owner.is_my_turn:
                return True
            return False

        effect.set_can_use_condition(condition)

        def activate():
            if card.owner:
                card.owner.memory += 1
                print(f"Player {card.owner.player_name} gains 1 memory from EX10-001")

        effect.set_on_process_callback(activate)

        return [effect]
