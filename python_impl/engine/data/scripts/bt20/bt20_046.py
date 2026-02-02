from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT20_046(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Effect 1: Cost Reduction
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-046 Cost Reduction")
        effect1.set_effect_description("[Your Turn] When this Digimon would digivolve into a Digimon card with the [Cyborg] or [Machine] trait, reduce the digivolution cost by 1.")
        effect1.is_inherited_effect = False
        effect1.cost_reduction = 1

        def condition1(context: Dict[str, Any]) -> bool:
            target_card = context.get("card_source")
            if target_card:
                traits = target_card.card_traits
                return "Cyborg" in traits or "Machine" in traits
            return False

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Effect 2: Inherited DP
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-046 Inherited DP")
        effect2.set_effect_description("[All Turns] This Digimon gets +1000 DP.")
        effect2.is_inherited_effect = True
        effect2.dp_modifier = 1000

        effects.append(effect2)

        return effects
