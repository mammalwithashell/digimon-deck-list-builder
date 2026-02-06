from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_028(CardScript):
    """Auto-transpiled from DCGO BT14_028.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-028 Blocker")
        effect0.set_effect_description("Blocker")
        # TODO: Blocker keyword - this Digimon can block
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDigivolutionCardDiscarded
        # [All Turns][Once Per Turn] When a digivolution card of an opponent's Digimon is trashed, this Digimon can't be deleted in battle until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-028 This Digimon can't be deleted by battle)")
        effect1.set_effect_description("[All Turns][Once Per Turn] When a digivolution card of an opponent's Digimon is trashed, this Digimon can't be deleted in battle until the end of your opponent's turn.")
        effect1.set_max_count_per_turn(1)

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
