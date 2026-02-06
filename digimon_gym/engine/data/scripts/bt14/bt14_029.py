from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_029(CardScript):
    """Auto-transpiled from DCGO BT14_029.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Trash any 3 digivolution cards from your opponent's Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-029 Trash digivolution cards and ")
        effect0.set_effect_description("[When Digivolving] Trash any 3 digivolution cards from your opponent's Digimon.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect0.set_can_use_condition(condition0)

        def process0():
            """Action: Trash Digivolution Cards"""
            # target.trash_digivolution_cards(count)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] If your opponent has no Digimon with as many or more digivolution cards than this Digimon, unsuspend this Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-029 Unsuspend this Digimon")
        effect1.set_effect_description("[When Attacking][Once Per Turn] If your opponent has no Digimon with as many or more digivolution cards than this Digimon, unsuspend this Digimon.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Unsuspend_BT14_029")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Conditions extracted from DCGO source:
            # Check: card is on battle area
            # card.permanent_of_this_card() is not None
            return True  # TODO: implement condition checks against game state

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
