from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_027(CardScript):
    """Auto-transpiled from DCGO BT14_027.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Return all level 3 Digimon to the hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-027 Return all level 3 Digimons to hand")
        effect0.set_effect_description("[On Play] Return all level 3 Digimon to the hand.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Return all level 3 Digimon to the hand.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-027 Return all level 3 Digimons to hand")
        effect1.set_effect_description("[When Digivolving] Return all level 3 Digimon to the hand.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
