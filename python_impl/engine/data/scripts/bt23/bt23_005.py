from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT23_005(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Main Effect: [Your Turn] When this Digimon would digivolve into a Digimon card with the [Reptile] or [Dragonkin] trait, reduce the digivolution cost by 1.
        # Note: Logic for interruptive cost reduction is complex and requires engine support.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-005 Main Effect")
        effect1.set_effect_description("[Your Turn] When this Digimon would digivolve into a Digimon card with the [Reptile] or [Dragonkin] trait, reduce the digivolution cost by 1.")

        def condition1(context: Dict[str, Any]) -> bool:
            if card.owner:
                return card.owner.is_my_turn
            return False

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Inherited Effect: [Your Turn] This Digimon gets +2000 DP.
        # Note: DP buff application requires engine support (Permanent.dp modification).
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-005 Inherited Effect")
        effect2.set_effect_description("[Your Turn] This Digimon gets +2000 DP.")
        effect2.is_inherited_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card.owner:
                return card.owner.is_my_turn
            return False

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
