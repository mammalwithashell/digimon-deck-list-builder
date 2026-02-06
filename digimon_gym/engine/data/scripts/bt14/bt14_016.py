from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_016(CardScript):
    """Auto-transpiled from DCGO BT14_016.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: raid
        # Raid
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-016 Raid")
        effect0.set_effect_description("Raid")
        # TODO: Raid keyword
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
