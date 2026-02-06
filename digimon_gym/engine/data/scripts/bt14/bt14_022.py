from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_022(CardScript):
    """Auto-transpiled from DCGO BT14_022.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] Trash any 1 digivolution card of 1 of your opponent's Digimon. Then, return 1 of your opponent's level 5 or lower Digimon with no digivolution cards to the hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-022 Trash digivolution cards an return 1 Digimon to hand")
        effect0.set_effect_description("[When Attacking] Trash any 1 digivolution card of 1 of your opponent's Digimon. Then, return 1 of your opponent's level 5 or lower Digimon with no digivolution cards to the hand.")
        effect0.is_on_attack = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Bounce, Trash Digivolution Cards"""
            # target_permanent.return_to_hand()
            # target.trash_digivolution_cards(count)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
