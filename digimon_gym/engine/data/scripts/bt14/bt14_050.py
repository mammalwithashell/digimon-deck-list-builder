from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_050(CardScript):
    """Auto-transpiled from DCGO BT14_050.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your opponent's Digimon cannot unsuspend until the end of your opponent's turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-050 Opponent's 1 Digimon can't unsuspend")
        effect0.set_effect_description("[On Play] 1 of your opponent's Digimon cannot unsuspend until the end of your opponent's turn.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your opponent's Digimon cannot unsuspend until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-050 Opponent's 1 Digimon can't unsuspend")
        effect1.set_effect_description("[When Digivolving] 1 of your opponent's Digimon cannot unsuspend until the end of your opponent's turn.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
