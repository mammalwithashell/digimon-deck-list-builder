from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

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
            timing = context.get("timing")
            if timing and timing != EffectTiming.OnUseAttack:
                 return False

            perm = context.get("permanent")
            if perm and perm.top_card:
                return "CS" in perm.top_card.card_traits
            return False

        def on_process(context: Dict[str, Any]):
            player = context.get("player")
            if player:
                player.draw()

        effect.set_can_use_condition(condition)
        effect.set_on_process_callback(on_process)

        return [effect]
