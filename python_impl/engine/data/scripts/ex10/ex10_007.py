from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class EX10_007(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # On Play / When Digivolving: +3000 DP to 1 Digimon
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-007: +3000 DP")
        effect1.timing = EffectTiming.OnEnterFieldAnyone

        def activate1():
            print("Selected 1 Digimon. Gave +3000 DP.")

        effect1.set_on_process_callback(activate1)
        effects.append(effect1)

        # Raid
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-007: <Raid>")
        effect2.timing = EffectTiming.OnAllyAttack
        effects.append(effect2)

        # ESS: +1000 DP
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-007 ESS: +1000 DP")
        effect3.is_inherited_effect = True
        effect3.dp_modifier = 1000

        def condition3(context: Dict[str, Any]) -> bool:
            permanent = context.get("permanent")
            if permanent and permanent.top_card.owner.is_my_turn:
                return True
            return False

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
