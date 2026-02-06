from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_019(CardScript):
    """Auto-transpiled from DCGO BT24_019.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: jamming
        # Jamming
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-019 Jamming")
        effect0.set_effect_description("Jamming")
        effect0._is_jamming = True
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
