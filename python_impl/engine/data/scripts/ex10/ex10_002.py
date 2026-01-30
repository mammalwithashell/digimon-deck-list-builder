from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class EX10_002(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect = ICardEffect()
        effect.set_effect_name("EX10-002 ESS: Draw 1 on Redirect")
        effect.set_effect_description("[All Turns] [Once Per Turn] When attack targets change, <Draw 1>.")
        effect.is_inherited_effect = True
        effect.timing = EffectTiming.OnAttackTargetChanged
        effect.max_count_per_turn = 1

        def condition(context: Dict[str, Any]) -> bool:
            return True

        effect.set_can_use_condition(condition)

        def activate():
            if card.owner:
                card.owner.draw()
                print(f"Player {card.owner.player_name} draws 1 card from EX10-002")

        effect.set_on_process_callback(activate)

        return [effect]
