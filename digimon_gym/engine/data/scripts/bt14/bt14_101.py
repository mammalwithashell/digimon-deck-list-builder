from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_101(CardScript):
    """Auto-transpiled from DCGO BT14_101.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] This Digimon gains <Raid> for the turn. Then, it may attack.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-101 This Digimon gains Raid and can attack")
        effect0.set_effect_description("[When Digivolving] This Digimon gains <Raid> for the turn. Then, it may attack.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] If you have a Tamer, this Digimon gains ��Security A. +1�� and <Piercing> for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-101 Effect")
        effect1.set_effect_description("[When Attacking] If you have a Tamer, this Digimon gains ��Security A. +1�� and <Piercing> for the turn.")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
