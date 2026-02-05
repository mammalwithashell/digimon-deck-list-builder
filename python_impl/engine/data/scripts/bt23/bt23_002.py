from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT23_002(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect = ICardEffect()
        effect.set_effect_name("BT23-002 Inherited Effect")
        effect.set_effect_description("[When Attacking] [Once Per Turn] If this Digimon has the [CS] trait, <Draw 1>.")
        effect.is_inherited_effect = True
        effect.is_on_attack = True
        effect.set_max_count_per_turn(1)

        def condition(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent
            if permanent and permanent.top_card:
                return "CS" in permanent.top_card.card_traits
            return False

        def on_process():
            if card.owner:
                card.owner.draw()

        effect.set_can_use_condition(condition)
        effect.set_on_process_callback(on_process)

        return [effect]
